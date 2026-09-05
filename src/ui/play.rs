use eframe::egui;

use crate::auth::{AuthManager, AuthState};
use crate::launch::manifest::ManifestVersion;
use crate::launch::run::LaunchProfile;
use crate::launch::{LaunchManager, LaunchState};

const OFFLINE_UUID: &str = "00000000-0000-0000-0000-000000000000";

#[derive(Default)]
pub struct PlayState {
    pub selected_version: Option<String>,
    pub offline_name: String,
}

pub fn show(ui: &mut egui::Ui, auth: &AuthManager, play: &mut PlayState, launch: &LaunchManager) {
    launch.ensure_versions(ui.ctx().clone());
    ui.add_space(32.0);
    ui.vertical_centered(|ui| {
        ui.heading("Terra Launcher");
        ui.add_space(16.0);

        match auth.state() {
            AuthState::SignedOut => {
                ui.label("Аккаунт не подключён.");
                ui.add_space(8.0);
                if ui.button("Войти через Microsoft").clicked() {
                    auth.start_login(ui.ctx().clone());
                }
                ui.add_space(24.0);
                ui.separator();
                ui.add_space(8.0);
                ui.label("Тестовый запуск без аккаунта (оффлайн):");
                ui.add_space(4.0);
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() / 2.0 - 110.0);
                    ui.label("Ник:");
                    ui.add(
                        egui::TextEdit::singleline(&mut play.offline_name)
                            .hint_text("Player")
                            .desired_width(160.0),
                    );
                });
                ui.add_space(8.0);
                let name = if play.offline_name.trim().is_empty() {
                    "Player".to_string()
                } else {
                    play.offline_name.trim().to_string()
                };
                launch_section(
                    ui,
                    play,
                    launch,
                    LaunchProfile {
                        username: name,
                        uuid: OFFLINE_UUID.to_string(),
                        access_token: "0".to_string(),
                    },
                );
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
                        .strong(),
                );
                if ui.button("Скопировать код").clicked() {
                    ui.ctx().output_mut(|o| o.copied_text = user_code.clone());
                }
                ui.add_space(4.0);
                ui.spinner();
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(500));
            }
            AuthState::InProgress(step) => {
                ui.label(step);
                ui.spinner();
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_millis(500));
            }
            AuthState::SignedIn(account) => {
                ui.label(format!("Вошёл как {}", account.username));
                if ui.small_button("Выйти").clicked() {
                    auth.sign_out();
                }
                ui.add_space(16.0);
                launch_section(
                    ui,
                    play,
                    launch,
                    LaunchProfile {
                        username: account.username.clone(),
                        uuid: account.uuid.clone(),
                        access_token: account.access_token.clone(),
                    },
                );
            }
            AuthState::Failed(err) => {
                ui.colored_label(egui::Color32::RED, format!("Ошибка входа: {err}"));
                ui.add_space(8.0);
                if ui.button("Попробовать снова").clicked() {
                    auth.start_login(ui.ctx().clone());
                }
            }
        }
    });
}

fn launch_section(
    ui: &mut egui::Ui,
    play: &mut PlayState,
    launch: &LaunchManager,
    profile: LaunchProfile,
) {
    let version = version_picker(ui, play, launch);
    ui.add_space(12.0);
    match launch.state() {
        LaunchState::Preparing(step) => {
            ui.label(step);
            ui.spinner();
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(300));
        }
        LaunchState::Running => {
            ui.label("Игра запущена 🎮");
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_secs(1));
        }
        state => {
            match state {
                LaunchState::Exited(code) => {
                    ui.label(format!("Игра завершилась (код {code})"));
                    ui.add_space(4.0);
                }
                LaunchState::Failed(err) => {
                    ui.colored_label(egui::Color32::RED, format!("Ошибка запуска: {err}"));
                    ui.add_space(4.0);
                }
                _ => {}
            }
            let button = egui::Button::new(egui::RichText::new("  ИГРАТЬ  ").size(24.0))
                .min_size(egui::vec2(220.0, 56.0));
            if ui.add_enabled(version.is_some(), button).clicked() {
                if let Some(v) = version {
                    launch.launch(ui.ctx().clone(), v, profile);
                }
            }
        }
    }
}

fn version_picker(
    ui: &mut egui::Ui,
    play: &mut PlayState,
    launch: &LaunchManager,
) -> Option<ManifestVersion> {
    match launch.versions() {
        None => {
            ui.spinner();
            ui.label("Загружаю список версий…");
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(300));
            None
        }
        Some(Err(err)) => {
            ui.colored_label(egui::Color32::RED, format!("Список версий: {err}"));
            None
        }
        Some(Ok(versions)) => {
            let releases: Vec<ManifestVersion> = versions
                .iter()
                .filter(|v| v.kind == "release")
                .cloned()
                .collect();
            if releases.is_empty() {
                ui.label("Нет доступных версий");
                return None;
            }
            let selected_id = play
                .selected_version
                .clone()
                .unwrap_or_else(|| releases[0].id.clone());
            egui::ComboBox::from_id_salt("mc_version")
                .selected_text(format!("Minecraft {selected_id}"))
                .show_ui(ui, |ui| {
                    for v in releases.iter().take(40) {
                        ui.selectable_value(&mut play.selected_version, Some(v.id.clone()), &v.id);
                    }
                });
            releases.iter().find(|v| v.id == selected_id).cloned()
        }
    }
}
