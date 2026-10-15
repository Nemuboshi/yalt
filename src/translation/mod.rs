//! Translation core: request types, HTTP transport, streaming, prompts, and
//! language handling.
//!
//! This module is independent of the GUI: it must not import egui.

pub mod client;
pub mod languages;
pub mod prompt;
pub mod stream;
pub mod think;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::Notify;

pub use client::{ApiMethod, Client, ClientError, Request, TranslationResult};
pub use languages::{Detection, LANGUAGES, Language};
pub use prompt::{Glossary, GlossaryError};
pub use stream::{SseDecoder, StreamError, StreamFormat, StreamResult};
pub use think::ThinkFilter;

/// A cancellation signal shared between the UI and a worker.
///
/// Cheap to clone; every clone observes the same state. Cancellation is
/// immediate and idempotent, and waiting is race-free: a signal raised before
/// or during a wait always ends it.
#[derive(Debug, Clone, Default)]
pub struct Cancel {
    flag: Arc<AtomicBool>,
    notify: Arc<Notify>,
}

impl Cancel {
    /// Create an uncancelled signal.
    pub fn new() -> Self {
        Self::default()
    }

    /// Raise the signal, waking every waiter.
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    /// Whether the signal has been raised.
    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }

    /// Wait until the signal is raised, returning immediately if it already is.
    pub async fn cancelled(&self) {
        loop {
            if self.is_cancelled() {
                return;
            }
            // Register interest before re-checking so a signal raised between
            // the check and the wait cannot be missed.
            let waiter = self.notify.notified();
            if self.is_cancelled() {
                return;
            }
            waiter.await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cancellation_is_observable_and_idempotent() {
        let cancel = Cancel::new();
        assert!(!cancel.is_cancelled());
        cancel.cancel();
        cancel.cancel();
        assert!(cancel.is_cancelled());
        cancel.cancelled().await;
    }

    #[tokio::test]
    async fn waiting_ends_when_cancelled() {
        let cancel = Cancel::new();
        let waiter = cancel.clone();
        let handle = tokio::spawn(async move { waiter.cancelled().await });
        cancel.cancel();
        tokio::time::timeout(std::time::Duration::from_secs(1), handle)
            .await
            .expect("waiter did not wake")
            .expect("waiter panicked");
    }

    #[tokio::test]
    async fn a_signal_raised_before_waiting_is_not_missed() {
        let cancel = Cancel::new();
        cancel.cancel();
        tokio::time::timeout(std::time::Duration::from_secs(1), cancel.cancelled())
            .await
            .expect("already-cancelled signal did not return");
    }
}
