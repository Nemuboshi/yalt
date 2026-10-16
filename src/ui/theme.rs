//! Visual theme: semantic colors, spacing, rounding, and typography.
//!
//! Values are taken from the design guide's token set, so components reference
//! roles rather than raw colors. Both appearances are supported and follow the
//! system preference.

use eframe::egui::{self, Color32, CornerRadius, FontFamily, FontId, Stroke, TextStyle, Vec2};

/// Spacing unit, in logical points.
pub const SPACE_1: f32 = 4.0;
/// Two spacing units.
pub const SPACE_2: f32 = 8.0;
/// Three spacing units.
pub const SPACE_3: f32 = 12.0;
/// Four spacing units.
pub const SPACE_4: f32 = 16.0;
/// Five spacing units.
pub const SPACE_5: f32 = 20.0;
/// Six spacing units.
pub const SPACE_6: f32 = 24.0;
/// Seven spacing units.
pub const SPACE_7: f32 = 32.0;

/// Corner rounding for controls.
pub const RADIUS_CONTROL: u8 = 6;
/// Corner rounding for editors and grouped surfaces.
pub const RADIUS_GROUP: u8 = 10;

/// Minimum interactive control height.
pub const CONTROL_HEIGHT: f32 = 30.0;

/// Width of the keyboard focus ring, in logical points.
pub const FOCUS_RING_WIDTH: f32 = 2.0;

/// Initial window size in logical pixels.
pub const INITIAL_SIZE: Vec2 = Vec2::new(760.0, 860.0);
/// Minimum usable window size in logical pixels.
pub const MIN_SIZE: Vec2 = Vec2::new(560.0, 720.0);

/// Semantic color roles for one appearance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Palette {
    /// Window background.
    pub window: Color32,
    /// Grouped content surface.
    pub content: Color32,
    /// Control background.
    pub control: Color32,
    /// Control background under the pointer.
    pub control_hover: Color32,
    /// Thin separators.
    pub separator: Color32,
    /// Primary text.
    pub label: Color32,
    /// Secondary text.
    pub secondary: Color32,
    /// Placeholder and disabled text.
    pub placeholder: Color32,
    /// Accent used for the primary action and focus.
    pub accent: Color32,
    /// Accent under the pointer.
    pub accent_hover: Color32,
    /// Accent while pressed.
    pub accent_pressed: Color32,
    /// Text drawn on an accent-filled surface.
    pub accent_label: Color32,
    /// Success status.
    pub success: Color32,
    /// Warning status.
    pub warning: Color32,
    /// Destructive status.
    pub destructive: Color32,
}

/// The light appearance.
pub const LIGHT: Palette = Palette {
    window: Color32::from_rgb(0xf5, 0xf5, 0xf7),
    content: Color32::from_rgb(0xff, 0xff, 0xff),
    control: Color32::from_rgb(0xff, 0xff, 0xff),
    control_hover: Color32::from_rgb(0xfa, 0xfa, 0xfc),
    separator: Color32::from_rgb(0xd6, 0xd6, 0xdb),
    label: Color32::from_rgb(0x1d, 0x1d, 0x1f),
    secondary: Color32::from_rgb(0x6e, 0x6e, 0x73),
    placeholder: Color32::from_rgb(0x8e, 0x8e, 0x93),
    accent: Color32::from_rgb(144, 209, 255),
    accent_hover: Color32::from_gray(220),
    accent_pressed: Color32::from_gray(165),
    accent_label: Color32::from_rgb(0, 83, 125),
    success: Color32::from_rgb(0x24, 0x8a, 0x3d),
    warning: Color32::from_rgb(0x9a, 0x67, 0x00),
    destructive: Color32::from_rgb(0xff, 0x3b, 0x30),
};

/// The dark appearance.
pub const DARK: Palette = Palette {
    window: Color32::from_rgb(0x1e, 0x1e, 0x1e),
    content: Color32::from_rgb(0x26, 0x26, 0x28),
    control: Color32::from_rgb(0x32, 0x32, 0x34),
    control_hover: Color32::from_rgb(0x3e, 0x3e, 0x40),
    separator: Color32::from_rgb(0x3a, 0x3a, 0x3c),
    label: Color32::from_rgb(0xf5, 0xf5, 0xf7),
    secondary: Color32::from_rgb(0x98, 0x98, 0x9d),
    placeholder: Color32::from_rgb(0x8e, 0x8e, 0x93),
    accent: Color32::from_rgb(0xc6, 0x9c, 0x7b),
    accent_hover: Color32::from_rgb(0xd6, 0xb1, 0x92),
    accent_pressed: Color32::from_rgb(0xe3, 0xc6, 0xac),
    accent_label: Color32::from_rgb(0x1d, 0x1d, 0x1f),
    success: Color32::from_rgb(0x30, 0xd1, 0x58),
    warning: Color32::from_rgb(0xff, 0xd6, 0x0a),
    destructive: Color32::from_rgb(0xff, 0x45, 0x3a),
};

