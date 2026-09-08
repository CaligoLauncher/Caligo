use eframe::egui;

use crate::auth::{AuthManager, AuthState};
use crate::launch::manifest::ManifestVersion;
use crate::launch::run::LaunchProfile;
use crate::launch::{LaunchManager, LaunchState};
use crate::skin::{self, SkinManager};
use crate::theme::ThemePreset;

const OFFLINE_UUID: &str = "00000000-0000-0000-0000-000000000000";
/// Ширина правой карточки «Группа» по умолчанию.
const PANEL_W: f32 = 232.0;
/// Высота кнопок нижнего дока по умолчанию.
const BTN_H: f32 = 48.0;
/// Ширина сегмента выбора сборки по умолчанию.
const VERSION_W: f32 = 236.0;
/// Ширина кнопки ИГРАТЬ по умолчанию.
const PLAY_W: f32 = 168.0;
/// Высота зоны приветствия сверху.
const GREETING_H: f32 = 86.0;

#[derive(Default)]
pub struct PlayState {
    pub selected_version: Option<String>,
    /// Имя выбранной сборки (вкладка «Сборки»). Задаёт подпись в доке;
    /// ручной выбор версии в доке сбрасывает выбор сборки.
    pub selected_instance: Option<String>,
    pub offline_name: String,
}

/// Кромка Liquid Glass: контур по периметру + яркий «блик» по верхней
/// грани — стекло ловит свет сверху (specular highlight из HIG).
/// Если модулю задан собственный бортик — рисуется только он.
fn glass_edge_styled(
    painter: &egui::Painter,
    rect: egui::Rect,
    rounding: egui::Rounding,
    custom: Option<egui::Stroke>,
) {
    if let Some(stroke) = custom {
        if stroke.width > 0.0 {
            painter.rect_stroke(rect, rounding, stroke);
        }
        return;
    }
    painter.rect_stroke(
        rect,
        rounding,
        egui::Stroke::new(1.0_f32, egui::Color32::from_white_alpha(14)),
    );
    let r = rounding.nw.max(rounding.ne);
    painter.line_segment(
        [
            egui::pos2(rect.min.x + r, rect.min.y + 0.5),
            egui::pos2(rect.max.x - r, rect.min.y + 0.5),
        ],
        egui::Stroke::new(1.0_f32, egui::Color32::from_white_alpha(36)),
    );
}

/// Главное меню — основной экран лаунчера. Редакторская вёрстка:
/// крупное приветствие в левом верхнем углу задаёт тон, в центре —
/// статичная 3D-кукла игрока, справа — плавающая карточка группы,
/// внизу — единый «док запуска»: сборка → статус → ИГРАТЬ.
/// Профиль и вход — в чипе титлбара сверху справа (см. `app.rs`).
pub fn show(
    ui: &mut egui::Ui,
    theme: &ThemePreset,
    auth: &AuthManager,
    play: &mut PlayState,
    launch: &LaunchManager,
    skin_mgr: &SkinManager,
) {
    launch.ensure_versions(ui.ctx().clone());
    let accent = theme.accent_color();
    let full = ui.available_rect_before_wrap();

    // Нижний док — одна стеклянная полоса на всю ширину.
    let version_w = theme.modules.version_button.width_or(VERSION_W);
    let version_h = theme.modules.version_button.height_or(BTN_H);
    let play_w = theme.modules.play_button.width_or(PLAY_W);
    let play_h = theme.modules.play_button.height_or(BTN_H);
    let row_h = version_h.max(play_h);
    let dock_h = row_h + 24.0;
    let dock_rect =
        egui::Rect::from_min_max(egui::pos2(full.min.x, full.max.y - dock_h), full.max);

    // Правая карточка группы — плавающая, не на всю высоту:
    // от приветствия до дока, с собственным скруглением.
    let panel_w = theme.modules.group_panel.width_or(PANEL_W);
    let panel_rect = egui::Rect::from_min_max(
        egui::pos2(full.max.x - panel_w, full.min.y + 6.0),
        egui::pos2(full.max.x, dock_rect.min.y - 14.0),
    );
    friends_panel(ui, panel_rect, theme);

    // Центральная зона левее карточки группы и выше дока.
    let center = egui::Rect::from_min_max(
        full.min,
        egui::pos2(panel_rect.min.x - 18.0, dock_rect.min.y),
    );

    // Приветствие: маленькая акцентная планка + крупный заголовок +
    // тихая подпись. Типографика — герой экрана.
    let (nick, _) = identity(auth, play);
    greeting(ui, center, theme, accent, nick.as_deref());

    // Игрок (и его группа, когда она появится) — по центру.
    let doll_rect = egui::Rect::from_min_max(
        egui::pos2(center.min.x, center.min.y + GREETING_H),
        egui::pos2(center.max.x, center.max.y - 12.0),
    );
    paperdoll_area(ui, doll_rect, auth, play, skin_mgr, accent);

    launch_dock(ui, dock_rect, theme, auth, play, launch, version_w, version_h, play_w, play_h);
}

