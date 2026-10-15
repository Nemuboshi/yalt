//! Application entry point.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("yalt")
            .with_inner_size(yalt::ui::theme::INITIAL_SIZE)
            .with_min_inner_size(yalt::ui::theme::MIN_SIZE),
        ..Default::default()
    };
    eframe::run_native(
        "yalt",
        options,
        Box::new(|cc| {
            let app = yalt::ui::YaltApp::new(&cc.egui_ctx)?;
            Ok(Box::new(app))
        }),
    )
}
