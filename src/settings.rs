//! Settings defaults, validation, and safe persistence.
//!
//! Settings are stored as versioned JSON in a Rust-specific configuration
//! directory so the previous Go application's settings are never read,
//! modified, or overwritten. API keys remain plaintext in that file by
//! decision; they are kept out of errors and diagnostics.

use std::fmt;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::translation::prompt::{self, Glossary, GlossaryError};
use crate::translation::{ApiMethod, languages};

/// Settings schema version written by this build.
pub const SETTINGS_VERSION: u32 = 1;

/// Configuration directory name, distinct from the previous application.
const CONFIG_DIR: &str = "yalt-rust";
/// Settings file name within the configuration directory.
const SETTINGS_FILE: &str = "settings.json";

/// Persisted application settings.
///
/// Defaults match the product specification. Fields are plain data; drafts and
/// temporary UI state live elsewhere.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Settings {
    /// Schema version of this document.
    #[serde(default)]
    pub version: u32,
    /// Provider base URL, without the endpoint suffix.
    #[serde(default = "default_api_base")]
    pub api_base: String,
    /// Provider API contract.
    #[serde(default)]
    pub api_method: ApiMethod,
    /// Optional bearer token. Stored in plaintext by decision.
    #[serde(default)]
    pub api_key: String,
    /// Provider model identifier.
    #[serde(default = "default_model")]
    pub model: String,
    /// Total request deadline in seconds, covering streaming.
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u32,
    /// Client identifier sent with requests.
    #[serde(default)]
    pub user_agent: String,
    /// Source language selection, `auto` for detection.
    #[serde(default = "default_auto")]
    pub source: String,
    /// Target language selection, `auto` for the automatic rule.
    #[serde(default = "default_auto")]
    pub target: String,
    /// Whether to request a streamed response.
    #[serde(default = "default_true")]
    pub stream: bool,
    /// Maximum output tokens.
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    /// Sampling temperature.
    #[serde(default)]
    pub temperature: f64,
    /// Extra instructions appended to the prompt.
    #[serde(default)]
    pub instruction: String,
    /// Ordered glossary terms.
    #[serde(default)]
    pub glossary: Vec<Glossary>,
    /// Whether to ask the provider to suppress reasoning output.
    #[serde(default = "default_true")]
    pub disable_thinking: bool,
    /// Whether the tray icon is enabled. Takes effect after restart.
    #[serde(default = "default_true")]
    pub tray_enabled: bool,
    /// Whether closing the window hides it instead of exiting.
    #[serde(default = "default_true")]
    pub close_to_tray: bool,
    /// Whether the window starts hidden. Takes effect after restart.
    #[serde(default)]
    pub start_hidden: bool,
}

fn default_api_base() -> String {
    "https://index-translate.bilibili.com/v1".to_owned()
}

fn default_model() -> String {
    "Index-Translate-35B-A3B".to_owned()
}

fn default_timeout() -> u32 {
    120
}

fn default_auto() -> String {
    "auto".to_owned()
}

fn default_max_tokens() -> u32 {
    1024
}

fn default_true() -> bool {
    true
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            version: SETTINGS_VERSION,
            api_base: default_api_base(),
            api_method: ApiMethod::default(),
            api_key: String::new(),
            model: default_model(),
            timeout_seconds: default_timeout(),
            user_agent: default_user_agent(),
            source: default_auto(),
            target: default_auto(),
            stream: true,
            max_tokens: default_max_tokens(),
            temperature: 0.0,
            instruction: String::new(),
            glossary: Vec::new(),
            disable_thinking: true,
            tray_enabled: true,
            close_to_tray: true,
            start_hidden: false,
        }
    }
}

/// Default client identifier, including the application version.
pub fn default_user_agent() -> String {
    format!("yalt/{}", env!("CARGO_PKG_VERSION"))
}

/// Reason a settings document was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// The endpoint was not a usable URL.
    Endpoint(String),
    /// The model was empty.
    ModelRequired,
    /// The deadline was outside the accepted range.
    TimeoutRange,
    /// The output token limit was not positive.
    MaxTokensPositive,
    /// The Responses API needs a higher minimum than other methods.
    MaxTokensForResponses,
    /// The temperature was negative or not finite.
    Temperature,
    /// The schema version is missing.
    VersionMissing,
    /// A glossary term was invalid.
    Glossary(GlossaryError),
}

