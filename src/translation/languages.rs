//! Language list, provider code mapping, local detection, and target selection.
//!
//! Detection uses the `lingua` detector in low-accuracy mode, matching the
//! language corpus and provider codes of the previous Go implementation.
//!
//! # Uncertainty
//!
//! Confidence values are not calibrated probabilities, and measurement on the
//! supported corpus shows that no confidence threshold separates correct from
//! incorrect detections: correct short inputs can score as low as 0.492 while
//! confidently wrong results (a kanji-only Japanese sample, or Russian read as
//! Bulgarian) score up to 1.000. The floor below is therefore a deliberate,
//! measured refinement rather than a correctness guarantee. It flags very short
//! or weakly attested input so the UI can offer "Language uncertain", and it
//! never overrides an explicit manual selection.

use std::sync::OnceLock;

use lingua::{LanguageDetector, LanguageDetectorBuilder};

/// Confidence floor below which a detection is reported as uncertain.
///
/// Measured against the supported corpus: reliable detections of ordinary text
/// score at or above 0.49, while the least reliable short samples (one or two
/// common words) score between 0.26 and 0.29. This floor catches the latter.
const UNCERTAINTY_FLOOR: f64 = 0.30;

/// A language offered in the source and target selectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Language {
    /// Provider language identifier, such as `en` or `ukr_cyrl`.
    pub code: &'static str,
    /// Human-readable English name.
    pub name: &'static str,
}

/// Languages offered in the selectors, matching the previous implementation.
pub const LANGUAGES: &[Language] = &[
    Language {
        code: "en",
        name: "English",
    },
    Language {
        code: "zh",
        name: "Chinese",
    },
    Language {
        code: "de",
        name: "German",
    },
    Language {
        code: "fr",
        name: "French",
    },
    Language {
        code: "es",
        name: "Spanish",
    },
    Language {
        code: "ja",
        name: "Japanese",
    },
    Language {
        code: "ko",
        name: "Korean",
    },
    Language {
        code: "pt",
        name: "Portuguese",
    },
    Language {
        code: "ru",
        name: "Russian",
    },
    Language {
        code: "ar",
        name: "Arabic",
    },
    Language {
        code: "it",
        name: "Italian",
    },
    Language {
        code: "nl",
        name: "Dutch",
    },
    Language {
        code: "pl",
        name: "Polish",
    },
    Language {
        code: "ro",
        name: "Romanian",
    },
    Language {
        code: "sv",
        name: "Swedish",
    },
    Language {
        code: "tr",
        name: "Turkish",
    },
    Language {
        code: "hi",
        name: "Hindi",
    },
    Language {
        code: "vi",
        name: "Vietnamese",
    },
    Language {
        code: "th",
        name: "Thai",
    },
    Language {
        code: "id",
        name: "Indonesian",
    },
    Language {
        code: "ukr_cyrl",
        name: "Ukrainian",
    },
    Language {
        code: "fas_arab",
        name: "Persian",
    },
    Language {
        code: "ces_latn",
        name: "Czech",
    },
    Language {
        code: "ell_grek",
        name: "Greek",
    },
    Language {
        code: "dan_latn",
        name: "Danish",
    },
    Language {
        code: "hun_latn",
        name: "Hungarian",
    },
    Language {
        code: "fin_latn",
        name: "Finnish",
    },
    Language {
        code: "nob_latn",
        name: "Norwegian",
    },
    Language {
        code: "slk_latn",
        name: "Slovak",
    },
    Language {
        code: "bul_cyrl",
        name: "Bulgarian",
    },
];

/// Chinese names used when building prompts, matching the previous behavior.
const LANGUAGE_NAMES: &[(&str, &str)] = &[
    ("en", "英语"),
    ("zh", "中文"),
    ("de", "德语"),
    ("fr", "法语"),
    ("es", "西班牙语"),
    ("ja", "日语"),
    ("ko", "韩语"),
    ("pt", "葡萄牙语"),
    ("ru", "俄语"),
    ("ar", "阿拉伯语"),
    ("it", "意大利语"),
    ("nl", "荷兰语"),
    ("pl", "波兰语"),
    ("ro", "罗马尼亚语"),
    ("sv", "瑞典语"),
    ("tr", "土耳其语"),
    ("hi", "印地语"),
    ("vi", "越南语"),
    ("th", "泰语"),
    ("id", "印尼语"),
    ("ukr_cyrl", "乌克兰语"),
    ("fas_arab", "波斯语"),
    ("ces_latn", "捷克语"),
    ("ell_grek", "希腊语"),
    ("dan_latn", "丹麦语"),
    ("hun_latn", "匈牙利语"),
    ("fin_latn", "芬兰语"),
    ("nob_latn", "书面挪威语"),
    ("slk_latn", "斯洛伐克语"),
    ("bul_cyrl", "保加利亚语"),
];

