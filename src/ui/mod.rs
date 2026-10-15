//! The eframe application: view composition and action handling.
//!
//! The UI renders state from [`crate::app`] and turns user input into actions.
//! It performs no blocking work and never talks to the network directly.

pub mod draft;
pub mod fonts;
pub mod settings;
pub mod theme;
pub mod translate;

use std::sync::Arc;

use crate::app::{App, StartError, Status};
use crate::settings::Settings;
use crate::tray::{self, Command as TrayCommand};

/// Which view is showing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum View {
    /// The translation view.
    Translate,
    /// The settings view.
    Settings,
}

/// The eframe application.
pub struct YaltApp {
    app: App,
    view: View,
    translate: translate::TranslateView,
    settings: settings::SettingsView,
    /// A message shown after leaving the settings view.
    notice: Option<String>,
    /// Copy feedback shown in the status bar.
    copy_feedback: Option<CopyFeedback>,
    /// The system tray, when one is enabled and registered.
    tray: tray::Tray,
    /// Commands raised by the tray menu.
    tray_commands: std::sync::mpsc::Receiver<TrayCommand>,
    /// Set once the user has asked to quit, so close-to-tray stops applying.
    quitting: bool,
    /// Whether the window is currently hidden in the tray.
    hidden: bool,
    /// Whether the start-hidden state has been re-asserted after the first
    /// paint, which eframe uses to reveal a freshly created window.
    reasserted: bool,
}

/// Feedback shown in the status bar after a copy attempt.
#[derive(Debug, Clone, PartialEq, Eq)]
struct CopyFeedback {
    /// Text to show.
    message: String,
    /// Whether the copy failed.
    failed: bool,
}

impl YaltApp {
    /// Create the application, loading settings and preparing the theme.
    ///
    /// `start_hidden` records whether the window was created hidden. If the
    /// tray then fails to register the window is revealed on the first frame,
    /// because hiding it would leave no way back.
    pub fn new(
        ctx: &egui::Context,
        start_hidden: bool,
    ) -> Result<Self, crate::settings::StoreError> {
        // Fonts are installed once, before the first frame.
        ctx.set_fonts(fonts::definitions());
        theme::apply(ctx);
        let mut app = App::new()?;
        // Wake the UI whenever a worker event arrives, so repaints happen only
        // when there is something new to show.
        let repaint = ctx.clone();
        let waker = Arc::new(move || repaint.request_repaint());
        app.set_waker(waker.clone());

        let settings = app.settings().clone();
        // The tray is registered here, in the constructor, because eframe runs
        // this on the main thread with its event loop already up.
        let (sender, tray_commands) = tray::command_channel();
        let tray = tray::Tray::new(
            settings.tray_enabled,
            tray::load_icon(),
            sender,
            tray::Waker::new(waker),
        );

        Ok(Self {
            translate: translate::TranslateView::new(&settings.source, &settings.target),
            settings: settings::SettingsView::default(),
            app,
            view: View::Translate,
            notice: None,
            copy_feedback: None,
            tray,
            tray_commands,
            quitting: false,
            hidden: start_hidden,
            reasserted: false,
        })
    }

    /// Whether the tray is registered, for tests.
    pub fn tray_registered(&self) -> bool {
        self.tray.is_registered()
    }

    /// Whether the window is hidden in the tray, for tests.
    pub fn is_hidden(&self) -> bool {
        self.hidden
    }

    /// Apply the desktop rules that need the viewport: start-hidden,
    /// close-to-tray, and the tray's own commands.
    ///
    /// This runs from `logic` rather than `ui`, because eframe runs no `ui`
    /// pass at all while the window is hidden. Tray commands would otherwise
    /// never be drained in exactly the state where they are the only way back,
    /// and close-to-tray could not be applied to the pass that would otherwise
    /// close the window.
    ///
    /// Close-to-tray is read from the saved settings every pass rather than
    /// captured at startup, so a change to it takes effect immediately.
    fn desktop(&mut self, ctx: &egui::Context) {
        let tray_registered = self.tray.is_registered();
        for command in self.drain_tray_commands() {
            match command {
                TrayCommand::Open => {
                    self.show(ctx);
                    self.switch_to(View::Translate);
                }
                TrayCommand::Settings => {
                    self.show(ctx);
                    self.switch_to(View::Settings);
                }
                TrayCommand::Quit => {
                    self.quit(ctx);
                    return;
                }
            }
        }

        let action = window_action(
            self.hidden,
            self.reasserted,
            ctx.input(|input| input.viewport().close_requested()),
            // An explicit Quit bypasses hide-on-close.
            self.app.settings().close_to_tray && !self.quitting,
            tray_registered,
        );
        match action {
            WindowAction::None => {}
            WindowAction::Hide => {
                ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                self.hide(ctx);
                self.reasserted = true;
            }
            WindowAction::Show => self.show(ctx),
        }
    }

