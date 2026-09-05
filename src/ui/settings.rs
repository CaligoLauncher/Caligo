use eframe::egui;

use crate::theme::ThemePreset;

#[derive(Default)]
pub struct SettingsState {
    pub theme_json: String,
    pub theme_error: Option<String>,
}

pub fn show(ui: &mut egui::Ui, state: &mut SettingsState, theme: &mut ThemePreset) {
    ui.heading("Настройки");
    ui.add_space(12.0);

    ui.label(egui::RichText::new("Тема оформления").strong());
    ui.add_space(4.0);

    let mut changed = false;
    changed |= ui.checkbox(&mut theme.dark, "Тёмная тема").changed();
    changed |= ui
        .add(egui::Slider::new(&mut theme.rounding, 0.0..=16.0).text("Скругление углов"))
        .changed();
    changed |= ui
        .add(egui::Slider::new(&mut theme.opacity, 0.5..=1.0).text("Непрозрачность фона"))
        .changed();
    if changed {
        theme.apply(ui.ctx());
    }

    ui.add_space(16.0);
    ui.collapsing("Тема из JSON-пресета", |ui| {
        ui.label("Вставь JSON-пресет темы и нажми «Применить».");
        ui.add(
            egui::TextEdit::multiline(&mut state.theme_json)
                .desired_rows(6)
                .desired_width(f32::INFINITY)
                .code_editor(),
        );
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
            ui.colored_label(egui::Color32::RED, err);
        }
    });
}
