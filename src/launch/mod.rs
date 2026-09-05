//! Vanilla Minecraft launch: version manifest, downloads, JVM args, process.

pub mod install;
pub mod manifest;
pub mod run;

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use eframe::egui;

use manifest::{ManifestVersion, VersionManifest, VERSION_MANIFEST_URL};
use run::LaunchProfile;

#[derive(Debug, Clone, Default)]
pub enum LaunchState {
    #[default]
    Idle,
    /// Downloading / preparing; the string names the current step.
    Preparing(String),
    Running,
    Exited(i32),
    Failed(String),
}

pub(crate) fn set_state(state: &Arc<Mutex<LaunchState>>, ctx: &egui::Context, new: LaunchState) {
    *state.lock().unwrap() = new;
    ctx.request_repaint();
}

/// Owns the launch state and runs installation + the game process on a
/// background thread, so the egui UI thread is never blocked.
#[derive(Default)]
pub struct LaunchManager {
    state: Arc<Mutex<LaunchState>>,
    versions: Arc<Mutex<Option<Result<Vec<ManifestVersion>, String>>>>,
    versions_requested: AtomicBool,
}

impl LaunchManager {
    pub fn state(&self) -> LaunchState {
        self.state.lock().unwrap().clone()
    }

    pub fn versions(&self) -> Option<Result<Vec<ManifestVersion>, String>> {
        self.versions.lock().unwrap().clone()
    }

    /// Fetch the Mojang version manifest once, in the background.
    pub fn ensure_versions(&self, ctx: egui::Context) {
        if self.versions_requested.swap(true, Ordering::SeqCst) {
            return;
        }
        let versions = Arc::clone(&self.versions);
        std::thread::spawn(move || {
            let result = fetch_versions();
            *versions.lock().unwrap() = Some(result);
            ctx.request_repaint();
        });
    }

    /// Download everything needed for `version` and start the game.
    pub fn launch(&self, ctx: egui::Context, version: ManifestVersion, profile: LaunchProfile) {
        let state = Arc::clone(&self.state);
        std::thread::spawn(move || {
            set_state(&state, &ctx, LaunchState::Preparing("Подготовка…".into()));
            let rt = match tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            {
                Ok(rt) => rt,
                Err(e) => {
                    return set_state(&state, &ctx, LaunchState::Failed(format!("runtime: {e}")))
                }
            };
            let http = reqwest::Client::new();
            let prepared = match rt.block_on(install::prepare(&http, &version, &state, &ctx)) {
                Ok(p) => p,
                Err(e) => return set_state(&state, &ctx, LaunchState::Failed(e)),
            };
            let mut cmd = match run::build_command(&prepared, &profile) {
                Ok(c) => c,
                Err(e) => return set_state(&state, &ctx, LaunchState::Failed(e)),
            };
            set_state(&state, &ctx, LaunchState::Preparing("Запускаю Java…".into()));
            match cmd.spawn() {
                Ok(mut child) => {
                    set_state(&state, &ctx, LaunchState::Running);
                    match child.wait() {
                        Ok(status) => set_state(
                            &state,
                            &ctx,
                            LaunchState::Exited(status.code().unwrap_or(-1)),
                        ),
                        Err(e) => set_state(
                            &state,
                            &ctx,
                            LaunchState::Failed(format!("Ожидание процесса: {e}")),
                        ),
                    }
                }
                Err(e) => {
                    let major = prepared
                        .version_json
                        .java_version
                        .as_ref()
                        .map(|j| j.major_version)
                        .unwrap_or(21);
                    set_state(
                        &state,
                        &ctx,
                        LaunchState::Failed(format!(
                            "Не удалось запустить Java: {e}. Установи Java {major} (например, Temurin JDK) или задай JAVA_HOME"
                        )),
                    );
                }
            }
        });
    }
}

fn fetch_versions() -> Result<Vec<ManifestVersion>, String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    rt.block_on(async {
        let manifest: VersionManifest = reqwest::get(VERSION_MANIFEST_URL)
            .await
            .map_err(|e| format!("Сеть: {e}"))?
            .error_for_status()
            .map_err(|e| format!("Манифест версий: {e}"))?
            .json()
            .await
            .map_err(|e| format!("Манифест версий: {e}"))?;
        Ok(manifest.versions)
    })
}
