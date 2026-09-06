//! 3D-рендер скина игрока — «бумажная кукла» в чистом egui.
//!
//! Никакого wgpu: параллелепипеды модели игрока проецируются в
//! текстурированные треугольники `egui::Mesh` с UV-развёрткой классического
//! скина 64×64. Скин тянется с серверов Mojang (по UUID или нику);
//! без скина рисуется силуэт в цветах темы.

use std::sync::{Arc, Mutex};

use base64::Engine;
use eframe::egui;

#[derive(Default)]
pub enum SkinState {
    #[default]
    NotRequested,
    Loading,
    Ready(egui::TextureHandle),
    Failed(String),
}

/// Владеет состоянием скина и качает его в фоне, не блокируя UI-поток.
#[derive(Default)]
pub struct SkinManager {
    state: Arc<Mutex<SkinState>>,
    key: Mutex<Option<String>>,
}

impl SkinManager {
    pub fn texture(&self) -> Option<egui::TextureHandle> {
        match &*self.state.lock().unwrap() {
            SkinState::Ready(t) => Some(t.clone()),
            _ => None,
        }
    }

    pub fn loading(&self) -> bool {
        matches!(&*self.state.lock().unwrap(), SkinState::Loading)
    }

    pub fn error(&self) -> Option<String> {
        match &*self.state.lock().unwrap() {
            SkinState::Failed(e) => Some(e.clone()),
            _ => None,
        }
    }

    /// `key` — UUID (с дефисами или без) либо ник. Перекачивает скин только
    /// при смене ключа, так что можно звать каждый кадр.
    pub fn ensure(&self, ctx: &egui::Context, key: Option<String>) {
        let mut cur = self.key.lock().unwrap();
        if *cur == key {
            return;
        }
        *cur = key.clone();
        drop(cur);
        let Some(key) = key else {
            *self.state.lock().unwrap() = SkinState::NotRequested;
            return;
        };
        *self.state.lock().unwrap() = SkinState::Loading;
        let state = Arc::clone(&self.state);
        let ctx = ctx.clone();
        std::thread::spawn(move || {
            let result = fetch_blocking(&key);
            let mut lock = state.lock().unwrap();
            match result {
                Ok(img) => {
                    let size = [img.width() as usize, img.height() as usize];
                    let color = egui::ColorImage::from_rgba_unmultiplied(size, img.as_raw());
                    *lock = SkinState::Ready(ctx.load_texture(
                        "player_skin",
                        color,
                        egui::TextureOptions::NEAREST,
                    ));
                }
                Err(e) => *lock = SkinState::Failed(e),
            }
            drop(lock);
            ctx.request_repaint();
        });
    }
}

fn fetch_blocking(key: &str) -> Result<image::RgbaImage, String> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    rt.block_on(async {
        let http = reqwest::Client::new();
        let compact = key.replace('-', "");
        let uuid = if compact.len() == 32 {
            compact
        } else {
            let resp = http
                .get(format!("https://api.mojang.com/users/profiles/minecraft/{key}"))
                .send()
                .await
                .map_err(|e| format!("Сеть: {e}"))?;
            if !resp.status().is_success() {
                return Err("такого ника нет в Mojang".to_string());
            }
            let v: serde_json::Value = resp.json().await.map_err(|e| format!("Сеть: {e}"))?;
            v["id"]
                .as_str()
                .ok_or_else(|| "такого ника нет в Mojang".to_string())?
                .to_string()
        };
        let v: serde_json::Value = http
            .get(format!(
                "https://sessionserver.mojang.com/session/minecraft/profile/{uuid}"
            ))
            .send()
            .await
            .map_err(|e| format!("Сеть: {e}"))?
            .json()
            .await
            .map_err(|e| format!("профиль не найден ({e})"))?;
        let b64 = v["properties"]
            .as_array()
            .and_then(|p| p.iter().find(|e| e["name"] == "textures"))
            .and_then(|e| e["value"].as_str())
            .ok_or_else(|| "профиль без текстур".to_string())?;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(b64)
            .map_err(|e| e.to_string())?;
        let t: serde_json::Value =
            serde_json::from_slice(&decoded).map_err(|e| e.to_string())?;
        let url = t["textures"]["SKIN"]["url"]
            .as_str()
            .ok_or_else(|| "скин не задан".to_string())?;
        let png = http
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Сеть: {e}"))?
            .bytes()
            .await
            .map_err(|e| format!("Сеть: {e}"))?;
        let img = image::load_from_memory(&png)
            .map_err(|e| e.to_string())?
            .to_rgba8();
        Ok(normalize_skin(img))
    })
}

