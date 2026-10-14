package main

import (
	"encoding/json"
	"errors"
	"fmt"
	"net"
	"net/url"
	"os"
	"path/filepath"
	"strings"
	"time"

	"index-go/translation"
)

const settingsVersion = 1

type Settings struct {
	Version      int                    `json:"version"`
	APIBase      string                 `json:"api_base"`
	APIMethod    string                 `json:"api_method"`
	APIKey       string                 `json:"api_key"`
	Model        string                 `json:"model"`
	Timeout      int                    `json:"timeout_seconds"`
	UserAgent    string                 `json:"user_agent"`
	Source       string                 `json:"source"`
	Target       string                 `json:"target"`
	Stream       bool                   `json:"stream"`
	MaxTokens    int                    `json:"max_tokens"`
	Temperature  float64                `json:"temperature"`
	Instruction  string                 `json:"instruction"`
	Glossary     []translation.Glossary `json:"glossary"`
	DisableThink bool                   `json:"disable_thinking"`
	TrayEnabled  bool                   `json:"tray_enabled"`
	CloseToTray  bool                   `json:"close_to_tray"`
	StartHidden  bool                   `json:"start_hidden"`
}

func DefaultSettings() Settings {
	return Settings{
		Version:      settingsVersion,
		APIBase:      "https://index-translate.bilibili.com/v1",
		APIMethod:    translation.APIMethodChatCompletions,
		Model:        "Index-Translate-35B-A3B",
		Timeout:      120,
		UserAgent:    "yalt/" + version,
		Source:       "auto",
		Target:       "auto",
		Stream:       true,
		MaxTokens:    1024,
		Temperature:  0,
		DisableThink: true,
		TrayEnabled:  true,
		CloseToTray:  true,
	}
}

func (s Settings) Validate() error {
	if err := validateEndpoint(s.APIBase); err != nil {
		return fmt.Errorf("api base: %w", err)
	}
	if s.APIMethod != "" && s.APIMethod != translation.APIMethodChatCompletions && s.APIMethod != translation.APIMethodResponses {
		return errors.New("API method is not supported")
	}
	if strings.TrimSpace(s.Model) == "" {
		return errors.New("model is required")
	}
	if s.Timeout < 1 || s.Timeout > 600 {
		return errors.New("timeout must be between 1 and 600 seconds")
	}
	if s.MaxTokens < 1 {
		return errors.New("maximum output tokens must be positive")
	}
	if s.APIMethod == translation.APIMethodResponses && s.MaxTokens < 16 {
		return errors.New("Responses API requires at least 16 output tokens")
	}
	if s.Temperature < 0 || s.Temperature != s.Temperature {
		return errors.New("temperature must be finite and nonnegative")
	}
	if s.Version == 0 {
		return errors.New("settings version is missing")
	}
	if err := translation.ValidateGlossary(s.Glossary); err != nil {
		return err
	}
	return nil
}

func (s Settings) Endpoint() string {
	method := s.APIMethod
	if method == "" {
		method = translation.APIMethodChatCompletions
	}
	return strings.TrimRight(s.APIBase, "/") + "/" + method
}

func validateEndpoint(raw string) error {
	u, err := url.Parse(raw)
	if err != nil || u.Scheme == "" || u.Host == "" || u.User != nil {
		return errors.New("must be a valid URL without credentials")
	}
	if u.Scheme != "https" && !(u.Scheme == "http" && isLoopbackHost(u.Hostname())) {
		return errors.New("use HTTPS, or HTTP with a loopback host")
	}
	if u.RawQuery != "" || u.Fragment != "" {
		return errors.New("must not contain a query or fragment")
	}
	return nil
}

func isLoopbackHost(host string) bool {
	if host == "localhost" {
		return true
	}
	ip := net.ParseIP(host)
	return ip != nil && ip.IsLoopback()
}

type SettingsStore struct {
	path string
}

func NewSettingsStore() (SettingsStore, error) {
	dir, err := os.UserConfigDir()
	if err != nil {
		return SettingsStore{}, fmt.Errorf("find user config directory: %w", err)
	}
	return SettingsStore{path: filepath.Join(dir, "yalt", "settings.json")}, nil
}

func (s SettingsStore) Load() (Settings, error) {
	data, err := os.ReadFile(s.path)
	if errors.Is(err, os.ErrNotExist) {
		return DefaultSettings(), nil
	}
	if err != nil {
		return Settings{}, fmt.Errorf("read settings: %w", err)
	}
	var settings Settings
	if err := json.Unmarshal(data, &settings); err != nil {
		return Settings{}, fmt.Errorf("settings file is corrupt: %w", err)
	}
	if err := settings.Validate(); err != nil {
		return Settings{}, fmt.Errorf("settings are invalid: %w", err)
	}
	return settings, nil
}

func (s SettingsStore) Save(settings Settings) error {
	if err := settings.Validate(); err != nil {
		return err
	}
	data, err := json.MarshalIndent(settings, "", "  ")
	if err != nil {
		return fmt.Errorf("encode settings: %w", err)
	}
	if err := os.MkdirAll(filepath.Dir(s.path), 0o700); err != nil {
		return fmt.Errorf("create settings directory: %w", err)
	}
	tmp, err := os.CreateTemp(filepath.Dir(s.path), "settings-*.tmp")
	if err != nil {
		return fmt.Errorf("create temporary settings file: %w", err)
	}
	tmpName := tmp.Name()
	defer os.Remove(tmpName)
	if err := tmp.Chmod(0o600); err != nil {
		_ = tmp.Close()
		return fmt.Errorf("set settings permissions: %w", err)
	}
	if _, err := tmp.Write(data); err != nil {
		_ = tmp.Close()
		return fmt.Errorf("write temporary settings: %w", err)
	}
	if err := tmp.Sync(); err != nil {
		_ = tmp.Close()
		return fmt.Errorf("sync temporary settings: %w", err)
	}
	if err := tmp.Close(); err != nil {
		return fmt.Errorf("close temporary settings: %w", err)
	}
	if err := replaceFile(tmpName, s.path); err != nil {
		return fmt.Errorf("replace settings: %w", err)
	}
	return nil
}

func replaceFile(source, destination string) error {
	if err := os.Rename(source, destination); err == nil {
		return nil
	}
	backup := destination + ".bak"
	_ = os.Remove(backup)
	if err := os.Rename(destination, backup); err != nil {
		return err
	}
	if err := os.Rename(source, destination); err != nil {
		_ = os.Rename(backup, destination)
		return err
	}
	_ = os.Remove(backup)
	return nil
}

func requestDeadline(settings Settings) time.Duration {
	return time.Duration(settings.Timeout) * time.Second
}
