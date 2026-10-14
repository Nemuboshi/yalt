package main

import (
	"context"
	"errors"
	"fmt"
	"io/fs"
	"log/slog"
	"os"
	"path/filepath"
	"sync"
	"time"

	"github.com/wailsapp/wails/v2"
	"github.com/wailsapp/wails/v2/pkg/options"
	"github.com/wailsapp/wails/v2/pkg/options/assetserver"
	windowsoptions "github.com/wailsapp/wails/v2/pkg/options/windows"
	"github.com/wailsapp/wails/v2/pkg/runtime"
	"index-go/translation"
)

const translationEvent = "translation"

type TranslationEvent struct {
	RequestID string `json:"requestID"`
	Sequence  uint64 `json:"sequence"`
	Kind      string `json:"kind"`
	Text      string `json:"text,omitempty"`
	Status    string `json:"status,omitempty"`
	Error     string `json:"error,omitempty"`
}

type TranslationInput struct {
	RequestID string `json:"requestID"`
	Text      string `json:"text"`
	Source    string `json:"source"`
	Target    string `json:"target"`
}

type activeTranslation struct {
	id     string
	cancel context.CancelFunc
	done   chan struct{}
}

type App struct {
	logger *slog.Logger
	store  SettingsStore

	mu        sync.Mutex
	ctx       context.Context
	settings  Settings
	active    *activeTranslation
	workers   sync.WaitGroup
	seq       uint64
	quitting  bool
	trayReady bool
}

func run(logger *slog.Logger, assets fs.FS, appVersion, appCommit, appDate string) error {
	store, err := NewSettingsStore()
	if err != nil {
		return err
	}
	settings, err := store.Load()
	if err != nil {
		logger.Warn("using default settings", "err", err)
		settings = DefaultSettings()
	}
	app := &App{logger: logger, store: store, settings: settings}

	assetServer := &assetserver.Options{Assets: assets}
	wailsApp := &options.App{
		Title:         "yalt",
		Width:         760,
		Height:        860,
		MinWidth:      560,
		MinHeight:     720,
		DisableResize: false,
		Frameless:     false,
		AssetServer:   assetServer,
		Windows: &windowsoptions.Options{
			WebviewUserDataPath: filepath.Join(os.TempDir(), "yalt-WebView2"),
		},
		BackgroundColour: &options.RGBA{R: 247, G: 243, B: 236, A: 1},
		OnStartup: func(ctx context.Context) {
			app.mu.Lock()
			app.ctx = ctx
			startHidden := app.settings.StartHidden
			app.mu.Unlock()
			if err := startTray(app); err != nil {
				logger.Warn("tray unavailable", "err", err)
			}
			if startHidden && app.isTrayReady() {
				runtime.WindowHide(ctx)
			}
		},
		OnBeforeClose: func(ctx context.Context) (prevent bool) {
			app.mu.Lock()
			hide := !app.quitting && app.settings.CloseToTray && app.trayReady
			app.mu.Unlock()
			if !hide {
				return false
			}
			runtime.WindowHide(ctx)
			return true
		},
		OnShutdown: func(ctx context.Context) {
			app.mu.Lock()
			app.quitting = true
			if app.active != nil {
				app.active.cancel()
				app.active = nil
			}
			app.mu.Unlock()
			done := make(chan struct{})
			go func() {
				app.workers.Wait()
				close(done)
			}()
			select {
			case <-done:
			case <-time.After(5 * time.Second):
				logger.Warn("translation workers did not stop before shutdown deadline")
			}
			stopTray()
		},
		Bind: []any{app},
	}
	if appVersion != "" {
		wailsApp.Title = fmt.Sprintf("yalt %s", appVersion)
	}
	_ = appCommit
	_ = appDate
	return wails.Run(wailsApp)
}

func (a *App) GetSettings() Settings {
	a.mu.Lock()
	defer a.mu.Unlock()
	return a.settings
}

func (a *App) GetLanguages() []translation.Language {
	return translation.Languages
}

func (a *App) GetVersion() string {
	return version
}

func (a *App) DetectLanguage(text string) translation.Detection {
	detection := translation.Detect(text)
	detection.Target = translation.SelectTarget("auto", "auto", detection)
	return detection
}

func (a *App) SaveSettings(settings Settings) error {
	if err := settings.Validate(); err != nil {
		return err
	}
	a.mu.Lock()
	previous := a.settings
	a.mu.Unlock()
	if err := a.store.Save(settings); err != nil {
		return err
	}
	a.mu.Lock()
	a.settings = settings
	a.mu.Unlock()
	if previous.TrayEnabled != settings.TrayEnabled {
		a.logger.Info("tray setting changed; restart required")
	}
	if settings.TrayEnabled && !a.isTrayReady() {
		if err := startTray(a); err != nil {
			a.logger.Warn("tray unavailable", "err", err)
		}
	}
	return nil
}

