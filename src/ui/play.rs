use eframe::egui;

use crate::auth::{AuthManager, AuthState};

pub fn show(ui: &mut egui::Ui, auth: &AuthManager) {
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
                ui.add_space(24.0);
                let play_button =
                    egui::Button::new(egui::RichText::new("  ИГРАТЬ  ").size(24.0))
                        .min_size(egui::vec2(220.0, 56.0));
                ui.add_enabled(false, play_button)
                    .on_disabled_hover_text("Запуск Minecraft — следующий этап");
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
