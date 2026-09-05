mod app;
mod auth;
mod theme;
mod ui;

use eframe::egui;

fn main() -> eframe::Result {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([960.0, 600.0])
            .with_min_inner_size([640.0, 400.0])
            .with_title("Terra Launcher"),
        ..Default::default()
    };
    eframe::run_native(
        "Terra Launcher",
        options,
        Box::new(|cc| Ok(Box::new(app::TerraLauncherApp::new(cc)))),
    )
}
