//! Prompt construction and glossary handling.
//!
//! Prompt wording, instruction normalization, and ordered glossary formatting
//! match the previous implementation. Glossary import is stricter than before:
//! values must be strings, and entries are validated before they replace
//! existing terms.

use std::collections::HashSet;
use std::fmt;

use super::languages;

/// A glossary entry mapping a source term to its required translation.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Glossary {
    /// Term as it appears in the source text.
    pub source: String,
    /// Required translation of the term.
    pub target: String,
}

/// Reason a glossary was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GlossaryError {
    /// A term had an empty source or target.
    EmptyTerm,
    /// Two terms share the same source, compared case-insensitively.
    DuplicateSource(String),
    /// The imported JSON was not a valid object of string pairs.
    InvalidJson(String),
}

impl fmt::Display for GlossaryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyTerm => f.write_str("glossary terms must have source and target values"),
            Self::DuplicateSource(source) => {
                write!(f, "duplicate glossary source term {source:?}")
            }
            Self::InvalidJson(message) => write!(f, "invalid glossary JSON: {message}"),
        }
    }
}

impl std::error::Error for GlossaryError {}

/// Validate glossary terms: non-empty pairs with unique sources.
pub fn validate_glossary(glossary: &[Glossary]) -> Result<(), GlossaryError> {
    let mut seen = HashSet::with_capacity(glossary.len());
    for term in glossary {
        let source = term.source.trim();
        let target = term.target.trim();
        if source.is_empty() || target.is_empty() {
            return Err(GlossaryError::EmptyTerm);
        }
        if !seen.insert(source.to_lowercase()) {
            return Err(GlossaryError::DuplicateSource(source.to_owned()));
        }
    }
    Ok(())
}

/// Parse an imported glossary from a JSON object of string pairs.
///
/// Rejects malformed JSON, non-string values, empty terms, and duplicate
/// sources. Entries are returned in a stable order.
pub fn parse_glossary_json(input: &str) -> Result<Vec<Glossary>, GlossaryError> {
    let values: serde_json::Map<String, serde_json::Value> = serde_json::from_str(input)
        .map_err(|error| GlossaryError::InvalidJson(error.to_string()))?;
    let mut glossary = Vec::with_capacity(values.len());
    for (source, value) in values {
        let Some(target) = value.as_str() else {
            return Err(GlossaryError::InvalidJson(format!(
                "term {source:?} must map to a string"
            )));
        };
        glossary.push(Glossary {
            source,
            target: target.to_owned(),
        });
    }
    validate_glossary(&glossary)?;
    Ok(glossary)
}