impl fmt::Display for ValidationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Endpoint(message) => write!(f, "api base: {message}"),
            Self::ModelRequired => f.write_str("model is required"),
            Self::TimeoutRange => f.write_str("timeout must be between 1 and 600 seconds"),
            Self::MaxTokensPositive => f.write_str("maximum output tokens must be positive"),
            Self::MaxTokensForResponses => {
                f.write_str("Responses API requires at least 16 output tokens")
            }
            Self::Temperature => f.write_str("temperature must be finite and nonnegative"),
            Self::VersionMissing => f.write_str("settings version is missing"),
            Self::Glossary(error) => write!(f, "{error}"),
        }
    }
}

impl std::error::Error for ValidationError {}

impl From<GlossaryError> for ValidationError {
    fn from(error: GlossaryError) -> Self {
        Self::Glossary(error)
    }
}

impl Settings {
    /// Validate the settings, returning the first problem found.
    pub fn validate(&self) -> Result<(), ValidationError> {
        validate_endpoint(&self.api_base).map_err(ValidationError::Endpoint)?;
        if self.model.trim().is_empty() {
            return Err(ValidationError::ModelRequired);
        }
        if self.timeout_seconds < 1 || self.timeout_seconds > 600 {
            return Err(ValidationError::TimeoutRange);
        }
        if self.max_tokens < 1 {
            return Err(ValidationError::MaxTokensPositive);
        }
        if self.api_method == ApiMethod::Responses && self.max_tokens < 16 {
            return Err(ValidationError::MaxTokensForResponses);
        }
        if !self.temperature.is_finite() || self.temperature < 0.0 {
            return Err(ValidationError::Temperature);
        }
        if self.version == 0 {
            return Err(ValidationError::VersionMissing);
        }
        prompt::validate_glossary(&self.glossary)?;
        Ok(())
    }

    /// Final request endpoint for the selected method.
    pub fn endpoint(&self) -> String {
        format!(
            "{}/{}",
            self.api_base.trim_end_matches('/'),
            self.api_method
        )
    }

    /// Request deadline as a duration.
    pub fn deadline(&self) -> std::time::Duration {
        std::time::Duration::from_secs(self.timeout_seconds.into())
    }
}

/// Validate a provider base URL.
///
/// Accepts HTTPS anywhere and HTTP only for explicit loopback hosts. Rejects
/// malformed URLs, embedded credentials, query strings, and fragments.
pub fn validate_endpoint(raw: &str) -> Result<(), String> {
    let url = reqwest::Url::parse(raw.trim())
        .map_err(|_| "must be a valid URL without credentials".to_owned())?;
    if url.host_str().is_none() {
        return Err("must be a valid URL without credentials".to_owned());
    }
    if !url.username().is_empty() || url.password().is_some() {
        return Err("must be a valid URL without credentials".to_owned());
    }
    match url.scheme() {
        "https" => {}
        "http" if is_loopback_host(url.host_str().unwrap_or_default()) => {}
        _ => return Err("use HTTPS, or HTTP with a loopback host".to_owned()),
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err("must not contain a query or fragment".to_owned());
    }
    Ok(())
}

/// Whether a host names the local machine.
fn is_loopback_host(host: &str) -> bool {
    if host.eq_ignore_ascii_case("localhost") {
        return true;
    }
    // IPv6 hosts are reported with surrounding brackets.
    let bare = host
        .strip_prefix('[')
        .and_then(|rest| rest.strip_suffix(']'))
        .unwrap_or(host);
    bare.parse::<std::net::IpAddr>()
        .map(|ip| ip.is_loopback())
        .unwrap_or(false)
}

/// Reason a settings file could not be used.
#[derive(Debug)]
pub enum StoreError {
    /// The configuration directory could not be located.
    NoConfigDir,
    /// The file could not be read or written.
    Io(std::io::Error),
    /// The file was not valid JSON for this schema.
    Corrupt(String),
    /// The file held a schema version this build does not support.
    UnsupportedVersion(u32),
    /// The stored settings failed validation.
    Invalid(ValidationError),
}

impl fmt::Display for StoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoConfigDir => f.write_str("could not locate a configuration directory"),
            Self::Io(error) => write!(f, "settings file: {error}"),
            Self::Corrupt(message) => write!(f, "settings file is corrupt: {message}"),
            Self::UnsupportedVersion(version) => {
                write!(f, "settings version {version} is not supported")
            }
            Self::Invalid(error) => write!(f, "settings are invalid: {error}"),
        }
    }
}

impl std::error::Error for StoreError {}