/// Result of a local language detection.
#[derive(Debug, Clone, PartialEq)]
pub struct Detection {
    /// Provider language code, empty when no language could be determined.
    pub code: String,
    /// Detector name for the language, or `Unknown`.
    pub name: String,
    /// Whether the result is too weak to assert.
    pub uncertain: bool,
    /// Relative detector confidence. Not a calibrated probability.
    pub confidence: f64,
    /// Target language implied by this detection.
    pub target: String,
}

/// Shared detector instance, built once on first use.
fn detector() -> &'static LanguageDetector {
    static DETECTOR: OnceLock<LanguageDetector> = OnceLock::new();
    DETECTOR.get_or_init(|| {
        LanguageDetectorBuilder::from_all_spoken_languages()
            .with_low_accuracy_mode()
            .build()
    })
}

/// Detect the language of `text` locally.
///
/// Returns an uncertain detection for empty or non-linguistic input, and never
/// performs any network work.
pub fn detect(text: &str) -> Detection {
    let text = text.trim();
    if text.is_empty() {
        return uncertain();
    }
    let detector = detector();
    // `None` means the detector could not classify the input.
    let Some(language) = detector.detect_language_of(text) else {
        return uncertain();
    };
    let confidence = detector
        .compute_language_confidence_values(text)
        .first()
        .map(|(_, value)| *value)
        .unwrap_or(0.0);
    let code = provider_language_code(&language.iso_code_639_1().to_string());
    Detection {
        name: language.to_string(),
        uncertain: confidence < UNCERTAINTY_FLOOR,
        target: target_for_language(&code).to_owned(),
        code,
        confidence,
    }
}

/// The uncertain result: no code, no name, target Chinese.
fn uncertain() -> Detection {
    Detection {
        code: String::new(),
        name: "Unknown".to_owned(),
        uncertain: true,
        confidence: 0.0,
        target: "zh".to_owned(),
    }
}

/// Map a bare ISO 639-1 code to the provider's language identifier.
pub fn provider_language_code(code: &str) -> String {
    let code = code.to_lowercase();
    match code.as_str() {
        "uk" => "ukr_cyrl",
        "fa" => "fas_arab",
        "cs" => "ces_latn",
        "el" => "ell_grek",
        "da" => "dan_latn",
        "hu" => "hun_latn",
        "fi" => "fin_latn",
        "nb" | "nn" => "nob_latn",
        "sk" => "slk_latn",
        "bg" => "bul_cyrl",
        "tl" => "fil",
        other => other,
    }
    .to_owned()
}

/// Target implied by a detected language: Chinese selects English, else Chinese.
pub fn target_for_language(code: &str) -> &'static str {
    if code.eq_ignore_ascii_case("zh") {
        "en"
    } else {
        "zh"
    }
}

/// Resolve the target language for a request.
///
/// A manual target always wins. A manual source selects English for Chinese and
/// Chinese otherwise. With both automatic, the detection decides.
pub fn select_target(source: &str, target: &str, detection: &Detection) -> String {
    if !target.is_empty() && !target.eq_ignore_ascii_case("auto") {
        return target.to_owned();
    }
    if !source.is_empty() && !source.eq_ignore_ascii_case("auto") {
        return target_for_language(source).to_owned();
    }
    target_for_language(&detection.code).to_owned()
}

/// Chinese display name for a language code, falling back to the code itself.
pub fn language_name(code: &str) -> String {
    let lowered = code.to_lowercase();
    LANGUAGE_NAMES
        .iter()
        .find(|(candidate, _)| *candidate == lowered)
        .map(|(_, name)| (*name).to_owned())
        .unwrap_or_else(|| code.to_owned())
}

