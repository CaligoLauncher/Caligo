use eframe::egui;

use crate::theme::ThemePreset;

#[derive(Default)]
pub struct SettingsState {
    pub theme_json: String,
    pub theme_error: Option<String>,
}

/// Секция настроек — карточка по рецепту Modrinth: заливка surface-3,
/// обводка surface-4, заголовок контрастный, описание приглушённое.
fn section(
    ui: &mut egui::Ui,
    theme: &ThemePreset,
    title: &str,
    desc: &str,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    egui::Frame::none()
        .fill(theme.card_fill())
        .stroke(theme.card_stroke())
        .rounding(egui::Rounding::same(16.0))
        .inner_margin(egui::Margin::same(16.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            // Типографика Tahoe: заголовок секции — Title 3 (15 pt,
            // полужирный), описание — Subheadline (11 pt).
            ui.label(
                egui::RichText::new(title)
                    .size(15.0)
                    .strong()
                    .color(theme.text_primary()),
            );
            if !desc.is_empty() {
                ui.label(
                    egui::RichText::new(desc)
                        .size(11.0)
                        .color(theme.text_tertiary()),
                );
            }
            ui.add_space(10.0);
            add_contents(ui);
        });
    ui.add_space(12.0);
}

pub fn show(ui: &mut egui::Ui, state: &mut SettingsState, theme: &mut ThemePreset) {
    ui.add_space(4.0);
    ui.label(
        egui::RichText::new("Настройки")
            .heading()
            .strong()
            .color(theme.text_primary()),
    );
    ui.add_space(2.0);
    ui.label(
        egui::RichText::new("Оформление меняется на лету — тема применяется сразу")
            .size(13.0)
            .color(theme.text_tertiary()),
    );
    ui.add_space(20.0);

    // Снимок темы для отрисовки карточек, пока саму тему правят слайдеры.
    let look = theme.clone();

    section(
        ui,
        &look,
        "Тема оформления",
        "Цвета, скругления и прозрачность панелей",
        |ui| {
            let mut changed = false;
            changed |= ui.checkbox(&mut theme.dark, "Тёмная тема").changed();
            changed |= ui
                .add(egui::Slider::new(&mut theme.rounding, 0.0..=16.0).text("Скругление углов"))
                .changed();
            changed |= ui
                .add(
                    egui::Slider::new(&mut theme.opacity, 0.5..=1.0)
                        .text("Непрозрачность фона"),
                )
                .changed();
            if changed {
                theme.apply(ui.ctx());
            }
        },
    );

    section(
        ui,
        &look,
        "Тема из JSON-пресета",
        "Пресет — это один файл: им можно делиться",
        |ui| {
            ui.add(
                egui::TextEdit::multiline(&mut state.theme_json)
                    .desired_rows(6)
                    .desired_width(f32::INFINITY)
                    .code_editor(),
            );
            ui.add_space(8.0);
            if ui.button("Применить").clicked() {
                match ThemePreset::from_json(&state.theme_json) {
                    Ok(parsed) => {
                        *theme = parsed;
                        theme.apply(ui.ctx());
                        state.theme_error = None;
                    }
                    Err(err) => state.theme_error = Some(format!("Ошибка в JSON: {err}")),
                }
            }
            if let Some(err) = &state.theme_error {
                ui.colored_label(egui::Color32::from_rgb(255, 120, 120), err);
            }
        },
    );
}