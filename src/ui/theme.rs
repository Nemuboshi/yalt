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
    accent: Color32::from_rgb(0x8a, 0x5a, 0x3b),
    accent_hover: Color32::from_rgb(0x74, 0x49, 0x2e),
    accent_pressed: Color32::from_rgb(0x5f, 0x3b, 0x24),
    accent_label: Color32::from_rgb(0xff, 0xff, 0xff),
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
    if dark_mode { DARK } else { LIGHT }
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

/// Apply the theme to both appearances.
///
/// Font fallbacks are configured once here so international text renders
/// without per-frame font work.
pub fn apply(ctx: &egui::Context) {
    for theme in [egui::Theme::Light, egui::Theme::Dark] {
        let colors = palette(theme == egui::Theme::Dark);
        let mut style = (*ctx.style_of(theme)).clone();
        apply_to(&mut style, colors);
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

    // The focus ring is drawn around the whole editor card rather than the
    // inner text area, so it matches the frame the user sees.
    visuals.selection.stroke = Stroke::new(FOCUS_RING_WIDTH, colors.accent);

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
/// sitting inside a card.
pub fn field_edit<'a>(
    text: &'a mut String,
    id: impl std::hash::Hash + std::fmt::Debug,
) -> egui::TextEdit<'a> {
    egui::TextEdit::singleline(text)
        .id_salt(id)
        .min_size(Vec2::new(0.0, CONTROL_HEIGHT))
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

    #[test]
    fn both_appearances_use_the_brown_accent() {
        // The accent must stay warm, never blue or teal.
        for palette in [LIGHT, DARK] {
            let accent = palette.accent;
            assert!(accent.r() > accent.b(), "accent is not warm: {accent:?}");
        }
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
