use eframe::egui;

use crate::auth::{AuthManager, AuthState};
use crate::launch::manifest::ManifestVersion;
use crate::launch::run::LaunchProfile;
use crate::launch::{LaunchManager, LaunchState};
use crate::skin::{self, SkinManager};
use crate::theme::ThemePreset;

const OFFLINE_UUID: &str = "00000000-0000-0000-0000-000000000000";
/// Ширина правой панели «Группа».
const PANEL_W: f32 = 232.0;
/// Высота нижних мини-кнопок.
const BTN_H: f32 = 48.0;
/// Ширина мини-кнопки выбора сборки.
const VERSION_W: f32 = 216.0;
/// Ширина мини-кнопки ИГРАТЬ.
const PLAY_W: f32 = 176.0;

#[derive(Default)]
pub struct PlayState {
    pub selected_version: Option<String>,
    pub offline_name: String,
}

/// Плавное смешение двух цветов.
fn mix(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t.clamp(0.0, 1.0)) as u8;
    egui::Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

/// Главное меню по макету: в центре — игрок (а в будущем и его группа),
/// справа — панель группы/друзей на всю высоту, внизу две мини-кнопки:
/// слева выбор сборки, справа ИГРАТЬ.
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

    // Правая панель — на всю высоту вкладки.
    let panel_rect =
        egui::Rect::from_min_max(egui::pos2(full.max.x - PANEL_W, full.min.y), full.max);
    friends_panel(ui, panel_rect, theme, auth, play);

    // Центральная зона левее панели.
    let center =
        egui::Rect::from_min_max(full.min, egui::pos2(panel_rect.min.x - 18.0, full.max.y));
    let bottom_y = center.max.y - BTN_H;

    // Центр — игрок (и его группа, когда она появится).
    let doll_rect = egui::Rect::from_min_max(
        egui::pos2(center.min.x, center.min.y + 4.0),
        egui::pos2(center.max.x, bottom_y - 16.0),
    );
    paperdoll_area(ui, doll_rect, auth, play, skin_mgr, accent);

    // Мини-кнопка слева: выбор версии/сборки.
    let version_rect = egui::Rect::from_min_size(
        egui::pos2(center.min.x, bottom_y),
        egui::vec2(VERSION_W.min(center.width() * 0.45), BTN_H),
    );
    version_button(ui, version_rect, theme, play, launch);

    // Мини-кнопка справа: ИГРАТЬ; статус — между кнопками.
    let play_rect = egui::Rect::from_min_size(
        egui::pos2(center.max.x - PLAY_W, bottom_y),
        egui::vec2(PLAY_W, BTN_H),
    );
    launch_controls(ui, play_rect, version_rect, theme, auth, play, launch);
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

/// Центральная зона: покачивающаяся 3D-кукла скина игрока, ник над головой.
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
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "Войди в аккаунт или введи ник справа —\nи твой персонаж появится здесь",
            egui::FontId::proportional(15.0),
            ui.visuals().weak_text_color(),
        );
        return;
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

/// Правая панель на всю высоту: аккаунт игрока сверху, ниже — группа/друзья
/// (пока заглушки: система друзей появится в будущих версиях).
fn friends_panel(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    theme: &ThemePreset,
    auth: &AuthManager,
    play: &mut PlayState,
) {
    let accent = theme.accent_color();
    let rounding = egui::Rounding::same(theme.rounding * 1.4);
    ui.painter().rect_filled(rect, rounding, theme.glass_fill());

    let inner = rect.shrink(14.0);
    let mut ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::top_down(egui::Align::Min)),
    );

    ui.label(egui::RichText::new("ТЫ").small().weak());
    ui.add_space(6.0);
    account_block(&mut ui, auth, play, accent);

    ui.add_space(14.0);
    thin_line(&mut ui);
    ui.add_space(14.0);

    ui.label(egui::RichText::new("ГРУППА").small().weak());
    ui.add_space(8.0);
    for i in 0..3 {
        ghost_friend_row(&mut ui, i);
        ui.add_space(6.0);
    }
    ui.add_space(8.0);
    ui.label(
        egui::RichText::new("Друзья и совместные сборки появятся в будущих версиях")
            .weak()
            .size(12.0),
    );
}

/// Тонкая разделительная линия панели.
fn thin_line(ui: &mut egui::Ui) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 1.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, 0.0, egui::Color32::from_white_alpha(14));
}

/// «Призрачный» слот друга: место, где встанет карточка живого человека.
fn ghost_friend_row(ui: &mut egui::Ui, i: usize) {
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 38.0), egui::Sense::hover());
    let rounding = egui::Rounding::same(10.0);
    let alpha = 5u8.saturating_sub(i as u8);
    ui.painter()
        .rect_filled(rect, rounding, egui::Color32::from_white_alpha(alpha.max(2)));
    ui.painter().circle_filled(
        egui::pos2(rect.min.x + 19.0, rect.center().y),
        11.0,
        egui::Color32::from_white_alpha(10),
    );
    let bar = egui::Rect::from_min_size(
        egui::pos2(rect.min.x + 38.0, rect.center().y - 4.0),
        egui::vec2((w - 56.0).max(20.0) * (0.9 - 0.15 * i as f32), 8.0),
    );
    ui.painter()
        .rect_filled(bar, egui::Rounding::same(4.0), egui::Color32::from_white_alpha(8));
}

