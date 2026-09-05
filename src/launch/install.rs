//! Downloading the version JSON, client jar, libraries, and assets
//! (with SHA1 verification, skipping files that already exist).

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use eframe::egui;
use futures::StreamExt;

use super::manifest::{
    rules_allow, Artifact, AssetIndexFile, ManifestVersion, VersionJson, RESOURCES_URL,
};
use super::{set_state, LaunchState};

pub struct Prepared {
    pub version_json: VersionJson,
    pub classpath: Vec<PathBuf>,
    pub game_dir: PathBuf,
}

/// Game data directory: %APPDATA%\.terra-launcher on Windows.
pub fn game_dir() -> PathBuf {
    if let Ok(appdata) = std::env::var("APPDATA") {
        PathBuf::from(appdata).join(".terra-launcher")
    } else if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".terra-launcher")
    } else {
        PathBuf::from(".terra-launcher")
    }
}

async fn download_file(
    http: &reqwest::Client,
    url: &str,
    dest: &Path,
    sha1: Option<&str>,
) -> Result<(), String> {
    if let Ok(meta) = tokio::fs::metadata(dest).await {
        if meta.len() > 0 {
            return Ok(());
        }
    }
    if let Some(parent) = dest.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .map_err(|e| format!("Создание папки {}: {e}", parent.display()))?;
    }
    let bytes = http
        .get(url)
        .send()
        .await
        .map_err(|e| format!("Сеть: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Загрузка {url}: {e}"))?
        .bytes()
        .await
        .map_err(|e| format!("Загрузка {url}: {e}"))?;
    if let Some(expected) = sha1 {
        let mut hasher = sha1_smol::Sha1::new();
        hasher.update(&bytes);
        if hasher.digest().to_string() != expected {
            return Err(format!("Контрольная сумма не совпала: {url}"));
        }
    }
    tokio::fs::write(dest, &bytes)
        .await
        .map_err(|e| format!("Запись {}: {e}", dest.display()))?;
    Ok(())
}

pub async fn prepare(
    http: &reqwest::Client,
    version: &ManifestVersion,
    state: &Arc<Mutex<LaunchState>>,
    ctx: &egui::Context,
) -> Result<Prepared, String> {
    let dir = game_dir();
    let vdir = dir.join("versions").join(&version.id);

    // 1. Version JSON.
    set_state(state, ctx, LaunchState::Preparing("Описание версии…".into()));
    let vjson_path = vdir.join(format!("{}.json", version.id));
    download_file(http, &version.url, &vjson_path, None).await?;
    let vjson: VersionJson = serde_json::from_str(
        &tokio::fs::read_to_string(&vjson_path)
            .await
            .map_err(|e| format!("Чтение {}: {e}", vjson_path.display()))?,
    )
    .map_err(|e| format!("Парсинг описания версии: {e}"))?;

    // 2. Client jar.
    set_state(state, ctx, LaunchState::Preparing("Клиент игры…".into()));
    let client_jar = vdir.join(format!("{}.jar", version.id));
    download_file(
        http,
        &vjson.downloads.client.url,
        &client_jar,
        Some(&vjson.downloads.client.sha1),
    )
    .await?;

    // 3. Libraries (filtered by OS rules), downloaded concurrently.
    let artifacts: Vec<Artifact> = vjson
        .libraries
        .iter()
        .filter(|l| l.rules.as_deref().map_or(true, rules_allow))
        .filter_map(|l| l.downloads.as_ref().and_then(|d| d.artifact.clone()))
        .collect();
    let total = artifacts.len();
    let mut classpath: Vec<PathBuf> = artifacts
        .iter()
        .map(|a| dir.join("libraries").join(&a.path))
        .collect();
    classpath.push(client_jar.clone());

    let counter = Arc::new(AtomicUsize::new(0));
    let results: Vec<Result<(), String>> = futures::stream::iter(artifacts.into_iter().map(|a| {
        let dest = dir.join("libraries").join(&a.path);
        let counter = Arc::clone(&counter);
        let state = Arc::clone(state);
        let ctx = ctx.clone();
        async move {
            download_file(http, &a.url, &dest, Some(&a.sha1)).await?;
            let n = counter.fetch_add(1, Ordering::Relaxed) + 1;
            set_state(
                &state,
                &ctx,
                LaunchState::Preparing(format!("Библиотеки: {n}/{total}")),
            );
            Ok::<(), String>(())
        }
    }))
    .buffer_unordered(8)
    .collect()
    .await;
    for r in results {
        r?;
    }

    // 4. Asset index + asset objects, downloaded concurrently.
    set_state(state, ctx, LaunchState::Preparing("Индекс ресурсов…".into()));
    let index_path = dir
        .join("assets")
        .join("indexes")
        .join(format!("{}.json", vjson.asset_index.id));
    download_file(
        http,
        &vjson.asset_index.url,
        &index_path,
        Some(&vjson.asset_index.sha1),
    )
    .await?;
    let index: AssetIndexFile = serde_json::from_str(
        &tokio::fs::read_to_string(&index_path)
            .await
            .map_err(|e| format!("Чтение {}: {e}", index_path.display()))?,
    )
    .map_err(|e| format!("Парсинг индекса ресурсов: {e}"))?;

    let objects: Vec<_> = index.objects.into_values().collect();
    let total = objects.len();
    let counter = Arc::new(AtomicUsize::new(0));
    let results: Vec<Result<(), String>> = futures::stream::iter(objects.into_iter().map(|o| {
        let prefix = o.hash[..2].to_string();
        let dest = dir
            .join("assets")
            .join("objects")
            .join(&prefix)
            .join(&o.hash);
        let url = format!("{RESOURCES_URL}/{prefix}/{}", o.hash);
        let counter = Arc::clone(&counter);
        let state = Arc::clone(state);
        let ctx = ctx.clone();
        async move {
            download_file(http, &url, &dest, Some(&o.hash)).await?;
            let n = counter.fetch_add(1, Ordering::Relaxed) + 1;
            if n % 50 == 0 || n == total {
                set_state(
                    &state,
                    &ctx,
                    LaunchState::Preparing(format!("Ресурсы: {n}/{total}")),
                );
            }
            Ok::<(), String>(())
        }
    }))
    .buffer_unordered(16)
    .collect()
    .await;
    for r in results {
        r?;
    }

    Ok(Prepared {
        version_json: vjson,
        classpath,
        game_dir: dir,
    })
}
