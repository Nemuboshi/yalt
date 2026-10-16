//! The translation view: language controls, input editor, and result.

use eframe::egui::{self, Align, Layout, RichText, ScrollArea, TextEdit, Ui};

use crate::app::{App, StartError};
use crate::translation::languages;

use super::theme::{self, SPACE_1, SPACE_2, SPACE_3, SPACE_4};

/// Rows the input editor shows before it scrolls.
const EDITOR_ROWS: usize = 8;

/// Smallest height the result box keeps on a short window.
const MIN_RESULT_HEIGHT: f32 = 80.0;

/// Persistent UI state for the translation view.
///
/// Input and output text live here because they are view state, not settings;
/// they are intentionally never written to the settings file.
#[derive(Debug, Default)]
pub struct TranslateView {
    /// Text being translated.
    pub input: String,
    /// Source selection, `auto` for detection.
    pub source: String,
    /// Target selection, `auto` for the automatic rule.
    pub target: String,
}

/// Actions the view asks the application to perform.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Submit the current input.
    Translate,
    /// Stop the active request.
    Stop,
    /// Clear the input and result.
    Clear,
    /// Copy the current result.
    Copy,
}

impl TranslateView {
    /// Create the view with the given saved selections.
    pub fn new(source: &str, target: &str) -> Self {
        Self {
            source: source.to_owned(),
            target: target.to_owned(),
            ..Self::default()
        }
    }

    /// Resolve the source code sent with a request.
    ///
    /// An automatic selection sends the detected code when detection is
    /// confident, and `auto` otherwise so the prompt does not assert an
    /// unverified language.
    pub fn resolved_source(&self, app: &App) -> String {
        if !self.source.eq_ignore_ascii_case("auto") {
            return self.source.clone();
        }
        match app.detection() {
            Some(detection) if !detection.uncertain && !detection.code.is_empty() => {
                detection.code.clone()
            }
            _ => "auto".to_owned(),
        }
    }

    /// Draw the view, returning any requested action.
    ///
    /// The layout is fixed: the language bar and the input box keep their
    /// heights, and the result box fills the rest of the panel. Text beyond
    /// each box's height scrolls inside that box.
    pub fn ui(&mut self, ui: &mut Ui, app: &mut App) -> Option<Action> {
        let colors = theme::palette(ui.visuals().dark_mode);
        let mut action;

        self.language_bar(ui, app, &colors);
        ui.add_space(SPACE_3);

        action = self.editor(ui, app, &colors);
        ui.add_space(SPACE_4);
        if let Some(requested) = self.result(ui, app, &colors) {
            action = Some(requested);
        }
        action
    }

