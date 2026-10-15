//! yalt: a desktop translator for OpenAI-compatible Chat Completions and
//! Responses APIs.
//!
//! Business logic lives in [`translation`] and [`settings`], which do not
//! depend on the GUI. The `ui`, `app`, and `platform` modules connect them to
//! eframe.

pub mod app;
pub mod settings;
pub mod translation;
