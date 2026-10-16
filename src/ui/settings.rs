//! The settings view: grouped fields editing a draft.

use eframe::egui::{self, Align, Layout, RichText, ScrollArea, Ui};

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
    /// Import failure shown in an informational dialog.
    import_error: Option<String>,
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
        self.import_error = None;
    }

    /// Discard the draft.
    pub fn close(&mut self) {
        self.draft = None;
        self.import_text.clear();
        self.import_error = None;
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
                        let response = theme::settings_edit(
                            ui,
                            theme::field_edit(&mut draft.api_base, "settings-api-base")
                                .desired_width(f32::INFINITY),
                        );
                        theme::text_edit_menu(ui, &response, &mut draft.api_base, false);
                    });
                    field(ui, "API method", &colors, |ui| {
                        let mut method = draft.settings.api_method;
                        ui.allocate_ui(egui::vec2(220.0, theme::CONTROL_HEIGHT), |ui| {
                            egui::ComboBox::from_id_salt("settings-api-method")
                                .truncate()
                                .selected_text(method_label(method))
                                .width(220.0)
                                .show_ui(ui, |ui| {
                                    theme::selectable_value(
                                        ui,
                                        &mut method,
                                        ApiMethod::ChatCompletions,
                                        "Chat Completions",
                                    );
                                    theme::selectable_value(
                                        ui,
                                        &mut method,
                                        ApiMethod::Responses,
                                        "OpenAI Responses",
                                    );
                                });
                        });
                        draft.settings.api_method = method;
                    });
                    field(ui, "API key (optional)", &colors, |ui| {
                        let response = theme::settings_edit(
                            ui,
                            theme::field_edit(&mut draft.api_key, "settings-api-key")
                                .password(true)
                                .desired_width(f32::INFINITY),
                        );
                        theme::text_edit_menu(ui, &response, &mut draft.api_key, true);
                    });
                    field(ui, "Model", &colors, |ui| {
                        let response = theme::settings_edit(
                            ui,
                            theme::field_edit(&mut draft.model, "settings-model")
                                .desired_width(f32::INFINITY),
                        );
                        theme::text_edit_menu(ui, &response, &mut draft.model, false);
                    });
                    field(ui, "Request deadline (1–600 seconds)", &colors, |ui| {
                        let response = theme::settings_edit(
                            ui,
                            theme::field_edit(&mut draft.timeout, "settings-timeout")
                                .desired_width(120.0),
                        );
                        theme::text_edit_menu(ui, &response, &mut draft.timeout, false);
                    });
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Request URL").color(colors.secondary));
                        theme::message(
                            ui,
                            "request-url",
                            &draft.endpoint_preview(),
                            colors.placeholder,
                        );
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
                                    let response = theme::settings_edit(
                                        ui,
                                        theme::field_edit(
                                            &mut draft.max_tokens,
                                            "settings-max-tokens",
                                        )
                                        .desired_width(120.0),
                                    );
                                    theme::text_edit_menu(
                                        ui,
                                        &response,
                                        &mut draft.max_tokens,
                                        false,
                                    );
                                },
                            );
                        });
                        ui.add_space(SPACE_4);
                        ui.vertical(|ui| {
                            field(ui, "Temperature", &colors, |ui| {
                                let response = theme::settings_edit(
                                    ui,
                                    theme::field_edit(
                                        &mut draft.temperature,
                                        "settings-temperature",
                                    )
                                    .desired_width(120.0),
                                );
                                theme::text_edit_menu(ui, &response, &mut draft.temperature, false);
                            });
                        });
                    });
                    field(ui, "Extra instructions", &colors, |ui| {
                        theme::multiline_field(
                            ui,
                            &mut draft.settings.instruction,
                            "settings-instruction",
                            3,
                            "Optional style or formatting rules",
                        );
                    });

                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Glossary").color(colors.secondary));
                        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                            if theme::button(ui, "+ Add term").clicked() {
                                draft.add_term();
                            }
                        });
                    });
                    ui.add_space(SPACE_2);
                    let mut remove = None;
                    ScrollArea::vertical()
                        .id_salt("settings-glossary")
                        .max_height(4.0 * (theme::CONTROL_HEIGHT + ui.spacing().item_spacing.y))
                        .auto_shrink([false, false])
                        .show(ui, |ui| {
                            if draft.settings.glossary.is_empty() {
                                ui.label(
                                    RichText::new("No terms added.").color(colors.placeholder),
                                );
                            }
                            for (index, term) in draft.settings.glossary.iter_mut().enumerate() {
                                ui.horizontal(|ui| {
                                    let width =
                                        ((ui.available_width() - 80.0) / 2.0 - 8.0).max(20.0);
                                    let source = theme::settings_edit(
                                        ui,
                                        theme::field_edit(
                                            &mut term.source,
                                            ("glossary-source", index),
                                        )
                                        .hint_text("Source term")
                                        .desired_width(width),
                                    );
                                    theme::text_edit_menu(ui, &source, &mut term.source, false);
                                    ui.label(RichText::new("→").color(colors.secondary));
                                    let target = theme::settings_edit(
                                        ui,
                                        theme::field_edit(
                                            &mut term.target,
                                            ("glossary-target", index),
                                        )
                                        .hint_text("Translation")
                                        .desired_width(width),
                                    );
                                    theme::text_edit_menu(ui, &target, &mut term.target, false);
                                    if theme::button(ui, "×")
                                        .on_hover_text("Remove glossary term")
                                        .clicked()
                                    {
                                        remove = Some(index);
                                    }
                                });
                            }
                        });
                    if let Some(index) = remove {
                        draft.remove_term(index);
                    }

                    ui.add_space(SPACE_2);
                    ui.label(RichText::new("Import glossary JSON").color(colors.secondary));
                    ui.horizontal(|ui| {
                        let width = (ui.available_width() - 90.0).clamp(20.0, 320.0);
                        let response = theme::settings_edit(
                            ui,
                            theme::field_edit(&mut self.import_text, "glossary-import")
                                .hint_text(r#"{"source": "target"}"#)
                                .desired_width(width),
                        );
                        theme::text_edit_menu(ui, &response, &mut self.import_text, false);
                        if theme::button(ui, "Import").clicked() {
                            let json = self.import_text.clone();
                            match draft.import_glossary(&json) {
                                Ok(()) => {
                                    self.import_text.clear();
                                    self.import_error = None;
                                }
                                Err(message) => self.import_error = Some(message),
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
                let color = if draft.feedback_is_error {
                    colors.destructive
                } else {
                    colors.success
                };
                theme::message(ui, "settings-feedback", &draft.feedback, color);
                ui.horizontal(|ui| {
                    ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                        if theme::primary_button(ui, "Save settings", true, &colors).clicked() {
                            action = Some(Action::Save);
                        }
                        if theme::text_button(ui, "Cancel").clicked() {
                            action = Some(Action::Cancel);
                        }
                        if theme::button(ui, "Restore defaults").clicked() {
                            draft.confirm_restore = true;
                        }
                    });
                });

                if draft.confirm_restore {
                    confirm_restore(ui, draft, &colors);
                }
            });
        if let Some(message) = &self.import_error {
            let mut open = true;
            let mut dismiss = false;
            egui::Window::new("Info")
                .id(egui::Id::new("glossary-import-info"))
                .frame(theme::dialog_frame(&colors))
                .open(&mut open)
                .collapsible(false)
                .resizable(false)
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .show(ui.ctx(), |ui| {
                    ui.set_width(320.0);
                    ui.label("Could not import glossary JSON.");
                    ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
                        ui.label(message);
                    });
                    ui.add_space(SPACE_3);
                    dismiss = theme::primary_button(ui, "OK", true, &colors).clicked();
                });
            if !open || dismiss {
                self.import_error = None;
            }
        }
        action
    }
}

