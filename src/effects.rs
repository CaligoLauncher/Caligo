//! Атмосферные эффекты — «фирменная» часть Caligo.
//!
//! Caligo — это мгла. По экрану медленно плывут светящиеся частицы-«светлячки»
//! в акцентном цвете: лёгкое мерцание, покачивание, подъём вверх.
//! Это дешёвый по ресурсам эффект (30 кадров/с, ~30 кружков),
//! но он сразу делает лаунчер живым, а не «системным».

use std::time::Duration;

use eframe::egui;

pub struct Mist {
    particles: Vec<Particle>,
}

struct Particle {
    /// Стартовая позиция в долях экрана (0..1).
    x: f32,
    y: f32,
    /// Радиус ядра, px.
    radius: f32,
    /// Скорость подъёма, экранов в секунду.
    speed: f32,
    /// Амплитуда бокового покачивания.
    sway: f32,
    /// Фаза — чтобы частицы жили не в такт.
    phase: f32,
    /// Базовая яркость 0..1.
    bright: f32,
}

impl Mist {
    pub fn new() -> Self {
        // Детерминированный LCG — обходимся без зависимости на rand.
        let mut seed: u32 = 0x00C0FFEE;
        let mut rnd = move || {
            seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
            (seed >> 8) as f32 / 16_777_216.0
        };
        let particles = (0..30)
            .map(|_| Particle {
                x: rnd(),
                y: rnd(),
                radius: 1.4 + rnd() * 2.6,
                speed: 0.008 + rnd() * 0.020,
                sway: 0.004 + rnd() * 0.012,
                phase: rnd() * std::f32::consts::TAU,
                bright: 0.3 + rnd() * 0.7,
            })
            .collect();
        Self { particles }
    }

    /// Рисует частицы на фоновом слое (под панелями).
    pub fn paint(&self, ctx: &egui::Context, accent: egui::Color32) {
        let t = ctx.input(|i| i.time) as f32;
        let screen = ctx.screen_rect();
        let painter = ctx.layer_painter(egui::LayerId::background());
        for p in &self.particles {
            // Медленный подъём с заворотом за верхний край.
            let y = (p.y - t * p.speed).rem_euclid(1.08) - 0.04;
            let x = (p.x + (t * 0.3 + p.phase).sin() * p.sway).rem_euclid(1.0);
            let pos = egui::pos2(
                screen.min.x + x * screen.width(),
                screen.min.y + y * screen.height(),
            );
            // Мерцание: у каждой частицы своя фаза.
            let tw = ((t * 0.8 + p.phase * 3.0).sin() * 0.5 + 0.5) * 0.6 + 0.4;
            let a = p.bright * tw;
            // Три круга — мягкое свечение вместо резкой точки.
            painter.circle_filled(pos, p.radius * 3.2, accent.gamma_multiply(0.04 * a));
            painter.circle_filled(pos, p.radius * 1.7, accent.gamma_multiply(0.10 * a));
            painter.circle_filled(pos, p.radius, accent.gamma_multiply(0.35 * a));
        }
        // ~30 fps достаточно для фонового эффекта и почти ничего не стоит.
        ctx.request_repaint_after(Duration::from_millis(33));
    }
}
