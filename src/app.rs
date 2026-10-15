//! Application coordination: request lifecycle, worker events, and detection.
//!
//! This module owns the boundary between the UI and background work. It does
//! not depend on egui: the UI renders the state exposed here and calls the
//! actions, and a waker callback tells it when new events have arrived.

use std::sync::Arc;
use std::time::Duration;

use tokio::runtime::Runtime;
use tokio::sync::mpsc;

use crate::settings::{LoadOutcome, Settings, SettingsStore, StoreError};
use crate::translation::client::Client;
use crate::translation::languages::{self, Detection};
use crate::translation::{Cancel, ClientError, Request};

/// Delay after the last edit before language detection runs.
const DETECTION_DEBOUNCE: Duration = Duration::from_millis(250);

/// Events in flight between workers and the UI.
///
/// Fragments are coalesced by draining this queue once per frame, so the
/// capacity also bounds how much text a single frame can append.
const EVENT_QUEUE_CAPACITY: usize = 256;

/// Deadline for workers to stop during shutdown.
const SHUTDOWN_DEADLINE: Duration = Duration::from_secs(5);

/// Lifecycle state of the translation view.
///
/// These are mutually exclusive states, not independent flags, so a request
/// cannot be simultaneously stopping and complete.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    /// Nothing has been requested yet, or the result was reset.
    Ready,
    /// A request was accepted and work is starting.
    Waiting,
    /// Text is arriving.
    Receiving,
    /// The request finished successfully.
    Complete,
    /// Stop was requested and the worker is winding down.
    Stopping,
    /// The request was stopped, or a failure left a partial result.
    Stopped,
    /// The request failed without producing output.
    Failed,
}

impl Status {
    /// Whether a request is currently in flight.
    pub fn is_active(self) -> bool {
        matches!(self, Self::Waiting | Self::Receiving | Self::Stopping)
    }
}

/// How a request ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    /// The provider signalled completion.
    Complete {
        /// Full visible text.
        text: String,
    },
    /// The request was stopped, or failed after producing partial text.
    Incomplete {
        /// Visible text produced before the request ended.
        text: String,
        /// Why the request ended.
        message: String,
    },
    /// The request failed before producing any text.
    Failed {
        /// Why the request failed.
        message: String,
    },
}

/// An event sent from a worker to the UI.
#[derive(Debug, Clone, PartialEq)]
pub enum AppEvent {
    /// Visible text arrived for a request.
    Delta {
        /// Request identity.
        id: u64,
        /// Newly visible text.
        text: String,
    },
    /// A request reached a terminal state. Sent exactly once per request.
    Finished {
        /// Request identity.
        id: u64,
        /// How the request ended.
        outcome: Outcome,
    },
    /// A debounced detection completed.
    Detected {
        /// Input revision this result belongs to.
        revision: u64,
        /// The detection result.
        detection: Detection,
    },
}

/// Why a translation could not be started.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartError {
    /// The input was empty or only whitespace.
    EmptyInput,
    /// A request is already running.
    Busy,
    /// The application is shutting down.
    ShuttingDown,
}

impl std::fmt::Display for StartError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyInput => f.write_str("enter some text to translate"),
            Self::Busy => f.write_str("a translation is already running"),
            Self::ShuttingDown => f.write_str("the application is shutting down"),
        }
    }
}

impl std::error::Error for StartError {}

/// State of the translation view, rendered by the UI.
#[derive(Debug, Clone, PartialEq)]
pub struct RequestState {
    /// Current lifecycle state.
    pub status: Status,
    /// Visible output accumulated so far.
    pub output: String,
    /// Status or error text for the current state.
    pub message: String,
    /// Identity of the active or most recent request.
    pub request_id: Option<u64>,
    /// Source language used by the current request.
    pub source: String,
    /// Target language used by the current request.
    pub target: String,
}

impl Default for RequestState {
    fn default() -> Self {
        Self {
            status: Status::Ready,
            output: String::new(),
            message: String::new(),
            request_id: None,
            source: String::new(),
            target: String::new(),
        }
    }
}

impl RequestState {
    /// Whether the current result is partial and should be labelled incomplete.
    pub fn is_incomplete(&self) -> bool {
        self.status == Status::Stopped && !self.output.is_empty()
    }

    /// Whether Copy is available.
    pub fn can_copy(&self) -> bool {
        !self.output.is_empty()
    }
}

