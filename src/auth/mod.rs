pub mod microsoft;

use std::sync::{Arc, Mutex};

/// A signed-in Minecraft account.
#[derive(Debug, Clone)]
pub struct MinecraftAccount {
    pub username: String,
    pub uuid: String,
    /// Minecraft services access token — used to launch the game.
    pub access_token: String,
}

#[derive(Debug, Clone, Default)]
pub enum AuthState {
    #[default]
    SignedOut,
    /// Waiting for the user to enter the device code in the browser.
    WaitingForUser {
        verification_uri: String,
        user_code: String,
    },
    /// The auth chain is running; the string names the current step.
    InProgress(String),
    SignedIn(MinecraftAccount),
    Failed(String),
}

/// Owns the auth state and runs the login flow on a background thread,
/// so the egui UI thread is never blocked.
#[derive(Default)]
pub struct AuthManager {
    state: Arc<Mutex<AuthState>>,
}

impl AuthManager {
    pub fn state(&self) -> AuthState {
        self.state.lock().unwrap().clone()
    }

    pub fn sign_out(&self) {
        *self.state.lock().unwrap() = AuthState::SignedOut;
    }

    /// Kick off the Microsoft device-code login flow in the background.
    pub fn start_login(&self, ctx: eframe::egui::Context) {
        let state = Arc::clone(&self.state);
        *state.lock().unwrap() = AuthState::InProgress("Подготовка…".to_string());
        std::thread::spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    *state.lock().unwrap() =
                        AuthState::Failed(format!("Не удалось запустить runtime: {e}"));
                    ctx.request_repaint();
                    return;
                }
            };
            let result = rt.block_on(microsoft::login(&state, &ctx));
            match result {
                Ok(account) => *state.lock().unwrap() = AuthState::SignedIn(account),
                Err(e) => *state.lock().unwrap() = AuthState::Failed(e),
            }
            ctx.request_repaint();
        });
    }
}
