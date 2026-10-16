//! Render the real view components with dummy settings for visual review.
use eframe::egui;
use yalt::ui::{settings::SettingsView, theme, translate::TranslateView};

struct Preview {
    app: yalt::app::App,
    translate: TranslateView,
    settings: SettingsView,
    frame: usize,
    dark: bool,
    settings_mode: bool,
    destination: std::path::PathBuf,
}

impl eframe::App for Preview {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        ctx.set_theme(if self.dark {
            egui::Theme::Dark
        } else {
            egui::Theme::Light
        });
        egui::Panel::bottom("preview-status").show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label("Ready");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let _ = theme::selectable_label(ui, self.settings_mode, "Settings");
                });
            });
        });
        egui::CentralPanel::default().show(ui, |ui| {
            if self.settings_mode {
                self.settings.ui(ui);
            } else {
                self.translate.ui(ui, &mut self.app);
            }
        });
        for event in ctx.input(|input| input.events.clone()) {
            if let egui::Event::Screenshot { image, .. } = event {
                let pixels: Vec<_> = image
                    .pixels
                    .iter()
                    .flat_map(|pixel| pixel.to_array())
                    .collect();
                image::save_buffer(
                    &self.destination,
                    &pixels,
                    image.width() as u32,
                    image.height() as u32,
                    image::ColorType::Rgba8,
                )
                .unwrap();
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
        self.frame += 1;
        if self.frame == 4 {
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
        }
        ctx.request_repaint();
    }
}

fn main() -> eframe::Result {
    let args: Vec<_> = std::env::args().collect();
    let dark = args.iter().any(|arg| arg == "dark");
    let settings_mode = args.iter().any(|arg| arg == "settings");
    let minimum = args.iter().any(|arg| arg == "minimum");
    let scale = if args.iter().any(|arg| arg == "scaled") {
        1.25
    } else {
        1.0
    };
    let destination = std::path::PathBuf::from(args.get(1).expect("screenshot destination"));
    let fixture = tempfile::tempdir().expect("isolated preview settings");
    let store = yalt::settings::SettingsStore::at(fixture.path().join("settings.json"));
    let app = yalt::app::App::with_store(store).unwrap();
    let mut settings = SettingsView::default();
    settings.open(&yalt::settings::Settings::default());
    let mut translate = TranslateView::new("auto", "auto");
    translate.input = "A quiet desktop translator.\n简洁的桌面翻译工具。".into();
    let size = if minimum {
        theme::MIN_SIZE
    } else {
        theme::INITIAL_SIZE
    };
    eframe::run_native(
        "Smartisan visual review",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default().with_inner_size(size),
            ..Default::default()
        },
        Box::new(move |cc| {
            cc.egui_ctx.set_fonts(yalt::ui::fonts::definitions());
            cc.egui_ctx.set_zoom_factor(scale);
            theme::apply(&cc.egui_ctx);
            Ok(Box::new(Preview {
                app,
                translate,
                settings,
                frame: 0,
                dark,
                settings_mode,
                destination,
            }))
        }),
    )
}