/// Приветствие в левом верхнем углу центральной зоны.
fn greeting(
    ui: &mut egui::Ui,
    center: egui::Rect,
    theme: &ThemePreset,
    accent: egui::Color32,
    nick: Option<&str>,
) {
    let painter = ui.painter();
    let x = center.min.x + 10.0;
    let y = center.min.y + 8.0;
    painter.rect_filled(
        egui::Rect::from_min_size(egui::pos2(x + 2.0, y), egui::vec2(34.0, 3.0)),
        egui::Rounding::same(1.5),
        accent,
    );
    let (title, subtitle) = match nick {
        Some(name) => (
            format!("Привет, {name}"),
            "Что запустим сегодня?".to_string(),
        ),
        None => (
            "Добро пожаловать".to_string(),
            "Войди в профиль справа сверху — и твой персонаж появится здесь".to_string(),
        ),
    };
    painter.text(
        egui::pos2(x, y + 10.0),
        egui::Align2::LEFT_TOP,
        title,
        egui::FontId::proportional(30.0),
        theme.text_primary(),
    );
    painter.text(
        egui::pos2(x + 2.0, y + 50.0),
        egui::Align2::LEFT_TOP,
        subtitle,
        egui::FontId::proportional(13.0),
        theme.text_tertiary(),
    );
}

/// Кто мы сейчас: ник + ключ для скина (UUID онлайн-аккаунта или оффлайн-ник).
fn identity(auth: &AuthManager, play: &PlayState) -> (Option<String>, Option<String>) {
    match auth.state() {
        AuthState::SignedIn(account) => {
            (Some(account.username.clone()), Some(account.uuid.clone()))
        }
        _ => {
            let name = play.offline_name.trim().to_string();
            if name.is_empty() {
                (None, None)
            } else {
                (Some(name.clone()), Some(name))
            }
        }
    }
}

/// Центральная зона: статичная 3D-кукла скина игрока, ник над головой.
/// Когда появится система групп, здесь встанут рядом куклы всей группы.
fn paperdoll_area(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    auth: &AuthManager,
    play: &PlayState,
    skin_mgr: &SkinManager,
    accent: egui::Color32,
) {
    if rect.height() < 120.0 {
        return; // слишком низкое окно — куклу не рисуем
    }
    let (name, key) = identity(auth, play);
    skin_mgr.ensure(ui.ctx(), key);

    if name.is_none() {
        return; // приветствие уже подсказало, где войти
    }
    let t = ui.input(|i| i.time) as f32;
    let tex = skin_mgr.texture();
    skin::paint_paperdoll(
        ui.painter(),
        rect,
        tex.as_ref(),
        name.as_deref(),
        accent,
        t,
    );
    if skin_mgr.loading() {
        ui.painter().text(
            egui::pos2(rect.center().x, rect.max.y - 8.0),
            egui::Align2::CENTER_BOTTOM,
            "Загружаю скин…",
            egui::FontId::proportional(12.0),
            ui.visuals().weak_text_color(),
        );
    } else if let Some(err) = skin_mgr.error() {
        ui.painter().text(
            egui::pos2(rect.center().x, rect.max.y - 8.0),
            egui::Align2::CENTER_BOTTOM,
            format!("Скин: {err}"),
            egui::FontId::proportional(12.0),
            ui.visuals().weak_text_color(),
        );
    }
}

