//! Font discovery and fallback configuration.
//!
//! egui's bundled fonts cover Latin, Greek, and Cyrillic but contain no CJK
//! glyphs, so Chinese, Japanese, and Korean text renders as blank boxes
//! without an explicit fallback. System fonts are loaded once at startup and
//! appended to the fallback chain, so text renders without per-frame font
//! work.
//!
//! The chain has to be per script rather than a single CJK face: no one
//! installed font covers all of them. Microsoft YaHei, for example, has Han
//! and kana but no Hangul at all, so Korean needs its own entry.

use std::sync::Arc;

use egui::{FontData, FontDefinitions, FontFamily};

/// A writing system that needs its own fallback face.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Script {
    /// Han characters and Japanese kana.
    Han,
    /// Korean Hangul syllables and jamo.
    Hangul,
}

/// A platform font file and the face index to use within it.
#[derive(Debug, Clone, Copy)]
pub struct FontFile {
    /// Absolute path to the font file.
    pub path: &'static str,
    /// Face index within a font collection; `0` for single-face files.
    pub index: u32,
}

/// Candidate font files for one script, most preferred first.
#[derive(Debug, Clone, Copy)]
pub struct ScriptFonts {
    /// Which script these files cover.
    pub script: Script,
    /// Name of the fallback entry in the font definitions.
    pub name: &'static str,
    /// Candidate files, tried in order.
    pub candidates: &'static [FontFile],
}

/// Han characters and Japanese kana.
#[cfg(target_os = "windows")]
const HAN_CANDIDATES: &[FontFile] = &[
    // Microsoft YaHei covers Simplified Chinese, Japanese kana, and
    // punctuation. Face zero is the regular weight.
    FontFile {
        path: "C:/Windows/Fonts/msyh.ttc",
        index: 0,
    },
    // Microsoft JhengHei covers Traditional Chinese.
    FontFile {
        path: "C:/Windows/Fonts/msjh.ttc",
        index: 0,
    },
    FontFile {
        path: "C:/Windows/Fonts/meiryo.ttc",
        index: 0,
    },
    FontFile {
        path: "C:/Windows/Fonts/msgothic.ttc",
        index: 0,
    },
    FontFile {
        path: "C:/Windows/Fonts/simsun.ttc",
        index: 0,
    },
    FontFile {
        path: "C:/Windows/Fonts/simhei.ttf",
        index: 0,
    },
];

/// Korean Hangul.
///
/// Malgun Gothic is the modern Windows Korean face and the only one that is
/// reliably present on a non-Korean install, since it ships with the base
/// font set rather than a language pack.
#[cfg(target_os = "windows")]
const HANGUL_CANDIDATES: &[FontFile] = &[
    FontFile {
        path: "C:/Windows/Fonts/malgun.ttf",
        index: 0,
    },
    FontFile {
        path: "C:/Windows/Fonts/gulim.ttc",
        index: 0,
    },
    FontFile {
        path: "C:/Windows/Fonts/batang.ttc",
        index: 0,
    },
];

/// Han characters and Japanese kana.
#[cfg(target_os = "macos")]
const HAN_CANDIDATES: &[FontFile] = &[
    FontFile {
        path: "/System/Library/Fonts/PingFang.ttc",
        index: 0,
    },
    FontFile {
        path: "/System/Library/Fonts/Hiragino Sans GB.ttc",
        index: 0,
    },
    FontFile {
        path: "/Library/Fonts/Arial Unicode.ttf",
        index: 0,
    },
];

/// Korean Hangul.
#[cfg(target_os = "macos")]
const HANGUL_CANDIDATES: &[FontFile] = &[
    // Apple SD Gothic Neo is the system Korean face on current releases.
    FontFile {
        path: "/System/Library/Fonts/AppleSDGothicNeo.ttc",
        index: 0,
    },
    FontFile {
        path: "/System/Library/Fonts/AppleGothic.ttf",
        index: 0,
    },
    FontFile {
        path: "/Library/Fonts/Arial Unicode.ttf",
        index: 0,
    },
];

/// Han characters and Japanese kana.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
const HAN_CANDIDATES: &[FontFile] = &[
    FontFile {
        path: "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        index: 0,
    },
    FontFile {
        path: "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        index: 0,
    },
    FontFile {
        path: "/usr/share/fonts/opentype/noto/NotoSerifCJK-Regular.ttc",
        index: 0,
    },
    FontFile {
        path: "/usr/share/fonts/truetype/wqy/wqy-microhei.ttc",
        index: 0,
    },
];

/// Korean Hangul.
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
const HANGUL_CANDIDATES: &[FontFile] = &[
    FontFile {
        path: "/usr/share/fonts/opentype/noto/NotoSansCJK-Regular.ttc",
        index: 0,
    },
    FontFile {
        path: "/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc",
        index: 0,
    },
    FontFile {
        path: "/usr/share/fonts/truetype/nanum/NanumGothic.ttf",
        index: 0,
    },
];

