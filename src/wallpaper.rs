//! The GPUI shell owns only `gpui/wallpaper.png`; legacy game/editor data is untouched.
use std::{
    env,
    fs::{self, File},
    io::{Cursor, Read},
    path::{Path, PathBuf},
};

use image::{ImageFormat, ImageReader, RgbaImage};

const MAX_FILE_BYTES: u64 = 32 * 1024 * 1024;
const MAX_PIXELS: u64 = 32_000_000;
const MAX_SIDE: u32 = 8192;
const CACHE_WIDTH: u32 = 2560;
const CACHE_HEIGHT: u32 = 1440;

pub type Result<T> = std::result::Result<T, String>;

pub fn directory() -> Result<PathBuf> {
    #[cfg(target_os = "windows")]
    let base = env::var_os("APPDATA")
        .filter(|value| !value.is_empty())
        .map(|base| PathBuf::from(base).join(".caligo"));

    #[cfg(not(target_os = "windows"))]
    let base = env::var_os("XDG_CONFIG_HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            env::var_os("HOME")
                .filter(|value| !value.is_empty())
                .map(|home| PathBuf::from(home).join(".config"))
        })
        .map(|base| base.join("caligo"));

    base.map(|base| base.join("gpui"))
        .ok_or_else(|| "Не найден пользовательский каталог настроек.".into())
}

fn read_bounded(path: &Path) -> Result<Vec<u8>> {
    let file = File::open(path).map_err(|e| format!("Не удалось открыть изображение: {e}"))?;
    let metadata = file.metadata().map_err(|e| format!("Не удалось прочитать файл: {e}"))?;
    if !metadata.is_file() {
        return Err("Выберите файл PNG или JPEG, не папку.".into());
    }
    if metadata.len() > MAX_FILE_BYTES {
        return Err("Изображение слишком большое: максимум 32 МБ.".into());
    }
    let mut bytes = Vec::new();
    file.take(MAX_FILE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Не удалось прочитать изображение: {e}"))?;
    if bytes.len() as u64 > MAX_FILE_BYTES {
        return Err("Изображение слишком большое: максимум 32 МБ.".into());
    }
    Ok(bytes)
}

fn check_dimensions(width: u32, height: u32) -> Result<()> {
    if width == 0 || height == 0
        || width > MAX_SIDE || height > MAX_SIDE
        || u64::from(width) * u64::from(height) > MAX_PIXELS
    {
        return Err("Изображение слишком большое: максимум 8192 px по стороне и 32 мегапикселя.".into());
    }
    Ok(())
}

fn decode(bytes: &[u8]) -> Result<RgbaImage> {
    let format = image::guess_format(bytes)
        .map_err(|_| "Не удалось распознать изображение. Выберите PNG или JPEG.".to_string())?;
    if !matches!(format, ImageFormat::Png | ImageFormat::Jpeg) {
        return Err("Сейчас поддерживаются только PNG и JPEG.".into());
    }
    let (width, height) = ImageReader::with_format(Cursor::new(bytes), format)
        .into_dimensions()
        .map_err(|e| format!("Повреждённое изображение: {e}"))?;
    check_dimensions(width, height)?;
    let mut reader = ImageReader::with_format(Cursor::new(bytes), format);
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(MAX_SIDE);
    limits.max_image_height = Some(MAX_SIDE);
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let decoded = reader.decode().map_err(|e| format!("Не удалось загрузить изображение: {e}"))?;
    // Static first image only; never schedule animated frames.
    let decoded = if width > CACHE_WIDTH || height > CACHE_HEIGHT {
        decoded.resize(CACHE_WIDTH, CACHE_HEIGHT, image::imageops::FilterType::Triangle)
    } else {
        decoded
    };
    Ok(decoded.into_rgba8())
}

pub fn restore(root: &Path) -> Result<Option<RgbaImage>> {
    let path = root.join("wallpaper.png");
    match fs::symlink_metadata(&path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("Не удалось прочитать сохранённые обои: {e}")),
        Ok(_) => decode(&read_bounded(&path)?).map(Some),
    }
}

pub fn choose(root: &Path, source: &Path) -> Result<RgbaImage> {
    // Validate before touching the last saved wallpaper.
    let image = decode(&read_bounded(source)?)?;
    fs::create_dir_all(root).map_err(|e| format!("Не удалось создать каталог настроек: {e}"))?;
    let mut temporary = tempfile::NamedTempFile::new_in(root)
        .map_err(|e| format!("Не удалось подготовить сохранение: {e}"))?;
    image.write_to(&mut temporary, ImageFormat::Png)
        .map_err(|e| format!("Не удалось сохранить изображение: {e}"))?;
    temporary.as_file().sync_all()
        .map_err(|e| format!("Не удалось завершить запись: {e}"))?;
    // Same-directory atomic replacement, including Windows. No delete-then-rename gap.
    temporary.persist(root.join("wallpaper.png"))
        .map_err(|e| format!("Не удалось заменить сохранённые обои: {}", e.error))?;
    Ok(image)
}