func (a *App) StartTranslation(input TranslationInput) error {
	if input.RequestID == "" {
		return errors.New("request ID is required")
	}
	if input.Text == "" {
		return errors.New("input text is empty")
	}
	a.mu.Lock()
	if a.quitting {
		a.mu.Unlock()
		return errors.New("application is shutting down")
	}
	if a.ctx == nil {
		a.mu.Unlock()
		return errors.New("application is not ready")
	}
	var previousDone <-chan struct{}
	if a.active != nil {
		previousDone = a.active.done
		a.active.cancel()
	}
	settings := a.settings
	ctx, cancel := context.WithTimeout(a.ctx, requestDeadline(settings))
	active := &activeTranslation{id: input.RequestID, cancel: cancel, done: make(chan struct{})}
	a.active = active
	a.workers.Add(1)
	a.mu.Unlock()

	a.emit(input.RequestID, "started", "", "Receiving translation", "")
	go func() {
		defer a.workers.Done()
		if previousDone != nil {
			<-previousDone
		}
		a.translate(ctx, cancel, settings, input, active.done)
	}()
	return nil
}

func (a *App) CancelTranslation(requestID string) {
	a.mu.Lock()
	defer a.mu.Unlock()
	if a.active != nil && a.active.id == requestID {
		a.active.cancel()
	}
}

func (a *App) translate(ctx context.Context, cancel context.CancelFunc, settings Settings, input TranslationInput, done chan struct{}) {
	defer cancel()
	defer close(done)
	client := translation.NewClient(translation.NewHTTPClient(requestDeadline(settings)))
	target := input.Target
	if target == "" || target == "auto" {
		target = translation.SelectTarget(input.Source, target, translation.Detect(input.Text))
	}
	result, err := client.Translate(ctx, translation.Request{
		APIBase:      settings.APIBase,
		APIMethod:    settings.APIMethod,
		APIKey:       settings.APIKey,
		Model:        settings.Model,
		Text:         input.Text,
		Source:       input.Source,
		Target:       target,
		Instruction:  settings.Instruction,
		Glossary:     settings.Glossary,
		MaxTokens:    settings.MaxTokens,
		Temperature:  settings.Temperature,
		Stream:       settings.Stream,
		DisableThink: settings.DisableThink,
		UserAgent:    settings.UserAgent,
	}, func(delta string) error {
		a.emit(input.RequestID, "delta", delta, "Receiving translation", "")
		return nil
	})
	if err != nil {
		status := "Translation failed"
		kind := "error"
		if errors.Is(ctx.Err(), context.Canceled) {
			status = "Translation stopped"
			kind = "cancelled"
		} else if errors.Is(ctx.Err(), context.DeadlineExceeded) {
			status = "Request deadline exceeded"
		}
		a.emit(input.RequestID, kind, result.Text, status, err.Error())
		a.clearActive(input.RequestID)
		return
	}
	a.emit(input.RequestID, "done", result.Text, "Translation complete", "")
	a.clearActive(input.RequestID)
}

func (a *App) clearActive(requestID string) {
	a.mu.Lock()
	defer a.mu.Unlock()
	if a.active != nil && a.active.id == requestID {
		a.active = nil
	}
}

func (a *App) emit(requestID, kind, text, status, message string) {
	a.mu.Lock()
	if a.quitting || a.ctx == nil {
		a.mu.Unlock()
		return
	}
	a.seq++
	event := TranslationEvent{
		RequestID: requestID,
		Sequence:  a.seq,
		Kind:      kind,
		Text:      text,
		Status:    status,
		Error:     message,
	}
	ctx := a.ctx
	a.mu.Unlock()
	runtime.EventsEmit(ctx, translationEvent, event)
}

func (a *App) isTrayReady() bool {
	a.mu.Lock()
	defer a.mu.Unlock()
	return a.trayReady
}

func (a *App) setTrayReady(value bool) {
	a.mu.Lock()
	a.trayReady = value
	a.mu.Unlock()
}

func (a *App) showWindow() {
	a.mu.Lock()
	ctx := a.ctx
	a.mu.Unlock()
	if ctx != nil {
		runtime.WindowShow(ctx)
		runtime.WindowUnminimise(ctx)
		runtime.WindowSetAlwaysOnTop(ctx, false)
	}
}

func (a *App) openSettings() {
	a.mu.Lock()
	ctx := a.ctx
	a.mu.Unlock()
	if ctx != nil {
		runtime.EventsEmit(ctx, "navigate", "settings")
	}
}

func (a *App) requestQuit() {
	a.mu.Lock()
	a.quitting = true
	ctx := a.ctx
	a.mu.Unlock()
	if ctx != nil {
		runtime.Quit(ctx)
	}
}
