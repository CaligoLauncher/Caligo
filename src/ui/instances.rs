use std::path::PathBuf;

use eframe::egui;
use serde::{Deserialize, Serialize};

use crate::launch::manifest::ManifestVersion;
use crate::launch::LaunchManager;
use crate::theme::ThemePreset;
use crate::ui::play::PlayState;

/// Сборка: имя + версия Minecraft. Хранится как отдельный JSON-файл
/// в `%APPDATA%\.caligo\instances\` — по файлу на сборку, чтобы сборки
/// можно было переносить и (в будущем) раздавать как манифесты.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Instance {
    pub name: String,
    pub version: String,
}

/// Состояние вкладки «Сборки»: библиотека, форма создания, ошибки.
#[derive(Default)]
pub struct InstancesState {
    instances: Vec<Instance>,
    loaded: bool,
    creating: bool,
    new_name: String,
    new_version: Option<String>,
    error: Option<String>,
}

fn instances_dir() -> PathBuf {
    crate::launch::install::game_dir().join("instances")
}

/// Имя файла из имени сборки: только буквы/цифры/дефис/подчёркивание.
fn sanitize(name: &str) -> String {
    let s: String = name
        .chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    if s.is_empty() {
        "instance".to_string()
    } else {
        s
    }
}

fn load_all() -> Vec<Instance> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(instances_dir()) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("json") {
            continue;
        }
        if let Ok(text) = std::fs::read_to_string(&path) {
            if let Ok(inst) = serde_json::from_str::<Instance>(&text) {
                out.push(inst);
            }
        }
    }
    out.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    out
}

fn save(inst: &Instance) -> Result<(), String> {
    let dir = instances_dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("Папка сборок: {e}"))?;
    let path = dir.join(format!("{}.json", sanitize(&inst.name)));
    let text = serde_json::to_string_pretty(inst).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| format!("Сохранение сборки: {e}"))
}

fn remove(inst: &Instance) {
    let path = instances_dir().join(format!("{}.json", sanitize(&inst.name)));
    let _ = std::fs::remove_file(path);
}

/// Русское множественное число для «сборка».
fn plural(n: usize) -> &'static str {
    let r10 = n % 10;
    let r100 = n % 100;
    if r10 == 1 && r100 != 11 {
        "сборка"
    } else if (2..=4).contains(&r10) && !(12..=14).contains(&r100) {
        "сборки"
    } else {
        "сборок"
    }
}

/// «Сборки» — библиотека реальных сборок: создание, выбор, удаление.
/// Выбранная сборка попадает в док главного меню и запускается кнопкой
/// «Играть». Модлоадеры и моды появятся на следующем этапе.
pub fn show(
    ui: &mut egui::Ui,
    theme: &ThemePreset,
    state: &mut InstancesState,
    play: &mut PlayState,
    launch: &LaunchManager,
) {
    if !state.loaded {
        state.instances = load_all();
        state.loaded = true;
    }
    launch.ensure_versions(ui.ctx().clone());

    ui.add_space(4.0);
    ui.label(
        egui::RichText::new("Сборки")
            .heading()
            .strong()
            .color(theme.text_primary()),
    );
    ui.add_space(2.0);
    let n = state.instances.len();
    let subline = if n == 0 {
        "Создай первую сборку — выбери имя и версию Minecraft".to_string()
    } else {
        format!("{n} {} · нажми на карточку, чтобы выбрать", plural(n))
    };
    ui.label(
        egui::RichText::new(subline)
            .size(13.0)
            .color(theme.text_tertiary()),
    );
    ui.add_space(16.0);

    if let Some(err) = state.error.clone() {
        ui.colored_label(
            egui::Color32::from_rgb(255, 120, 120),
            egui::RichText::new(err).size(12.0),
        );
        ui.add_space(8.0);
    }

    if state.creating {
        create_form(ui, theme, state, play, launch);
        ui.add_space(16.0);
    }

    let card = egui::vec2(190.0, 158.0);
    let mut delete_at: Option<usize> = None;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(16.0, 16.0);
        if create_card(ui, card, theme).clicked() {
            state.creating = true;
            state.error = None;
        }
        for (i, inst) in state.instances.iter().enumerate() {
            let selected = play.selected_instance.as_deref() == Some(inst.name.as_str());
            let (clicked, deleted) = instance_card(ui, card, theme, inst, selected, i);
            if clicked {
                play.selected_instance = Some(inst.name.clone());
                play.selected_version = Some(inst.version.clone());
            }
            if deleted {
                delete_at = Some(i);
            }
        }
    });
    if let Some(i) = delete_at {
        let inst = state.instances.remove(i);
        remove(&inst);
        if play.selected_instance.as_deref() == Some(inst.name.as_str()) {
            play.selected_instance = None;
        }
    }
}