/// Select the palette for the current system appearance.
pub fn palette(dark_mode: bool) -> Palette {
    if dark_mode {
        DARK
    } else {
        let visuals = egui::Visuals::light();
        Palette {
            window: visuals.panel_fill,
            content: visuals.faint_bg_color,
            control: visuals.extreme_bg_color,
            control_hover: visuals.widgets.hovered.bg_fill,
            separator: visuals.widgets.inactive.bg_stroke.color,
            label: visuals.text_color(),
            secondary: visuals.weak_text_color(),
            placeholder: visuals.weak_text_color(),
            accent: visuals.selection.bg_fill,
            accent_hover: visuals.widgets.hovered.bg_fill,
            accent_pressed: visuals.widgets.active.bg_fill,
            accent_label: visuals.selection.stroke.color,
            destructive: visuals.error_fg_color,
            warning: visuals.warn_fg_color,
            ..LIGHT
        }
    }
}

/// Editor and output text style.
pub fn editor_text() -> FontId {
    FontId::new(15.0, FontFamily::Proportional)
}

/// Section heading text style.
pub fn heading_text() -> FontId {
    FontId::new(15.0, FontFamily::Proportional)
}

/// Caption text style.
pub fn caption_text() -> FontId {
    FontId::new(11.0, FontFamily::Proportional)
}

/// Apply typography and spacing to both appearances, retaining default light visuals.
pub fn apply(ctx: &egui::Context) {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        let colors = palette(theme == egui::Theme::Dark);
        let mut style = (*ctx.style_of(theme)).clone();
        apply_to(&mut style, colors);
        if theme == egui::Theme::Light {
            style.visuals = egui::Visuals::light();
        }
        ctx.set_style_of(theme, style);
    }
}

/// Fill in the semantic token values for one palette.
fn apply_to(style: &mut egui::Style, colors: Palette) {
    style.spacing.item_spacing = Vec2::new(SPACE_2, SPACE_2);
    style.spacing.button_padding = Vec2::new(SPACE_3, SPACE_1 + 2.0);
    style.spacing.interact_size.y = CONTROL_HEIGHT;
    style.spacing.window_margin = egui::Margin::same(SPACE_4 as i8);

    style
        .text_styles
        .insert(TextStyle::Body, FontId::new(13.0, FontFamily::Proportional));
    style.text_styles.insert(
        TextStyle::Button,
        FontId::new(13.0, FontFamily::Proportional),
    );
    style.text_styles.insert(TextStyle::Heading, heading_text());
    style.text_styles.insert(TextStyle::Small, caption_text());
    style.text_styles.insert(
        TextStyle::Monospace,
        FontId::new(13.0, FontFamily::Monospace),
    );

    let visuals = &mut style.visuals;
    visuals.override_text_color = Some(colors.label);
    visuals.panel_fill = colors.window;
    visuals.window_fill = colors.window;
    visuals.extreme_bg_color = colors.control;
    visuals.faint_bg_color = colors.content;
    visuals.window_stroke = Stroke::new(1.0, colors.separator);
    visuals.selection.bg_fill = colors.accent;
    // egui draws selected text (editor selections and chosen combo items) in
    // `selection.stroke.color` over `selection.bg_fill`, so the stroke color
    // must contrast with the accent-filled background or the text disappears.
    visuals.selection.stroke = Stroke::new(1.0, colors.accent_label);
    visuals.hyperlink_color = colors.accent;

    let radius = CornerRadius::same(RADIUS_CONTROL);
    visuals.widgets.noninteractive.bg_fill = colors.window;
    visuals.widgets.noninteractive.weak_bg_fill = colors.window;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0, colors.separator);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0, colors.label);
    visuals.widgets.noninteractive.corner_radius = radius;

    visuals.widgets.inactive.bg_fill = colors.control;
    visuals.widgets.inactive.weak_bg_fill = colors.control;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0, colors.separator);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0, colors.label);
    visuals.widgets.inactive.corner_radius = radius;

    visuals.widgets.hovered.bg_fill = colors.control_hover;
    visuals.widgets.hovered.weak_bg_fill = colors.control_hover;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0, colors.separator);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0, colors.label);
    visuals.widgets.hovered.corner_radius = radius;

    visuals.widgets.active.bg_fill = colors.control_hover;
    visuals.widgets.active.weak_bg_fill = colors.control_hover;
    visuals.widgets.active.bg_stroke = Stroke::new(2.0, colors.accent);
    visuals.widgets.active.fg_stroke = Stroke::new(1.0, colors.label);
    visuals.widgets.active.corner_radius = radius;

    visuals.widgets.open.bg_fill = colors.control;
    visuals.widgets.open.weak_bg_fill = colors.control;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0, colors.separator);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0, colors.label);
    visuals.widgets.open.corner_radius = radius;

    // Only the text controls may be selected; interface text is not.
    style.interaction.selectable_labels = false;
}

