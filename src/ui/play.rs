use eframe::egui;

pub fn show(ui: &mut egui::Ui) {
    ui.add_space(48.0);
    ui.vertical_centered(|ui| {
        ui.heading("Terra Launcher");
        ui.add_space(8.0);
        ui.label("Аккаунт не подключён — вход через Microsoft появится позже.");
        ui.add_space(32.0);
        let play_button = egui::Button::new(egui::RichText::new("  ИГРАТЬ  ").size(24.0))
            .min_size(egui::vec2(220.0, 56.0));
        let response = ui
            .add_enabled(false, play_button)
            .on_disabled_hover_text("Запуск Minecraft ещё не реализован");
        if response.clicked() {
            // Coming soon: vanilla launch pipeline.
        }
    });
}