/// Старые скины 64×32 приводим к современной развёртке 64×64:
/// правые конечности копируются на места левых.
fn normalize_skin(img: image::RgbaImage) -> image::RgbaImage {
    if img.height() >= 64 {
        return img;
    }
    let mut canvas = image::RgbaImage::new(64, 64);
    image::imageops::replace(&mut canvas, &img, 0, 0);
    let arm = image::imageops::crop_imm(&canvas, 40, 16, 16, 16).to_image();
    image::imageops::replace(&mut canvas, &arm, 32, 48);
    let leg = image::imageops::crop_imm(&canvas, 0, 16, 16, 16).to_image();
    image::imageops::replace(&mut canvas, &leg, 16, 48);
    canvas
}

// ---------- геометрия модели ----------

struct Box3 {
    center: [f32; 3],
    /// Ширина (x), высота (y), глубина (z) в пикселях скина.
    size: [f32; 3],
    /// Левый верхний угол развёртки этой части в текстуре 64×64.
    uv: [f32; 2],
    /// Раздутие бокса (для слоя «шапки» поверх головы).
    grow: f32,
}

const BOXES: [Box3; 7] = [
    // тело
    Box3 { center: [0.0, 18.0, 0.0], size: [8.0, 12.0, 4.0], uv: [16.0, 16.0], grow: 0.0 },
    // правая рука (на экране слева)
    Box3 { center: [-6.0, 18.0, 0.0], size: [4.0, 12.0, 4.0], uv: [40.0, 16.0], grow: 0.0 },
    // левая рука
    Box3 { center: [6.0, 18.0, 0.0], size: [4.0, 12.0, 4.0], uv: [32.0, 48.0], grow: 0.0 },
    // правая нога
    Box3 { center: [-2.0, 6.0, 0.0], size: [4.0, 12.0, 4.0], uv: [0.0, 16.0], grow: 0.0 },
    // левая нога
    Box3 { center: [2.0, 6.0, 0.0], size: [4.0, 12.0, 4.0], uv: [16.0, 48.0], grow: 0.0 },
    // голова
    Box3 { center: [0.0, 28.0, 0.0], size: [8.0, 8.0, 8.0], uv: [0.0, 0.0], grow: 0.0 },
    // шапка (второй слой головы, чуть больше)
    Box3 { center: [0.0, 28.0, 0.0], size: [8.0, 8.0, 8.0], uv: [32.0, 0.0], grow: 0.55 },
];

struct Face {
    normal: [f32; 3],
    corners: [[f32; 3]; 4],
    /// u0, v0, u1, v1 в пикселях текстуры.
    uv: [f32; 4],
}

fn box_faces(b: &Box3) -> [Face; 6] {
    let hw = b.size[0] / 2.0 + b.grow;
    let hh = b.size[1] / 2.0 + b.grow;
    let hd = b.size[2] / 2.0 + b.grow;
    let (u, v) = (b.uv[0], b.uv[1]);
    let (w, h, d) = (b.size[0], b.size[1], b.size[2]);
    let p = |x: f32, y: f32, z: f32| [b.center[0] + x, b.center[1] + y, b.center[2] + z];
    [
        // перед (+z)
        Face {
            normal: [0.0, 0.0, 1.0],
            corners: [p(-hw, hh, hd), p(hw, hh, hd), p(hw, -hh, hd), p(-hw, -hh, hd)],
            uv: [u + d, v + d, u + d + w, v + d + h],
        },
        // зад (-z)
        Face {
            normal: [0.0, 0.0, -1.0],
            corners: [p(hw, hh, -hd), p(-hw, hh, -hd), p(-hw, -hh, -hd), p(hw, -hh, -hd)],
            uv: [u + 2.0 * d + w, v + d, u + 2.0 * d + 2.0 * w, v + d + h],
        },
        // правый бок (-x)
        Face {
            normal: [-1.0, 0.0, 0.0],
            corners: [p(-hw, hh, -hd), p(-hw, hh, hd), p(-hw, -hh, hd), p(-hw, -hh, -hd)],
            uv: [u, v + d, u + d, v + d + h],
        },
        // левый бок (+x)
        Face {
            normal: [1.0, 0.0, 0.0],
            corners: [p(hw, hh, hd), p(hw, hh, -hd), p(hw, -hh, -hd), p(hw, -hh, hd)],
            uv: [u + d + w, v + d, u + 2.0 * d + w, v + d + h],
        },
        // верх (+y)
        Face {
            normal: [0.0, 1.0, 0.0],
            corners: [p(-hw, hh, -hd), p(hw, hh, -hd), p(hw, hh, hd), p(-hw, hh, hd)],
            uv: [u + d, v, u + d + w, v + d],
        },
        // низ (-y)
        Face {
            normal: [0.0, -1.0, 0.0],
            corners: [p(-hw, -hh, hd), p(hw, -hh, hd), p(hw, -hh, -hd), p(-hw, -hh, -hd)],
            uv: [u + d + w, v, u + d + 2.0 * w, v + d],
        },
    ]
}

struct DrawFace {
    depth: f32,
    pts: [egui::Pos2; 4],
    uv: [egui::Pos2; 4],
    color: egui::Color32,
}

