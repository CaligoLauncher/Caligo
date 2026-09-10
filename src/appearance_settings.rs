//! Small versioned wallpaper presentation preferences, independent of image bytes.
use std::{
    fs::{self, File},
    io::{Read, Write},
    path::Path,
};

use serde::{Deserialize, Serialize};

const FILE_NAME: &str = "appearance.json";
const MAX_BYTES: u64 = 4096;

pub type Result<T> = std::result::Result<T, String>;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WallpaperMode {
    #[default]
    Cover,
    Contain,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppearanceSettings {
    version: u32,
    pub mode: WallpaperMode,
    pub dim_percent: u8,
}

impl Default for AppearanceSettings {
    fn default() -> Self {
        Self { version: 1, mode: WallpaperMode::Cover, dim_percent: 0 }
    }
}

impl AppearanceSettings {
    fn validate(&self) -> Result<()> {
        if self.version != 1 {
            return Err("Версия настроек оформления не поддерживается. Файл сохранён без изменений.".into());
        }
        if self.dim_percent > 60 {
            return Err("Затемнение должно быть от 0 до 60%. Файл сохранён без изменений.".into());
        }
        Ok(())
    }

    /// Black with the requested alpha; no new image buffer or decoder pass.
    pub fn overlay_rgba(self) -> u32 {
        (u32::from(self.dim_percent) * 255 + 50) / 100
    }
}

pub fn load(root: &Path) -> Result<AppearanceSettings> {
    let path = root.join(FILE_NAME);
    // A dangling symlink is an error, not an absent preference file.
    match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(AppearanceSettings::default()),
        Err(e) => return Err(format!("Не удалось проверить оформление: {e}")),
        Ok(meta) if !meta.file_type().is_file() => {
            return Err("Настройки оформления должны быть обычным файлом.".into());
        }
        Ok(_) => {}
    }
    let file = File::open(&path).map_err(|e| format!("Не удалось открыть оформление: {e}"))?;
    if file.metadata().map_err(|e| e.to_string())?.len() > MAX_BYTES {
        return Err("Файл оформления слишком большой. Он оставлен без изменений.".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_BYTES + 1).read_to_end(&mut bytes)
        .map_err(|e| format!("Не удалось прочитать оформление: {e}"))?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("Файл оформления слишком большой. Он оставлен без изменений.".into());
    }
    let value: AppearanceSettings = serde_json::from_slice(&bytes)
        .map_err(|_| "Не удалось прочитать формат оформления. Файл оставлен без изменений.".to_owned())?;
    value.validate()?;
    Ok(value)
}

fn write_atomic(root: &Path, settings: AppearanceSettings) -> Result<()> {
    settings.validate()?;
    let bytes = serde_json::to_vec_pretty(&settings).map_err(|e| e.to_string())?;
    fs::create_dir_all(root).map_err(|e| format!("Не удалось создать каталог оформления: {e}"))?;
    let mut temp = tempfile::NamedTempFile::new_in(root)
        .map_err(|e| format!("Не удалось подготовить сохранение оформления: {e}"))?;
    temp.write_all(&bytes).map_err(|e| format!("Не удалось записать оформление: {e}"))?;
    temp.as_file().sync_all().map_err(|e| format!("Не удалось завершить запись оформления: {e}"))?;
    temp.persist(root.join(FILE_NAME))
        .map_err(|e| format!("Не удалось сохранить оформление: {}", e.error))?;
    Ok(())
}

pub fn save(root: &Path, settings: AppearanceSettings) -> Result<()> {
    // Never silently overwrite an unrecognised/corrupt file on an ordinary edit.
    // Multiple running processes are last-writer-wins, not a locking protocol.
    load(root)?;
    write_atomic(root, settings)
}