/// Плавающая карточка справа: группа/друзья
/// (пока заглушка: система друзей появится в будущих версиях).
fn friends_panel(ui: &mut egui::Ui, rect: egui::Rect, theme: &ThemePreset) {
    let accent = theme.accent_color();
    let style = &theme.modules.group_panel;
    // Regular-стекло: по HIG крупные элементы непрозрачнее мелких,
    // чтобы контент поверх оставался читаемым.
    let rounding = egui::Rounding::same(style.rounding_or(20.0));
    ui.painter()
        .rect_filled(rect, rounding, style.fill_or(theme.glass_regular()));
    glass_edge_styled(ui.painter(), rect, rounding, style.border_override());

    let inner = rect.shrink(14.0);
    let mut ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );

    ui.horizontal(|ui| {
        let (dot, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
        ui.painter()
            .circle_filled(dot.center(), 3.0, accent.gamma_multiply(0.9));
        ui.add_space(2.0);
        ui.label(
            egui::RichText::new("Группа")
                .strong()
                .size(15.0)
                .color(theme.text_primary()),
        );
    });

    // Спокойная заглушка: один мягкий круг с «+», без фальшивых рядов.
    let c = egui::pos2(rect.center().x, rect.center().y - 24.0);
    ui.painter()
        .circle_filled(c, 30.0, accent.gamma_multiply(0.10));
    ui.painter()
        .circle_filled(c, 22.0, accent.gamma_multiply(0.16));
    ui.painter().text(
        c,
        egui::Align2::CENTER_CENTER,
        "+",
        egui::FontId::proportional(24.0),
        accent,
    );
    ui.painter().text(
        egui::pos2(c.x, c.y + 44.0),
        egui::Align2::CENTER_CENTER,
        "Пока никого",
        egui::FontId::proportional(13.0),
        theme.text_body(),
    );
    ui.painter().text(
        egui::pos2(c.x, c.y + 62.0),
        egui::Align2::CENTER_CENTER,
        "Друзья и совместные сборки",
        egui::FontId::proportional(11.0),
        theme.text_tertiary(),
    );
    ui.painter().text(
        egui::pos2(c.x, c.y + 76.0),
        egui::Align2::CENTER_CENTER,
        "появятся в будущих версиях",
        egui::FontId::proportional(11.0),
        theme.text_tertiary(),
    );
}

/// Док запуска: одна стеклянная полоса внизу экрана.
/// Слева — сегмент сборки/версии, по центру — статус, справа — ИГРАТЬ.
#[allow(clippy::too_many_arguments)]
fn launch_dock(
    ui: &mut egui::Ui,
    dock: egui::Rect,
    theme: &ThemePreset,
    auth: &AuthManager,
    play: &mut PlayState,
    launch: &LaunchManager,
    version_w: f32,
    version_h: f32,
    play_w: f32,
    play_h: f32,
) {
    // Подложка дока: regular-стекло со скруглением карточек.
    let rounding = egui::Rounding::same(18.0);
    ui.painter()
        .rect_filled(dock, rounding, theme.glass_regular());
    glass_edge_styled(ui.painter(), dock, rounding, None);

    let inner = dock.shrink2(egui::vec2(14.0, 0.0));
    let version_rect = egui::Rect::from_min_size(
        egui::pos2(inner.min.x, dock.center().y - version_h / 2.0),
        egui::vec2(version_w.min(inner.width() * 0.45), version_h),
    );
    version_segment(ui, version_rect, theme, play, launch);

    let play_rect = egui::Rect::from_min_size(
        egui::pos2(inner.max.x - play_w, dock.center().y - play_h / 2.0),
        egui::vec2(play_w, play_h),
    );
    launch_controls(ui, play_rect, version_rect, theme, auth, play, launch);
}

/// Векторная иконка куба (сборка) — штриховая, в стиле иконок сайдбара.
fn paint_cube_icon(painter: &egui::Painter, center: egui::Pos2, color: egui::Color32) {
    let stroke = egui::Stroke::new(1.4_f32, color);
    let p = |x: f32, y: f32| center + egui::vec2(x, y);
    let top = p(0.0, -6.0);
    let ne = p(5.2, -3.0);
    let se = p(5.2, 3.0);
    let bottom = p(0.0, 6.0);
    let sw = p(-5.2, 3.0);
    let nw = p(-5.2, -3.0);
    let mid = p(0.0, 0.0);
    for seg in [
        [top, ne],
        [ne, se],
        [se, bottom],
        [bottom, sw],
        [sw, nw],
        [nw, top],
        [mid, nw],
        [mid, ne],
        [mid, bottom],
    ] {
        painter.line_segment(seg, stroke);
    }
}