    /// Collect tray commands without blocking the frame.
    fn drain_tray_commands(&self) -> Vec<TrayCommand> {
        let mut commands = Vec::new();
        while let Ok(command) = self.tray_commands.try_recv() {
            commands.push(command);
        }
        commands
    }

    /// Hide the window to the tray.
    fn hide(&mut self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(false));
        self.hidden = true;
    }

    /// Show and focus the window, restoring it if it was minimised.
    fn show(&mut self, ctx: &egui::Context) {
        ctx.send_viewport_cmd(egui::ViewportCommand::Visible(true));
        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
        ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        self.hidden = false;
    }

    /// Quit the application, cancelling any active work.
    ///
    /// Quit bypasses close-to-tray, so the window is allowed to close and the
    /// request deadline does not hold up the exit.
    fn quit(&mut self, ctx: &egui::Context) {
        self.quitting = true;
        self.app.shutdown();
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    /// Access the application state, for tests.
    pub fn app(&self) -> &App {
        &self.app
    }

    /// Access the application state mutably, for tests.
    pub fn app_mut(&mut self) -> &mut App {
        &mut self.app
    }

    /// The current view, for tests.
    pub fn view(&self) -> View {
        self.view
    }

    /// The translation view state, for tests.
    pub fn translate(&self) -> &translate::TranslateView {
        &self.translate
    }

    /// The translation view state mutably, for tests.
    pub fn translate_mut(&mut self) -> &mut translate::TranslateView {
        &mut self.translate
    }

    /// Switch views, loading or discarding the settings draft as appropriate.
    fn switch_to(&mut self, view: View) {
        match view {
            View::Settings => {
                let settings = self.app.settings().clone();
                self.settings.open(&settings);
            }
            View::Translate => self.settings.close(),
        }
        self.view = view;
    }

    /// Handle an action from the translation view.
    fn handle_translate(&mut self, action: translate::Action, ctx: &egui::Context) {
        match action {
            translate::Action::Translate => {
                let source = self.translate.resolved_source(&self.app);
                let target = self.translate.target.clone();
                let text = self.translate.input.clone();
                if let Err(error) = self.app.start_translation(&text, &source, &target) {
                    self.notice = Some(start_error(&error));
                } else {
                    self.notice = None;
                }
                ctx.request_repaint();
            }
            translate::Action::Stop => {
                self.app.stop_translation();
                ctx.request_repaint();
            }
            translate::Action::Clear => {
                self.translate.input.clear();
                self.app.reset_result();
                self.copy_feedback = None;
                self.app.note_input_changed("");
            }
            translate::Action::Copy => {
                let text = self.app.state().output.clone();
                ctx.copy_text(text);
                // Copy feedback belongs in the status bar, not in the result
                // card, so the layout stays stable.
                self.copy_feedback = Some(CopyFeedback {
                    message: "Copied to clipboard".to_owned(),
                    failed: false,
                });
            }
            translate::Action::OpenSettings => self.switch_to(View::Settings),
        }
    }

    /// Handle an action from the settings view.
    fn handle_settings(&mut self, action: settings::Action) {
        match action {
            settings::Action::Save => {
                let Some(draft) = self.settings.draft.as_ref() else {
                    return;
                };
                match draft.to_settings() {
                    Ok(settings) => match self.app.save_settings(settings.clone()) {
                        Ok(()) => {
                            // Saved changes apply to future requests.
                            self.translate.source = settings.source.clone();
                            self.translate.target = settings.target.clone();
                            self.settings.close();
                            self.view = View::Translate;
                            self.notice = Some(restart_notice(&settings));
                        }
                        Err(error) => {
                            if let Some(draft) = self.settings.draft.as_mut() {
                                draft.feedback = error.to_string();
                                draft.feedback_is_error = true;
                            }
                        }
                    },
                    Err(message) => {
                        // A failed save preserves the draft for correction.
                        if let Some(draft) = self.settings.draft.as_mut() {
                            draft.feedback = message;
                            draft.feedback_is_error = true;
                        }
                    }
                }
            }
            settings::Action::Cancel => {
                self.settings.close();
                self.view = View::Translate;
            }
        }
    }

    /// Draw the toolbar.
    fn toolbar(&mut self, ui: &mut egui::Ui, colors: &theme::Palette) {
        ui.horizontal(|ui| {
            ui.add_space(SPACE_TOOLBAR);
            for (view, label) in [(View::Translate, "Translate"), (View::Settings, "Settings")] {
                let selected = self.view == view;
                let text = RichText::new(label).color(if selected {
                    colors.label
                } else {
                    colors.secondary
                });
                let fill = if selected {
                    colors.control
                } else {
                    colors.window
                };
                let button = egui::Button::new(text)
                    .fill(fill)
                    .stroke(egui::Stroke::new(1.0, colors.separator))
                    .corner_radius(theme::RADIUS_CONTROL);
                if ui.add(button).clicked() && !selected {
                    self.switch_to(view);
                }
            }
        });
    }

    /// Draw the notice strip, if any.
    fn notice(&mut self, ui: &mut egui::Ui, colors: &theme::Palette) {
        let Some(notice) = self.notice.clone() else {
            return;
        };
        ui.horizontal(|ui| {
            ui.add_space(theme::SPACE_4);
            ui.label(RichText::new(notice).color(colors.secondary));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Dismiss").clicked() {
                    self.notice = None;
                }
                ui.add_space(theme::SPACE_4);
            });
        });
        ui.add_space(theme::SPACE_2);
    }
}

