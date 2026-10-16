//! Font fallback configuration.
//!
//! egui's bundled fonts cover Latin, Greek, and Cyrillic but contain no CJK
//! glyphs, so Chinese, Japanese, and Korean text renders as blank boxes
//! without an explicit fallback.
//!
//! One font fixes that: LXGW Neo XiHei Plus is embedded in the binary and
//! covers Han, kana, and Hangul, so the same face renders every supported
//! script on every platform. Embedding it means the application does not
//! depend on which fonts happen to be installed, and the chain is built
//! once at startup, not per frame.
//!
//! The font file itself is not committed to git. The build script stages a
//! copy in the build directory, from a local install or a fetch of the
//! official release, and it is read from there below.

use std::sync::Arc;

use eframe::egui::{FontData, FontDefinitions, FontFamily};

/// The embedded fallback font, LXGW Neo XiHei Plus (SIL OFL 1.1).
///
/// An `include_bytes` font keeps packaged builds self-contained: the text
/// cannot break by moving the executable away from its assets or by running
/// it on a machine with no CJK fonts installed. The path is the stage the
/// build script fills.
const FALLBACK_FONT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/LXGWNeoXiHeiPlus.ttf"));

/// Name of the fallback entry in the font definitions.
const FALLBACK_NAME: &str = "cjk-fallback";

/// Build font definitions with the embedded CJK face appended.
pub fn definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        FALLBACK_NAME.to_owned(),
        Arc::new(FontData::from_static(FALLBACK_FONT)),
    );

    // Append rather than prepend: the default font stays primary, and the
    // fallback is consulted only for glyphs the earlier faces cannot draw.
    for family in [FontFamily::Proportional, FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .push(FALLBACK_NAME.to_owned());
    }
    fonts
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::FontId;
    use eframe::egui::epaint::text::TextOptions;
    use eframe::egui::text::Fonts;

    /// Sample text for each script, including the punctuation that appears
    /// in real translations.
    const SAMPLES: &[&str] = &[
        "你好，世界。这是简体中文测试。",
        "日本語のテスト、カタカナも。",
        "한국어 테스트",
        "안녕하세요",
    ];

    #[test]
    fn definitions_always_include_the_default_families() {
        let fonts = definitions();
        assert!(fonts.families.contains_key(&FontFamily::Proportional));
        assert!(fonts.families.contains_key(&FontFamily::Monospace));
    }

    #[test]
    fn fallback_is_appended_after_the_default_font() {
        let fonts = definitions();
        let proportional = &fonts.families[&FontFamily::Proportional];
        assert!(
            proportional.contains(&FALLBACK_NAME.to_owned()),
            "{FALLBACK_NAME} was not added to the chain"
        );
        assert_eq!(
            proportional.last().map(String::as_str),
            Some(FALLBACK_NAME)
        );
        let monospace = &fonts.families[&FontFamily::Monospace];
        assert_eq!(monospace.last().map(String::as_str), Some(FALLBACK_NAME));
    }

    #[test]
    fn every_script_can_be_drawn() {
        // This is the check that matters: the embedded face must actually
        // cover every script, including the punctuation.
        let fonts = definitions();
        let mut fonts = Fonts::new(TextOptions::default(), fonts);
        let font_id = FontId::new(15.0, FontFamily::Proportional);
        for sample in SAMPLES {
            assert!(
                fonts.has_glyphs(&font_id, sample),
                "text cannot be drawn: {sample:?}"
            );
        }
    }

    #[test]
    fn the_embedded_font_is_a_readable_ttf() {
        // A corrupt or replaced asset must fail here, not silently in the UI.
        assert!(
            FALLBACK_FONT.starts_with(&[0x00, 0x01, 0x00, 0x00])
                || FALLBACK_FONT.starts_with(b"OTTO"),
            "the embedded font must be a TTF or OTF"
        );
    }
}