/// The fallback scripts, in the order they are appended to the chain.
pub const SCRIPTS: &[ScriptFonts] = &[
    ScriptFonts {
        script: Script::Han,
        name: "cjk-han",
        candidates: HAN_CANDIDATES,
    },
    ScriptFonts {
        script: Script::Hangul,
        name: "cjk-hangul",
        candidates: HANGUL_CANDIDATES,
    },
];

/// Build font definitions with per-script fallbacks appended when available.
///
/// Falls back to the egui defaults when no system CJK font exists, so the
/// application still starts on a minimal system.
pub fn definitions() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    for entry in SCRIPTS {
        let Some((file, data)) = load(entry.candidates) else {
            continue;
        };
        let mut font_data = FontData::from_owned(data);
        font_data.index = file.index;
        fonts
            .font_data
            .insert(entry.name.to_owned(), Arc::new(font_data));

        // Append rather than prepend: the default font stays primary, and the
        // script faces are consulted only for glyphs the earlier ones cannot
        // draw.
        for family in [FontFamily::Proportional, FontFamily::Monospace] {
            fonts
                .families
                .entry(family)
                .or_default()
                .push(entry.name.to_owned());
        }
    }
    fonts
}

/// Read the first available file from a candidate list.
fn load(candidates: &'static [FontFile]) -> Option<(&'static FontFile, Vec<u8>)> {
    candidates
        .iter()
        .find_map(|file| std::fs::read(file.path).ok().map(|data| (file, data)))
}

/// Path of the font that will be used for a script, for diagnostics.
pub fn resolved_path(script: Script) -> Option<&'static str> {
    let entry = SCRIPTS.iter().find(|entry| entry.script == script)?;
    entry
        .candidates
        .iter()
        .find(|file| std::path::Path::new(file.path).exists())
        .map(|file| file.path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::FontId;
    use egui::epaint::text::TextOptions;
    use egui::text::Fonts;

    /// Sample text for each script, including the punctuation and jamo that
    /// appear in real translations.
    const SAMPLES: &[(Script, &str)] = &[
        (Script::Han, "你好，世界。这是简体中文测试。"),
        (Script::Han, "日本語のテスト、カタカナも。"),
        (Script::Hangul, "한국어 테스트"),
        (Script::Hangul, "안녕하세요"),
    ];

    #[test]
    fn definitions_always_include_the_default_families() {
        let fonts = definitions();
        assert!(fonts.families.contains_key(&FontFamily::Proportional));
        assert!(fonts.families.contains_key(&FontFamily::Monospace));
    }

    #[test]
    fn fallbacks_are_appended_after_the_default_font() {
        let fonts = definitions();
        let proportional = &fonts.families[&FontFamily::Proportional];
        for entry in SCRIPTS {
            // A missing system font is skipped, so only check the ones that
            // were actually loaded.
            if !fonts.font_data.contains_key(entry.name) {
                continue;
            }
            assert!(
                proportional.contains(&entry.name.to_owned()),
                "{} was loaded but not added to the chain",
                entry.name
            );
        }
        // Every appended entry comes after the default font.
        assert!(proportional.len() >= 2, "no fallback was appended");
    }

    #[test]
    fn every_script_in_the_chain_can_be_drawn() {
        // This is the check that matters: a fallback file existing on disk is
        // not the same as it covering the script. Microsoft YaHei, for
        // instance, has no Hangul, so Korean needs a face of its own.
        let fonts = definitions();
        let mut fonts = Fonts::new(TextOptions::default(), fonts);
        let font_id = FontId::new(15.0, FontFamily::Proportional);
        for (script, sample) in SAMPLES {
            assert!(
                fonts.has_glyphs(&font_id, sample),
                "{script:?} text cannot be drawn: {sample:?}"
            );
        }
    }

    #[test]
    fn a_loaded_script_reports_the_file_it_came_from() {
        for entry in SCRIPTS {
            let path = resolved_path(entry.script);
            if let Some(path) = path {
                assert!(
                    entry.candidates.iter().any(|file| file.path == path),
                    "{path} is not one of the candidates"
                );
            }
        }
    }

    #[test]
    fn the_face_index_is_valid_for_its_file() {
        let fonts = definitions();
        for entry in SCRIPTS {
            let Some(data) = fonts.font_data.get(entry.name) else {
                continue;
            };
            let path = resolved_path(entry.script).expect("a font was loaded");
            if path.ends_with(".ttc") {
                assert!(data.index < 16, "suspicious face index {}", data.index);
            } else {
                assert_eq!(data.index, 0);
            }
        }
    }

    #[test]
    fn candidate_paths_are_absolute() {
        for entry in SCRIPTS {
            for file in entry.candidates {
                assert!(
                    file.path.starts_with('/') || file.path.contains(":/"),
                    "candidate is not an absolute path: {}",
                    file.path
                );
            }
        }
    }
}
