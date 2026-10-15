//! Request lifecycle tests.
//!
//! These drive the application against a local HTTP fixture server, so no
//! external provider or credentials are needed. Coordination is deterministic:
//! the fixture signals the test when it has reached a point, and the test
//! waits for the resulting application event rather than sleeping.

use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::mpsc as std_mpsc;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use yalt::app::{App, Outcome, Status};
use yalt::settings::{Settings, SettingsStore};

/// A local HTTP server that serves one scripted response per connection.
struct Fixture {
    port: u16,
    /// Signals raised by the server, in order.
    signals: std_mpsc::Receiver<String>,
    _handle: std::thread::JoinHandle<()>,
}

/// How the fixture should behave for a request.
enum Behaviour {
    /// Stream these SSE chunks, then hold the connection open.
    StreamThenHold(Vec<String>),
    /// Stream these chunks and close.
    Stream(Vec<String>),
    /// Return a buffered JSON body.
    Buffered(String),
    /// Return a non-success status.
    Status(u16, String),
}

impl Fixture {
    /// Start a fixture that serves `behaviour` for every connection.
    fn start(behaviour: Behaviour) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind fixture");
        let port = listener.local_addr().expect("addr").port();
        let (tx, rx) = std_mpsc::channel();
        let behaviour = Arc::new(Mutex::new(behaviour));
        let handle = std::thread::spawn(move || {
            for stream in listener.incoming() {
                let Ok(stream) = stream else { break };
                let behaviour = Arc::clone(&behaviour);
                let tx = tx.clone();
                std::thread::spawn(move || serve(stream, &behaviour, &tx));
            }
        });
        Self {
            port,
            signals: rx,
            _handle: handle,
        }
    }

    fn base_url(&self) -> String {
        format!("http://127.0.0.1:{}/v1", self.port)
    }

    /// Wait for the server to raise a named signal.
    fn wait_for(&self, name: &str) {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            let remaining = deadline.saturating_duration_since(Instant::now());
            assert!(
                !remaining.is_zero(),
                "timed out waiting for signal {name:?}"
            );
            match self.signals.recv_timeout(remaining) {
                Ok(signal) if signal == name => return,
                Ok(_) => continue,
                Err(error) => panic!("waiting for {name:?} failed: {error}"),
            }
        }
    }
}

/// Read one HTTP request and write the scripted response.
fn serve(stream: TcpStream, behaviour: &Arc<Mutex<Behaviour>>, signals: &std_mpsc::Sender<String>) {
    let mut reader = BufReader::new(stream.try_clone().expect("clone stream"));
    let mut request_line = String::new();
    if reader.read_line(&mut request_line).is_err() {
        return;
    }
    // Drain headers.
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) if line == "\r\n" || line == "\n" => break,
            Ok(_) => {}
            Err(_) => return,
        }
    }
    let mut stream = stream;
    let behaviour = behaviour.lock().expect("behaviour lock");
    match &*behaviour {
        Behaviour::Status(status, body) => {
            let response = format!(
                "HTTP/1.1 {status} Error\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = signals.send("responded".to_owned());
        }
        Behaviour::Buffered(body) => {
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: application/json\r\nConnection: close\r\n\r\n{body}",
                body.len()
            );
            let _ = stream.write_all(response.as_bytes());
            let _ = signals.send("responded".to_owned());
        }
        Behaviour::Stream(chunks) | Behaviour::StreamThenHold(chunks) => {
            let header = "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n";
            let _ = stream.write_all(header.as_bytes());
            for chunk in chunks {
                let framed = format!("{:x}\r\n{chunk}\r\n", chunk.len());
                if stream.write_all(framed.as_bytes()).is_err() {
                    return;
                }
                let _ = stream.flush();
                let _ = signals.send("chunk".to_owned());
                std::thread::sleep(Duration::from_millis(20));
            }
            let _ = signals.send("streamed".to_owned());
            if matches!(&*behaviour, Behaviour::StreamThenHold(_)) {
                // Keep the connection open so the request cannot complete.
                let _ = signals.send("holding".to_owned());
                std::thread::sleep(Duration::from_secs(10));
            } else {
                let _ = stream.write_all(b"0\r\n\r\n");
                let _ = signals.send("closed".to_owned());
            }
        }
    }
}

/// Build an app pointed at a fixture, with streaming configured as requested.
fn app_for(fixture: &Fixture, stream: bool) -> (tempfile::TempDir, App) {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = SettingsStore::at(dir.path().join("settings.json"));
    let settings = Settings {
        api_base: fixture.base_url(),
        stream,
        timeout_seconds: 30,
        ..Settings::default()
    };
    store.save(&settings).expect("save settings");
    let mut app = App::with_store(store).expect("app");
    app.set_waker(Arc::new(|| {}));
    (dir, app)
}