pub fn reset(root: &Path) -> Result<()> {
    match fs::remove_file(root.join("wallpaper.png")) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("Не удалось сбросить обои: {e}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(path: &Path, color: [u8; 4]) {
        RgbaImage::from_pixel(4, 3, image::Rgba(color)).save(path).unwrap();
    }

    #[test]
    fn missing_settings_are_defaults_without_creating_a_directory() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("missing");
        assert!(restore(&root).unwrap().is_none());
        assert!(!root.exists());
        reset(&root).unwrap();
        assert!(!root.exists());
    }

    #[test]
    fn selection_survives_source_deletion_and_can_be_reset() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("фон.png");
        let root = temp.path().join("gpui");
        fixture(&source, [231, 42, 81, 255]);
        let selected = choose(&root, &source).unwrap();
        fs::remove_file(source).unwrap();
        assert_eq!(restore(&root).unwrap().unwrap(), selected);
        reset(&root).unwrap();
        assert!(restore(&root).unwrap().is_none());
    }

    #[test]
    fn invalid_selection_preserves_previous_wallpaper() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("ok.png");
        let invalid = temp.path().join("bad.png");
        let root = temp.path().join("gpui");
        fixture(&source, [1, 2, 3, 255]);
        choose(&root, &source).unwrap();
        let before = fs::read(root.join("wallpaper.png")).unwrap();
        fs::write(&invalid, b"not an image").unwrap();
        assert!(choose(&root, &invalid).is_err());
        assert_eq!(fs::read(root.join("wallpaper.png")).unwrap(), before);
    }

    #[test]
    fn replacing_an_existing_wallpaper_works() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("image.png");
        let root = temp.path().join("gpui");
        fixture(&source, [255, 0, 0, 255]);
        choose(&root, &source).unwrap();
        fixture(&source, [0, 0, 255, 255]);
        choose(&root, &source).unwrap();
        assert_eq!(restore(&root).unwrap().unwrap().get_pixel(0, 0).0, [0, 0, 255, 255]);
    }

    #[test]
    fn corrupt_cache_is_reported_not_deleted() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("wallpaper.png");
        fs::write(&path, b"damaged").unwrap();
        assert!(restore(temp.path()).is_err());
        assert_eq!(fs::read(path).unwrap(), b"damaged");
    }

    #[test]
    fn reset_never_removes_originals_or_legacy_data() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("original.png");
        let legacy = temp.path().join("shell-a.json");
        let root = temp.path().join("gpui");
        fixture(&source, [1, 2, 3, 255]);
        fs::write(&legacy, b"legacy").unwrap();
        choose(&root, &source).unwrap();
        reset(&root).unwrap();
        assert!(source.exists());
        assert_eq!(fs::read(legacy).unwrap(), b"legacy");
    }

    #[test]
    fn huge_dimensions_and_other_formats_are_rejected() {
        assert!(check_dimensions(8193, 1).is_err());
        assert!(check_dimensions(8000, 8000).is_err());
        assert!(check_dimensions(0, 1).is_err());
        assert!(check_dimensions(3840, 2160).is_ok());
        assert!(decode(b"GIF89a").is_err());
    }

    #[test]
    fn large_images_are_downscaled_without_stretching() {
        let image = RgbaImage::from_pixel(3000, 1500, image::Rgba([2, 4, 8, 255]));
        let mut bytes = Cursor::new(Vec::new());
        image.write_to(&mut bytes, ImageFormat::Png).unwrap();
        let cached = decode(bytes.get_ref()).unwrap();
        assert_eq!(cached.dimensions(), (2560, 1280));
        assert_eq!(cached.get_pixel(0, 0).0, [2, 4, 8, 255]);
    }

    #[test]
    fn oversized_file_is_rejected_before_decode() {
        let temp = tempfile::tempdir().unwrap();
        let path = temp.path().join("big.png");
        File::create(&path).unwrap().set_len(MAX_FILE_BYTES + 1).unwrap();
        assert!(read_bounded(&path).unwrap_err().contains("32 МБ"));
    }

    #[test]
    fn failed_persist_keeps_existing_destination() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("image.png");
        let root = temp.path().join("gpui");
        fixture(&source, [1, 2, 3, 255]);
        fs::create_dir_all(root.join("wallpaper.png")).unwrap();
        assert!(choose(&root, &source).is_err());
        assert!(root.join("wallpaper.png").is_dir());
        assert!(reset(&root).is_err());
    }
}