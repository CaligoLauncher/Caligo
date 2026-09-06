mod app;
mod auth;
mod background;
mod effects;
mod launch;
mod theme;
mod ui;

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([1000.0, 620.0])
            .with_min_inner_size([720.0, 440.0])
            .with_decorations(false)
            .with_title("Caligo"),
        ..Default::default()
    };
    eframe::run_native(
        "Caligo",
        options,
        Box::new(|cc| Ok(Box::new(app::CaligoApp::new(cc)))),
    )
}
