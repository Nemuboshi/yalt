//go:build windows

package main

import (
	"sync"

	"github.com/getlantern/systray"
)

var (
	trayOnce sync.Once
	trayStop = make(chan struct{})
)

func startTray(app *App) error {
	app.mu.Lock()
	enabled := app.settings.TrayEnabled
	app.mu.Unlock()
	if !enabled {
		return nil
	}
	trayOnce.Do(func() {
		go systray.Run(func() {
			systray.SetIcon(trayIcon)
			systray.SetTitle("yalt")
			systray.SetTooltip("yalt")
			open := systray.AddMenuItem("Open Translator", "Show the translator")
			settings := systray.AddMenuItem("Settings", "Open settings")
			quit := systray.AddMenuItem("Quit", "Exit yalt")
			app.setTrayReady(true)
			go func() {
				for {
					select {
					case <-open.ClickedCh:
						app.showWindow()
					case <-settings.ClickedCh:
						app.showWindow()
						app.openSettings()
					case <-quit.ClickedCh:
						app.requestQuit()
						return
					case <-trayStop:
						return
					}
				}
			}()
		}, func() {
			app.setTrayReady(false)
		})
	})
	return nil
}

func stopTray() {
	select {
	case <-trayStop:
	default:
		close(trayStop)
		systray.Quit()
	}
}