/// Сегмент дока слева: какая сборка/версия будет запущена.
fn version_segment(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    theme: &ThemePreset,
    play: &mut PlayState,
    launch: &LaunchManager,
) {
    // Монохромная капсула: цвет остаётся только у главного действия.
    let style = &theme.modules.version_button;
    let rounding = egui::Rounding::same(style.rounding_or(rect.height() / 2.0));
    if let Some(fill) = style.fill {
        ui.painter()
            .rect_filled(rect, rounding, crate::theme::color_arr(fill));
    } else {
        ui.painter().rect_filled(rect, rounding, theme.glass_dim());
        ui.painter().rect_filled(rect, rounding, theme.card_fill());
    }
    glass_edge_styled(ui.painter(), rect, rounding, style.border_override());
    let icon_c = egui::pos2(rect.min.x + 22.0, rect.center().y);
    paint_cube_icon(ui.painter(), icon_c, theme.text_body());
    let inner = egui::Rect::from_min_max(
        egui::pos2(rect.min.x + 38.0, rect.min.y + 6.0),
        rect.max - egui::vec2(12.0, 6.0),
    );
    let mut ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    match launch.versions() {
        None => {
            ui.spinner();
            ui.label(egui::RichText::new("Версии…").size(13.0));
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(300));
        }
        Some(Err(err)) => {
            ui.colored_label(
                egui::Color32::from_rgb(255, 120, 120),
                egui::RichText::new("Версии: ошибка").size(13.0),
            )
            .on_hover_text(err);
        }
        Some(Ok(versions)) => {
            let releases: Vec<ManifestVersion> = versions
                .iter()
                .filter(|v| v.kind == "release")
                .cloned()
                .collect();
            if releases.is_empty() {
                ui.label("Нет версий");
                return;
            }
            let selected_id = play
                .selected_version
                .clone()
                .unwrap_or_else(|| releases[0].id.clone());
            // Подпись: имя выбранной сборки, иначе «Minecraft {версия}».
            let label = match &play.selected_instance {
                Some(name) => name.clone(),
                None => format!("Minecraft {selected_id}"),
            };
            let mut manual_pick = false;
            egui::ComboBox::from_id_salt("mc_version")
                .selected_text(label)
                .width(ui.available_width())
                .show_ui(&mut ui, |ui| {
                    for v in releases.iter().take(40) {
                        if ui
                            .selectable_value(
                                &mut play.selected_version,
                                Some(v.id.clone()),
                                &v.id,
                            )
                            .clicked()
                        {
                            manual_pick = true;
                        }
                    }
                });
            // Ручной выбор версии — это отказ от сборки.
            if manual_pick {
                play.selected_instance = None;
            }
        }
    }
}

/// Кнопка ИГРАТЬ справа в доке + статус запуска между сегментами.
fn launch_controls(
    ui: &mut egui::Ui,
    play_rect: egui::Rect,
    version_rect: egui::Rect,
    theme: &ThemePreset,
    auth: &AuthManager,
    play: &mut PlayState,
    launch: &LaunchManager,
) {
    let accent = theme.accent_color();
    let status_rect = egui::Rect::from_min_max(
        egui::pos2(version_rect.max.x + 14.0, play_rect.min.y),
        egui::pos2(play_rect.min.x - 14.0, play_rect.max.y),
    );
    let mut status_ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(status_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    match launch.state() {
        LaunchState::Preparing(step) => {
            play_button_at(ui, play_rect, theme, false);
            status_ui.spinner();
            status_ui.label(egui::RichText::new(step).size(13.0));
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(300));
        }
        LaunchState::Running => {
            play_button_at(ui, play_rect, theme, false);
            status_ui.colored_label(accent, "Игра запущена");
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_secs(1));
        }
        state => {
            let version = selected_version(play, launch);
            let resp = play_button_at(ui, play_rect, theme, version.is_some());
            if resp.clicked() {
                if let Some(v) = version {
                    launch.launch(ui.ctx().clone(), v, profile_for(auth, play));
                }
            }
            match state {
                LaunchState::Exited(code) => {
                    status_ui.label(
                        egui::RichText::new(format!("Игра завершилась (код {code})")).size(13.0),
                    );
                }
                LaunchState::Failed(err) => {
                    status_ui.colored_label(
                        egui::Color32::from_rgb(255, 120, 120),
                        egui::RichText::new(err).size(12.0),
                    );
                }
                _ => {}
            }
        }
    }
}