/// Explicit user action: replace only presentation preferences, not the wallpaper.
pub fn reset(root: &Path) -> Result<()> {
    let path = root.join(FILE_NAME);
    match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(format!("Не удалось проверить оформление: {e}")),
        Ok(meta) if !meta.file_type().is_file() => {
            return Err("Нельзя сбросить оформление: вместо файла находится папка или ссылка.".into());
        }
        Ok(_) => {}
    }
    write_atomic(root, AppearanceSettings::default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_preferences_keep_previous_cover_and_create_nothing() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("missing");
        assert_eq!(load(&root).unwrap(), AppearanceSettings::default());
        reset(&root).unwrap();
        assert!(!root.exists());
    }

    #[test]
    fn fit_and_dim_survive_reload_and_replacement() {
        let temp = tempfile::tempdir().unwrap();
        let value = AppearanceSettings { mode: WallpaperMode::Contain, dim_percent: 40, ..Default::default() };
        save(temp.path(), value).unwrap();
        assert_eq!(load(temp.path()).unwrap(), value);
        save(temp.path(), AppearanceSettings::default()).unwrap();
        assert_eq!(load(temp.path()).unwrap(), AppearanceSettings::default());
    }

    #[test]
    fn reads_do_not_rewrite_preferences() {
        let temp = tempfile::tempdir().unwrap();
        let bytes = br#"{"version":1, "mode":"contain", "dim_percent":20}"#;
        fs::write(temp.path().join(FILE_NAME), bytes).unwrap();
        assert_eq!(load(temp.path()).unwrap().dim_percent, 20);
        assert_eq!(fs::read(temp.path().join(FILE_NAME)).unwrap(), bytes);
    }

    #[test]
    fn corrupt_future_and_unknown_data_are_not_silently_overwritten() {
        let temp = tempfile::tempdir().unwrap();
        for bytes in [
            "broken",
            r#"{"version":2,"mode":"cover","dim_percent":0}"#,
            r#"{"version":1,"mode":"tile","dim_percent":0}"#,
            r#"{"version":1,"mode":"cover","dim_percent":61}"#,
            r#"{"version":1,"mode":"cover","dim_percent":-1}"#,
            r#"{"version":1,"mode":"cover","dim_percent":0,"new_field":true}"#,
        ] {
            fs::write(temp.path().join(FILE_NAME), bytes).unwrap();
            assert!(load(temp.path()).is_err());
            assert!(save(temp.path(), AppearanceSettings::default()).is_err());
            assert_eq!(fs::read_to_string(temp.path().join(FILE_NAME)).unwrap(), bytes);
        }
    }

    #[test]
    fn explicit_reset_recovers_corrupt_preferences_without_touching_images_or_legacy_data() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        fs::write(root.join(FILE_NAME), b"broken").unwrap();
        fs::write(root.join("wallpaper.png"), b"image data").unwrap();
        fs::write(root.join("shell-a.json"), b"legacy").unwrap();
        reset(root).unwrap();
        assert_eq!(load(root).unwrap(), AppearanceSettings::default());
        assert_eq!(fs::read(root.join("wallpaper.png")).unwrap(), b"image data");
        assert_eq!(fs::read(root.join("shell-a.json")).unwrap(), b"legacy");
    }

    #[test]
    fn oversized_preferences_are_bounded_and_preserved() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join(FILE_NAME);
        File::create(&path).unwrap().set_len(MAX_BYTES + 1).unwrap();
        assert!(load(temp.path()).is_err());
        assert!(save(temp.path(), AppearanceSettings::default()).is_err());
        assert_eq!(fs::metadata(path).unwrap().len(), MAX_BYTES + 1);
    }

    #[test]
    fn invalid_candidate_keeps_last_saved_preferences() {
        let temp = tempfile::tempdir().unwrap();
        save(temp.path(), AppearanceSettings::default()).unwrap();
        let before = fs::read(temp.path().join(FILE_NAME)).unwrap();
        let bad = AppearanceSettings { dim_percent: 100, ..Default::default() };
        assert!(save(temp.path(), bad).is_err());
        assert_eq!(fs::read(temp.path().join(FILE_NAME)).unwrap(), before);
    }

    #[test]
    fn blocked_destination_is_not_removed() {
        let temp = tempfile::tempdir().unwrap();
        fs::create_dir(temp.path().join(FILE_NAME)).unwrap();
        assert!(load(temp.path()).is_err());
        assert!(save(temp.path(), AppearanceSettings::default()).is_err());
        assert!(reset(temp.path()).is_err());
        assert!(temp.path().join(FILE_NAME).is_dir());
    }

    #[test]
    fn image_and_preferences_fail_independently() {
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("wallpaper.png"), b"broken image").unwrap();
        save(temp.path(), AppearanceSettings::default()).unwrap();
        assert!(load(temp.path()).is_ok());
        assert_eq!(fs::read(temp.path().join("wallpaper.png")).unwrap(), b"broken image");
    }

    #[test]
    fn dimming_alpha_is_bounded_and_black() {
        for (percent, alpha) in [(0, 0), (20, 51), (40, 102), (60, 153)] {
            let settings = AppearanceSettings { dim_percent: percent, ..Default::default() };
            assert_eq!(settings.overlay_rgba(), alpha);
            assert_eq!(settings.overlay_rgba() >> 8, 0);
        }
    }
}