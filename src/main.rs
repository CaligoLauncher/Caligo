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
        Box::new(|cc| Ok(Box::new(TerraLauncherApp::new(cc)))),
    )
}

#[derive(Default)]
struct TerraLauncherApp {}

impl TerraLauncherApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Dark theme by default; runtime theming (JSON presets) comes later.
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        Self::default()
    }
}

impl eframe::App for TerraLauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(48.0);
                ui.heading("Terra Launcher");
                ui.add_space(8.0);
                ui.label("Custom Minecraft launcher — in development");
            });
        });
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn sanity() {
        assert_eq!(2 + 2, 4);
    }
}
