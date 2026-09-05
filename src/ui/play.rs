use eframe::egui;

use crate::auth::{AuthManager, AuthState};
use crate::launch::manifest::ManifestVersion;
use crate::launch::run::LaunchProfile;
use crate::launch::{LaunchManager, LaunchState};
use crate::theme::ThemePreset;

const OFFLINE_UUID: &str = "00000000-0000-0000-0000-000000000000";

#[derive(Default)]
pub struct PlayState {
    pub selected_version: Option<String>,
    pub offline_name: String,
}

pub fn show(
    ui: &mut egui::Ui,
    theme: &ThemePreset,
    auth: &AuthManager,
    play: &mut PlayState,
    launch: &LaunchManager,
) {
    launch.ensure_versions(ui.ctx().clone());
    let accent = theme.accent_color();

    // Контент прижат к низу, как в референсе: большая карточка запуска снизу,
    // статус аккаунта сверху.
    ui.with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
        ui.add_space(8.0);
        account_line(ui, auth, play, accent);
    });

    let card_h = 120.0;
    let bottom = ui.available_rect_before_wrap();
    let card_rect = egui::Rect::from_min_size(
        egui::pos2(bottom.min.x, bottom.max.y - card_h),
        egui::vec2(bottom.width(), card_h),
    );
    let mut card_ui = ui.new_child(egui::UiBuilder::new().max_rect(card_rect));
    launch_card(&mut card_ui, theme, auth, play, launch);
}

/// Верхняя строка: кто вошёл / вход / оффлайн-ник.
fn account_line(ui: &mut egui::Ui, auth: &AuthManager, play: &mut PlayState, accent: egui::Color32) {
    match auth.state() {
        AuthState::SignedOut => {
            ui.horizontal(|ui| {
                ui.label("Аккаунт:");
                ui.colored_label(ui.visuals().weak_text_color(), "не подключён");
                if ui.button("Войти через Microsoft").clicked() {
                    auth.start_login(ui.ctx().clone());
                }
            });
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                ui.label("Оффлайн-ник:");
                ui.add(
                    egui::TextEdit::singleline(&mut play.offline_name)
                        .hint_text("Player")
                        .desired_width(160.0),
                );
            });
        }
        AuthState::WaitingForUser {
            verification_uri,
            user_code,
        } => {
            ui.label("Открой ссылку и введи код:");
            ui.hyperlink(&verification_uri);
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(&user_code)
                    .size(28.0)
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
                ui.label(step);
            });
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(500));
        }
        AuthState::SignedIn(account) => {
            ui.horizontal(|ui| {
                ui.label("Аккаунт:");
                ui.colored_label(accent, &account.username);
                if ui.small_button("Выйти").clicked() {
                    auth.sign_out();
                }
            });
        }
        AuthState::Failed(err) => {
            ui.colored_label(egui::Color32::RED, format!("Ошибка входа: {err}"));
            if ui.button("Попробовать снова").clicked() {
                auth.start_login(ui.ctx().clone());
            }
        }
    }
}

/// Большая нижняя карточка: выбор версии + кнопка ИГРАТЬ + прогресс.
fn launch_card(
    ui: &mut egui::Ui,
    theme: &ThemePreset,
    auth: &AuthManager,
    play: &mut PlayState,
    launch: &LaunchManager,
) {
    let accent = theme.accent_color();
    let rect = ui.max_rect();
    ui.painter().rect_filled(
        rect,
        egui::Rounding::same(theme.rounding * 1.4),
        theme.glass_fill(),
    );

    let inner = rect.shrink(20.0);
    let mut ui = ui.new_child(egui::UiBuilder::new().max_rect(inner));
    ui.horizontal_centered(|ui| {
        // Слева: версия.
        ui.vertical(|ui| {
            ui.label(egui::RichText::new("ВЕРСИЯ").small().weak());
            ui.add_space(2.0);
            version_picker(ui, play, launch);
        });

        // Справа: кнопка + статус.
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(4.0);
            match launch.state() {
                LaunchState::Preparing(step) => {
                    ui.spinner();
                    ui.label(step);
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_millis(300));
                }
                LaunchState::Running => {
                    ui.colored_label(accent, "Игра запущена 🎮");
                    ui.ctx()
                        .request_repaint_after(std::time::Duration::from_secs(1));
                }
                state => {
                    let version = selected_version(play, launch);
                    let button = egui::Button::new(
                        egui::RichText::new("  ИГРАТЬ  ")
                            .size(26.0)
                            .strong()
                            .color(egui::Color32::WHITE),
                    )
                    .fill(accent)
                    .rounding(egui::Rounding::same(theme.rounding * 1.2))
                    .min_size(egui::vec2(220.0, 60.0));
                    if ui.add_enabled(version.is_some(), button).clicked() {
                        if let Some(v) = version {
                            launch.launch(ui.ctx().clone(), v, profile_for(auth, play));
                        }
                    }
                    ui.add_space(12.0);
                    match state {
                        LaunchState::Exited(code) => {
                            ui.label(format!("Игра завершилась (код {code})"));
                        }
                        LaunchState::Failed(err) => {
                            ui.colored_label(
                                egui::Color32::from_rgb(255, 120, 120),
                                format!("Ошибка: {err}"),
                            );
                        }
                        _ => {}
                    }
                }
            }
        });
    });
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

fn version_picker(ui: &mut egui::Ui, play: &mut PlayState, launch: &LaunchManager) {
    match launch.versions() {
        None => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Загружаю…");
            });
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(300));
        }
        Some(Err(err)) => {
            ui.colored_label(egui::Color32::RED, format!("Список версий: {err}"));
        }
        Some(Ok(versions)) => {
            let releases: Vec<ManifestVersion> = versions
                .iter()
                .filter(|v| v.kind == "release")
                .cloned()
                .collect();
            if releases.is_empty() {
                ui.label("Нет доступных версий");
                return;
            }
            let selected_id = play
                .selected_version
                .clone()
                .unwrap_or_else(|| releases[0].id.clone());
            egui::ComboBox::from_id_salt("mc_version")
                .selected_text(format!("Minecraft {selected_id}"))
                .width(180.0)
                .show_ui(ui, |ui| {
                    for v in releases.iter().take(40) {
                        ui.selectable_value(&mut play.selected_version, Some(v.id.clone()), &v.id);
                    }
                });
        }
    }
}
