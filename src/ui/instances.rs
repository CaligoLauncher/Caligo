use eframe::egui;

pub fn show(ui: &mut egui::Ui) {
    ui.heading("Сборки");
    ui.add_space(12.0);
    ui.label("Здесь будут твои сборки Minecraft: версии, модлоадеры (Fabric/Forge/Quilt), моды.");
    ui.add_space(8.0);
    ui.label("Пока пусто — создание сборок появится после реализации запуска ванильной версии.");
}
