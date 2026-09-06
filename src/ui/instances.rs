use eframe::egui;

use crate::theme::ThemePreset;

pub fn show(ui: &mut egui::Ui, theme: &ThemePreset) {
    let accent = theme.accent_color();
    ui.add_space(4.0);
    ui.heading("Сборки");
    ui.label(
        egui::RichText::new("Версии, модлоадеры и модпаки — скоро здесь")
            .weak(),
    );
    ui.add_space(18.0);

    let card = egui::vec2(180.0, 110.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(14.0, 14.0);
        create_card(ui, card, accent);
        for i in 0..2 {
            ghost_card(ui, card, i);
        }
    });
}

/// Карточка «создать сборку»: подсвечивается акцентом, пока неактивна.
fn create_card(ui: &mut egui::Ui, size: egui::Vec2, accent: egui::Color32) {
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::hover());
    let hover = ui
        .ctx()
        .animate_bool(response.id.with("h"), response.hovered());
    let rounding = egui::Rounding::same(14.0);
    ui.painter().rect_filled(
        rect,
        rounding,
        accent.gamma_multiply(0.10 + 0.06 * hover),
    );
    ui.painter().rect_stroke(
        rect,
        rounding,
        egui::Stroke::new(1.0, accent.gamma_multiply(0.45 + 0.25 * hover)),
    );
    ui.painter().text(
        rect.center() - egui::vec2(0.0, 12.0),
        egui::Align2::CENTER_CENTER,
        "+",
        egui::FontId::proportional(30.0),
        accent,
    );
    ui.painter().text(
        rect.center() + egui::vec2(0.0, 16.0),
        egui::Align2::CENTER_CENTER,
        "Создать сборку",
        egui::FontId::proportional(14.0),
        ui.visuals().text_color(),
    );
    response.on_hover_text("Появится вместе с модлоадерами (Fabric/Forge/Quilt)");
}

/// «Призрачная» карточка-заглушка под будущие сборки.
fn ghost_card(ui: &mut egui::Ui, size: egui::Vec2, _i: usize) {
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let rounding = egui::Rounding::same(14.0);
    ui.painter()
        .rect_filled(rect, rounding, egui::Color32::from_white_alpha(4));
    ui.painter().rect_stroke(
        rect,
        rounding,
        egui::Stroke::new(1.0, egui::Color32::from_white_alpha(10)),
    );
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "Пусто",
        egui::FontId::proportional(13.0),
        ui.visuals().weak_text_color(),
    );
}