/// A running request, tracked so it can be stopped.
struct Active {
    cancel: Cancel,
}

/// Application state and background coordination.
pub struct App {
    store: SettingsStore,
    settings: Settings,
    /// Taken on shutdown, since stopping the runtime consumes it.
    runtime: Option<Runtime>,
    client: Client,
    state: RequestState,
    active: Option<Active>,
    next_request: u64,
    events_tx: mpsc::Sender<AppEvent>,
    events_rx: mpsc::Receiver<AppEvent>,
    waker: Option<Arc<dyn Fn() + Send + Sync>>,
    detection_revision: u64,
    detection: Option<Detection>,
    recoverable: Option<StoreError>,
    shutting_down: bool,
}

impl App {
    /// Create the application, loading saved settings.
    ///
    /// A missing file yields fresh defaults. An unreadable file is reported
    /// through [`App::recoverable_error`] and left untouched.
    pub fn new() -> Result<Self, StoreError> {
        let store = SettingsStore::new()?;
        Self::with_store(store)
    }

    /// Create the application around an explicit settings store.
    pub fn with_store(store: SettingsStore) -> Result<Self, StoreError> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .map_err(StoreError::Io)?;
        let client = Client::new()
            .map_err(|error| StoreError::Io(std::io::Error::other(error.to_string())))?;
        let (settings, recoverable) = match store.load() {
            LoadOutcome::Loaded(settings) | LoadOutcome::Fresh(settings) => (settings, None),
            LoadOutcome::Recoverable { settings, error } => (settings, Some(error)),
        };
        let (events_tx, events_rx) = mpsc::channel(EVENT_QUEUE_CAPACITY);
        Ok(Self {
            store,
            settings,
            runtime: Some(runtime),
            client,
            state: RequestState::default(),
            active: None,
            next_request: 0,
            events_tx,
            events_rx,
            waker: None,
            detection_revision: 0,
            detection: None,
            recoverable,
            shutting_down: false,
        })
    }

    /// Register the callback used to wake the UI when an event arrives.
    pub fn set_waker(&mut self, waker: Arc<dyn Fn() + Send + Sync>) {
        self.waker = Some(waker);
    }

    /// Current saved settings.
    pub fn settings(&self) -> &Settings {
        &self.settings
    }

    /// The error from an unreadable settings file, if any.
    pub fn recoverable_error(&self) -> Option<&StoreError> {
        self.recoverable.as_ref()
    }

    /// Path of the settings file.
    pub fn settings_path(&self) -> std::path::PathBuf {
        self.store.path().to_path_buf()
    }

    /// Current translation view state.
    pub fn state(&self) -> &RequestState {
        &self.state
    }

    /// The most recent detection result, if any.
    pub fn detection(&self) -> Option<&Detection> {
        self.detection.as_ref()
    }

    /// Validate and persist new settings.
    ///
    /// On success the new settings apply to future requests. On failure the
    /// previous settings stay in effect.
    pub fn save_settings(&mut self, settings: Settings) -> Result<(), StoreError> {
        self.store.save(&settings)?;
        self.settings = settings;
        self.recoverable = None;
        Ok(())
    }

    /// Start a translation from the current input and language selections.
    ///
    /// The request snapshots the text, language decisions, and settings, so
    /// later edits cannot change it.
    pub fn start_translation(
        &mut self,
        text: &str,
        source: &str,
        target: &str,
    ) -> Result<u64, StartError> {
        if self.shutting_down {
            return Err(StartError::ShuttingDown);
        }
        if self.state.status.is_active() {
            return Err(StartError::Busy);
        }
        if text.trim().is_empty() {
            return Err(StartError::EmptyInput);
        }

        // Resolve the language decisions once, from a detection snapshot.
        let detection = self
            .detection
            .clone()
            .filter(|_| source.eq_ignore_ascii_case("auto"))
            .unwrap_or_else(|| languages::detect(text));
        let source = if source.eq_ignore_ascii_case("auto") {
            if detection.uncertain {
                "auto".to_owned()
            } else {
                detection.code.clone()
            }
        } else {
            source.to_owned()
        };
        let target = languages::select_target(&source, target, &detection);

        let request = Request {
            api_base: self.settings.api_base.clone(),
            api_method: self.settings.api_method,
            api_key: self.settings.api_key.clone(),
            model: self.settings.model.clone(),
            text: text.to_owned(),
            source: source.clone(),
            target: target.clone(),
            instruction: self.settings.instruction.clone(),
            glossary: self.settings.glossary.clone(),
            max_tokens: self.settings.max_tokens,
            temperature: self.settings.temperature,
            stream: self.settings.stream,
            disable_think: self.settings.disable_thinking,
            user_agent: self.settings.user_agent.clone(),
        };
        let deadline = self.settings.deadline();

        self.next_request += 1;
        let id = self.next_request;
        let cancel = Cancel::new();
        self.active = Some(Active {
            cancel: cancel.clone(),
        });
        self.state = RequestState {
            status: Status::Waiting,
            output: String::new(),
            message: "Waiting for the provider".to_owned(),
            request_id: Some(id),
            source,
            target,
        };

        let client = self.client.clone();
        let events = self.events_tx.clone();
        let waker = self.waker.clone();
        self.runtime().spawn(async move {
            run_request(client, request, id, cancel, deadline, events, waker).await;
        });
        Ok(id)
    }

    /// The async runtime, which is absent only after shutdown.
    fn runtime(&self) -> &Runtime {
        self.runtime
            .as_ref()
            .expect("runtime is available until shutdown")
    }

    /// Request that the active translation stop.
    ///
    /// The worker reports the terminal state, so the result stays consistent.
    pub fn stop_translation(&mut self) {
        if let Some(active) = &self.active
            && self.state.status.is_active()
            && self.state.status != Status::Stopping
        {
            self.state.status = Status::Stopping;
            self.state.message = "Stopping".to_owned();
            active.cancel.cancel();
        }
    }

    /// Note that the input changed, scheduling a debounced detection.
    ///
    /// Each edit invalidates earlier detections, so a stale result can never
    /// replace a newer one.
    pub fn note_input_changed(&mut self, text: &str) {
        self.detection_revision += 1;
        let revision = self.detection_revision;
        if text.trim().is_empty() {
            self.detection = None;
            return;
        }
        let events = self.events_tx.clone();
        let waker = self.waker.clone();
        let text = text.to_owned();
        self.runtime().spawn(async move {
            tokio::time::sleep(DETECTION_DEBOUNCE).await;
            let detection = tokio::task::spawn_blocking(move || languages::detect(&text))
                .await
                .unwrap_or_else(|_| languages::detect(""));
            if events
                .send(AppEvent::Detected {
                    revision,
                    detection,
                })
                .await
                .is_ok()
                && let Some(waker) = waker
            {
                waker();
            }
        });
    }

    /// Drain pending worker events without blocking.
    ///
    /// Returns the events applied this call. Stale events are discarded.
    pub fn poll_events(&mut self) -> Vec<AppEvent> {
        let mut applied = Vec::new();
        while let Ok(event) = self.events_rx.try_recv() {
            match &event {
                AppEvent::Detected { revision, .. } => {
                    if *revision != self.detection_revision {
                        continue;
                    }
                }
                AppEvent::Delta { id, .. } | AppEvent::Finished { id, .. } => {
                    if Some(*id) != self.state.request_id {
                        continue;
                    }
                }
            }
            self.apply(&event);
            applied.push(event);
        }
        applied
    }

    /// Apply one event to the view state.
    fn apply(&mut self, event: &AppEvent) {
        match event {
            AppEvent::Detected { detection, .. } => {
                self.detection = Some(detection.clone());
            }
            AppEvent::Delta { text, .. } => {
                if self.state.status == Status::Waiting {
                    self.state.status = Status::Receiving;
                    self.state.message = "Receiving translation".to_owned();
                }
                self.state.output.push_str(text);
            }
            AppEvent::Finished { outcome, .. } => {
                self.active = None;
                match outcome {
                    Outcome::Complete { text } => {
                        self.state.output = text.clone();
                        self.state.status = Status::Complete;
                        self.state.message = "Translation complete".to_owned();
                    }
                    Outcome::Incomplete { text, message } => {
                        self.state.output = text.clone();
                        self.state.status = Status::Stopped;
                        self.state.message = message.clone();
                    }
                    Outcome::Failed { message } => {
                        self.state.status = Status::Failed;
                        self.state.message = message.clone();
                    }
                }
            }
        }
    }

    /// Reset the view to Ready, discarding the current result.
    pub fn reset_result(&mut self) {
        if self.state.status.is_active() {
            return;
        }
        self.state.output.clear();
        self.state.message.clear();
        self.state.status = Status::Ready;
        self.state.request_id = None;
    }

    /// Stop accepting work, cancel active work, and wait for workers to finish.
    ///
    /// Bounded by [`SHUTDOWN_DEADLINE`] so exit cannot hang.
    pub fn shutdown(&mut self) {
        self.shutting_down = true;
        if let Some(active) = self.active.take() {
            active.cancel.cancel();
        }
        // Stopping the runtime waits for spawned work, bounded so exit cannot
        // hang on a stalled connection.
        if let Some(runtime) = self.runtime.take() {
            runtime.shutdown_timeout(SHUTDOWN_DEADLINE);
        }
    }
}