/// Форма создания сборки: имя + версия Minecraft.
fn create_form(
    ui: &mut egui::Ui,
    theme: &ThemePreset,
    state: &mut InstancesState,
    play: &mut PlayState,
    launch: &LaunchManager,
) {
    let frame = egui::Frame::none()
        .fill(theme.card_fill())
        .stroke(theme.card_stroke())
        .rounding(egui::Rounding::same(14.0))
        .inner_margin(egui::Margin::same(14.0));
    frame.show(ui, |ui| {
        ui.label(
            egui::RichText::new("Новая сборка")
                .strong()
                .size(15.0)
                .color(theme.text_primary()),
        );
        ui.add_space(8.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Имя").size(13.0));
            ui.add(
                egui::TextEdit::singleline(&mut state.new_name)
                    .hint_text("Моя сборка")
                    .desired_width(220.0),
            );
            ui.add_space(10.0);
            ui.label(egui::RichText::new("Версия").size(13.0));
            match launch.versions() {
                None => {
                    ui.spinner();
                }
                Some(Err(_)) => {
                    ui.colored_label(
                        egui::Color32::from_rgb(255, 120, 120),
                        "версии не загрузились",
                    );
                }
                Some(Ok(versions)) => {
                    let releases: Vec<ManifestVersion> = versions
                        .iter()
                        .filter(|v| v.kind == "release")
                        .cloned()
                        .collect();
                    if state.new_version.is_none() {
                        state.new_version = releases.first().map(|v| v.id.clone());
                    }
                    let label = state.new_version.clone().unwrap_or_default();
                    egui::ComboBox::from_id_salt("new_instance_version")
                        .selected_text(label)
                        .show_ui(ui, |ui| {
                            for v in releases.iter().take(40) {
                                ui.selectable_value(
                                    &mut state.new_version,
                                    Some(v.id.clone()),
                                    &v.id,
                                );
                            }
                        });
                }
            }
        });
        ui.add_space(10.0);
        ui.horizontal(|ui| {
            let name = state.new_name.trim().to_string();
            let ready = !name.is_empty() && state.new_version.is_some();
            if ui
                .add_enabled(ready, egui::Button::new("Создать сборку"))
                .clicked()
            {
                if state
                    .instances
                    .iter()
                    .any(|i| i.name.to_lowercase() == name.to_lowercase())
                {
                    state.error = Some(format!("Сборка «{name}» уже есть — выбери другое имя"));
                } else if let Some(version) = state.new_version.clone() {
                    let inst = Instance {
                        name: name.clone(),
                        version: version.clone(),
                    };
                    match save(&inst) {
                        Ok(()) => {
                            state.instances.push(inst);
                            state
                                .instances
                                .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
                            play.selected_instance = Some(name);
                            play.selected_version = Some(version);
                            state.new_name.clear();
                            state.creating = false;
                            state.error = None;
                        }
                        Err(e) => state.error = Some(e),
                    }
                }
            }
            if ui.button("Отмена").clicked() {
                state.creating = false;
                state.new_name.clear();
                state.error = None;
            }
        });
    });
}

/// Карточка «создать сборку»: пунктирная рамка, разгорается при
/// наведении — приглашение к действию вместо пустоты.
fn create_card(ui: &mut egui::Ui, size: egui::Vec2, theme: &ThemePreset) -> egui::Response {
    let accent = theme.accent_color();
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let hover = ui
        .ctx()
        .animate_bool(response.id.with("h"), response.hovered());
    let press = ui
        .ctx()
        .animate_bool(response.id.with("p"), response.is_pointer_button_down_on());
    let rect = rect.shrink(rect.width() * 0.015 * press);
    let p = ui.painter();
    p.rect_filled(
        rect,
        egui::Rounding::same(16.0),
        theme.accent_bg().gamma_multiply(0.35 + 0.55 * hover),
    );
    // Пунктирная рамка по четырём сторонам.
    let stroke = egui::Stroke::new(1.0_f32, accent.gamma_multiply(0.45 + 0.35 * hover));
    let corners = [
        (rect.left_top(), rect.right_top()),
        (rect.right_top(), rect.right_bottom()),
        (rect.right_bottom(), rect.left_bottom()),
        (rect.left_bottom(), rect.left_top()),
    ];
    for (a, b) in corners {
        for shape in egui::Shape::dashed_line(&[a, b], stroke, 6.0, 5.0) {
            p.add(shape);
        }
    }
    let c = egui::pos2(rect.center().x, rect.center().y - 14.0);
    p.circle_filled(c, 19.0, accent.gamma_multiply(0.16 + 0.12 * hover));
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
        egui::FontId::proportional(13.0),
        theme.text_body(),
    );
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Хэш имени → пара оттенков для градиентной обложки карточки.
fn cover_colors(name: &str) -> (egui::Color32, egui::Color32) {
    let mut h: u32 = 2166136261;
    for b in name.as_bytes() {
        h ^= *b as u32;
        h = h.wrapping_mul(16777619);
    }
    let hue1 = (h % 360) as f32 / 360.0;
    let hue2 = ((h / 360) % 360) as f32 / 360.0;
    let c1 = egui::Color32::from(egui::ecolor::Hsva::new(hue1, 0.55, 0.45, 1.0));
    let c2 = egui::Color32::from(egui::ecolor::Hsva::new(hue2, 0.60, 0.28, 1.0));
    (c1, c2)
}

