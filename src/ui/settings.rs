//! The settings view: grouped fields editing a draft.

use egui::{Align, Layout, RichText, ScrollArea, Ui};

use crate::settings::{Settings, ValidationError};
use crate::translation::ApiMethod;

use super::draft::Draft;
use super::theme::{self, SPACE_2, SPACE_3, SPACE_4, SPACE_6};

/// Persistent UI state for the settings view.
#[derive(Debug, Default)]
pub struct SettingsView {
    /// The working draft, present only while the view is open.
    pub draft: Option<Draft>,
    /// Text of the glossary import field.
    pub import_text: String,
}

/// Actions the settings view asks the application to perform.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Validate and persist the draft.
    Save,
    /// Discard the draft and leave the view.
    Cancel,
}

impl SettingsView {
    /// Begin editing a draft copied from saved settings.
    pub fn open(&mut self, settings: &Settings) {
        self.draft = Some(Draft::new(settings));
        self.import_text.clear();
    }

    /// Discard the draft.
    pub fn close(&mut self) {
        self.draft = None;
        self.import_text.clear();
    }

    /// Whether a draft is open.
    pub fn is_open(&self) -> bool {
        self.draft.is_some()
    }

    /// Draw the view, returning any requested action.
    pub fn ui(&mut self, ui: &mut Ui) -> Option<Action> {
        let colors = theme::palette(ui.visuals().dark_mode);
        let mut action = None;
        let draft = self.draft.as_mut()?;

        ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.label(RichText::new("Settings").font(theme::heading_text()));
                ui.add_space(SPACE_4);

                group(ui, "Connection", &colors, |ui| {
                    field(ui, "API base URL", &colors, |ui| {
                        ui.add(
                            theme::field_edit(&mut draft.api_base, "settings-api-base")
                                .desired_width(f32::INFINITY),
                        );
                    });
                    field(ui, "API method", &colors, |ui| {
                        let mut method = draft.settings.api_method;
                        egui::ComboBox::from_id_salt("settings-api-method")
                            .selected_text(method_label(method))
                            .width(220.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(
                                    &mut method,
                                    ApiMethod::ChatCompletions,
                                    "Chat Completions",
                                );
                                ui.selectable_value(
                                    &mut method,
                                    ApiMethod::Responses,
                                    "OpenAI Responses",
                                );
                            });
                        draft.settings.api_method = method;
                    });
                    field(ui, "API key (optional)", &colors, |ui| {
                        ui.add(
                            theme::field_edit(&mut draft.api_key, "settings-api-key")
                                .password(true)
                                .desired_width(f32::INFINITY),
                        );
                    });
                    field(ui, "Model", &colors, |ui| {
                        ui.add(
                            theme::field_edit(&mut draft.model, "settings-model")
                                .desired_width(f32::INFINITY),
                        );
                    });
                    field(ui, "Request deadline (1–600 seconds)", &colors, |ui| {
                        ui.add(
                            theme::field_edit(&mut draft.timeout, "settings-timeout")
                                .desired_width(120.0),
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Request URL").color(colors.secondary));
                        ui.label(RichText::new(draft.endpoint_preview()).color(colors.placeholder));
                    });
                });

                ui.add_space(SPACE_4);
                group(ui, "Translation", &colors, |ui| {
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            let minimum = draft.min_output_tokens();
                            field(
                                ui,
                                &format!("Maximum output tokens (min {minimum})"),
                                &colors,
                                |ui| {
                                    ui.add(
                                        theme::field_edit(
                                            &mut draft.max_tokens,
                                            "settings-max-tokens",
                                        )
                                        .desired_width(120.0),
                                    );
                                },
                            );
                        });
                        ui.add_space(SPACE_4);
                        ui.vertical(|ui| {
                            field(ui, "Temperature", &colors, |ui| {
                                ui.add(
                                    theme::field_edit(
                                        &mut draft.temperature,
                                        "settings-temperature",
                                    )
                                    .desired_width(120.0),
                                );
                            });
                        });
                    });
                    field(ui, "Extra instructions", &colors, |ui| {
                        ui.add(
                            egui::TextEdit::multiline(&mut draft.settings.instruction)
                                .id_salt("settings-instruction")
                                .desired_rows(3)
                                .desired_width(f32::INFINITY)
                                .hint_text("Optional style or formatting rules"),
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Glossary").color(colors.secondary));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if ui.button("+ Add term").clicked() {
                                draft.add_term();
                            }
                        });
                    });
                    ui.add_space(SPACE_2);
                    if draft.settings.glossary.is_empty() {
                        ui.label(RichText::new("No terms added.").color(colors.placeholder));
                    }
                    let mut remove = None;
                    for (index, term) in draft.settings.glossary.iter_mut().enumerate() {
                        ui.horizontal(|ui| {
                            // IDs are keyed on the row's own identity so they stay
                            // stable when another row is removed.
                            ui.add(
                                theme::field_edit(&mut term.source, ("glossary-source", index))
                                    .hint_text("Source term")
                                    .desired_width(180.0),
                            );
                            ui.label(RichText::new("→").color(colors.secondary));
                            ui.add(
                                theme::field_edit(&mut term.target, ("glossary-target", index))
                                    .hint_text("Translation")
                                    .desired_width(180.0),
                            );
                            if ui
                                .button("×")
                                .on_hover_text("Remove glossary term")
                                .clicked()
                            {
                                remove = Some(index);
                            }
                        });
                    }
                    if let Some(index) = remove {
                        draft.remove_term(index);
                    }

                    ui.add_space(SPACE_2);
                    ui.label(RichText::new("Import glossary JSON").color(colors.secondary));
                    ui.horizontal(|ui| {
                        ui.add(
                            theme::field_edit(&mut self.import_text, "glossary-import")
                                .hint_text(r#"{"source": "target"}"#)
                                .desired_width(320.0),
                        );
                        if ui.button("Import").clicked() {
                            let json = self.import_text.clone();
                            match draft.import_glossary(&json) {
                                Ok(()) => self.import_text.clear(),
                                Err(message) => {
                                    draft.feedback = message;
                                    draft.feedback_is_error = true;
                                }
                            }
                        }
                    });

                    ui.add_space(SPACE_2);
                    ui.checkbox(&mut draft.settings.stream, "Stream output");
                    ui.checkbox(&mut draft.settings.disable_thinking, "Hide reasoning text");
                });

                ui.add_space(SPACE_4);
                group(ui, "Desktop", &colors, |ui| {
                    ui.checkbox(
                        &mut draft.settings.tray_enabled,
                        "Enable tray icon (restart required)",
                    );
                    ui.checkbox(&mut draft.settings.close_to_tray, "Close window to tray");
                    ui.checkbox(
                        &mut draft.settings.start_hidden,
                        "Start hidden (restart required)",
                    );
                });

                ui.add_space(SPACE_6);
                ui.horizontal(|ui| {
                    if !draft.feedback.is_empty() {
                        let color = if draft.feedback_is_error {
                            colors.destructive
                        } else {
                            colors.success
                        };
                        ui.label(RichText::new(draft.feedback.clone()).color(color));
                    }
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if primary_button(ui, "Save settings", &colors).clicked() {
                            action = Some(Action::Save);
                        }
                        if ui.button("Cancel").clicked() {
                            action = Some(Action::Cancel);
                        }
                        if ui.button("Restore defaults").clicked() {
                            draft.confirm_restore = true;
                        }
                    });
                });

                if draft.confirm_restore {
                    confirm_restore(ui, draft, &colors);
                }
            });
        action
    }
}

