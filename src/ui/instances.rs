use eframe::egui;

use crate::theme::ThemePreset;

/// «Сборки» — библиотека модпаков. Сборок пока нет, но карточки уже
/// выглядят как настоящие — по рецепту карточек Modrinth App: заливка
/// surface-3, обводка 1px surface-4, скругление 16, при наведении
/// осветление, при нажатии лёгкое «утапливание».
pub fn show(ui: &mut egui::Ui, theme: &ThemePreset) {
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new("Сборки")
            .heading()
            .strong()
            .color(theme.text_primary()),
    );
    ui.add_space(2.0);
    ui.label(
        egui::RichText::new("Версии, модлоадеры и модпаки — скоро здесь")
            .size(13.0)
            .color(theme.text_tertiary()),
    );
    ui.add_space(20.0);

    let card = egui::vec2(190.0, 150.0);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(16.0, 16.0);
        create_card(ui, card, theme);
        for i in 0..2 {
            ghost_card(ui, card, theme, i);
        }
    });
}

/// Карточка «создать сборку»: акцентная тонировка, разгорается при
/// наведении, слегка сжимается при нажатии (живой отклик).
fn create_card(ui: &mut egui::Ui, size: egui::Vec2, theme: &ThemePreset) {
    let accent = theme.accent_color();
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let hover = ui
        .ctx()
        .animate_bool(response.id.with("h"), response.hovered());
    let press = ui
        .ctx()
        .animate_bool(response.id.with("p"), response.is_pointer_button_down_on());
    let rect = rect.shrink(rect.width() * 0.015 * press);
    let rounding = egui::Rounding::same(16.0);
    let p = ui.painter();
    p.rect_filled(
        rect,
        rounding,
        theme.accent_bg().gamma_multiply(0.8 + 0.5 * hover),
    );
    p.rect_stroke(
        rect,
        rounding,
        egui::Stroke::new(1.0, accent.gamma_multiply(0.45 + 0.3 * hover)),
    );
    let c = egui::pos2(rect.center().x, rect.center().y - 14.0);
    p.circle_filled(c, 19.0, accent.gamma_multiply(0.18 + 0.10 * hover));
    p.text(
        c,
        egui::Align2::CENTER_CENTER,
        "+",
        egui::FontId::proportional(26.0),
        accent,
    );
    p.text(
        rect.center() + egui::vec2(0.0, 26.0),
        egui::Align2::CENTER_CENTER,
        "Создать сборку",
        egui::FontId::proportional(14.5),
        theme.text_primary(),
    );
    response
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text("Появится вместе с модлоадерами (Fabric/Forge/Quilt)");
}

/// «Скелет» будущей карточки сборки: квадрат иконки + строки названия и
/// подписи — как настоящая карточка библиотеки, только пустая. Каждая
/// следующая — бледнее (мягкое затухание ряда).
fn ghost_card(ui: &mut egui::Ui, size: egui::Vec2, theme: &ThemePreset, i: usize) {
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let rounding = egui::Rounding::same(16.0);
    let p = ui.painter();
    let fade = 1.0 - i as f32 * 0.35;
    p.rect_filled(rect, rounding, theme.card_fill().gamma_multiply(0.5 * fade));
    p.rect_stroke(
        rect,
        rounding,
        egui::Stroke::new(1.0, theme.surface(4).gamma_multiply(0.7 * fade)),
    );
    let icon = egui::Rect::from_min_size(
        rect.min + egui::vec2(16.0, 16.0),
        egui::vec2(44.0, 44.0),
    );
    p.rect_filled(
        icon,
        egui::Rounding::same(10.0),
        egui::Color32::from_white_alpha((8.0 * fade) as u8),
    );
    let title = egui::Rect::from_min_size(
        egui::pos2(rect.min.x + 16.0, icon.max.y + 16.0),
        egui::vec2(rect.width() * 0.62 * fade.max(0.5), 10.0),
    );
    p.rect_filled(
        title,
        egui::Rounding::same(5.0),
        egui::Color32::from_white_alpha((10.0 * fade) as u8),
    );
    let sub = egui::Rect::from_min_size(
        egui::pos2(rect.min.x + 16.0, title.max.y + 8.0),
        egui::vec2(rect.width() * 0.4 * fade.max(0.5), 8.0),
    );
    p.rect_filled(
        sub,
        egui::Rounding::same(4.0),
        egui::Color32::from_white_alpha((6.0 * fade) as u8),
    );
}