/// Whether `code` names a language in the offered list.
pub fn is_known_language(code: &str) -> bool {
    LANGUAGES.iter().any(|language| language.code == code)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_the_reference_corpus() {
        let cases = [
            ("今天天气很好，我们一起去公园散步吧。", "zh"),
            ("今天天氣很好，我們一起去公園散步吧。", "zh"),
            ("今日は天気が良いので、一緒に公園を散歩しましょう。", "ja"),
            (
                "The weather is lovely today. Let us take a walk in the park.",
                "en",
            ),
            (
                "Le temps est magnifique aujourd'hui. Allons nous promener au parc.",
                "fr",
            ),
            (
                "Сьогодні чудова погода. Давайте прогуляємося в парку.",
                "ukr_cyrl",
            ),
        ];
        for (text, want) in cases {
            let got = detect(text);
            assert_eq!(got.code, want, "text = {text:?}, name = {}", got.name);
            assert!(!got.uncertain, "detector marked {text:?} uncertain");
            assert!(
                got.confidence > 0.0,
                "confidence for {text:?} was not positive"
            );
        }
    }

    #[test]
    fn distinguishes_japanese_from_chinese() {
        // Kana must not be read as Chinese merely because Han characters appear.
        assert_eq!(detect("これはテストです").code, "ja");
        assert_eq!(detect("今日はいい天気").code, "ja");
    }

    #[test]
    fn empty_input_has_no_language() {
        let got = detect(" \t\n");
        assert!(got.uncertain);
        assert!(got.code.is_empty());
        assert_eq!(got.target, "zh");
    }

    #[test]
    fn non_linguistic_input_is_uncertain() {
        for text in ["12345 67890", "😀🎉👍", "!!!", "a"] {
            let got = detect(text);
            assert!(got.uncertain, "{text:?} should be uncertain, got {got:?}");
        }
    }

    #[test]
    fn short_ambiguous_input_is_uncertain() {
        // Measured: these score below the floor and must not be asserted.
        for text in ["hello", "hallo", "привет"] {
            let got = detect(text);
            assert!(got.uncertain, "{text:?} should be uncertain, got {got:?}");
        }
    }

    #[test]
    fn maps_iso_codes_to_provider_codes() {
        assert_eq!(provider_language_code("uk"), "ukr_cyrl");
        assert_eq!(provider_language_code("fa"), "fas_arab");
        assert_eq!(provider_language_code("cs"), "ces_latn");
        assert_eq!(provider_language_code("el"), "ell_grek");
        assert_eq!(provider_language_code("da"), "dan_latn");
        assert_eq!(provider_language_code("hu"), "hun_latn");
        assert_eq!(provider_language_code("fi"), "fin_latn");
        assert_eq!(provider_language_code("nb"), "nob_latn");
        assert_eq!(provider_language_code("nn"), "nob_latn");
        assert_eq!(provider_language_code("sk"), "slk_latn");
        assert_eq!(provider_language_code("bg"), "bul_cyrl");
        assert_eq!(provider_language_code("tl"), "fil");
        assert_eq!(provider_language_code("en"), "en");
    }

    #[test]
    fn manual_target_wins() {
        let detection = Detection {
            code: "zh".into(),
            ..uncertain()
        };
        assert_eq!(select_target("auto", "de", &detection), "de");
    }

    #[test]
    fn automatic_target_follows_source_and_detection() {
        let chinese = Detection {
            code: "zh".into(),
            ..uncertain()
        };
        let english = Detection {
            code: "en".into(),
            ..uncertain()
        };
        // Manual source, automatic target.
        assert_eq!(select_target("zh", "auto", &english), "en");
        assert_eq!(select_target("en", "auto", &chinese), "zh");
        // Both automatic.
        assert_eq!(select_target("auto", "auto", &chinese), "en");
        assert_eq!(select_target("auto", "auto", &english), "zh");
        // Uncertain detection proposes Chinese.
        assert_eq!(select_target("auto", "auto", &uncertain()), "zh");
    }

    #[test]
    fn every_listed_language_has_a_name() {
        for language in LANGUAGES {
            assert_ne!(
                language_name(language.code),
                language.code,
                "{} has no display name",
                language.code
            );
        }
        assert_eq!(language_name("en"), "英语");
        assert_eq!(language_name("nob_latn"), "书面挪威语");
        // Unknown codes fall back to the code itself.
        assert_eq!(language_name("xx"), "xx");
    }
}
