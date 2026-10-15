//! Settings draft editing, kept separate from persisted settings.
//!
//! Opening Settings copies the saved settings into a draft. Editing only
//! changes the draft, so Cancel discards changes and a failed save leaves the
//! saved configuration intact.

use crate::settings::{Settings, ValidationError};
use crate::translation::languages;
use crate::translation::prompt::{self, Glossary, GlossaryError};

/// Which settings section is expanded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    /// Connection fields.
    Connection,
    /// Translation fields.
    Translation,
    /// Desktop integration fields.
    Desktop,
}

/// Text fields of the draft, held as strings so partial input is preserved.
#[derive(Debug, Clone, PartialEq)]
pub struct Draft {
    /// Working copy of the persisted settings.
    pub settings: Settings,
    /// Endpoint text as typed.
    pub api_base: String,
    /// API key as typed.
    pub api_key: String,
    /// Model as typed.
    pub model: String,
    /// Deadline text as typed.
    pub timeout: String,
    /// Output token text as typed.
    pub max_tokens: String,
    /// Temperature text as typed.
    pub temperature: String,
    /// Feedback shown near the Save button.
    pub feedback: String,
    /// Whether the feedback reports a failure.
    pub feedback_is_error: bool,
    /// Whether the restore-defaults confirmation is open.
    pub confirm_restore: bool,
}

impl Draft {
    /// Create a draft from saved settings.
    pub fn new(settings: &Settings) -> Self {
        Self {
            settings: settings.clone(),
            api_base: settings.api_base.clone(),
            api_key: settings.api_key.clone(),
            model: settings.model.clone(),
            timeout: settings.timeout_seconds.to_string(),
            max_tokens: settings.max_tokens.to_string(),
            temperature: format_number(settings.temperature),
            feedback: String::new(),
            feedback_is_error: false,
            confirm_restore: false,
        }
    }

    /// Replace the draft with the defaults, keeping it unapplied.
    pub fn restore_defaults(&mut self) {
        let defaults = Settings {
            user_agent: crate::settings::default_user_agent(),
            ..Settings::default()
        };
        *self = Self::new(&defaults);
        self.feedback = "Defaults restored. Save to apply them.".to_owned();
    }

    /// Collect the draft into settings, reporting the first invalid field.
    pub fn to_settings(&self) -> Result<Settings, String> {
        let timeout: u32 = self
            .timeout
            .trim()
            .parse()
            .map_err(|_| "Request deadline must be a whole number of seconds.".to_owned())?;
        let max_tokens: u32 = self
            .max_tokens
            .trim()
            .parse()
            .map_err(|_| "Maximum output tokens must be a whole number.".to_owned())?;
        let temperature: f64 = self
            .temperature
            .trim()
            .parse()
            .map_err(|_| "Temperature must be a number.".to_owned())?;

        let mut settings = self.settings.clone();
        settings.api_base = self.api_base.trim().to_owned();
        settings.api_key = self.api_key.trim().to_owned();
        settings.model = self.model.trim().to_owned();
        settings.timeout_seconds = timeout;
        settings.max_tokens = max_tokens;
        settings.temperature = temperature;
        settings.validate().map_err(describe)?;
        Ok(settings)
    }

    /// Replace the draft glossary after validating an import.
    ///
    /// The existing terms are preserved when the import is rejected.
    pub fn import_glossary(&mut self, json: &str) -> Result<(), String> {
        match prompt::parse_glossary_json(json) {
            Ok(glossary) => {
                self.settings.glossary = glossary;
                self.feedback.clear();
                self.feedback_is_error = false;
                Ok(())
            }
            Err(error) => Err(describe_glossary(&error)),
        }
    }

    /// Append an empty glossary row.
    pub fn add_term(&mut self) {
        self.settings.glossary.push(Glossary {
            source: String::new(),
            target: String::new(),
        });
    }

    /// Remove a glossary row.
    pub fn remove_term(&mut self, index: usize) {
        if index < self.settings.glossary.len() {
            self.settings.glossary.remove(index);
        }
    }

    /// Final request endpoint shown next to the connection fields.
    pub fn endpoint_preview(&self) -> String {
        let base = self.api_base.trim().trim_end_matches('/');
        if base.is_empty() {
            return "—".to_owned();
        }
        format!("{base}/{}", self.settings.api_method)
    }

    /// Minimum output tokens accepted for the selected method.
    pub fn min_output_tokens(&self) -> u32 {
        if self.settings.api_method == crate::translation::ApiMethod::Responses {
            16
        } else {
            1
        }
    }
}

/// Format a temperature for display without a trailing `.0`.
fn format_number(value: f64) -> String {
    if value.fract() == 0.0 {
        format!("{}", value as i64)
    } else {
        value.to_string()
    }
}

/// Actionable message for a validation failure.
pub fn describe(error: ValidationError) -> String {
    error.to_string()
}

/// Actionable message for a glossary failure.
pub fn describe_glossary(error: &GlossaryError) -> String {
    format!("Could not import glossary: {error}")
}