/// Confirmation shown before the draft is replaced with defaults.
fn confirm_restore(ui: &mut Ui, draft: &mut Draft, colors: &theme::Palette) {
    let mut confirm = false;
    let mut cancel = false;
    egui::Window::new("Restore defaults?")
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ui.ctx(), |ui| {
            ui.label("This replaces the settings in the form. Save settings to apply them.");
            ui.add_space(SPACE_3);
            ui.horizontal(|ui| {
                if primary_button(ui, "Restore defaults", colors).clicked() {
                    confirm = true;
                }
                if ui.button("Cancel").clicked() {
                    cancel = true;
                }
            });
        });
    if confirm {
        draft.confirm_restore = false;
        draft.restore_defaults();
    } else if cancel {
        draft.confirm_restore = false;
    }
}

/// Draw a titled group with a separator.
fn group(ui: &mut Ui, title: &str, colors: &theme::Palette, contents: impl FnOnce(&mut Ui)) {
    ui.label(RichText::new(title).font(theme::heading_text()));
    ui.add_space(SPACE_2);
    egui::Frame::new()
        .fill(colors.content)
        .stroke(egui::Stroke::new(1.0, colors.separator))
        .corner_radius(theme::RADIUS_GROUP)
        .inner_margin(SPACE_3)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            contents(ui);
        });
}

/// Draw a labelled field.
fn field(ui: &mut Ui, label: &str, colors: &theme::Palette, contents: impl FnOnce(&mut Ui)) {
    ui.label(RichText::new(label).color(colors.secondary));
    contents(ui);
    ui.add_space(SPACE_2);
}

/// Display name for an API method.
fn method_label(method: ApiMethod) -> &'static str {
    match method {
        ApiMethod::ChatCompletions => "Chat Completions",
        ApiMethod::Responses => "OpenAI Responses",
    }
}

/// The accent-filled primary action.
fn primary_button(ui: &mut Ui, label: &str, colors: &theme::Palette) -> egui::Response {
    ui.add(
        egui::Button::new(RichText::new(label).color(colors.accent_label))
            .fill(colors.accent)
            .corner_radius(theme::RADIUS_CONTROL),
    )
}

/// Message for a save failure.
pub fn save_error_message(error: &ValidationError) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opening_creates_a_draft_from_saved_settings() {
        let settings = Settings {
            model: "custom".into(),
            ..Settings::default()
        };
        let mut view = SettingsView::default();
        assert!(!view.is_open());
        view.open(&settings);
        assert!(view.is_open());
        assert_eq!(view.draft.as_ref().unwrap().model, "custom");
    }

    #[test]
    fn closing_discards_the_draft() {
        let mut view = SettingsView::default();
        view.open(&Settings::default());
        view.draft.as_mut().unwrap().model = "changed".into();
        view.close();
        assert!(!view.is_open());
    }

    #[test]
    fn reopening_starts_from_saved_settings_again() {
        let settings = Settings::default();
        let mut view = SettingsView::default();
        view.open(&settings);
        view.draft.as_mut().unwrap().model = "changed".into();
        // Cancel discards, then reopening shows the saved value again.
        view.close();
        view.open(&settings);
        assert_eq!(view.draft.as_ref().unwrap().model, settings.model);
    }
}