/// The frame used for text editors nested inside a card.
///
/// The card already supplies the surface and its border, so the editor itself
/// paints nothing: this keeps the text area seamless inside the card and
/// prevents a second, smaller focus ring from appearing around it.
pub fn editor_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(Color32::TRANSPARENT)
        .stroke(Stroke::NONE)
        .inner_margin(egui::Margin::same(SPACE_1 as i8))
}

/// A single-line settings field, sized to line up with buttons on its row.
///
/// The field keeps its own visible border because it stands alone rather than
/// sitting inside a card. Text is centered vertically within the fixed height,
/// which keeps it on the same baseline as the button next to it.
pub fn field_edit<'a>(
    text: &'a mut String,
    id: impl std::hash::Hash + std::fmt::Debug,
) -> egui::TextEdit<'a> {
    egui::TextEdit::singleline(text)
        .clip_text(true)
        .id_salt(id)
        .min_size(Vec2::new(0.0, CONTROL_HEIGHT))
        .vertical_align(egui::Align::Center)
}

/// Settings fields retain a visible focus outline independent of selection colors.
pub fn settings_edit(ui: &mut egui::Ui, edit: egui::TextEdit<'_>) -> egui::Response {
    let response = text_edit(ui, edit);
    paint_field_focus(ui, response.rect, response.has_focus());
    response
}

fn paint_field_focus(ui: &egui::Ui, rect: egui::Rect, focused: bool) {
    if focused {
        ui.painter().rect_stroke(
            rect,
            CornerRadius::same(RADIUS_CONTROL),
            Stroke::new(FOCUS_RING_WIDTH, ui.visuals().hyperlink_color),
            egui::StrokeKind::Inside,
        );
    }
}

/// Keep dynamic messages inside one row; scroll to read overflow.
pub fn message(
    ui: &mut egui::Ui,
    id: impl std::hash::Hash + std::fmt::Debug,
    text: &str,
    color: Color32,
) {
    let height = ui.text_style_height(&TextStyle::Body) + SPACE_2;
    egui::ScrollArea::both()
        .id_salt(id)
        .max_height(height)
        .min_scrolled_height(height)
        .min_scrolled_width(0.0)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add(egui::Label::new(egui::RichText::new(text).color(color)).extend());
        });
}

/// A multiline field whose viewport stays the same size as its text grows.
pub fn multiline_field(
    ui: &mut egui::Ui,
    text: &mut String,
    id: impl std::hash::Hash + std::fmt::Debug,
    rows: usize,
    hint: &str,
) -> egui::Response {
    let height = ui.text_style_height(&TextStyle::Body) * rows as f32 + 2.0 * SPACE_1;
    let scroll = egui::ScrollArea::vertical()
        .id_salt(("multiline-field-scroll", &id))
        .max_height(height)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let response = text_edit(
                ui,
                egui::TextEdit::multiline(text)
                    .id_salt(id)
                    .desired_rows(rows)
                    .desired_width(f32::INFINITY)
                    .hint_text(hint),
            );
            text_edit_menu(ui, &response, text, false);
            response
        });
    paint_field_focus(ui, scroll.inner_rect, scroll.inner.has_focus());
    scroll.inner
}