/// Language choices for a selector: automatic first, then the language list.
pub fn language_choices(auto_label: &str) -> Vec<(String, String)> {
    let mut choices = vec![("auto".to_owned(), auto_label.to_owned())];
    choices.extend(
        languages::LANGUAGES
            .iter()
            .map(|language| (language.code.to_owned(), language.name.to_owned())),
    );
    choices
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::translation::ApiMethod;

    #[test]
    fn a_new_draft_mirrors_saved_settings() {
        let settings = Settings {
            api_key: "secret".into(),
            ..Settings::default()
        };
        let draft = Draft::new(&settings);
        assert_eq!(draft.api_key, "secret");
        assert_eq!(draft.settings, settings);
        assert_eq!(draft.to_settings().unwrap(), settings);
    }

    #[test]
    fn cancel_discards_changes() {
        let settings = Settings::default();
        let mut draft = Draft::new(&settings);
        draft.model = "changed".into();
        // Discarding means rebuilding the draft from the saved settings.
        let draft = Draft::new(&settings);
        assert_eq!(draft.model, settings.model);
    }

    #[test]
    fn restore_defaults_changes_only_the_draft() {
        let settings = Settings {
            model: "custom".into(),
            ..Settings::default()
        };
        let mut draft = Draft::new(&settings);
        draft.restore_defaults();
        assert_eq!(draft.model, Settings::default().model);
        // The saved settings are untouched until Save is called.
        assert_eq!(settings.model, "custom");
    }

    #[test]
    fn restore_defaults_reports_that_saving_is_required() {
        let mut draft = Draft::new(&Settings::default());
        draft.restore_defaults();
        assert!(draft.feedback.contains("Save"), "{}", draft.feedback);
    }

    #[test]
    fn trims_text_fields_when_collecting() {
        let mut draft = Draft::new(&Settings::default());
        draft.api_base = "  https://example.com/v1  ".into();
        draft.model = "  my-model  ".into();
        let settings = draft.to_settings().unwrap();
        assert_eq!(settings.api_base, "https://example.com/v1");
        assert_eq!(settings.model, "my-model");
    }

    #[test]
    fn reports_non_numeric_input_actionably() {
        let mut draft = Draft::new(&Settings::default());
        draft.timeout = "soon".into();
        let error = draft.to_settings().unwrap_err();
        assert!(error.contains("deadline"), "{error}");
    }

    #[test]
    fn reports_invalid_settings_actionably() {
        let mut draft = Draft::new(&Settings::default());
        draft.model = "  ".into();
        let error = draft.to_settings().unwrap_err();
        assert!(error.contains("model"), "{error}");
    }

    #[test]
    fn a_failed_collect_does_not_change_the_draft() {
        let mut draft = Draft::new(&Settings::default());
        draft.model = String::new();
        assert!(draft.to_settings().is_err());
        // The draft is preserved so the user can correct it.
        assert!(draft.model.is_empty());
        assert_eq!(draft.settings, Settings::default());
    }

    #[test]
    fn endpoint_preview_reflects_the_method() {
        let mut draft = Draft::new(&Settings::default());
        assert!(draft.endpoint_preview().ends_with("/chat/completions"));
        draft.settings.api_method = ApiMethod::Responses;
        assert!(draft.endpoint_preview().ends_with("/responses"));
    }

    #[test]
    fn responses_requires_more_output_tokens() {
        let mut draft = Draft::new(&Settings::default());
        assert_eq!(draft.min_output_tokens(), 1);
        draft.settings.api_method = ApiMethod::Responses;
        assert_eq!(draft.min_output_tokens(), 16);
    }

    #[test]
    fn a_valid_import_replaces_the_glossary() {
        let mut draft = Draft::new(&Settings::default());
        draft.add_term();
        draft.import_glossary(r#"{"API": "接口"}"#).unwrap();
        assert_eq!(
            draft.settings.glossary,
            vec![Glossary {
                source: "API".into(),
                target: "接口".into()
            }]
        );
    }

    #[test]
    fn a_failed_import_preserves_the_existing_glossary() {
        let mut draft = Draft::new(&Settings::default());
        draft.import_glossary(r#"{"keep": "保留"}"#).unwrap();
        let before = draft.settings.glossary.clone();
        assert!(draft.import_glossary("not json").is_err());
        assert_eq!(draft.settings.glossary, before);
    }

    #[test]
    fn an_import_with_a_duplicate_source_is_rejected() {
        let mut draft = Draft::new(&Settings::default());
        assert!(draft.import_glossary(r#"{"a": "1", "A": "2"}"#).is_err());
        assert!(draft.settings.glossary.is_empty());
    }

    #[test]
    fn glossary_rows_can_be_added_and_removed() {
        let mut draft = Draft::new(&Settings::default());
        draft.add_term();
        draft.add_term();
        assert_eq!(draft.settings.glossary.len(), 2);
        draft.remove_term(0);
        assert_eq!(draft.settings.glossary.len(), 1);
        // An out-of-range removal is ignored.
        draft.remove_term(99);
        assert_eq!(draft.settings.glossary.len(), 1);
    }

    #[test]
    fn language_choices_start_with_automatic() {
        let choices = language_choices("Detect language");
        assert_eq!(
            choices[0],
            ("auto".to_owned(), "Detect language".to_owned())
        );
        assert_eq!(choices.len(), languages::LANGUAGES.len() + 1);
        assert!(choices.iter().any(|(code, _)| code == "ukr_cyrl"));
    }
}