/// Карточка сборки в стиле постера: градиентная обложка из хэша имени,
/// крупная первая буква, имя и версия. Клик — выбрать; ✕ при
/// наведении — удалить. Возвращает (выбрана, удалена).
fn instance_card(
    ui: &mut egui::Ui,
    size: egui::Vec2,
    theme: &ThemePreset,
    inst: &Instance,
    selected: bool,
    index: usize,
) -> (bool, bool) {
    let accent = theme.accent_color();
    let (rect, response) = ui.allocate_exact_size(size, egui::Sense::click());
    let id = ui.id().with("instance").with(index);
    let hover = ui
        .ctx()
        .animate_bool(id.with("h"), response.hovered());
    let rounding = egui::Rounding::same(16.0);
    let p = ui.painter();
    // Свечение «из мглы» при наведении.
    if hover > 0.0 {
        p.rect_filled(
            rect.expand(6.0 * hover),
            egui::Rounding::same(20.0),
            accent.gamma_multiply(0.08 * hover),
        );
    }
    p.rect_filled(rect, rounding, theme.card_fill());
    // Обложка: диагональный градиент из двух оттенков имени.
    let cover = egui::Rect::from_min_max(
        rect.min + egui::vec2(8.0, 8.0),
        egui::pos2(rect.max.x - 8.0, rect.min.y + 92.0),
    );
    let (c1, c2) = cover_colors(&inst.name);
    let mut mesh = egui::Mesh::default();
    let mix = egui::Color32::from_rgb(
        ((c1.r() as u16 + c2.r() as u16) / 2) as u8,
        ((c1.g() as u16 + c2.g() as u16) / 2) as u8,
        ((c1.b() as u16 + c2.b() as u16) / 2) as u8,
    );
    mesh.colored_vertex(cover.left_top(), c1);
    mesh.colored_vertex(cover.right_top(), mix);
    mesh.colored_vertex(cover.left_bottom(), mix);
    mesh.colored_vertex(cover.right_bottom(), c2);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 3, 2);
    p.add(egui::Shape::mesh(mesh));
    // Крупная первая буква — «постер» сборки.
    let letter = inst
        .name
        .chars()
        .next()
        .map(|c| c.to_uppercase().to_string())
        .unwrap_or_default();
    p.text(
        cover.center(),
        egui::Align2::CENTER_CENTER,
        letter,
        egui::FontId::proportional(40.0),
        egui::Color32::from_white_alpha(210),
    );
    // Имя и версия.
    p.text(
        egui::pos2(rect.min.x + 12.0, cover.max.y + 10.0),
        egui::Align2::LEFT_TOP,
        &inst.name,
        egui::FontId::proportional(14.0),
        theme.text_primary(),
    );
    p.text(
        egui::pos2(rect.min.x + 12.0, cover.max.y + 30.0),
        egui::Align2::LEFT_TOP,
        format!("Minecraft {}", inst.version),
        egui::FontId::proportional(12.0),
        theme.text_tertiary(),
    );
    // Рамка: акцентная у выбранной, обычная у остальных.
    if selected {
        p.rect_stroke(rect, rounding, egui::Stroke::new(1.5_f32, accent));
        p.text(
            egui::pos2(rect.max.x - 12.0, cover.max.y + 30.0),
            egui::Align2::RIGHT_TOP,
            "выбрана",
            egui::FontId::proportional(11.0),
            accent,
        );
    } else {
        p.rect_stroke(rect, rounding, theme.card_stroke());
    }
    // ✕ удаления — только при наведении на карточку.
    let mut deleted = false;
    if hover > 0.3 {
        let x_rect = egui::Rect::from_center_size(
            egui::pos2(rect.max.x - 18.0, rect.min.y + 18.0),
            egui::vec2(22.0, 22.0),
        );
        let x_resp = ui.interact(x_rect, id.with("del"), egui::Sense::click());
        let x_hover = ui.ctx().animate_bool(id.with("delh"), x_resp.hovered());
        let fill = if x_hover > 0.0 {
            egui::Color32::from_rgba_unmultiplied(232, 17, 35, (60.0 + 140.0 * x_hover) as u8)
        } else {
            egui::Color32::from_black_alpha(120)
        };
        ui.painter().circle_filled(x_rect.center(), 10.0, fill);
        let sc = x_rect.center();
        let stroke = egui::Stroke::new(1.3_f32, egui::Color32::WHITE);
        ui.painter()
            .line_segment([sc + egui::vec2(-3.5, -3.5), sc + egui::vec2(3.5, 3.5)], stroke);
        ui.painter()
            .line_segment([sc + egui::vec2(-3.5, 3.5), sc + egui::vec2(3.5, -3.5)], stroke);
        if x_resp.on_hover_text("Удалить сборку").clicked() {
            deleted = true;
        }
    }
    let clicked = response.clicked() && !deleted;
    (
        clicked,
        deleted,
    )
}