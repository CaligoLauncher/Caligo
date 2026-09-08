use eframe::egui;

use crate::theme::{color_arr, ModuleStyle, ThemePreset};

#[derive(Default)]
pub struct SettingsState {
    pub theme_json: String,
    pub theme_error: Option<String>,
    pub category: usize,
}

/// Секция настроек — карточка: заливка card_fill, обводка card_stroke,
/// заголовок контрастный, описание приглушённое.
fn section(
    ui: &mut egui::Ui,
    theme: &ThemePreset,
    title: &str,
    desc: &str,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    egui::Frame::none()
        .fill(theme.card_fill())
        .stroke(egui::Stroke::NONE)
        .rounding(egui::Rounding::same(10.0))
        .inner_margin(egui::Margin::same(16.0))
        .show(ui, |ui| {
            ui.set_min_width(ui.available_width());
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

/// Слайдер-переопределение: галочка «своё значение» + слайдер.
/// Пока галочка снята, модуль наследует значение темы (default).
fn override_slider(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut Option<f32>,
    default: f32,
    range: std::ops::RangeInclusive<f32>,
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        let mut on = value.is_some();
        if ui.checkbox(&mut on, label).changed() {
            *value = if on { Some(default) } else { None };
            changed = true;
        }
        if let Some(v) = value.as_mut() {
            changed |= ui.add(egui::Slider::new(v, range)).changed();
        } else {
            let mut d = default;
            ui.add_enabled(false, egui::Slider::new(&mut d, range));
        }
    });
    changed
}

/// Цвет-переопределение: галочка + пипетка RGBA (альфа = прозрачность).
fn override_color(
    ui: &mut egui::Ui,
    label: &str,
    value: &mut Option<[u8; 4]>,
    default: [u8; 4],
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        let mut on = value.is_some();
        if ui.checkbox(&mut on, label).changed() {
            *value = if on { Some(default) } else { None };
            changed = true;
        }
        if let Some(c) = value.as_mut() {
            let mut col = color_arr(*c);
            if ui
                .color_edit_button_srgba(&mut col)
                .changed()
            {
                *c = [col.r(), col.g(), col.b(), col.a()];
                changed = true;
            }
        } else {
            let mut col = color_arr(default);
            ui.add_enabled_ui(false, |ui| {
                ui.color_edit_button_srgba(&mut col);
            });
        }
    });
    changed
}

/// Редактор одного модуля: раскрывающаяся секция со всеми
/// переопределениями (размеры, углы, цвет+прозрачность, бортики)
/// и кнопкой сброса.
fn module_editor(
    ui: &mut egui::Ui,
    id: &str,
    title: &str,
    style: &mut ModuleStyle,
    defaults: ModuleDefaults,
) -> bool {
    let mut changed = false;
    let header = if style.is_custom() {
        format!("{title}  •")
    } else {
        title.to_string()
    };
    egui::CollapsingHeader::new(egui::RichText::new(header).size(13.0).strong())
        .id_salt(id)
        .show(ui, |ui| {
            ui.add_space(4.0);
            if let Some(dw) = defaults.width {
                changed |= override_slider(ui, "Ширина", &mut style.width, dw, defaults.width_range());
            }
            if let Some(dh) = defaults.height {
                changed |=
                    override_slider(ui, "Высота", &mut style.height, dh, defaults.height_range());
            }
            changed |= override_slider(
                ui,
                "Скругление углов",
                &mut style.rounding,
                defaults.rounding,
                0.0..=40.0,
            );
            changed |= override_color(ui, "Цвет и прозрачность", &mut style.fill, defaults.fill);
            changed |= override_color(
                ui,
                "Цвет бортика",
                &mut style.border_color,
                defaults.border_color,
            );
            changed |= override_slider(
                ui,
                "Толщина бортика",
                &mut style.border_width,
                1.0,
                0.0..=6.0,
            );
            ui.add_space(4.0);
            if style.is_custom() && ui.small_button("Сбросить модуль").clicked() {
                *style = ModuleStyle::default();
                changed = true;
            }
            ui.add_space(2.0);
        });
    changed
}