/// Кнопка ИГРАТЬ: капсула, единственный цветной контроль экрана,
/// пульсирующее свечение и треугольник запуска перед текстом.
fn play_button_at(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    theme: &ThemePreset,
    enabled: bool,
) -> egui::Response {
    let accent = theme.accent_color();
    let style = &theme.modules.play_button;
    let response = ui.interact(
        rect,
        ui.id().with("play_btn"),
        if enabled {
            egui::Sense::click()
        } else {
            egui::Sense::hover()
        },
    );
    let hover = ui
        .ctx()
        .animate_bool(response.id.with("hover"), enabled && response.hovered());
    let t = ui.input(|i| i.time) as f32;
    // Медленное «дыхание» свечения; перерисовка уже идёт из-за частиц.
    let pulse = ((t * 1.6).sin() * 0.5 + 0.5) * 0.5 + 0.5;
    // Капсула (радиус = половине высоты) — предпочтительная форма
    // контролов Tahoe. Акцент нанесён на СТЕКЛО кнопки, а не на текст:
    // это единственный цветной контроль на экране.
    let base_rounding = style.rounding_or(rect.height() / 2.0);
    let rounding = egui::Rounding::same(base_rounding);

    let painter = ui.painter();
    if enabled {
        painter.rect_filled(
            rect.expand(9.0 + 3.0 * hover),
            egui::Rounding::same(base_rounding + 9.0),
            accent.gamma_multiply(0.10 * pulse + 0.14 * hover),
        );
    }
    // Полный, непрозрачный акцент: главное действие не должно тонуть
    // в тёмном фоне.
    let fill = if enabled {
        style.fill_or(accent)
    } else {
        theme.glass_clear()
    };
    painter.rect_filled(rect, rounding, fill);
    if enabled && hover > 0.0 {
        painter.rect_filled(
            rect,
            rounding,
            egui::Color32::from_white_alpha((22.0 * hover) as u8),
        );
    }
    // Блик по верхней половине: тонированное стекло тоже ловит свет.
    let sheen = egui::Rect::from_min_max(
        rect.min,
        egui::pos2(rect.max.x, rect.min.y + rect.height() * 0.5),
    );
    painter.rect_filled(sheen, rounding, egui::Color32::from_white_alpha(24));
    // Собственный бортик кнопки (по умолчанию его нет).
    if let Some(stroke) = style.border_override() {
        if stroke.width > 0.0 {
            painter.rect_stroke(rect, rounding, stroke);
        }
    }
    let text_color = if enabled {
        egui::Color32::WHITE
    } else {
        egui::Color32::from_gray(140)
    };
    // Треугольник запуска + текст — по центру как одна группа.
    let galley = painter.layout_no_wrap(
        "Играть".to_string(),
        egui::FontId::proportional(16.0),
        text_color,
    );
    let tri_w = 9.0;
    let gap = 9.0;
    let total = tri_w + gap + galley.size().x;
    let left = rect.center().x - total / 2.0;
    let cy = rect.center().y;
    painter.add(egui::Shape::convex_polygon(
        vec![
            egui::pos2(left, cy - 5.5),
            egui::pos2(left, cy + 5.5),
            egui::pos2(left + tri_w, cy),
        ],
        text_color,
        egui::Stroke::NONE,
    ));
    painter.galley(
        egui::pos2(left + tri_w + gap, cy - galley.size().y / 2.0),
        galley,
        text_color,
    );
    if enabled {
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        response.on_hover_text("Выбери версию или сборку слева")
    }
}

/// Профиль для запуска: онлайн-аккаунт или оффлайн-режим по нику.
fn profile_for(auth: &AuthManager, play: &PlayState) -> LaunchProfile {
    match auth.state() {
        AuthState::SignedIn(account) => LaunchProfile {
            username: account.username.clone(),
            uuid: account.uuid.clone(),
            access_token: account.access_token.clone(),
        },
        _ => {
            let name = if play.offline_name.trim().is_empty() {
                "Player".to_string()
            } else {
                play.offline_name.trim().to_string()
            };
            LaunchProfile {
                username: name,
                uuid: OFFLINE_UUID.to_string(),
                access_token: "0".to_string(),
            }
        }
    }
}

fn selected_version(play: &PlayState, launch: &LaunchManager) -> Option<ManifestVersion> {
    let versions = launch.versions()?.ok()?;
    let releases: Vec<ManifestVersion> = versions
        .iter()
        .filter(|v| v.kind == "release")
        .cloned()
        .collect();
    let selected_id = match play.selected_version.clone() {
        Some(id) => id,
        None => releases.first()?.id.clone(),
    };
    releases.iter().find(|v| v.id == selected_id).cloned()
}