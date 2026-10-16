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
    /// Presentation only; copying always uses the original result.
    markdown: bool,
    markdown_cache: egui_commonmark::CommonMarkCache,
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
                            if theme::selectable_value(
                                ui,
                                &mut self.source,
                                "auto".to_owned(),
                                "Detect language",
                            )
                            .clicked()
                            {
                                changed = true;
                            }
                            for language in languages::LANGUAGES {
                                if theme::selectable_value(
                                    ui,
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
                            if theme::selectable_value(
                                ui,
                                &mut self.target,
                                "auto".to_owned(),
                                "Auto",
                            )
                            .clicked()
                            {
                                changed = true;
                            }
                            for language in languages::LANGUAGES {
                                theme::selectable_value(
                                    ui,
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

        let card = theme::surface(colors).show(ui, |ui| {
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
                    if theme::primary_button(ui, "Translate", can_translate, colors).clicked() {
                        action = Some(Action::Translate);
                    }
                    if theme::text_button(ui, "Clear").clicked() {
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

        let card = theme::surface(colors).show(ui, |ui| {
            // Reserve the action row so the text area takes the rest.
            let reserve = theme::CONTROL_HEIGHT + SPACE_2;
            let height = (ui.available_height() - reserve).max(MIN_RESULT_HEIGHT);
            let focused = ScrollArea::both()
                .id_salt(("translation-output", self.markdown))
                .min_scrolled_width(0.0)
                .max_height(height)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if self.markdown {
                        markdown_result(ui, &mut self.markdown_cache, &state.output);
                        return false;
                    }
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
                theme::selectable_value(ui, &mut self.markdown, false, "Plain");
                theme::selectable_value(ui, &mut self.markdown, true, "Markdown");
                if state.status.is_active() && theme::button(ui, "Stop").clicked() {
                    action = Some(Action::Stop);
                }
                if state.is_incomplete() {
                    ui.label(RichText::new("Incomplete result").color(colors.warning));
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui
                        .add_enabled_ui(state.can_copy(), |ui| {
                            theme::text_button(ui, "Copy result")
                        })
                        .inner
                        .clicked()
                    {
                        action = Some(Action::Copy);
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

/// Document typography is independent of the compact form-control theme.
fn markdown_result(ui: &mut Ui, cache: &mut egui_commonmark::CommonMarkCache, text: &str) {
    ui.scope(|ui| {
        let style = ui.style_mut();
        style.interaction.selectable_labels = true;
        style
            .text_styles
            .insert(egui::TextStyle::Body, theme::editor_text());
        style
            .text_styles
            .insert(egui::TextStyle::Heading, egui::FontId::proportional(30.0));
        style
            .text_styles
            .insert(egui::TextStyle::Monospace, egui::FontId::monospace(14.0));
        style.spacing.button_padding.y = 0.0;
        style.spacing.item_spacing.y = 4.0;
        let line_height = ui.text_style_height(&egui::TextStyle::Body);
        ui.spacing_mut().interact_size.y = line_height;
        // commonmark maps strong spans to egui's strong text color. Tag those
        // glyphs explicitly so static fonts such as XiHei can be emboldened.
        let text_color = ui.visuals().text_color();
        ui.visuals_mut().widgets.active.fg_stroke.color = MARKDOWN_STRONG;
        let layer = ui.layer_id();
        let start = ui.ctx().graphics(|graphics| {
            graphics
                .get(layer)
                .map_or(egui::layers::ShapeIdx(0), |list| list.next_idx())
        });
        egui_commonmark::CommonMarkViewer::new().show(ui, cache, text);
        let offset = 0.5 / ui.ctx().pixels_per_point();
        ui.ctx().graphics_mut(|graphics| {
            if let Some(list) = graphics.get_mut(layer) {
                let end = list.next_idx();
                for index in start.0..end.0 {
                    list.mutate_shape(egui::layers::ShapeIdx(index), |clipped| {
                        embolden_markdown(&mut clipped.shape, text_color, offset);
                    });
                }
            }
        });
    });
}

// A private marker used only between Markdown layout and painting. It never
// reaches the screen; normal text, links and code retain their own colors.
const MARKDOWN_STRONG: egui::Color32 = egui::Color32::from_rgb(1, 2, 3);

/// Synthetic weight preserves the preferred font and its original text metrics.
/// Only glyph triangles are repeated, leaving selection and code backgrounds alone.
fn embolden_markdown(shape: &mut egui::epaint::Shape, color: egui::Color32, offset: f32) {
    match shape {
        egui::epaint::Shape::Vec(shapes) => {
            for shape in shapes {
                embolden_markdown(shape, color, offset);
            }
        }
        egui::epaint::Shape::Text(text) => {
            if !text
                .galley
                .job
                .sections
                .iter()
                .any(|section| section.format.color == MARKDOWN_STRONG)
            {
                return;
            }
            let galley = std::sync::Arc::make_mut(&mut text.galley);
            for section in &mut std::sync::Arc::make_mut(&mut galley.job).sections {
                if section.format.color == MARKDOWN_STRONG {
                    section.format.color = color;
                }
            }
            for placed in &mut galley.rows {
                let visuals = &mut std::sync::Arc::make_mut(&mut placed.row).visuals;
                let mut extra = Vec::new();
                let mut mapped = vec![None; visuals.mesh.vertices.len()];
                for index in visuals.glyph_vertex_range.clone() {
                    if visuals.mesh.vertices[index].color == MARKDOWN_STRONG {
                        let mut vertex = visuals.mesh.vertices[index];
                        vertex.color = color;
                        vertex.pos.x += offset;
                        mapped[index] = Some(visuals.mesh.vertices.len() as u32);
                        visuals.mesh.vertices.push(vertex);
                    }
                }
                for triangle in visuals.mesh.indices.as_chunks::<3>().0 {
                    if let [Some(a), Some(b), Some(c)] = triangle.map(|i| mapped[i as usize]) {
                        extra.extend([a, b, c]);
                    }
                }
                for vertex in &mut visuals.mesh.vertices {
                    if vertex.color == MARKDOWN_STRONG {
                        vertex.color = color;
                    }
                }
                galley.num_vertices += visuals.mesh.vertices.len() - mapped.len();
                galley.num_indices += extra.len();
                visuals.mesh.indices.extend(extra);
                visuals.mesh_bounds.max.x += offset;
            }
            galley.mesh_bounds.max.x += offset;
        }
        _ => {}
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

/// Message shown when a request cannot start.
pub fn start_error_message(error: &StartError) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn markdown_strong_adds_glyph_weight_without_changing_layout() {
        for dark in [false, true] {
            let ctx = egui::Context::default();
            ctx.set_visuals(if dark {
                egui::Visuals::dark()
            } else {
                egui::Visuals::light()
            });
            theme::apply(&ctx);
            let mut cache = egui_commonmark::CommonMarkCache::default();
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                markdown_result(
                    ui,
                    &mut cache,
                    "Normal\n\n**Normal**\n\n***Italic***\n\n**`Code`**",
                );
            });
            output.textures_delta.clear();
            let texts: Vec<_> = output
                .shapes
                .iter()
                .filter_map(|shape| {
                    if let egui::epaint::Shape::Text(text) = &shape.shape {
                        Some(text)
                    } else {
                        None
                    }
                })
                .collect();
            let normal: Vec<_> = texts
                .iter()
                .filter(|text| text.galley.text() == "Normal")
                .collect();
            assert_eq!(normal.len(), 2);
            assert_eq!(normal[0].galley.size(), normal[1].galley.size());
            assert_eq!(
                normal[1].galley.num_vertices,
                normal[0].galley.num_vertices * 2
            );
            for text in texts {
                assert!(
                    text.galley
                        .job
                        .sections
                        .iter()
                        .all(|section| section.format.color != MARKDOWN_STRONG)
                );
                for row in &text.galley.rows {
                    assert!(row.visuals.mesh.is_valid());
                    assert!(
                        row.visuals
                            .mesh
                            .vertices
                            .iter()
                            .all(|vertex| vertex.color != MARKDOWN_STRONG)
                    );
                }
            }
        }
    }

    #[test]
    fn markdown_headings_and_list_markers_use_document_metrics() {
        let ctx = egui::Context::default();
        theme::apply(&ctx);
        let mut cache = egui_commonmark::CommonMarkCache::default();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(500.0, 400.0),
                )),
                ..Default::default()
            },
            |ui| {
                markdown_result(ui, &mut cache, "# Title\n\n- First item\n- Second item\n");
                assert_eq!(ui.spacing().interact_size.y, theme::CONTROL_HEIGHT);
            },
        );
        output.textures_delta.clear();
        let text = |label: &str| {
            output
                .shapes
                .iter()
                .find_map(|shape| {
                    if let egui::epaint::Shape::Text(text) = &shape.shape
                        && text.galley.text() == label
                    {
                        Some(text)
                    } else {
                        None
                    }
                })
                .unwrap()
        };
        assert!(text("Title").galley.job.sections[0].format.font_id.size > 24.0);
        let markers: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| {
                if let egui::epaint::Shape::Circle(circle) = &shape.shape {
                    Some(circle.center.y)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(markers.len(), 2);
        for (marker, label) in markers.iter().zip(["First item", "Second item"]) {
            let text = text(label);
            let center = text.pos.y + text.galley.size().y / 2.0;
            assert!(
                (marker - center).abs() < 2.0,
                "marker {marker} does not align with text {center}"
            );
        }
        output.textures_delta.clear();
    }

    #[test]
    fn markdown_renders_every_stream_prefix_inside_a_bounded_viewport() {
        let ctx = egui::Context::default();
        let mut cache = egui_commonmark::CommonMarkCache::default();
        let markdown = "# Translation\n\n**Bold** and 中文.\n\n- one\n- two\n\n```rust\nfn main() {}\n```\n\n| A | B |\n|---|---|\n| 1 | 2 |\n";
        for end in markdown
            .char_indices()
            .map(|(index, _)| index)
            .chain([markdown.len()])
        {
            let mut output = ctx.run_ui(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(400.0, 300.0),
                    )),
                    ..Default::default()
                },
                |ui| {
                    egui::CentralPanel::default().show(ui, |ui| {
                        let scroll = ScrollArea::both()
                            .max_height(100.0)
                            .min_scrolled_width(0.0)
                            .auto_shrink([false, false])
                            .show(ui, |ui| {
                                markdown_result(ui, &mut cache, &markdown[..end]);
                            });
                        assert!(scroll.inner_rect.height() <= 100.0);
                        assert!(scroll.inner_rect.width() <= 384.0);
                    });
                },
            );
            output.textures_delta.clear();
        }
    }
}
