package main

import (
	"path/filepath"
	"testing"

	"index-go/translation"
)

func TestValidateEndpoint(t *testing.T) {
	tests := []struct {
		name string
		url  string
		ok   bool
	}{
		{"https", "https://example.com/v1", true},
		{"loopback http", "http://127.0.0.1:8080/v1", true},
		{"plain http", "http://example.com/v1", false},
		{"credentials", "https://user:pass@example.com/v1", false},
		{"query", "https://example.com/v1?x=1", false},
	}
	for _, test := range tests {
		t.Run(test.name, func(t *testing.T) {
			if got := validateEndpoint(test.url); (got == nil) != test.ok {
				t.Fatalf("validateEndpoint(%q) error = %v", test.url, got)
			}
		})
	}
}

func TestSettingsStorePreservesAPIKey(t *testing.T) {
	store := SettingsStore{path: filepath.Join(t.TempDir(), "settings.json")}
	settings := DefaultSettings()
	settings.APIKey = "secret-token"

	if err := store.Save(settings); err != nil {
		t.Fatalf("Save() error = %v", err)
	}
	got, err := store.Load()
	if err != nil {
		t.Fatalf("Load() error = %v", err)
	}
	if got.APIKey != settings.APIKey {
		t.Errorf("APIKey = %q, want %q", got.APIKey, settings.APIKey)
	}
}

func TestSettingsEndpointUsesSelectedAPIMethod(t *testing.T) {
	settings := DefaultSettings()
	if got, want := settings.Endpoint(), "https://index-translate.bilibili.com/v1/chat/completions"; got != want {
		t.Errorf("default endpoint = %q, want %q", got, want)
	}
	settings.APIMethod = translation.APIMethodResponses
	if got, want := settings.Endpoint(), "https://index-translate.bilibili.com/v1/responses"; got != want {
		t.Errorf("Responses endpoint = %q, want %q", got, want)
	}
}

func TestResponsesSettingsRequireMinimumOutputTokens(t *testing.T) {
	settings := DefaultSettings()
	settings.APIMethod = translation.APIMethodResponses
	settings.MaxTokens = 15
	if err := settings.Validate(); err == nil {
		t.Fatal("Validate() accepted fewer than 16 output tokens for Responses")
	}
	settings.MaxTokens = 16
	if err := settings.Validate(); err != nil {
		t.Fatalf("Validate() rejected 16 output tokens: %v", err)
	}
}