/// Show an editor without letting a secondary press move its selection.
///
/// egui 0.36 handles any pointer button as the start of text selection. Hide
/// secondary-button pointer input only while drawing the editor, then restore
/// it for the context menu. Keep the editor interactive so it retains focus.
pub fn text_edit(ui: &mut egui::Ui, edit: egui::TextEdit<'_>) -> egui::Response {
    let secondary =
        ui.input(|input| input.pointer.secondary_down() || input.pointer.secondary_released());
    let slot = ui.next_auto_id().with("text-edit-menu-owner");
    if let Some(id) = ui.ctx().data_mut(|data| data.get_temp::<egui::Id>(slot))
        && let Some(previous) = ui.ctx().read_response(id)
        && (previous.context_menu_opened() || (secondary && previous.contains_pointer()))
    {
        ui.memory_mut(|memory| memory.request_focus(id));
    }
    let pointer = secondary.then(|| ui.input_mut(|input| std::mem::take(&mut input.pointer)));
    let output = edit.show(ui);
    if let Some(pointer) = pointer {
        ui.input_mut(|input| input.pointer = pointer);
    }
    ui.ctx()
        .data_mut(|data| data.insert_temp(slot, output.response.id));
    let response = if secondary {
        output.response.interact(egui::Sense::click_and_drag())
    } else {
        output.response.response
    };

    // TextEdit's built-in scroll request uses the cursor from before pointer
    // selection. Follow the actual drag endpoint, including while held still
    // outside the viewport, and stop requesting frames when the drag ends.
    if response.dragged_by(egui::PointerButton::Primary)
        && let Some(pointer) = response.interact_pointer_pos()
    {
        let viewport = ui.clip_rect();
        let outside = if pointer.y < viewport.top() {
            pointer.y - viewport.top()
        } else if pointer.y > viewport.bottom() {
            pointer.y - viewport.bottom()
        } else {
            0.0
        };
        if outside != 0.0 {
            let dt = ui.input(|input| input.stable_dt).min(0.05);
            let delta = -outside.clamp(-200.0, 200.0) * 10.0 * dt;
            ui.scroll_with_delta_animation(
                egui::vec2(0.0, delta),
                egui::style::ScrollAnimation::none(),
            );
            ui.ctx().request_repaint();
        }
    }
    response
}

/// Attach the standard Cut/Copy/Paste/Select-all menu to a text edit.
///
/// `text` is the buffer the edit shows, and `secret` marks a field whose
/// contents must never reach the clipboard, such as a password. A read-only
/// buffer keeps only the actions that apply to it, so the translation result
/// offers Copy and Select all.
///
/// Copy and Cut read the selection straight out of the buffer. The viewport
/// clipboard commands only take effect while the editor holds focus, which
/// opening a menu disturbs, so they cannot carry these two reliably. Paste
/// has no such route: egui cannot read the clipboard, so the command is the
/// only way to ask the platform for its contents.
pub fn text_edit_menu(
    ui: &egui::Ui,
    response: &egui::Response,
    text: &mut dyn egui::TextBuffer,
    secret: bool,
) {
    use egui::text::{CCursor, CCursorRange};

    let id = response.id;
    let ctx = ui.ctx();
    let range = egui::TextEdit::load_state(ctx, id).and_then(|state| state.cursor.char_range());
    let has_selection = range.is_some_and(|range| !range.is_empty()) && !text.as_str().is_empty();
    let mutable = text.is_mutable();

    response.context_menu(|ui| {
        let ctx = ui.ctx().clone();
        // Paste acts on the focused widget, so the editor takes focus back
        // from the menu button that was clicked.
        let refocus = |ctx: &egui::Context| ctx.memory_mut(|mem| mem.request_focus(id));

        if mutable
            && !secret
            && ui
                .add_enabled(has_selection, egui::Button::new("Cut"))
                .clicked()
        {
            if let Some(range) = range {
                ctx.copy_text(range.slice_str(text.as_str()).to_owned());
                let cursor = text.delete_selected(&range);
                if let Some(mut state) = egui::TextEdit::load_state(&ctx, id) {
                    state.cursor.set_char_range(Some(CCursorRange::one(cursor)));
                    state.store(&ctx, id);
                }
            }
            refocus(&ctx);
            ui.close();
        }
        if !secret
            && ui
                .add_enabled(has_selection, egui::Button::new("Copy"))
                .clicked()
        {
            if let Some(range) = range {
                ctx.copy_text(range.slice_str(text.as_str()).to_owned());
            }
            refocus(&ctx);
            ui.close();
        }
        if mutable && ui.button("Paste").clicked() {
            refocus(&ctx);
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestPaste);
            ui.close();
        }
        if ui.button("Select all").clicked() {
            if let Some(mut state) = egui::TextEdit::load_state(&ctx, id) {
                state.cursor.set_char_range(Some(CCursorRange::two(
                    CCursor::new(0),
                    CCursor::new(text.as_str().chars().count()),
                )));
                state.store(&ctx, id);
            }
            refocus(&ctx);
            ui.close();
        }
    });
}