/// Run one translation request and report its terminal state exactly once.
async fn run_request(
    client: Client,
    request: Request,
    id: u64,
    cancel: Cancel,
    deadline: Duration,
    events: mpsc::Sender<AppEvent>,
    waker: Option<Arc<dyn Fn() + Send + Sync>>,
) {
    let mut full = String::new();

    // The total deadline covers connection setup, headers, and streaming. The
    // request future is scoped so its borrow of the accumulator ends before the
    // outcome is assembled from it.
    let bounded = {
        let work = client.translate(&request, &cancel, |delta| {
            full.push_str(delta);
            // Emit each fragment immediately so text appears as it arrives.
            // Events are drained once per frame, which coalesces them into a
            // single append. A full queue drops the fragment rather than
            // blocking the worker; nothing is lost because the terminal event
            // carries the complete text.
            if events
                .try_send(AppEvent::Delta {
                    id,
                    text: delta.to_owned(),
                })
                .is_ok()
                && let Some(waker) = &waker
            {
                waker();
            }
        });
        tokio::select! {
            biased;
            () = cancel.cancelled() => None,
            result = tokio::time::timeout(deadline, work) => Some(result),
        }
    };

    let outcome = match bounded {
        None => Outcome::Incomplete {
            text: full.clone(),
            message: "Translation stopped".to_owned(),
        },
        Some(Err(_elapsed)) => Outcome::Incomplete {
            text: full.clone(),
            message: "Request deadline exceeded".to_owned(),
        },
        Some(Ok(Ok(result))) => Outcome::Complete { text: result.text },
        Some(Ok(Err(error))) => {
            let message = describe(&error);
            if full.trim().is_empty() {
                Outcome::Failed { message }
            } else {
                Outcome::Incomplete {
                    text: full.clone(),
                    message,
                }
            }
        }
    };

    // The terminal event is delivered even when the request was cancelled, so
    // the view always leaves its active state. Awaiting is safe: the queue is
    // drained every frame, and a dropped receiver returns immediately.
    let _ = events.send(AppEvent::Finished { id, outcome }).await;
    if let Some(waker) = waker {
        waker();
    }
}

