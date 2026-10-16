//! Application entry point.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use eframe::egui;

fn main() -> eframe::Result {
    // Whether the window may start hidden has to be decided before it is
    // created: a window created visible and hidden a frame later is briefly
    // on screen, and the request races with its creation. The tray is
    // registered later, in the application constructor, so this is only an
    // intent; a tray that fails to register reveals the window immediately.
    let start_hidden = yalt::settings::SettingsStore::new()
        .and_then(|store| store.load_settings())
        .map(|settings| settings.start_hidden)
        .unwrap_or(false)
        && yalt::tray::supported();

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("yalt")
        .with_inner_size(yalt::ui::theme::INITIAL_SIZE)
        .with_min_inner_size(yalt::ui::theme::MIN_SIZE)
        .with_visible(!start_hidden);
    // The window icon comes from the same embedded asset as the tray icon, so
    // a packaged build shows it without needing the file alongside.
    if let Some(icon) = yalt::tray::load_icon() {
        viewport = viewport.with_icon(icon.to_icon_data());
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    eframe::run_native(
        "yalt",
        options,
        Box::new(move |cc| {
            let app = yalt::ui::YaltApp::new(&cc.egui_ctx, start_hidden)?;
            Ok(Box::new(app))
        }),
    )
}