/// Draw a focus ring around a card when one of its children holds focus.
///
/// The ring is painted at `rect`, which is the card's outer rectangle, so it
/// surrounds the visible frame instead of only the inner text area.
pub fn paint_focus_ring(ui: &egui::Ui, rect: egui::Rect, colors: &Palette, focused: bool) {
    if !focused {
        return;
    }
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(RADIUS_GROUP),
        Stroke::new(FOCUS_RING_WIDTH, colors.accent),
        egui::StrokeKind::Outside,
    );
}

/// Palette for a theme.
pub fn palette_for(theme: egui::Theme) -> Palette {
    palette(theme == egui::Theme::Dark)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct EditorProbe {
        ctx: egui::Context,
        text: String,
        rect: egui::Rect,
        id: egui::Id,
        offset: f32,
        focused: bool,
        selection_painted: bool,
        time: f64,
    }

    impl EditorProbe {
        fn new() -> Self {
            Self {
                ctx: egui::Context::default(),
                text: (0..100)
                    .map(|n| format!("Line {n}: selectable text\n"))
                    .collect(),
                rect: egui::Rect::NOTHING,
                id: egui::Id::NULL,
                offset: 0.0,
                focused: false,
                selection_painted: false,
                time: 0.0,
            }
        }

        fn frame(&mut self, events: Vec<egui::Event>) {
            self.time += 1.0 / 60.0;
            let input = egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(400.0, 300.0),
                )),
                time: Some(self.time),
                events,
                ..Default::default()
            };
            let mut output = self.ctx.run_ui(input, |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    let scroll = egui::ScrollArea::vertical()
                        .max_height(100.0)
                        .show(ui, |ui| {
                            let response = text_edit(
                                ui,
                                egui::TextEdit::multiline(&mut self.text)
                                    .id_salt("probe")
                                    .desired_width(f32::INFINITY),
                            );
                            self.rect = response.rect;
                            self.id = response.id;
                            text_edit_menu(ui, &response, &mut self.text, false);
                            self.focused = response.has_focus();
                        });
                    self.offset = scroll.state.offset.y;
                });
            });
            self.selection_painted = output.shapes.iter().any(|shape| {
                if let egui::epaint::Shape::Text(text) = &shape.shape {
                    text.galley.text() == self.text
                        && text.galley.rows.iter().any(|row| {
                            row.visuals.mesh.vertices.iter().any(|vertex| {
                                vertex.color == self.ctx.global_style().visuals.selection.bg_fill
                            })
                        })
                } else {
                    false
                }
            });
            output.textures_delta.clear();
        }

        fn button(&mut self, pos: egui::Pos2, button: egui::PointerButton, pressed: bool) {
            self.frame(vec![
                egui::Event::PointerMoved(pos),
                egui::Event::PointerButton {
                    pos,
                    button,
                    pressed,
                    modifiers: egui::Modifiers::NONE,
                },
            ]);
        }
    }

    #[test]
    fn secondary_press_and_release_preserve_selection() {
        use egui::text::{CCursor, CCursorRange};
        let mut probe = EditorProbe::new();
        probe.frame(vec![]);
        probe
            .ctx
            .memory_mut(|memory| memory.request_focus(probe.id));
        probe.frame(vec![]);
        let range = CCursorRange::two(CCursor::new(1), CCursor::new(12));
        let mut state = egui::TextEdit::load_state(&probe.ctx, probe.id).unwrap();
        state.cursor.set_char_range(Some(range));
        state.store(&probe.ctx, probe.id);
        let pos = probe.rect.min + egui::vec2(50.0, 10.0);
        for pressed in [true, false] {
            probe.button(pos, egui::PointerButton::Secondary, pressed);
            assert!(probe.focused, "focus ring disappeared on pressed={pressed}");
            assert!(
                probe.selection_painted,
                "selection was not painted on pressed={pressed}"
            );
            assert_eq!(
                egui::TextEdit::load_state(&probe.ctx, probe.id)
                    .unwrap()
                    .cursor
                    .char_range(),
                Some(range),
                "secondary pressed={pressed}, focused={:?}",
                probe.ctx.memory(|memory| memory.focused()),
            );
        }
        assert!(
            probe
                .ctx
                .read_response(probe.id)
                .unwrap()
                .context_menu_opened()
        );
        for _ in 0..3 {
            probe.frame(vec![]);
            assert!(probe.focused, "focus ring disappeared while menu was open");
            assert!(
                probe.selection_painted,
                "selection disappeared while menu was open"
            );
            assert_eq!(
                egui::TextEdit::load_state(&probe.ctx, probe.id)
                    .unwrap()
                    .cursor
                    .char_range(),
                Some(range),
            );
        }
    }

    #[test]
    fn selection_drag_scrolls_while_pointer_is_held_outside() {
        let mut probe = EditorProbe::new();
        probe.frame(vec![]);
        let start = probe.rect.min + egui::vec2(20.0, 10.0);
        probe.button(start, egui::PointerButton::Primary, true);
        probe.frame(vec![egui::Event::PointerMoved(egui::pos2(start.x, 160.0))]);
        for _ in 0..30 {
            probe.frame(vec![]);
        }
        assert!(
            probe.offset > 100.0,
            "drag did not scroll: {}",
            probe.offset
        );
        let range = egui::TextEdit::load_state(&probe.ctx, probe.id)
            .unwrap()
            .cursor
            .char_range()
            .unwrap();
        assert!(range.primary.index > range.secondary.index + 100);
    }

    #[test]
    fn message_overflow_stays_inside_its_viewport() {
        let ctx = egui::Context::default();
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(300.0, 200.0),
                )),
                ..Default::default()
            },
            |ui| {
                egui::CentralPanel::default().show(ui, |ui| {
                    let top = ui.cursor().top();
                    let height = ui.text_style_height(&TextStyle::Body) + SPACE_2;
                    message(
                        ui,
                        "overflow-test",
                        &"long message\n".repeat(100),
                        Color32::WHITE,
                    );
                    assert!(ui.min_rect().width() <= 284.0);
                    assert!(ui.cursor().top() - top <= height + ui.spacing().item_spacing.y + 1.0);
                });
            },
        );
        output.textures_delta.clear();
    }

    #[test]
    fn focused_settings_field_paints_a_visible_outline() {
        for theme in [egui::Theme::Light, egui::Theme::Dark] {
            let ctx = egui::Context::default();
            ctx.set_theme(theme);
            apply(&ctx);
            let mut text = "Settings value".to_owned();
            let id = egui::Id::new("focused-field");
            let mut output = ctx.run_ui(egui::RawInput::default(), |ui| {
                ui.memory_mut(|memory| memory.request_focus(id));
                settings_edit(ui, egui::TextEdit::singleline(&mut text).id(id));
            });
            let color = ctx.style_of(theme).visuals.hyperlink_color;
            assert!(output.shapes.iter().any(|shape| matches!(
                &shape.shape,
                egui::epaint::Shape::Rect(rect)
                    if rect.stroke.width == FOCUS_RING_WIDTH && rect.stroke.color == color
            )));
            output.textures_delta.clear();
        }
    }

    #[test]
    fn light_appearance_uses_default_visuals() {
        let ctx = egui::Context::default();
        apply(&ctx);
        let actual = &ctx.style_of(egui::Theme::Light).visuals;
        let expected = egui::Visuals::light();
        assert_eq!(actual.selection.bg_fill, expected.selection.bg_fill);
        assert_eq!(actual.selection.stroke, expected.selection.stroke);
        assert_eq!(actual.panel_fill, expected.panel_fill);
        assert_eq!(palette(false).accent, expected.selection.bg_fill);
    }

    #[test]
    fn text_contrasts_with_its_background() {
        for palette in [LIGHT, DARK] {
            let background = palette.window;
            let label = palette.label;
            let difference = (label.r() as i32 - background.r() as i32).abs()
                + (label.g() as i32 - background.g() as i32).abs()
                + (label.b() as i32 - background.b() as i32).abs();
            assert!(difference > 200, "insufficient contrast: {difference}");
        }
    }

    #[test]
    fn spacing_follows_the_four_point_scale() {
        for value in [
            SPACE_1, SPACE_2, SPACE_3, SPACE_4, SPACE_5, SPACE_6, SPACE_7,
        ] {
            assert_eq!(value % 4.0, 0.0, "{value} is off the 4-point scale");
        }
    }

    #[test]
    fn the_window_opens_at_the_reference_size() {
        assert_eq!(INITIAL_SIZE, Vec2::new(760.0, 860.0));
        // The minimum must be smaller than the initial size on both axes.
        assert_eq!(MIN_SIZE, Vec2::new(560.0, 720.0));
    }
}
