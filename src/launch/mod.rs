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

            // Проверяем Java ДО запуска: иначе JVM молча падает с
            // «Could not create the Java Virtual Machine».
            let required = prepared
                .version_json
                .java_version
                .as_ref()
                .map(|j| j.major_version)
                .unwrap_or(21);
            set_state(&state, &ctx, LaunchState::Preparing("Проверяю Java…".into()));
            let java = match run::check_java() {
                Ok(j) => j,
                Err(e) => return set_state(&state, &ctx, LaunchState::Failed(e)),
            };
            if java.major < required {
                return set_state(
                    &state,
                    &ctx,
                    LaunchState::Failed(format!(
                        "Установлена Java {} ({}), а для Minecraft {} нужна Java {}.\nУстанови её: winget install EclipseAdoptium.Temurin.{}.JDK — и перезапусти лаунчер",
                        java.major, java.version_line, version.id, required, required
                    )),
                );
            }
            if !java.is_64bit {
                return set_state(
                    &state,
                    &ctx,
                    LaunchState::Failed(
                        "Установлена 32-битная Java — ей не хватит памяти для игры.\nПоставь 64-битную Java 21 (Temurin JDK)".into(),
                    ),
                );
            }

            let mut cmd = match run::build_command(&prepared, &profile) {
                Ok(c) => c,
                Err(e) => return set_state(&state, &ctx, LaunchState::Failed(e)),
            };
            set_state(&state, &ctx, LaunchState::Preparing("Запускаю Java…".into()));
            match cmd.spawn() {
                Ok(mut child) => {
                    set_state(&state, &ctx, LaunchState::Running);
                    let out_tail = run::spawn_tail_reader(child.stdout.take(), 40);
                    let err_tail = run::spawn_tail_reader(child.stderr.take(), 40);
                    match child.wait() {
                        Ok(status) => {
                            let code = status.code().unwrap_or(-1);
                            if code == 0 {
                                set_state(&state, &ctx, LaunchState::Exited(code));
                            } else {
                                let err_lines = err_tail.join().unwrap_or_default();
                                let out_lines = out_tail.join().unwrap_or_default();
                                let tail: Vec<String> = if !err_lines.is_empty() {
                                    err_lines
                                } else {
                                    out_lines
                                };
                                let last: Vec<&str> = tail
                                    .iter()
                                    .rev()
                                    .take(8)
                                    .map(|s| s.as_str())
                                    .collect::<Vec<_>>()
                                    .into_iter()
                                    .rev()
                                    .collect();
                                set_state(
                                    &state,
                                    &ctx,
                                    LaunchState::Failed(format!(
                                        "Игра завершилась с ошибкой (код {code}).\n{}",
                                        last.join("\n")
                                    )),
                                );
                            }
                        }
                        Err(e) => set_state(
                            &state,
                            &ctx,
                            LaunchState::Failed(format!("Ожидание процесса: {e}")),
                        ),
                    }
                }
                Err(e) => {
                    set_state(
                        &state,
                        &ctx,
                        LaunchState::Failed(format!(
                            "Не удалось запустить Java: {e}. Установи Java {required} (например, Temurin JDK) или задай JAVA_HOME"
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
