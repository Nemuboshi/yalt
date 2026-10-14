//go:build !windows

package main

func startTray(app *App) error {
	return nil
}

func stopTray() {}