/// Left inset for the toolbar contents.
const SPACE_TOOLBAR: f32 = theme::SPACE_4;

/// What the window should do this pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowAction {
    /// Leave the window as it is.
    None,
    /// Hide the window, keeping the application running in the tray.
    Hide,
    /// Show and focus the window.
    Show,
}

/// Decide what to do with the window this pass.
///
/// Hiding is only safe while a tray exists to restore the window from, so
/// close-to-tray is ignored and a start-hidden launch is undone when no tray
/// registered. `close_requested` is the window manager asking to close, which
/// is what the title-bar button produces.
///
/// `reasserted` records whether the hidden state has already been re-applied
/// once. eframe shows a window after it has painted it, so a window created
/// hidden has to be hidden again on the first pass; doing it every pass would
/// fight the user's own Show.
fn window_action(
    hidden: bool,
    reasserted: bool,
    close_requested: bool,
    hide_on_close: bool,
    tray_registered: bool,
) -> WindowAction {
    if hidden {
        return if tray_registered {
            // Stay hidden. Re-assert once, in case eframe revealed the window
            // when it first painted it.
            if reasserted {
                WindowAction::None
            } else {
                WindowAction::Hide
            }
        } else {
            // No tray means no way back to a hidden window.
            WindowAction::Show
        };
    }
    if close_requested && hide_on_close && tray_registered {
        WindowAction::Hide
    } else {
        WindowAction::None
    }
}

impl eframe::App for YaltApp {
    /// Runs every pass, including while the window is hidden and no `ui` pass
    /// happens. Tray commands, close-to-tray, and start-hidden all live here so
    /// they keep working in exactly the states where the window is not drawn.
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Apply pending worker events without blocking. This also coalesces
        // every fragment received since the last frame into one append.
        self.app.poll_events();
        self.desktop(ctx);
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let colors = theme::palette_for(ui.ctx().theme());

        egui::Panel::top("toolbar")
            .frame(
                egui::Frame::new()
                    .fill(colors.window)
                    .inner_margin(egui::Margin::symmetric(0, theme::SPACE_2 as i8)),
            )
            .show(ui, |ui| {
                self.toolbar(ui, &colors);
            });
        egui::Panel::bottom("status")
            .frame(
                egui::Frame::new()
                    .fill(colors.window)
                    .inner_margin(egui::Margin::symmetric(
                        theme::SPACE_4 as i8,
                        theme::SPACE_2 as i8,
                    )),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    // Copy feedback shares the status bar so the result card
                    // keeps a stable layout.
                    if let Some(feedback) = &self.copy_feedback {
                        let color = if feedback.failed {
                            colors.destructive
                        } else {
                            colors.success
                        };
                        ui.label(RichText::new(&feedback.message).color(color));
                        ui.separator();
                    }
                    let state = self.app.state();
                    let text = status_summary(state);
                    let color = match state.status {
                        Status::Failed => colors.destructive,
                        Status::Complete => colors.success,
                        Status::Stopped => colors.warning,
                        _ => colors.secondary,
                    };
                    ui.label(RichText::new(text).color(color));
                });
            });

        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(colors.window)
                    .inner_margin(egui::Margin::same(theme::SPACE_4 as i8)),
            )
            .show(ui, |ui| {
                self.notice(ui, &colors);
                match self.view {
                    View::Translate => {
                        if let Some(action) = self.translate.ui(ui, &mut self.app) {
                            self.handle_translate(action, ui.ctx());
                        }
                    }
                    View::Settings => {
                        if let Some(action) = self.settings.ui(ui) {
                            self.handle_settings(action);
                        }
                    }
                }
            });
    }

    fn on_exit(&mut self) {
        self.app.shutdown();
    }
}

