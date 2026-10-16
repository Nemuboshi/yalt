//! Language list, provider code mapping, local detection, and target selection.
//!
//! Detection uses `whatlang`, a trigram/script classifier whose models are a
//! few hundred kilobytes rather than a few hundred megabytes. The previous
//! implementation used `lingua`, whose per-language models dominated the
//! binary; `whatlang` was chosen after measuring both against the corpus below.
//!
//! # Uncertainty
//!
//! Confidence values are not calibrated probabilities, and measurement on the
//! supported corpus shows that no confidence threshold separates correct from
//! incorrect detections: a confident kanji-only Japanese sample can be read as
//! Chinese, while a correct short sample can score near zero. What the
//! threshold below *can* do is flag very short or weakly attested input so the
//! UI can offer "Language uncertain"; it never overrides a manual selection.
//!
//! Two further guards make a detection usable as a provider language code:
//!
//! - `whatlang` classifies against all 70-odd languages it knows, several of
//!   which the selectors do not offer. A result outside [`LANGUAGES`] is
//!   reported as uncertain rather than silently passed to the provider.
//! - Scripts that identify a single language are reported at full confidence
//!   without looking at the text, so `12345` is not a language. The confidence
//!   floor rejects those.

use std::sync::OnceLock;

use whatlang::{Detector, Lang};

/// Confidence floor below which a detection is reported as uncertain.
///
/// Measured against the supported corpus. Ordinary sentences score at or above
/// this; the samples it catches are one or two common words (`"hello"` scores
/// 0.03), or text whose script alone decided the language (a bare `"12345"`
/// reports English at 1.0, because the Latin script maps to English when no
/// other language fits). It is a heuristic, not a calibrated probability.
const UNCERTAINTY_FLOOR: f64 = 0.30;

/// A language offered in the source and target selectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Language {
    /// Provider language identifier, such as `en` or `ukr_cyrl`.
    pub code: &'static str,
    /// Human-readable English name.
    pub name: &'static str,
    /// The detector's own identifier for this language.
    ///
    /// ISO 639-3, because that is what `whatlang` reports; it differs from the
    /// provider code for most languages.
    pub detector_code: &'static str,
}