/// Confirmation shown before the draft is replaced with defaults.
fn confirm_restore(ui: &mut Ui, draft: &mut Draft, colors: &theme::Palette) {
    let mut confirm = false;
    let mut cancel = false;
    egui::Window::new("Restore defaults?")
        .frame(theme::dialog_frame(colors))
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ui.ctx(), |ui| {
            ui.label("This replaces the settings in the form. Save settings to apply them.");
            ui.add_space(SPACE_3);
            ui.horizontal(|ui| {
                if theme::primary_button(ui, "Restore defaults", true, colors).clicked() {
                    confirm = true;
                }
                if theme::text_button(ui, "Cancel").clicked() {
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
    theme::surface(colors).show(ui, |ui| {
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

/// Message for a save failure.
pub fn save_error_message(error: &ValidationError) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_json_opens_info_without_bottom_error_feedback() {
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        let mut view = SettingsView::default();
        view.open(&Settings::default());
        view.import_text = "not json".into();
        let mut render = |events| {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(760.0, 1800.0),
                    )),
                    events,
                    ..Default::default()
                },
                |ui| {
                    view.ui(ui);
                },
            );
            output.textures_delta.clear();
            output
        };
        let output = render(Vec::new());
        let import = output
            .shapes
            .iter()
            .find_map(|shape| {
                if let egui::epaint::Shape::Text(text) = &shape.shape
                    && text.galley.text() == "Import"
                {
                    Some(text.pos + text.galley.size() / 2.0)
                } else {
                    None
                }
            })
            .unwrap();
        render(vec![
            egui::Event::PointerMoved(import),
            egui::Event::PointerButton {
                pos: import,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ]);
        render(vec![egui::Event::PointerButton {
            pos: import,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }]);
        let output = render(Vec::new());
        assert!(output.shapes.iter().any(|shape| {
            matches!(&shape.shape, egui::epaint::Shape::Text(text)
                if text.galley.text() == "Could not import glossary JSON.")
        }));
        assert!(view.import_error.is_some());
        assert!(view.draft.as_ref().unwrap().feedback.is_empty());
        assert!(!view.draft.as_ref().unwrap().feedback_is_error);
        assert_eq!(view.import_text, "not json");
        view.close();
        assert!(view.import_error.is_none());
    }

    #[test]
    fn oversized_content_does_not_expand_the_settings_form() {
        fn measure(settings: &Settings, width: f32, feedback: &str) -> egui::Vec2 {
            let ctx = egui::Context::default();
            theme::apply(&ctx);
            let mut view = SettingsView::default();
            view.open(settings);
            view.draft.as_mut().unwrap().feedback = feedback.into();
            let mut size = egui::Vec2::ZERO;
            for _ in 0..3 {
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(width, 3000.0),
                        )),
                        ..Default::default()
                    },
                    |ui| {
                        egui::CentralPanel::default().show(ui, |ui| {
                            view.ui(ui);
                            size = ui.min_size();
                        });
                    },
                );
                size.y = output
                    .shapes
                    .iter()
                    .find_map(|shape| {
                        if let egui::epaint::Shape::Text(text) = &shape.shape
                            && text.galley.text() == "Desktop"
                        {
                            Some(text.pos.y)
                        } else {
                            None
                        }
                    })
                    .expect("Desktop section should be visible in the tall test viewport");
                output.textures_delta.clear();
            }
            size
        }
        let short = Settings::default();
        let mut long = short.clone();
        long.instruction = "Long instruction text\n".repeat(200);
        long.api_base = format!("https://example.com/{}", "long-path/".repeat(200));
        long.glossary = (0..100)
            .map(|index| crate::translation::prompt::Glossary {
                source: format!("{index}{}", "source".repeat(100)),
                target: "translation".repeat(100),
            })
            .collect();
        for width in [560.0, 760.0] {
            let baseline = measure(&short, width, "");
            let oversized = measure(&long, width, &"Very long error\n".repeat(100));
            assert!(
                (baseline.y - oversized.y).abs() < 1.0,
                "content changed form height: {baseline:?} -> {oversized:?}"
            );
            assert!(
                oversized.x <= width,
                "content overflowed width: {oversized:?}"
            );
        }
    }

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