    /// Source and target selectors.
    fn language_bar(&mut self, ui: &mut Ui, app: &mut App, colors: &theme::Palette) {
        let source_changed = {
            let mut changed = false;
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new("From").color(colors.secondary));
                let selected = self.source_label(app);
                ui.allocate_ui(egui::vec2(170.0, theme::CONTROL_HEIGHT), |ui| {
                    egui::ComboBox::from_id_salt("source-language")
                        .selected_text(&selected)
                        .truncate()
                        .width(170.0)
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_value(
                                    &mut self.source,
                                    "auto".to_owned(),
                                    "Detect language",
                                )
                                .clicked()
                            {
                                changed = true;
                            }
                            for language in languages::LANGUAGES {
                                if ui
                                    .selectable_value(
                                        &mut self.source,
                                        language.code.to_owned(),
                                        language.name,
                                    )
                                    .clicked()
                                {
                                    changed = true;
                                }
                            }
                        })
                        .response
                        .on_hover_text(&selected);
                });
                ui.add_space(SPACE_2);
                ui.label(RichText::new("→").color(colors.secondary));
                ui.add_space(SPACE_2);
                ui.label(RichText::new("To").color(colors.secondary));
                let selected = self.target_label(app);
                ui.allocate_ui(egui::vec2(170.0, theme::CONTROL_HEIGHT), |ui| {
                    egui::ComboBox::from_id_salt("target-language")
                        .selected_text(&selected)
                        .truncate()
                        .width(170.0)
                        .show_ui(ui, |ui| {
                            if ui
                                .selectable_value(&mut self.target, "auto".to_owned(), "Auto")
                                .clicked()
                            {
                                changed = true;
                            }
                            for language in languages::LANGUAGES {
                                ui.selectable_value(
                                    &mut self.target,
                                    language.code.to_owned(),
                                    language.name,
                                );
                            }
                        })
                        .response
                        .on_hover_text(&selected);
                });
            });
            changed
        };
        // A manual source selection clears any auto-detected marking.
        if source_changed {
            app.reset_result();
        }
    }

    /// Label for the source selector, marking an automatic detection.
    fn source_label(&self, app: &App) -> String {
        if !self.source.eq_ignore_ascii_case("auto") {
            return language_label(&self.source);
        }
        match app.detection() {
            Some(detection) if !detection.uncertain && !detection.code.is_empty() => {
                format!("{} (auto-detected)", language_label(&detection.code))
            }
            Some(detection) if detection.uncertain && !self.input.trim().is_empty() => {
                "Language uncertain".to_owned()
            }
            _ => "Detect language".to_owned(),
        }
    }

    /// Label for the target selector, showing the resolved automatic choice.
    fn target_label(&self, app: &App) -> String {
        if !self.target.eq_ignore_ascii_case("auto") {
            return language_label(&self.target);
        }
        let source = self.resolved_source(app);
        let detection = app
            .detection()
            .cloned()
            .unwrap_or_else(|| languages::detect(&self.input));
        let resolved = languages::select_target(&source, "auto", &detection);
        format!("{} (auto)", language_label(&resolved))
    }

    /// Input editor and its action row.
    ///
    /// The editor keeps a fixed height and scrolls internally, so a long
    /// input never pushes the result off the view.
    fn editor(&mut self, ui: &mut Ui, app: &mut App, colors: &theme::Palette) -> Option<Action> {
        let mut action = None;
        let active = app.state().status.is_active();

        ui.horizontal(|ui| {
            ui.label(RichText::new("Your text").color(colors.secondary));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.label(RichText::new("Ctrl / ⌘ Enter").color(colors.placeholder));
            });
        });
        ui.add_space(SPACE_2);

        let card = egui::Frame::new()
            .fill(colors.content)
            .stroke(egui::Stroke::new(1.0, colors.separator))
            .corner_radius(theme::RADIUS_GROUP)
            .inner_margin(SPACE_3)
            .show(ui, |ui| {
                let focused = ScrollArea::vertical()
                    .id_salt("translation-input-scroll")
                    .max_height(editor_height(ui))
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let response = theme::text_edit(
                            ui,
                            TextEdit::multiline(&mut self.input)
                                .id_salt("translation-input")
                                .desired_rows(EDITOR_ROWS)
                                .desired_width(f32::INFINITY)
                                .font(theme::editor_text())
                                .frame(theme::editor_frame())
                                .hint_text("Write or paste something…"),
                        );
                        theme::text_edit_menu(ui, &response, &mut self.input, false);

                        // Submit with the platform shortcut, but never while an
                        // input method is composing text.
                        if !active && !composing(ui) {
                            let submit = ui.input(|input| {
                                let enter = input.key_pressed(egui::Key::Enter);
                                let modifier = if cfg!(target_os = "macos") {
                                    input.modifiers.command
                                } else {
                                    input.modifiers.ctrl
                                };
                                enter && modifier
                            });
                            if submit && response.has_focus() && !self.input.trim().is_empty() {
                                action = Some(Action::Translate);
                            }
                        }
                        response.has_focus()
                    })
                    .inner;

                ui.add_space(SPACE_2);
                ui.horizontal(|ui| {
                    let count = self.input.chars().count();
                    ui.label(RichText::new(format!("{count} characters")).color(colors.secondary));
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let can_translate = !active && !self.input.trim().is_empty();
                        if primary_button(ui, "Translate", can_translate, colors).clicked() {
                            action = Some(Action::Translate);
                        }
                        if ui.button("Clear").clicked() {
                            action = Some(Action::Clear);
                        }
                    });
                });
                focused
            });
        // The ring surrounds the whole card, matching the visible frame.
        theme::paint_focus_ring(ui, card.response.rect, colors, card.inner);
        action
    }

    /// Result surface and its action row.
    ///
    /// The box fills whatever height is left in the panel, so there is no
    /// dead space under it.
    fn result(&mut self, ui: &mut Ui, app: &mut App, colors: &theme::Palette) -> Option<Action> {
        let mut action = None;
        let state = app.state();

        ui.label(RichText::new("Translation").color(colors.secondary));
        ui.add_space(SPACE_2);

        let card = egui::Frame::new()
            .fill(colors.content)
            .stroke(egui::Stroke::new(1.0, colors.separator))
            .corner_radius(theme::RADIUS_GROUP)
            .inner_margin(SPACE_3)
            .show(ui, |ui| {
                // Reserve the action row so the text area takes the rest.
                let reserve = theme::CONTROL_HEIGHT + SPACE_2;
                let height = (ui.available_height() - reserve).max(MIN_RESULT_HEIGHT);
                let focused = ScrollArea::vertical()
                    .id_salt("translation-output")
                    .max_height(height)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        // A `&str` buffer is read-only but still selectable, so
                        // the result can be selected and copied by hand while
                        // remaining uneditable.
                        let mut text: &str = &state.output;
                        let color = if state.output.is_empty() {
                            colors.placeholder
                        } else {
                            colors.label
                        };
                        let response = theme::text_edit(
                            ui,
                            TextEdit::multiline(&mut text)
                                .id_salt("translation-output-text")
                                // Fill the box: the whole result surface is one
                                // text area, not just the lines that hold text.
                                .min_size(egui::Vec2::new(0.0, height))
                                .desired_width(f32::INFINITY)
                                .font(theme::editor_text())
                                .frame(theme::editor_frame())
                                .text_color(color),
                        );
                        theme::text_edit_menu(ui, &response, &mut text, false);
                        response.has_focus()
                    })
                    .inner;

                ui.add_space(SPACE_2);
                ui.horizontal(|ui| {
                    if state.is_incomplete() {
                        ui.label(RichText::new("Incomplete result").color(colors.warning));
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        let can_copy = state.can_copy();
                        if ui
                            .add_enabled(can_copy, egui::Button::new("Copy result"))
                            .clicked()
                        {
                            action = Some(Action::Copy);
                        }
                        if state.status.is_active() && ui.button("Stop").clicked() {
                            action = Some(Action::Stop);
                        }
                    });
                });
                focused
            });
        // The ring surrounds the whole card, matching the visible frame.
        theme::paint_focus_ring(ui, card.response.rect, colors, card.inner);
        action
    }
}

