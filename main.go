package main

import (
	"embed"
	"log/slog"
	"os"
)

//go:embed frontend/dist
var assets embed.FS

//go:embed build/windows/icon.ico
var trayIcon []byte

var (
	version = "dev"
	commit  = "none"
	date    = "unknown"
)

func main() {
	logger := slog.New(slog.NewTextHandler(os.Stderr, &slog.HandlerOptions{Level: slog.LevelInfo}))
	if err := run(logger, assets, version, commit, date); err != nil {
		logger.Error("application stopped", "err", err)
		os.Exit(1)
	}
}
