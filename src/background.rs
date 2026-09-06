//! Фоновое изображение, «фальш-стекло» и виньетка.
//!
//! Из одной картинки готовятся две текстуры: обычная и заранее размытая.
//! Панели рисуются поверх среза размытой версии (с нужным скруглением) —
//! на статичном фоне это неотличимо от настоящего live-blur.
//! Виньетка по краям добавляет глубины и убирает «плоскость».
//! Настоящий blur-шейдер (wgpu) — апгрейд на этапе финальной полировки.

use std::path::PathBuf;

use eframe::egui;

use crate::theme::ThemePreset;

pub struct Background {
    normal: Option<egui::TextureHandle>,
    blurred: Option<egui::TextureHandle>,
}

impl Background {
    fn empty() -> Self {
        Self {
            normal: None,
            blurred: None,
        }
    }

    /// Ищет background.(png|jpg|jpeg) в папке данных лаунчера
    /// (APPDATA/.caligo) и готовит обычную + размытую текстуры.
    pub fn load(ctx: &egui::Context) -> Self {
        let Some(path) = find_background() else {
            return Self::empty();
        };
        let Ok(img) = image::open(&path) else {
            return Self::empty();
        };
        // Ограничиваем размер: меньше памяти, быстрее blur.
        let img = img.thumbnail(1920, 1920);
        let rgba = img.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        let normal = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
        let blurred_rgba = image::imageops::fast_blur(&rgba, 12.0);
        let blurred = egui::ColorImage::from_rgba_unmultiplied(size, blurred_rgba.as_raw());
        Self {
            normal: Some(ctx.load_texture("background", normal, egui::TextureOptions::LINEAR)),
            blurred: Some(ctx.load_texture(
                "background_blur",
                blurred,
                egui::TextureOptions::LINEAR,
            )),
        }
    }

    /// Рисует фон на весь экран, размытые срезы (с закруглениями)
    /// под «стеклянными» зонами и виньетку по краям.
    pub fn paint(
        &self,
        ctx: &egui::Context,
        theme: &ThemePreset,
        glass_rects: &[(egui::Rect, egui::Rounding)],
    ) {
        let painter = ctx.layer_painter(egui::LayerId::background());
        let screen = ctx.screen_rect();
        if let (Some(normal), Some(blurred)) = (&self.normal, &self.blurred) {
            let uv = cover_uv(normal.size_vec2(), screen);
            painter.image(normal.id(), screen, uv, egui::Color32::WHITE);
            for (rect, rounding) in glass_rects {
                let rect = rect.intersect(screen);
                if rect.is_positive() {
                    let mut shape = egui::epaint::RectShape::new(
                        rect,
                        *rounding,
                        egui::Color32::WHITE,
                        egui::Stroke::NONE,
                    );
                    shape.fill_texture_id = blurred.id();
                    shape.uv = sub_uv(uv, screen, rect);
                    painter.add(shape);
                }
            }
        } else {
            // Фолбэк без картинки: мягкий вертикальный градиент из цвета
            // темы с едва заметным акцентным подтоном внизу.
            let top = theme.background_color();
            let deep = darken(top, 0.45);
            let accent = theme.accent_color();
            let bottom = egui::Color32::from_rgb(
                deep.r().saturating_add(accent.r() / 18),
                deep.g().saturating_add(accent.g() / 18),
                deep.b().saturating_add(accent.b() / 18),
            );
            vgradient(&painter, screen, top, bottom);
        }
        vignette(&painter, screen);
    }
}

fn find_background() -> Option<PathBuf> {
    let dir = crate::launch::install::game_dir();
    for name in ["background.png", "background.jpg", "background.jpeg"] {
        let p = dir.join(name);
        if p.exists() {
            return Some(p);
        }
    }
    None
}

/// UV-прямоугольник для масштабирования картинки в режиме «cover»
/// (заполнить весь экран, обрезая лишнее, без искажения пропорций).
fn cover_uv(tex_size: egui::Vec2, screen: egui::Rect) -> egui::Rect {
    let screen_aspect = screen.width() / screen.height();
    let tex_aspect = tex_size.x / tex_size.y;
    if tex_aspect > screen_aspect {
        let w = screen_aspect / tex_aspect;
        let x0 = (1.0 - w) / 2.0;
        egui::Rect::from_min_max(egui::pos2(x0, 0.0), egui::pos2(x0 + w, 1.0))
    } else {
        let h = tex_aspect / screen_aspect;
        let y0 = (1.0 - h) / 2.0;
        egui::Rect::from_min_max(egui::pos2(0.0, y0), egui::pos2(1.0, y0 + h))
    }
}

/// UV-срез, соответствующий под-прямоугольнику экрана.
fn sub_uv(full_uv: egui::Rect, screen: egui::Rect, part: egui::Rect) -> egui::Rect {
    let fx = |x: f32| full_uv.min.x + (x - screen.min.x) / screen.width() * full_uv.width();
    let fy = |y: f32| full_uv.min.y + (y - screen.min.y) / screen.height() * full_uv.height();
    egui::Rect::from_min_max(
        egui::pos2(fx(part.min.x), fy(part.min.y)),
        egui::pos2(fx(part.max.x), fy(part.max.y)),
    )
}

/// Виньетка: мягкое затемнение краёв.低-контрастная, но убирает
/// ощущение «плоской системной» картинки и ведёт взгляд к центру.
fn vignette(painter: &egui::Painter, rect: egui::Rect) {
    let clear = egui::Color32::TRANSPARENT;
    let h = rect.height();
    let w = rect.width();
    // Низ — самый выраженный: добавляет визуального «веса».
    vgradient(
        painter,
        egui::Rect::from_min_max(egui::pos2(rect.min.x, rect.max.y - h * 0.38), rect.max),
        clear,
        egui::Color32::from_black_alpha(85),
    );
    // Верх — мягче.
    vgradient(
        painter,
        egui::Rect::from_min_max(rect.min, egui::pos2(rect.max.x, rect.min.y + h * 0.22)),
        egui::Color32::from_black_alpha(55),
        clear,
    );
    // Бока — едва заметные.
    hgradient(
        painter,
        egui::Rect::from_min_max(rect.min, egui::pos2(rect.min.x + w * 0.12, rect.max.y)),
        egui::Color32::from_black_alpha(38),
        clear,
    );
    hgradient(
        painter,
        egui::Rect::from_min_max(egui::pos2(rect.max.x - w * 0.12, rect.min.y), rect.max),
        clear,
        egui::Color32::from_black_alpha(38),
    );
}

fn vgradient(
    painter: &egui::Painter,
    rect: egui::Rect,
    top: egui::Color32,
    bottom: egui::Color32,
) {
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));
}

fn hgradient(
    painter: &egui::Painter,
    rect: egui::Rect,
    left: egui::Color32,
    right: egui::Color32,
) {
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(rect.left_top(), left);
    mesh.colored_vertex(rect.right_top(), right);
    mesh.colored_vertex(rect.right_bottom(), right);
    mesh.colored_vertex(rect.left_bottom(), left);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(egui::Shape::mesh(mesh));
}

fn darken(c: egui::Color32, f: f32) -> egui::Color32 {
    egui::Color32::from_rgb(
        (c.r() as f32 * f) as u8,
        (c.g() as f32 * f) as u8,
        (c.b() as f32 * f) as u8,
    )
}