/// Height of the input editor's viewport, in points.
fn editor_height(ui: &Ui) -> f32 {
    let row = ui.fonts_mut(|fonts| fonts.row_height(&theme::editor_text()));
    row * EDITOR_ROWS as f32 + 2.0 * SPACE_1
}

/// Whether an input method is currently composing text.
///
/// A non-empty pre-edit string means the IME is active, so the platform
/// shortcut must not submit while the user is still choosing characters.
fn composing(ui: &Ui) -> bool {
    ui.input(|input| {
        input.events.iter().any(|event| {
            matches!(
                event,
                egui::Event::Ime(egui::ImeEvent::Preedit { text, .. }) if !text.is_empty()
            )
        })
    })
}

/// Human-readable name for a language code.
fn language_label(code: &str) -> String {
    languages::LANGUAGES
        .iter()
        .find(|language| language.code == code)
        .map(|language| language.name.to_owned())
        .unwrap_or_else(|| code.to_owned())
}

/// The accent-filled primary action.
fn primary_button(
    ui: &mut Ui,
    label: &str,
    enabled: bool,
    colors: &theme::Palette,
) -> egui::Response {
    let text_color = if enabled {
        colors.accent_label
    } else {
        colors.placeholder
    };
    let fill = if enabled {
        colors.accent
    } else {
        colors.control
    };
    ui.add_enabled(
        enabled,
        egui::Button::new(RichText::new(label).color(text_color))
            .fill(fill)
            .corner_radius(theme::RADIUS_CONTROL),
    )
}

/// Message shown when a request cannot start.
pub fn start_error_message(error: &StartError) -> String {
    error.to_string()
}