/// Рисует «бумажную куклу»: мягкое покачивание, плоское затенение,
/// свечение и тень под ногами, ник над головой.
pub fn paint_paperdoll(
    painter: &egui::Painter,
    rect: egui::Rect,
    tex: Option<&egui::TextureHandle>,
    name: Option<&str>,
    accent: egui::Color32,
    t: f32,
) {
    let yaw: f32 = -0.45 + (t * 0.35).sin() * 0.35;
    let pitch: f32 = 0.10;
    let (sy, cy) = yaw.sin_cos();
    let (sp, cp) = pitch.sin_cos();
    let scale = (rect.height() * 0.78 / 38.0)
        .min(rect.width() * 0.35 / 20.0)
        .max(2.0);
    let cx = rect.center().x;
    let bob = (t * 0.9).sin() * 0.5;
    let feet_y = rect.center().y + scale * 18.0;

    // Свечение и тень под ногами.
    painter.add(egui::epaint::EllipseShape {
        center: egui::pos2(cx, feet_y + scale * 0.9),
        radius: egui::vec2(scale * 11.0, scale * 2.8),
        fill: accent.gamma_multiply(0.10),
        stroke: egui::Stroke::NONE,
    });
    painter.add(egui::epaint::EllipseShape {
        center: egui::pos2(cx, feet_y + scale * 0.9),
        radius: egui::vec2(scale * 7.0, scale * 1.7),
        fill: egui::Color32::from_black_alpha(90),
        stroke: egui::Stroke::NONE,
    });

    let project = |pt: [f32; 3]| -> (egui::Pos2, f32) {
        let x1 = pt[0] * cy + pt[2] * sy;
        let z1 = -pt[0] * sy + pt[2] * cy;
        let y = pt[1] + bob;
        let y1 = y * cp - z1 * sp;
        let z2 = y * sp + z1 * cp;
        (egui::pos2(cx + x1 * scale, feet_y - y1 * scale), z2)
    };
    let rot_normal = |n: [f32; 3]| -> [f32; 3] {
        let x1 = n[0] * cy + n[2] * sy;
        let z1 = -n[0] * sy + n[2] * cy;
        let y1 = n[1] * cp - z1 * sp;
        let z2 = n[1] * sp + z1 * cp;
        [x1, y1, z2]
    };
    // Направление света: сверху-спереди-слева.
    let light = [0.30_f32, 0.55, 0.78];
    let silhouette = egui::Color32::from_rgb(82, 96, 122);

    let mut faces: Vec<DrawFace> = Vec::with_capacity(BOXES.len() * 3);
    for b in &BOXES {
        if tex.is_none() && b.grow > 0.0 {
            continue; // силуэту шапка не нужна
        }
        for f in box_faces(b) {
            let n = rot_normal(f.normal);
            if n[2] <= 0.02 {
                continue; // грань смотрит от нас
            }
            let mut pts = [egui::Pos2::ZERO; 4];
            let mut depth = 0.0;
            for (i, c) in f.corners.iter().enumerate() {
                let (pos, z) = project(*c);
                pts[i] = pos;
                depth += z;
            }
            depth /= 4.0;
            let dot = (n[0] * light[0] + n[1] * light[1] + n[2] * light[2]).max(0.0);
            let shade = 0.55 + 0.45 * dot;
            let (uv, color) = if tex.is_some() {
                let q = |x: f32, y: f32| egui::pos2(x / 64.0, y / 64.0);
                (
                    [
                        q(f.uv[0], f.uv[1]),
                        q(f.uv[2], f.uv[1]),
                        q(f.uv[2], f.uv[3]),
                        q(f.uv[0], f.uv[3]),
                    ],
                    egui::Color32::from_gray((255.0 * shade) as u8),
                )
            } else {
                ([egui::epaint::WHITE_UV; 4], silhouette.gamma_multiply(shade))
            };
            faces.push(DrawFace { depth, pts, uv, color });
        }
    }
    // Художник рисует с дальних граней к ближним.
    faces.sort_by(|a, b| a.depth.total_cmp(&b.depth));

    let mut mesh = egui::Mesh::default();
    if let Some(tex) = tex {
        mesh.texture_id = tex.id();
    }
    for f in &faces {
        let i = mesh.vertices.len() as u32;
        for k in 0..4 {
            mesh.vertices.push(egui::epaint::Vertex {
                pos: f.pts[k],
                uv: f.uv[k],
                color: f.color,
            });
        }
        mesh.add_triangle(i, i + 1, i + 2);
        mesh.add_triangle(i, i + 2, i + 3);
    }
    painter.add(egui::Shape::mesh(mesh));

    if let Some(name) = name {
        let head_top = feet_y - scale * 33.5;
        painter.text(
            egui::pos2(cx, head_top - 10.0),
            egui::Align2::CENTER_BOTTOM,
            name,
            egui::FontId::proportional(18.0),
            egui::Color32::from_rgb(235, 240, 248),
        );
    }
}