/// Значения по умолчанию, которые модуль наследует от темы: их видно
/// в выключенных контролах, и они же подставляются при включении.
struct ModuleDefaults {
    width: Option<f32>,
    height: Option<f32>,
    rounding: f32,
    fill: [u8; 4],
    border_color: [u8; 4],
}

impl ModuleDefaults {
    fn width_range(&self) -> std::ops::RangeInclusive<f32> {
        let w = self.width.unwrap_or(100.0);
        (w * 0.4).max(16.0)..=(w * 2.5).max(64.0)
    }

    fn height_range(&self) -> std::ops::RangeInclusive<f32> {
        let h = self.height.unwrap_or(40.0);
        (h * 0.5).max(16.0)..=(h * 2.5).max(48.0)
    }
}

fn rgba(c: egui::Color32) -> [u8; 4] {
    [c.r(), c.g(), c.b(), c.a()]
}

pub fn show(ui: &mut egui::Ui, state: &mut SettingsState, theme: &mut ThemePreset) {
    super::components::page_title(ui,"Настройки","Оформление, модули и переносимые темы",theme);
    ui.horizontal_wrapped(|ui| {
        for (i,name) in ["Оформление","Модули","JSON-пресет"].iter().enumerate() {
            if ui.add_sized([120.0,40.0],egui::SelectableLabel::new(state.category==i,*name)).clicked() {
                state.category=i;
            }
        }
    });
    ui.add_space(16.0);

    // Снимок темы для отрисовки карточек и значений по умолчанию,
    // пока саму тему правят контролы.
    let look = theme.clone();
    let mut changed = false;

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            if state.category == 0 {
            section(
                ui,
                &look,
                "Тема оформления",
                "Базовые цвета и формы — их наследуют все модули",
                |ui| {
                    changed |= ui.checkbox(&mut theme.dark, "Тёмная тема").changed();
                    ui.horizontal(|ui| {
                        ui.label("Акцентный цвет");
                        let mut col = look.accent_color();
                        if ui.color_edit_button_srgba(&mut col).changed() {
                            theme.accent = rgba(col);
                            changed = true;
                        }
                    });
                    ui.horizontal(|ui| {
                        ui.label("Цвет фона панелей");
                        let mut col = color_arr(look.background);
                        if ui.color_edit_button_srgba(&mut col).changed() {
                            theme.background = rgba(col);
                            changed = true;
                        }
                    });
                    changed |= ui
                        .add(
                            egui::Slider::new(&mut theme.rounding, 0.0..=16.0)
                                .text("Скругление виджетов"),
                        )
                        .changed();
                    changed |= ui
                        .add(
                            egui::Slider::new(&mut theme.opacity, 0.3..=1.0)
                                .text("Непрозрачность панелей"),
                        )
                        .changed();
                },
            );

            }
            if state.category == 1 {
            section(
                ui,
                &look,
                "Модули интерфейса",
                "Каждый модуль можно расширять и менять его углы, цвет, прозрачность и бортики",
                |ui| {
                    changed |= module_editor(
                        ui,
                        "m_titlebar",
                        "Верхняя панель",
                        &mut theme.modules.titlebar,
                        ModuleDefaults {
                            width: None,
                            height: Some(48.0),
                            rounding: 0.0,
                            fill: [0, 0, 0, 0],
                            border_color: rgba(egui::Color32::from_white_alpha(14)),
                        },
                    );
                    changed |= module_editor(
                        ui,
                        "m_sidebar",
                        "Боковое меню",
                        &mut theme.modules.sidebar,
                        ModuleDefaults {
                            width: Some(184.0),
                            height: None,
                            rounding: 0.0,
                            fill: rgba(look.surface(1)),
                            border_color: rgba(egui::Color32::from_white_alpha(14)),
                        },
                    );
                    changed |= module_editor(
                        ui,
                        "m_version",
                        "Кнопка выбора сборки",
                        &mut theme.modules.version_button,
                        ModuleDefaults {
                            width: Some(260.0),
                            height: Some(48.0),
                            rounding: 10.0,
                            fill: rgba(look.card_fill()),
                            border_color: rgba(egui::Color32::from_white_alpha(14)),
                        },
                    );
                    changed |= module_editor(
                        ui,
                        "m_play",
                        "Кнопка «Играть»",
                        &mut theme.modules.play_button,
                        ModuleDefaults {
                            width: Some(172.0),
                            height: Some(48.0),
                            rounding: 10.0,
                            fill: rgba(look.accent_color()),
                            border_color: rgba(egui::Color32::from_white_alpha(30)),
                        },
                    );
                    changed |= module_editor(
                        ui,
                        "m_chip",
                        "Кнопка профиля",
                        &mut theme.modules.profile_chip,
                        ModuleDefaults {
                            width: None,
                            height: Some(32.0),
                            rounding: 8.0,
                            fill: rgba(look.surface(2)),
                            border_color: rgba(egui::Color32::from_white_alpha(10)),
                        },
                    );
                    changed |= module_editor(
                        ui,
                        "m_tabcard",
                        "Область содержимого",
                        &mut theme.modules.tab_card,
                        ModuleDefaults {
                            width: None,
                            height: None,
                            rounding: 0.0,
                            fill: [0,0,0,0],
                            border_color: rgba(look.surface(4)),
                        },
                    );
                },
            );

            }
            if state.category == 0 {
            section(
                ui,
                &look,
                "Фон и атмосфера",
                "Задник без картинки, свечения, виньетка и частицы",
                |ui| {
                    let bg = &mut theme.modules.background;
                    changed |= override_color(
                        ui,
                        "Верхний цвет задника",
                        &mut bg.top,
                        rgba(look.background_color()),
                    );
                    changed |= override_color(
                        ui,
                        "Нижний цвет задника",
                        &mut bg.bottom,
                        rgba(look.background_color()),
                    );
                    changed |= ui
                        .checkbox(&mut bg.glow, "Акцентные свечения на фоне")
                        .changed();
                    if bg.glow {
                        changed |= ui
                            .add(
                                egui::Slider::new(&mut bg.glow_strength, 0.2..=2.5)
                                    .text("Сила свечений"),
                            )
                            .changed();
                    }
                    changed |= ui
                        .add(
                            egui::Slider::new(&mut bg.vignette, 0.0..=1.5)
                                .text("Виньетка по краям"),
                        )
                        .changed();
                    changed |= ui
                        .checkbox(&mut theme.modules.mist, "Частицы «мглы»")
                        .changed();
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new(
                            "Своя картинка фона: положи background.png (или .jpg) в папку данных лаунчера — .caligo в AppData",
                        )
                        .size(11.0)
                        .color(look.text_tertiary()),
                    );
                },
            );

            }
            if state.category == 2 {
            section(
                ui,
                &look,
                "Тема из JSON-пресета",
                "Пресет — один файл со всеми настройками, им можно делиться",
                |ui| {
                    ui.horizontal(|ui| {
                        if ui.button("Выгрузить текущую тему").clicked() {
                            state.theme_json = serde_json::to_string_pretty(&look)
                                .unwrap_or_default();
                            state.theme_error = None;
                        }
                        if ui.button("Сбросить всё").clicked() {
                            *theme = ThemePreset::default();
                            changed = true;
                            state.theme_error = None;
                        }
                    });
                    ui.add_space(6.0);
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
                                changed = true;
                                state.theme_error = None;
                            }
                            Err(err) => {
                                state.theme_error = Some(format!("Ошибка в JSON: {err}"))
                            }
                        }
                    }
                    if let Some(err) = &state.theme_error {
                        ui.colored_label(egui::Color32::from_rgb(255, 120, 120), err);
                    }
                },
            );
            }
        });

    if changed {
        theme.apply(ui.ctx());
    }
}