/// Drain events until the request reaches a terminal state.
fn wait_for_finish(app: &mut App, timeout: Duration) -> Outcome {
    let deadline = Instant::now() + timeout;
    loop {
        for event in app.poll_events() {
            if let yalt::app::AppEvent::Finished { outcome, .. } = event {
                return outcome;
            }
        }
        assert!(
            Instant::now() < deadline,
            "request did not finish: {:?}",
            app.state()
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Chat Completions chunks carrying `content`, followed by `[DONE]`.
fn chat_chunks(parts: &[&str]) -> Vec<String> {
    let mut chunks: Vec<String> = parts
        .iter()
        .map(|part| {
            format!(
                "data: {{\"choices\":[{{\"delta\":{{\"content\":{}}}}}]}}\n\n",
                json_string(part)
            )
        })
        .collect();
    chunks.push("data: [DONE]\n\n".to_owned());
    chunks
}

/// Encode a string as a JSON literal.
fn json_string(value: &str) -> String {
    serde_json::to_string(value).expect("encode string")
}

#[test]
fn a_streamed_request_completes_and_reports_its_text() {
    let fixture = Fixture::start(Behaviour::Stream(chat_chunks(&["你", "好"])));
    let (_dir, mut app) = app_for(&fixture, true);
    app.start_translation("Hello", "en", "zh").expect("start");
    let outcome = wait_for_finish(&mut app, Duration::from_secs(5));
    assert_eq!(
        outcome,
        Outcome::Complete {
            text: "你好".into()
        }
    );
    assert_eq!(app.state().status, Status::Complete);
    assert_eq!(app.state().output, "你好");
    assert!(app.state().can_copy());
    assert!(!app.state().is_incomplete());
}

#[test]
fn a_buffered_request_completes_and_reports_its_text() {
    let body = r#"{"choices":[{"message":{"content":"你好"},"finish_reason":"stop"}]}"#;
    let fixture = Fixture::start(Behaviour::Buffered(body.to_owned()));
    let (_dir, mut app) = app_for(&fixture, false);
    app.start_translation("Hello", "en", "zh").expect("start");
    let outcome = wait_for_finish(&mut app, Duration::from_secs(5));
    assert_eq!(
        outcome,
        Outcome::Complete {
            text: "你好".into()
        }
    );
    assert_eq!(app.state().output, "你好");
}

#[test]
fn stopping_preserves_partial_output() {
    // The fixture streams one chunk and then holds the connection open.
    let fixture = Fixture::start(Behaviour::StreamThenHold(chat_chunks(&["部分"])));
    let (_dir, mut app) = app_for(&fixture, true);
    app.start_translation("Hello", "en", "zh").expect("start");

    // Wait until the partial text has been received by the application.
    let deadline = Instant::now() + Duration::from_secs(5);
    while app.state().output.is_empty() {
        app.poll_events();
        assert!(Instant::now() < deadline, "partial text never arrived");
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(app.state().status, Status::Receiving);

    app.stop_translation();
    assert_eq!(app.state().status, Status::Stopping);
    let outcome = wait_for_finish(&mut app, Duration::from_secs(5));
    assert!(
        matches!(&outcome, Outcome::Incomplete { text, .. } if text == "部分"),
        "unexpected outcome: {outcome:?}"
    );
    assert_eq!(app.state().status, Status::Stopped);
    assert_eq!(app.state().output, "部分");
    // Partial output stays available and is labelled incomplete.
    assert!(app.state().can_copy());
    assert!(app.state().is_incomplete());
}

#[test]
fn a_provider_error_after_partial_output_is_incomplete() {
    let mut chunks = chat_chunks(&["部分"]);
    // Replace the terminating event with a provider error.
    chunks.pop();
    chunks.push("data: {\"error\":{\"message\":\"rate limited\"}}\n\n".to_owned());
    let fixture = Fixture::start(Behaviour::Stream(chunks));
    let (_dir, mut app) = app_for(&fixture, true);
    app.start_translation("Hello", "en", "zh").expect("start");
    let outcome = wait_for_finish(&mut app, Duration::from_secs(5));
    match outcome {
        Outcome::Incomplete { text, message } => {
            assert_eq!(text, "部分");
            assert!(message.contains("rate limited"), "{message}");
        }
        other => panic!("expected an incomplete result, got {other:?}"),
    }
    assert!(app.state().is_incomplete());
}

#[test]
fn a_failure_without_output_is_reported_as_failed() {
    let fixture = Fixture::start(Behaviour::Status(
        429,
        r#"{"error":"slow down"}"#.to_owned(),
    ));
    let (_dir, mut app) = app_for(&fixture, false);
    app.start_translation("Hello", "en", "zh").expect("start");
    let outcome = wait_for_finish(&mut app, Duration::from_secs(5));
    match outcome {
        Outcome::Failed { message } => assert!(message.contains("429"), "{message}"),
        other => panic!("expected a failure, got {other:?}"),
    }
    assert_eq!(app.state().status, Status::Failed);
    assert!(!app.state().can_copy());
}

#[test]
fn empty_input_is_rejected_without_starting_a_request() {
    let fixture = Fixture::start(Behaviour::Buffered("{}".to_owned()));
    let (_dir, mut app) = app_for(&fixture, false);
    assert!(app.start_translation("   ", "en", "zh").is_err());
    assert_eq!(app.state().status, Status::Ready);
}

#[test]
fn a_second_request_is_rejected_while_one_is_active() {
    let fixture = Fixture::start(Behaviour::StreamThenHold(chat_chunks(&["a"])));
    let (_dir, mut app) = app_for(&fixture, true);
    app.start_translation("Hello", "en", "zh").expect("first");
    assert!(app.start_translation("Again", "en", "zh").is_err());
    app.stop_translation();
    wait_for_finish(&mut app, Duration::from_secs(5));
}

#[test]
fn the_request_snapshots_its_settings_and_languages() {
    let fixture = Fixture::start(Behaviour::Stream(chat_chunks(&["ok"])));
    let (_dir, mut app) = app_for(&fixture, true);
    app.start_translation("Hello", "en", "de").expect("start");
    // The resolved target is recorded with the request.
    assert_eq!(app.state().target, "de");
    assert_eq!(app.state().source, "en");
    wait_for_finish(&mut app, Duration::from_secs(5));
}

#[test]
fn an_automatic_target_resolves_once_at_start() {
    let fixture = Fixture::start(Behaviour::Stream(chat_chunks(&["ok"])));
    let (_dir, mut app) = app_for(&fixture, true);
    // A manual Chinese source with an automatic target selects English.
    app.start_translation("你好", "zh", "auto").expect("start");
    assert_eq!(app.state().target, "en");
    wait_for_finish(&mut app, Duration::from_secs(5));
}

#[test]
fn reset_clears_a_finished_result() {
    let fixture = Fixture::start(Behaviour::Stream(chat_chunks(&["ok"])));
    let (_dir, mut app) = app_for(&fixture, true);
    app.start_translation("Hello", "en", "zh").expect("start");
    wait_for_finish(&mut app, Duration::from_secs(5));
    assert!(app.state().can_copy());
    app.reset_result();
    assert_eq!(app.state().status, Status::Ready);
    assert!(!app.state().can_copy());
}

#[test]
fn shutdown_cancels_active_work_within_its_deadline() {
    let fixture = Fixture::start(Behaviour::StreamThenHold(chat_chunks(&["a"])));
    let (_dir, mut app) = app_for(&fixture, true);
    app.start_translation("Hello", "en", "zh").expect("start");
    fixture.wait_for("streamed");
    let started = Instant::now();
    app.shutdown();
    assert!(
        started.elapsed() < Duration::from_secs(6),
        "shutdown exceeded its deadline"
    );
    // New work is refused once shutting down.
    assert!(app.start_translation("Hello", "en", "zh").is_err());
}

#[test]
fn a_stale_detection_result_is_discarded() {
    let fixture = Fixture::start(Behaviour::Buffered("{}".to_owned()));
    let (_dir, mut app) = app_for(&fixture, false);
    app.note_input_changed("The weather is lovely today.");
    // A second edit invalidates the first detection.
    app.note_input_changed("完全不同的一段中文文本内容。");
    // Wait for the debounce to fire and be processed.
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        app.poll_events();
        if app.detection().is_some() {
            break;
        }
        assert!(Instant::now() < deadline, "detection never arrived");
        std::thread::sleep(Duration::from_millis(10));
    }
    // Only the newest revision is applied.
    assert_eq!(app.detection().map(|d| d.code.as_str()), Some("zh"));
}

#[test]
fn detection_does_not_run_for_empty_input() {
    let fixture = Fixture::start(Behaviour::Buffered("{}".to_owned()));
    let (_dir, mut app) = app_for(&fixture, false);
    app.note_input_changed("   ");
    std::thread::sleep(Duration::from_millis(400));
    app.poll_events();
    assert!(app.detection().is_none());
}