/// One-line status summary for the status bar.
fn status_summary(state: &crate::app::RequestState) -> String {
    match state.status {
        Status::Ready => "Ready".to_owned(),
        Status::Waiting => "Waiting for the provider".to_owned(),
        Status::Receiving => "Receiving translation".to_owned(),
        Status::Stopping => "Stopping".to_owned(),
        Status::Complete => "Translation complete".to_owned(),
        Status::Stopped if state.message.is_empty() => "Translation stopped".to_owned(),
        Status::Stopped => state.message.clone(),
        Status::Failed if state.message.is_empty() => "Translation failed".to_owned(),
        Status::Failed => state.message.clone(),
    }
}

/// Message shown when a translation cannot start.
fn start_error(error: &StartError) -> String {
    error.to_string()
}

/// Notice shown after saving settings that require a restart.
fn restart_notice(settings: &Settings) -> String {
    let mut parts = Vec::new();
    if settings.tray_enabled {
        parts.push("tray changes take effect after restart");
    }
    if settings.start_hidden {
        parts.push("start hidden takes effect after restart");
    }
    if parts.is_empty() {
        "Settings saved".to_owned()
    } else {
        format!("Settings saved; {}", parts.join(", "))
    }
}

use egui::RichText;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_saved_settings_notice_mentions_restart_when_needed() {
        let settings = Settings {
            tray_enabled: true,
            ..Settings::default()
        };
        assert!(restart_notice(&settings).contains("restart"));
        let settings = Settings {
            tray_enabled: false,
            start_hidden: false,
            ..Settings::default()
        };
        assert_eq!(restart_notice(&settings), "Settings saved");
    }

    #[test]
    fn status_summaries_are_human_readable() {
        let mut state = crate::app::RequestState::default();
        assert_eq!(status_summary(&state), "Ready");
        state.status = Status::Receiving;
        assert_eq!(status_summary(&state), "Receiving translation");
        state.status = Status::Failed;
        state.message = "provider returned HTTP 429".into();
        assert_eq!(status_summary(&state), "provider returned HTTP 429");
    }

    #[test]
    fn closing_hides_only_when_a_tray_can_restore_the_window() {
        // The normal case: close-to-tray is on and the tray registered.
        assert_eq!(
            window_action(false, false, true, true, true),
            WindowAction::Hide
        );
    }

    #[test]
    fn closing_exits_when_the_tray_is_missing() {
        // Hiding without a tray would strand the window with no way back.
        assert_eq!(
            window_action(false, false, true, true, false),
            WindowAction::None
        );
    }

    #[test]
    fn closing_exits_when_close_to_tray_is_off() {
        assert_eq!(
            window_action(false, false, true, false, true),
            WindowAction::None
        );
    }

    #[test]
    fn quit_bypasses_close_to_tray() {
        // Quit is folded into `hide_on_close`, which is cleared while
        // quitting, so a close request is never swallowed.
        assert_eq!(
            window_action(false, false, true, false, true),
            WindowAction::None
        );
    }

    #[test]
    fn a_hidden_window_is_revealed_when_no_tray_registered() {
        // Start-hidden without a tray would leave the window unreachable.
        assert_eq!(
            window_action(true, false, false, true, false),
            WindowAction::Show
        );
    }

    #[test]
    fn a_hidden_window_stays_hidden_while_the_tray_exists() {
        // The first pass re-asserts the hidden state, because eframe shows a
        // window once it has painted.
        assert_eq!(
            window_action(true, false, false, true, true),
            WindowAction::Hide
        );
        // Afterwards it is left alone, so a tray command's Show is not undone.
        assert_eq!(
            window_action(true, true, false, true, true),
            WindowAction::None
        );
    }

    #[test]
    fn a_visible_window_does_nothing_without_a_close_request() {
        assert_eq!(
            window_action(false, false, false, true, true),
            WindowAction::None
        );
    }

    #[test]
    fn restart_is_only_announced_for_settings_that_need_it() {
        let settings = Settings {
            tray_enabled: false,
            start_hidden: true,
            ..Settings::default()
        };
        let notice = restart_notice(&settings);
        assert!(notice.contains("start hidden"));
        // Close-to-tray applies immediately, so it is not a restart item.
        assert!(!notice.contains("close"));
    }
}