/// Human-readable message for a client failure.
fn describe(error: &ClientError) -> String {
    match error {
        ClientError::Cancelled => "Translation stopped".to_owned(),
        ClientError::Deadline => "Request deadline exceeded".to_owned(),
        other => other.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn statuses_report_activity_consistently() {
        assert!(!Status::Ready.is_active());
        assert!(Status::Waiting.is_active());
        assert!(Status::Receiving.is_active());
        assert!(Status::Stopping.is_active());
        assert!(!Status::Complete.is_active());
        assert!(!Status::Stopped.is_active());
        assert!(!Status::Failed.is_active());
    }

    #[test]
    fn a_stopped_result_with_text_is_incomplete() {
        let state = RequestState {
            status: Status::Stopped,
            output: "partial".into(),
            ..RequestState::default()
        };
        assert!(state.is_incomplete());
        assert!(state.can_copy());
    }

    #[test]
    fn a_complete_result_is_not_incomplete() {
        let state = RequestState {
            status: Status::Complete,
            output: "done".into(),
            ..RequestState::default()
        };
        assert!(!state.is_incomplete());
    }

    #[test]
    fn an_empty_result_cannot_be_copied() {
        assert!(!RequestState::default().can_copy());
    }

    #[test]
    fn a_cancelled_request_is_reported_as_stopped() {
        assert_eq!(describe(&ClientError::Cancelled), "Translation stopped");
        assert_eq!(
            describe(&ClientError::Deadline),
            "Request deadline exceeded"
        );
    }
}