impl From<std::io::Error> for StoreError {
    fn from(error: std::io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<ValidationError> for StoreError {
    fn from(error: ValidationError) -> Self {
        Self::Invalid(error)
    }
}

/// Outcome of loading settings.
#[derive(Debug)]
pub enum LoadOutcome {
    /// A valid settings file was read.
    Loaded(Settings),
    /// No settings file exists yet, so defaults apply.
    Fresh(Settings),
    /// The file exists but could not be used. The file is left untouched.
    Recoverable {
        /// Defaults to use while the file remains unreadable.
        settings: Settings,
        /// Why the file could not be used.
        error: StoreError,
    },
}

/// Reads and writes the settings file.
#[derive(Debug, Clone)]
pub struct SettingsStore {
    path: PathBuf,
}

impl LoadOutcome {
    /// The settings that apply, whichever outcome this is.
    pub fn settings(&self) -> &Settings {
        match self {
            Self::Loaded(settings) | Self::Fresh(settings) => settings,
            Self::Recoverable { settings, .. } => settings,
        }
    }
}

impl SettingsStore {
    /// Store settings under the platform configuration directory.
    ///
    /// Uses a separate application identity, so the previous application's
    /// settings are never touched.
    pub fn new() -> Result<Self, StoreError> {
        let dir = dirs::config_dir().ok_or(StoreError::NoConfigDir)?;
        Ok(Self::at(dir.join(CONFIG_DIR).join(SETTINGS_FILE)))
    }

    /// Store settings at an explicit path.
    pub fn at(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Path of the settings file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Load settings, distinguishing fresh, valid, and unreadable files.
    ///
    /// An unreadable or unsupported file is retained: this never deletes or
    /// overwrites it, and ordinary saving is blocked until the caller
    /// explicitly recovers.
    pub fn load(&self) -> LoadOutcome {
        let data = match std::fs::read(&self.path) {
            Ok(data) => data,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return LoadOutcome::Fresh(Settings::default());
            }
            Err(error) => {
                return LoadOutcome::Recoverable {
                    settings: Settings::default(),
                    error: StoreError::Io(error),
                };
            }
        };
        match parse_settings(&data) {
            Ok(settings) => LoadOutcome::Loaded(settings),
            Err(error) => LoadOutcome::Recoverable {
                settings: Settings::default(),
                error,
            },
        }
    }

    /// Load settings, using defaults when the file is missing or unreadable.
    ///
    /// For callers that need the values before the application exists, such as
    /// deciding whether to create the window hidden. Use [`Self::load`] when
    /// an unreadable file must be reported or retained.
    pub fn load_settings(&self) -> Result<Settings, StoreError> {
        match self.load() {
            LoadOutcome::Loaded(settings) | LoadOutcome::Fresh(settings) => Ok(settings),
            LoadOutcome::Recoverable { error, .. } => Err(error),
        }
    }

    /// Validate and persist settings, replacing the previous file atomically.
    ///
    /// A failure leaves the previous valid file in place.
    pub fn save(&self, settings: &Settings) -> Result<(), StoreError> {
        settings.validate()?;
        let data = serde_json::to_vec_pretty(settings)
            .map_err(|error| StoreError::Corrupt(error.to_string()))?;
        let directory = self.path.parent().ok_or(StoreError::NoConfigDir)?;
        std::fs::create_dir_all(directory)?;
        write_atomically(directory, &self.path, &data)
    }
}

/// Parse and validate a settings document.
fn parse_settings(data: &[u8]) -> Result<Settings, StoreError> {
    let settings: Settings =
        serde_json::from_slice(data).map_err(|error| StoreError::Corrupt(error.to_string()))?;
    if settings.version != SETTINGS_VERSION {
        return Err(StoreError::UnsupportedVersion(settings.version));
    }
    settings.validate()?;
    Ok(settings)
}

/// Write `data` to `destination` through a temporary file in the same
/// directory, then replace the destination.
///
/// The temporary file is flushed and closed before replacement, and the
/// previous file is restored if replacement fails.
fn write_atomically(directory: &Path, destination: &Path, data: &[u8]) -> Result<(), StoreError> {
    use std::io::Write;

    let temporary = directory.join(format!("settings-{}.tmp", std::process::id()));
    let result = (|| -> std::io::Result<()> {
        let mut file = std::fs::File::create(&temporary)?;
        restrict_permissions(&file)?;
        file.write_all(data)?;
        file.sync_all()?;
        file.sync_data()?;
        drop(file);
        replace_file(&temporary, destination)
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result.map_err(StoreError::Io)
}

/// Replace `destination` with `source`, restoring the previous file on failure.
fn replace_file(source: &Path, destination: &Path) -> std::io::Result<()> {
    if std::fs::rename(source, destination).is_ok() {
        return Ok(());
    }
    // Windows cannot rename over an existing file, so move it aside first.
    let backup = destination.with_extension("json.bak");
    let _ = std::fs::remove_file(&backup);
    if !destination.exists() {
        return std::fs::rename(source, destination);
    }
    std::fs::rename(destination, &backup)?;
    match std::fs::rename(source, destination) {
        Ok(()) => {
            let _ = std::fs::remove_file(&backup);
            Ok(())
        }
        Err(error) => {
            // Put the previous valid file back.
            let _ = std::fs::rename(&backup, destination);
            Err(error)
        }
    }
}

/// Restrict the settings file to the current user where the platform supports it.
#[cfg(unix)]
fn restrict_permissions(file: &std::fs::File) -> std::io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    file.set_permissions(std::fs::Permissions::from_mode(0o600))
}

/// Restrict the settings file to the current user where the platform supports it.
#[cfg(not(unix))]
fn restrict_permissions(_file: &std::fs::File) -> std::io::Result<()> {
    // Windows inherits the user profile's access control list.
    Ok(())
}

/// Documented settings path for each platform, for build and support docs.
///
/// These are the paths the platform's own convention produces, so the table is
/// the same on every build. Only the current platform's entry is the path this
/// build actually uses; the others are what the same code produces there.
pub fn documented_paths() -> Vec<(&'static str, String)> {
    // Each platform's `dirs::config_dir` equivalent.
    const WINDOWS: &str = "%APPDATA%";
    const MACOS: &str = "$HOME/Library/Application Support";
    const LINUX: &str = "$XDG_CONFIG_HOME (or $HOME/.config)";

    let join = |base: &str| {
        let separator = if base.starts_with('%') { '\\' } else { '/' };
        format!("{base}{separator}{CONFIG_DIR}{separator}{SETTINGS_FILE}")
    };
    vec![
        ("Windows", join(WINDOWS)),
        ("macOS", join(MACOS)),
        ("Linux", join(LINUX)),
    ]
}

/// Whether `code` is an acceptable source or target selection.
pub fn is_valid_selection(code: &str) -> bool {
    code.eq_ignore_ascii_case("auto") || languages::is_known_language(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_store() -> (tempfile::TempDir, SettingsStore) {
        let dir = tempfile::tempdir().unwrap();
        let store = SettingsStore::at(dir.path().join("settings.json"));
        (dir, store)
    }

    #[test]
    fn defaults_match_the_specification() {
        let settings = Settings::default();
        assert_eq!(settings.api_base, "https://index-translate.bilibili.com/v1");
        assert_eq!(settings.api_method, ApiMethod::ChatCompletions);
        assert!(settings.api_key.is_empty());
        assert_eq!(settings.model, "Index-Translate-35B-A3B");
        assert_eq!(settings.timeout_seconds, 120);
        assert_eq!(settings.source, "auto");
        assert_eq!(settings.target, "auto");
        assert!(settings.stream);
        assert_eq!(settings.max_tokens, 1024);
        assert_eq!(settings.temperature, 0.0);
        assert!(settings.instruction.is_empty());
        assert!(settings.glossary.is_empty());
        assert!(settings.disable_thinking);
        assert!(settings.tray_enabled);
        assert!(settings.close_to_tray);
        assert!(!settings.start_hidden);
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn endpoint_uses_the_selected_method() {
        let mut settings = Settings::default();
        assert_eq!(
            settings.endpoint(),
            "https://index-translate.bilibili.com/v1/chat/completions"
        );
        settings.api_method = ApiMethod::Responses;
        assert_eq!(
            settings.endpoint(),
            "https://index-translate.bilibili.com/v1/responses"
        );
    }

    #[test]
    fn endpoint_tolerates_a_trailing_slash() {
        let settings = Settings {
            api_base: "https://example.com/v1/".into(),
            ..Settings::default()
        };
        assert_eq!(
            settings.endpoint(),
            "https://example.com/v1/chat/completions"
        );
    }

    #[test]
    fn validates_endpoints() {
        let cases = [
            ("https://example.com/v1", true),
            ("http://127.0.0.1:8080/v1", true),
            ("http://localhost:8080/v1", true),
            ("http://[::1]:8080/v1", true),
            ("http://example.com/v1", false),
            ("https://user:pass@example.com/v1", false),
            ("https://example.com/v1?x=1", false),
            ("https://example.com/v1#frag", false),
            ("not a url", false),
            ("", false),
        ];
        for (url, ok) in cases {
            assert_eq!(validate_endpoint(url).is_ok(), ok, "url = {url:?}");
        }
    }

    #[test]
    fn rejects_an_empty_model() {
        let settings = Settings {
            model: "   ".into(),
            ..Settings::default()
        };
        assert_eq!(settings.validate(), Err(ValidationError::ModelRequired));
    }

    #[test]
    fn rejects_an_out_of_range_deadline() {
        for seconds in [0, 601] {
            let settings = Settings {
                timeout_seconds: seconds,
                ..Settings::default()
            };
            assert_eq!(settings.validate(), Err(ValidationError::TimeoutRange));
        }
        assert!(
            Settings {
                timeout_seconds: 1,
                ..Settings::default()
            }
            .validate()
            .is_ok()
        );
        assert!(
            Settings {
                timeout_seconds: 600,
                ..Settings::default()
            }
            .validate()
            .is_ok()
        );
    }

    #[test]
    fn rejects_non_positive_output_tokens() {
        let settings = Settings {
            max_tokens: 0,
            ..Settings::default()
        };
        assert_eq!(settings.validate(), Err(ValidationError::MaxTokensPositive));
    }

    #[test]
    fn responses_requires_sixteen_output_tokens() {
        let mut settings = Settings {
            api_method: ApiMethod::Responses,
            max_tokens: 15,
            ..Settings::default()
        };
        assert_eq!(
            settings.validate(),
            Err(ValidationError::MaxTokensForResponses)
        );
        settings.max_tokens = 16;
        assert!(settings.validate().is_ok());
    }

    #[test]
    fn rejects_non_finite_or_negative_temperature() {
        for temperature in [-1.0, f64::NAN, f64::INFINITY] {
            let settings = Settings {
                temperature,
                ..Settings::default()
            };
            assert_eq!(
                settings.validate(),
                Err(ValidationError::Temperature),
                "temperature = {temperature}"
            );
        }
    }

    #[test]
    fn rejects_a_missing_version() {
        let settings = Settings {
            version: 0,
            ..Settings::default()
        };
        assert_eq!(settings.validate(), Err(ValidationError::VersionMissing));
    }

    #[test]
    fn rejects_an_invalid_glossary() {
        let settings = Settings {
            glossary: vec![
                Glossary {
                    source: "a".into(),
                    target: "1".into(),
                },
                Glossary {
                    source: "A".into(),
                    target: "2".into(),
                },
            ],
            ..Settings::default()
        };
        assert!(matches!(
            settings.validate(),
            Err(ValidationError::Glossary(_))
        ));
    }

    #[test]
    fn round_trips_settings_including_the_api_key() {
        let (_dir, store) = temp_store();
        let settings = Settings {
            api_key: "secret-token".into(),
            ..Settings::default()
        };
        store.save(&settings).unwrap();
        match store.load() {
            LoadOutcome::Loaded(loaded) => {
                assert_eq!(loaded, settings);
                assert_eq!(loaded.api_key, "secret-token");
            }
            other => panic!("expected loaded settings, got {other:?}"),
        }
    }

    #[test]
    fn a_missing_file_yields_fresh_defaults() {
        let (_dir, store) = temp_store();
        assert!(matches!(store.load(), LoadOutcome::Fresh(_)));
    }

    #[test]
    fn a_corrupt_file_is_retained_and_reported() {
        let (dir, store) = temp_store();
        std::fs::write(store.path(), b"{ not json").unwrap();
        let outcome = store.load();
        assert!(
            matches!(
                outcome,
                LoadOutcome::Recoverable {
                    error: StoreError::Corrupt(_),
                    ..
                }
            ),
            "{outcome:?}"
        );
        // The unreadable file is left exactly as it was.
        assert_eq!(std::fs::read(store.path()).unwrap(), b"{ not json");
        assert!(dir.path().exists());
    }

    #[test]
    fn an_unsupported_version_is_retained_and_reported() {
        let (_dir, store) = temp_store();
        let settings = Settings {
            version: 99,
            ..Settings::default()
        };
        std::fs::write(store.path(), serde_json::to_vec(&settings).unwrap()).unwrap();
        let outcome = store.load();
        assert!(
            matches!(
                outcome,
                LoadOutcome::Recoverable {
                    error: StoreError::UnsupportedVersion(99),
                    ..
                }
            ),
            "{outcome:?}"
        );
    }

    #[test]
    fn invalid_stored_settings_are_reported_without_replacing_the_file() {
        let (_dir, store) = temp_store();
        let settings = Settings {
            model: String::new(),
            ..Settings::default()
        };
        let raw = serde_json::to_vec(&settings).unwrap();
        std::fs::write(store.path(), &raw).unwrap();
        let outcome = store.load();
        assert!(
            matches!(
                outcome,
                LoadOutcome::Recoverable {
                    error: StoreError::Invalid(_),
                    ..
                }
            ),
            "{outcome:?}"
        );
        assert_eq!(std::fs::read(store.path()).unwrap(), raw);
    }

    #[test]
    fn saving_rejects_invalid_settings_without_writing() {
        let (_dir, store) = temp_store();
        let settings = Settings {
            model: String::new(),
            ..Settings::default()
        };
        assert!(store.save(&settings).is_err());
        assert!(!store.path().exists(), "an invalid save created a file");
    }

    #[test]
    fn a_failed_save_preserves_the_previous_file() {
        let (_dir, store) = temp_store();
        let good = Settings {
            model: "first".into(),
            ..Settings::default()
        };
        store.save(&good).unwrap();
        let bad = Settings {
            model: String::new(),
            ..Settings::default()
        };
        assert!(store.save(&bad).is_err());
        match store.load() {
            LoadOutcome::Loaded(loaded) => assert_eq!(loaded.model, "first"),
            other => panic!("previous settings were lost: {other:?}"),
        }
    }

    #[test]
    fn saving_replaces_an_existing_file() {
        let (_dir, store) = temp_store();
        store
            .save(&Settings {
                model: "first".into(),
                ..Settings::default()
            })
            .unwrap();
        store
            .save(&Settings {
                model: "second".into(),
                ..Settings::default()
            })
            .unwrap();
        match store.load() {
            LoadOutcome::Loaded(loaded) => assert_eq!(loaded.model, "second"),
            other => panic!("expected loaded settings, got {other:?}"),
        }
    }

    #[test]
    fn saving_leaves_no_temporary_files_behind() {
        let (dir, store) = temp_store();
        store.save(&Settings::default()).unwrap();
        let leftovers: Vec<_> = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".tmp"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "temporary files remain: {leftovers:?}"
        );
    }

    #[test]
    fn the_config_directory_is_rust_specific() {
        let store = SettingsStore::new().unwrap();
        let path = store.path().display().to_string();
        assert!(
            path.contains(CONFIG_DIR),
            "unexpected settings path: {path}"
        );
        // It must not point at the previous application's directory.
        assert!(
            !path.replace('\\', "/").contains("/yalt/settings.json"),
            "settings path overlaps the Go application: {path}"
        );
    }

    #[test]
    fn the_documented_paths_name_all_three_platforms() {
        let paths = documented_paths();
        let names: Vec<&str> = paths.iter().map(|(name, _)| *name).collect();
        assert_eq!(names, ["Windows", "macOS", "Linux"]);
        for (name, path) in &paths {
            assert!(path.contains(CONFIG_DIR), "{name} path: {path}");
            assert!(path.ends_with(SETTINGS_FILE), "{name} path: {path}");
            // Each entry must use its own platform's separator.
            if *name == "Windows" {
                assert!(path.contains('\\'), "{name} path: {path}");
            } else {
                assert!(!path.contains('\\'), "{name} path: {path}");
            }
        }
        // The documented directory must be the one this build actually uses,
        // so the table cannot drift from the code.
        let actual = SettingsStore::new().unwrap();
        assert!(
            actual.path().display().to_string().contains(CONFIG_DIR),
            "documented name does not match the store"
        );
    }

    #[test]
    fn the_default_user_agent_carries_the_version() {
        let agent = default_user_agent();
        assert!(agent.starts_with("yalt/"), "{agent}");
        assert!(agent.len() > "yalt/".len(), "{agent}");
    }

    #[test]
    fn selections_accept_auto_and_listed_languages() {
        assert!(is_valid_selection("auto"));
        assert!(is_valid_selection("AUTO"));
        assert!(is_valid_selection("ukr_cyrl"));
        assert!(!is_valid_selection("klingon"));
    }
}