/// Build the provider prompt for one request.
///
/// `target` is a resolved provider language code, never `auto`.
pub fn build_prompt(
    text: &str,
    target: &str,
    source: &str,
    instruction: &str,
    glossary: &[Glossary],
) -> String {
    let target_name = languages::language_name(target);
    let source_name = if source.is_empty() || source.eq_ignore_ascii_case("auto") {
        String::new()
    } else {
        languages::language_name(source)
    };

    let mut constraints: Vec<String> = Vec::new();
    if !glossary.is_empty() {
        let terms: Vec<String> = glossary
            .iter()
            .map(|term| format!("{}→{}", term.source.trim(), term.target.trim()))
            .collect();
        constraints.push(format!("【硬性要求】专名/术语对照: {}", terms.join("、")));
    }
    for line in instruction.trim().split('\n') {
        // Strip a leading list marker ("1. ", "2) ") then surrounding space,
        // matching the previous normalization.
        let line = line.trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ' ');
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line.starts_with('【') {
            constraints.push(line.to_owned());
        } else {
            constraints.push(format!("【注意】{line}"));
        }
    }

    let text = text.trim();
    if constraints.is_empty() {
        return if source_name.is_empty() {
            format!(
                "请将以下文本翻译为{target_name}，直接输出翻译结果，不要进行任何解释。\n\n{text}"
            )
        } else {
            format!(
                "请将以下{source_name}文本翻译为{target_name}，直接输出翻译结果，不要进行任何解释。\n\n{text}"
            )
        };
    }

    let header = if source_name.is_empty() {
        "文本".to_owned()
    } else {
        format!("{source_name}文本")
    };
    let lines: Vec<String> = constraints
        .iter()
        .enumerate()
        .map(|(index, constraint)| format!("{}. {}", index + 1, constraint))
        .collect();
    format!(
        "请将以下{header}翻译成{target_name}，并且严格遵循所有约束要求。\n\n【源文】\n{text}\n\n【约束要求】\n{}\n\n只输出译文，不要有任何额外说明。",
        lines.join("\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn term(source: &str, target: &str) -> Glossary {
        Glossary {
            source: source.into(),
            target: target.into(),
        }
    }

    #[test]
    fn builds_a_plain_prompt() {
        let prompt = build_prompt("Hello", "zh", "en", "", &[]);
        assert!(prompt.contains("英语文本"), "{prompt}");
        assert!(prompt.contains("中文"), "{prompt}");
        assert!(prompt.contains("Hello"), "{prompt}");
        assert!(!prompt.contains("约束"), "{prompt}");
    }

    #[test]
    fn builds_a_prompt_without_a_known_source() {
        let prompt = build_prompt("Hello", "zh", "auto", "", &[]);
        assert!(prompt.contains("请将以下文本翻译为中文"), "{prompt}");
    }

    #[test]
    fn includes_glossary_and_instruction_constraints() {
        let prompt = build_prompt("Hello", "zh", "en", "Keep JSON", &[term("API", "接口")]);
        for want in ["英语文本", "中文", "API→接口", "【注意】Keep JSON", "Hello"] {
            assert!(
                prompt.contains(want),
                "prompt is missing {want:?}: {prompt}"
            );
        }
    }

    #[test]
    fn normalizes_numbered_instructions() {
        let prompt = build_prompt("x", "zh", "auto", "1. First rule\n2. Second rule", &[]);
        assert!(prompt.contains("【注意】First rule"), "{prompt}");
        assert!(prompt.contains("【注意】Second rule"), "{prompt}");
    }

    #[test]
    fn keeps_existing_bracketed_instructions() {
        let prompt = build_prompt("x", "zh", "auto", "【要求】Be terse", &[]);
        assert!(prompt.contains("1. 【要求】Be terse"), "{prompt}");
    }

    #[test]
    fn preserves_glossary_row_order() {
        let prompt = build_prompt("x", "zh", "auto", "", &[term("b", "乙"), term("a", "甲")]);
        let first = prompt.find("b→乙").expect("first term");
        let second = prompt.find("a→甲").expect("second term");
        assert!(first < second, "glossary order changed: {prompt}");
    }

    #[test]
    fn parses_a_json_object() {
        let glossary = parse_glossary_json(r#"{"alpha": "阿尔法", "beta": "贝塔"}"#).unwrap();
        assert_eq!(glossary.len(), 2);
        assert!(glossary.contains(&term("alpha", "阿尔法")));
    }

    #[test]
    fn rejects_non_string_values() {
        let error = parse_glossary_json(r#"{"alpha": 1}"#).unwrap_err();
        assert!(matches!(error, GlossaryError::InvalidJson(_)), "{error:?}");
    }

    #[test]
    fn rejects_malformed_json() {
        assert!(parse_glossary_json("not json").is_err());
        assert!(parse_glossary_json(r#"["alpha"]"#).is_err());
    }

    #[test]
    fn rejects_empty_terms() {
        assert_eq!(
            parse_glossary_json(r#"{"": "x"}"#).unwrap_err(),
            GlossaryError::EmptyTerm
        );
        assert_eq!(
            parse_glossary_json(r#"{"a": "  "}"#).unwrap_err(),
            GlossaryError::EmptyTerm
        );
    }

    #[test]
    fn rejects_duplicate_sources_case_insensitively() {
        let error = validate_glossary(&[term("term", "one"), term("TERM", "two")]).unwrap_err();
        assert!(
            matches!(error, GlossaryError::DuplicateSource(_)),
            "{error:?}"
        );
    }

    #[test]
    fn accepts_distinct_sources() {
        assert!(validate_glossary(&[term("a", "1"), term("b", "2")]).is_ok());
    }
}
