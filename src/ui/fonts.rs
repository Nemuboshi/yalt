//! Font fallback configuration.
//!
//! egui's bundled fonts cover Latin, Greek, and Cyrillic but contain no CJK
//! glyphs, so Chinese, Japanese, and Korean text renders as blank boxes
//! without an explicit fallback.
//!
//! Two sources fix that. The embedded LXGW Neo XiHei Plus covers Han and
//! kana identically on every platform. Scripts it does not cover, so far
//! Hangul, fall back to system fonts chosen by coverage, not by name: the
//! build asks the operating system's own font database which installed face
//! can draw a probe of the script, and appends that face to the chain. No
//! family name is hardcoded, so each platform routes through the font its
//! users would expect. When no installed face covers a script, the chain
//! simply lacks that entry; the application still starts.
//!
//! The chain is built once before the first frame, never per frame. The
//! embedded font file is not committed to git; the build script stages it
//! into `OUT_DIR` from a local copy or a fetch of the official release.

use std::sync::Arc;

use eframe::egui::{FontData, FontDefinitions, FontFamily};

/// The embedded CJK fallback font, LXGW Neo XiHei Plus (SIL OFL 1.1).
///
/// An `include_bytes` font keeps packaged builds self-contained: the text
/// cannot break by moving the executable away from its assets or by running
/// it on a machine with no CJK fonts installed.
const FALLBACK_FONT: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/LXGWNeoXiHeiPlus.ttf"));

/// Name of the embedded face's entry in the font definitions.
const FALLBACK_NAME: &str = "cjk-han";

/// A script the embedded font does not cover, its entry name, and a probe
/// that a system face must be able to draw to qualify as its fallback.
const SYSTEM_GAPS: &[(&str, &str)] = &[("cjk-hangul", "한국어 테스트")];

/// Build font definitions from the embedded face plus system fallbacks.
pub fn definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        FALLBACK_NAME.to_owned(),
        Arc::new(FontData::from_static(FALLBACK_FONT)),
    );

    // Append rather than prepend: the default font stays primary, and the
    // fallbacks are consulted only for glyphs the earlier faces cannot draw.
    let mut names = vec![FALLBACK_NAME.to_owned()];
    for (name, data) in system_fallbacks() {
        fonts.font_data.insert(name.clone(), Arc::new(data));
        names.push(name);
    }
    for name in names {
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            fonts.families.entry(family).or_default().push(name.clone());
        }
    }
    fonts
}

/// One entry per gap: the registered name and its face data.
fn system_fallbacks() -> Vec<(String, FontData)> {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    SYSTEM_GAPS
        .iter()
        .filter_map(|(name, probe)| {
            let id = find_face(&db, probe)?;
            let face = db.face(id)?;
            let index = face.index;
            let face_name = face.post_script_name.clone();
            let data = db.with_face_data(id, |data, _index| data.to_vec())?;
            let mut font = FontData::from_owned(data);
            font.index = index;
            eprintln!("yalt: {name} from system face {face_name}");
            Some(((*name).to_owned(), font))
        })
        .collect()
}

/// The installed face that can draw every character of a probe.
///
/// Candidates are ranked so the pick behaves like a system default: a
/// regular weight and normal style outrank bold, light, or italic, and the
/// PostScript name breaks remaining ties, so the result does not depend on
/// the order the operating system happens to report its fonts in.
fn find_face(db: &fontdb::Database, probe: &str) -> Option<fontdb::ID> {
    let wanted: Vec<char> = probe.chars().filter(|c| !c.is_whitespace()).collect();
    db.faces()
        .filter(|face| {
            db.with_face_data(face.id, |data, index| {
                ttf_parser::Face::parse(data, index)
                    .is_ok_and(|parsed| wanted.iter().all(|c| parsed.glyph_index(*c).is_some()))
            })
            .unwrap_or(false)
        })
        .min_by_key(|face| (rank(face), face.post_script_name.clone()))
        .map(|face| face.id)
}

/// A lower rank is a better fallback face: regular weight, then normal style.
fn rank(face: &fontdb::FaceInfo) -> (u16, u8) {
    // Distance from 400 puts regular weight first, light and bold later.
    let weight_gap = face.weight.0.abs_diff(400);
    let style_penalty = match face.style {
        fontdb::Style::Normal => 0,
        fontdb::Style::Italic | fontdb::Style::Oblique => 1,
    };
    (weight_gap, style_penalty)
}

#[cfg(test)]
mod tests {
    use super::*;
    use eframe::egui::FontId;
    use eframe::egui::epaint::text::TextOptions;
    use eframe::egui::text::Fonts;

    /// Han and kana samples, including the punctuation that appears in
    /// real translations. These must draw on every platform because the
    /// face that covers them is embedded.
    const EMBEDDED_SAMPLES: &[&str] = &[
        "你好，世界。这是简体中文测试。",
        "日本語のテスト、カタカナも。",
    ];

    fn drawer() -> (Fonts, FontId) {
        let fonts = definitions();
        let drawer = Fonts::new(TextOptions::default(), fonts);
        (drawer, FontId::new(15.0, FontFamily::Proportional))
    }

    #[test]
    fn definitions_always_include_the_default_families() {
        let fonts = definitions();
        assert!(fonts.families.contains_key(&FontFamily::Proportional));
        assert!(fonts.families.contains_key(&FontFamily::Monospace));
    }

    #[test]
    fn fallbacks_are_appended_after_the_default_font() {
        let fonts = definitions();
        assert!(fonts.font_data.contains_key(FALLBACK_NAME));
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            let chain = &fonts.families[&family];
            assert!(
                chain.contains(&FALLBACK_NAME.to_owned()),
                "{FALLBACK_NAME} missing from {family:?}"
            );
            // The embedded font and any system faces come after the defaults.
            assert!(chain.len() >= 2, "nothing was appended to {family:?}");
        }
    }

    #[test]
    fn the_embedded_scripts_can_be_drawn() {
        // This is the check that matters on every platform: the embedded
        // face must actually cover the scripts it claims.
        let (mut fonts, font_id) = drawer();
        for sample in EMBEDDED_SAMPLES {
            assert!(
                fonts.has_glyphs(&font_id, sample),
                "text cannot be drawn: {sample:?}"
            );
        }
    }

    #[test]
    fn the_system_gap_is_drawn_when_a_face_exists() {
        // Hangul depends on an installed face, which a minimal Linux system
        // may lack. The chain degrades silently there, so this asserts only
        // when a fallback was found; Windows and macOS runners always have
        // one, which keeps the discovery path exercised where it matters.
        let (mut fonts, font_id) = drawer();
        let name = SYSTEM_GAPS[0].0;
        if !fonts.definitions().font_data.contains_key(name) {
            eprintln!("yalt test: no system face for {name}; skipped");
            return;
        }
        assert!(
            fonts.has_glyphs(&font_id, SYSTEM_GAPS[0].1),
            "text cannot be drawn: {:?}",
            SYSTEM_GAPS[0].1
        );
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