/// Languages offered in the selectors, matching the previous implementation.
pub const LANGUAGES: &[Language] = &[
    Language {
        code: "en",
        name: "English",
        detector_code: "eng",
    },
    Language {
        code: "zh",
        name: "Chinese",
        detector_code: "cmn",
    },
    Language {
        code: "de",
        name: "German",
        detector_code: "deu",
    },
    Language {
        code: "fr",
        name: "French",
        detector_code: "fra",
    },
    Language {
        code: "es",
        name: "Spanish",
        detector_code: "spa",
    },
    Language {
        code: "ja",
        name: "Japanese",
        detector_code: "jpn",
    },
    Language {
        code: "ko",
        name: "Korean",
        detector_code: "kor",
    },
    Language {
        code: "pt",
        name: "Portuguese",
        detector_code: "por",
    },
    Language {
        code: "ru",
        name: "Russian",
        detector_code: "rus",
    },
    Language {
        code: "ar",
        name: "Arabic",
        detector_code: "ara",
    },
    Language {
        code: "it",
        name: "Italian",
        detector_code: "ita",
    },
    Language {
        code: "nl",
        name: "Dutch",
        detector_code: "nld",
    },
    Language {
        code: "pl",
        name: "Polish",
        detector_code: "pol",
    },
    Language {
        code: "ro",
        name: "Romanian",
        detector_code: "ron",
    },
    Language {
        code: "sv",
        name: "Swedish",
        detector_code: "swe",
    },
    Language {
        code: "tr",
        name: "Turkish",
        detector_code: "tur",
    },
    Language {
        code: "hi",
        name: "Hindi",
        detector_code: "hin",
    },
    Language {
        code: "vi",
        name: "Vietnamese",
        detector_code: "vie",
    },
    Language {
        code: "th",
        name: "Thai",
        detector_code: "tha",
    },
    Language {
        code: "id",
        name: "Indonesian",
        detector_code: "ind",
    },
    Language {
        code: "ukr_cyrl",
        name: "Ukrainian",
        detector_code: "ukr",
    },
    Language {
        code: "fas_arab",
        name: "Persian",
        detector_code: "pes",
    },
    Language {
        code: "ces_latn",
        name: "Czech",
        detector_code: "ces",
    },
    Language {
        code: "ell_grek",
        name: "Greek",
        detector_code: "ell",
    },
    Language {
        code: "dan_latn",
        name: "Danish",
        detector_code: "dan",
    },
    Language {
        code: "hun_latn",
        name: "Hungarian",
        detector_code: "hun",
    },
    Language {
        code: "fin_latn",
        name: "Finnish",
        detector_code: "fin",
    },
    Language {
        code: "nob_latn",
        name: "Norwegian",
        detector_code: "nob",
    },
    Language {
        code: "slk_latn",
        name: "Slovak",
        detector_code: "slk",
    },
    Language {
        code: "bul_cyrl",
        name: "Bulgarian",
        detector_code: "bul",
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
///
/// The detector is restricted to the offered languages so it cannot return a
/// language that has no provider code; the explicit check in [`detect`] is the
/// backstop for the scripts that bypass the candidate list.
fn detector() -> &'static Detector {
    static DETECTOR: OnceLock<Detector> = OnceLock::new();
    DETECTOR.get_or_init(|| Detector::with_allowlist(offered_languages()))
}

/// The detector's identifiers for every offered language.
fn offered_languages() -> Vec<Lang> {
    LANGUAGES
        .iter()
        .filter_map(|language| Lang::from_code(language.detector_code))
        .collect()
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
    let Some(info) = detector().detect(text) else {
        return uncertain();
    };
    let confidence = info.confidence();
    // A language the selectors do not offer cannot be named or targeted, so it
    // is reported the same way as no detection at all.
    let Some(code) = provider_code_for(info.lang().code()) else {
        return Detection {
            confidence,
            ..uncertain()
        };
    };
    Detection {
        name: info.lang().eng_name().to_owned(),
        uncertain: confidence < UNCERTAINTY_FLOOR,
        target: target_for_language(&code).to_owned(),
        code,
        confidence,
    }
}

/// The provider code for a detector identifier, when the language is offered.
fn provider_code_for(detector_code: &str) -> Option<String> {
    LANGUAGES
        .iter()
        .find(|language| language.detector_code == detector_code)
        .map(|language| language.code.to_owned())
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
    fn a_language_outside_the_offered_list_is_uncertain() {
        // A script that identifies one language is reported without consulting
        // the candidate list, so the detector can name a language the
        // selectors do not offer. Bengali is one; it has no provider code, so
        // it must be reported the same way as no detection at all.
        let sample = "বাংলা ভাষা দক্ষিণ এশিয়ার একটি গুরুত্বপূর্ণ ভাষা।";
        let info = Detector::new()
            .detect(sample)
            .expect("the detector classified the sample");
        assert_eq!(info.lang().code(), "ben");
        assert_eq!(provider_code_for("ben"), None);
        let got = detect(sample);
        assert!(got.uncertain, "got {got:?}");
        assert!(got.code.is_empty(), "got {got:?}");
    }

    #[test]
    fn detector_codes_match_the_offered_languages() {
        // Every offered language must be resolvable by the detector, and no
        // two may share an identifier.
        for language in LANGUAGES {
            assert!(
                Lang::from_code(language.detector_code).is_some(),
                "{} names an unknown detector language {}",
                language.code,
                language.detector_code
            );
        }
        for (index, language) in LANGUAGES.iter().enumerate() {
            let duplicates = LANGUAGES[index + 1..]
                .iter()
                .filter(|other| other.detector_code == language.detector_code)
                .count();
            assert_eq!(duplicates, 0, "{} is listed twice", language.detector_code);
        }
    }

    #[test]
    fn maps_detector_codes_to_provider_codes() {
        assert_eq!(provider_code_for("ukr").as_deref(), Some("ukr_cyrl"));
        assert_eq!(provider_code_for("pes").as_deref(), Some("fas_arab"));
        assert_eq!(provider_code_for("ces").as_deref(), Some("ces_latn"));
        assert_eq!(provider_code_for("ell").as_deref(), Some("ell_grek"));
        assert_eq!(provider_code_for("dan").as_deref(), Some("dan_latn"));
        assert_eq!(provider_code_for("hun").as_deref(), Some("hun_latn"));
        assert_eq!(provider_code_for("fin").as_deref(), Some("fin_latn"));
        assert_eq!(provider_code_for("nob").as_deref(), Some("nob_latn"));
        assert_eq!(provider_code_for("slk").as_deref(), Some("slk_latn"));
        assert_eq!(provider_code_for("bul").as_deref(), Some("bul_cyrl"));
        assert_eq!(provider_code_for("eng").as_deref(), Some("en"));
        // Languages the detector knows but the selectors do not offer.
        assert_eq!(provider_code_for("epo"), None);
        assert_eq!(provider_code_for("ben"), None);
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