/// Блок аккаунта в правой панели: вход / оффлайн-ник / статус входа.
fn account_block(
    ui: &mut egui::Ui,
    auth: &AuthManager,
    play: &mut PlayState,
    accent: egui::Color32,
) {
    match auth.state() {
        AuthState::SignedOut => {
            ui.add(
                egui::TextEdit::singleline(&mut play.offline_name)
                    .hint_text("Ник (оффлайн)")
                    .desired_width(ui.available_width()),
            );
            ui.add_space(6.0);
            if ui
                .add_sized(
                    egui::vec2(ui.available_width(), 30.0),
                    egui::Button::new("Войти через Microsoft"),
                )
                .clicked()
            {
                auth.start_login(ui.ctx().clone());
            }
        }
        AuthState::WaitingForUser {
            verification_uri,
            user_code,
        } => {
            ui.label(egui::RichText::new("Открой ссылку и введи код:").size(13.0));
            ui.hyperlink(&verification_uri);
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(&user_code)
                    .size(24.0)
                    .monospace()
                    .strong()
                    .color(accent),
            );
            if ui.button("Скопировать код").clicked() {
                ui.ctx().output_mut(|o| o.copied_text = user_code.clone());
            }
            ui.spinner();
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(500));
        }
        AuthState::InProgress(step) => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(egui::RichText::new(step).size(13.0));
            });
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(500));
        }
        AuthState::SignedIn(account) => {
            ui.horizontal(|ui| {
                ui.colored_label(accent, egui::RichText::new(&account.username).strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.small_button("Выйти").clicked() {
                        auth.sign_out();
                    }
                });
            });
        }
        AuthState::Failed(err) => {
            ui.colored_label(
                egui::Color32::from_rgb(255, 120, 120),
                egui::RichText::new(format!("Ошибка входа: {err}")).size(12.0),
            );
            if ui.button("Попробовать снова").clicked() {
                auth.start_login(ui.ctx().clone());
            }
        }
    }
}

/// Мини-кнопка слева внизу: выбор версии/сборки на стеклянной подложке.
fn version_button(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    theme: &ThemePreset,
    play: &mut PlayState,
    launch: &LaunchManager,
) {
    ui.painter().rect_filled(
        rect,
        egui::Rounding::same(theme.rounding * 1.2),
        theme.glass_fill(),
    );
    let inner = rect.shrink2(egui::vec2(12.0, 6.0));
    let mut ui = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(inner)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    ui.label(egui::RichText::new("📦").size(15.0));
    ui.add_space(4.0);
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
            egui::ComboBox::from_id_salt("mc_version")
                .selected_text(format!("Minecraft {selected_id}"))
                .width(ui.available_width())
                .show_ui(&mut ui, |ui| {
                    for v in releases.iter().take(40) {
                        ui.selectable_value(&mut play.selected_version, Some(v.id.clone()), &v.id);
                    }
                });
        }
    }
}

/// Мини-кнопка ИГРАТЬ справа внизу + статус запуска между кнопками.
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
        egui::pos2(version_rect.max.x + 12.0, play_rect.min.y),
        egui::pos2(play_rect.min.x - 12.0, play_rect.max.y),
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
            status_ui.colored_label(accent, "Игра запущена 🎮");
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
                        egui::RichText::new(format!("Ошибка: {err}")).size(13.0),
                    );
                }
                _ => {}
            }
        }
    }
}

/// Кнопка ИГРАТЬ в заданном прямоугольнике: пульсирующий ореол,
/// разгорается при наведении — фирменная, не системная.
fn play_button_at(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    theme: &ThemePreset,
    enabled: bool,
) -> egui::Response {
    let accent = theme.accent_color();
    let response = ui.interact(rect, ui.id().with("play_btn"), egui::Sense::click());
    let hover = ui
        .ctx()
        .animate_bool(response.id.with("hover"), enabled && response.hovered());
    let t = ui.input(|i| i.time) as f32;
    // Медленное «дыхание» свечения; перерисовка уже идёт из-за частиц.
    let pulse = ((t * 1.6).sin() * 0.5 + 0.5) * 0.5 + 0.5;
    let rounding = egui::Rounding::same(theme.rounding * 1.2);

    let painter = ui.painter();
    if enabled {
        painter.rect_filled(
            rect.expand(8.0 + 3.0 * hover),
            egui::Rounding::same(theme.rounding * 1.2 + 8.0),
            accent.gamma_multiply(0.06 * pulse + 0.10 * hover),
        );
        painter.rect_filled(
            rect.expand(3.0 + 2.0 * hover),
            egui::Rounding::same(theme.rounding * 1.2 + 3.0),
            accent.gamma_multiply(0.12 * pulse + 0.12 * hover),
        );
    }
    let fill = if enabled {
        mix(accent, egui::Color32::WHITE, hover * 0.15)
    } else {
        egui::Color32::from_rgb(58, 61, 68)
    };
    painter.rect_filled(rect, rounding, fill);
    let sheen = egui::Rect::from_min_max(
        rect.min,
        egui::pos2(rect.max.x, rect.min.y + rect.height() * 0.45),
    );
    painter.rect_filled(sheen, rounding, egui::Color32::from_white_alpha(10));
    let text_color = if enabled {
        egui::Color32::WHITE
    } else {
        egui::Color32::from_gray(140)
    };
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "ИГРАТЬ",
        egui::FontId::proportional(20.0),
        text_color,
    );
    if enabled {
        response.clone().on_hover_cursor(egui::CursorIcon::PointingHand);
    }
    response
}

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