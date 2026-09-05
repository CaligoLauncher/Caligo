use std::time::Instant;

use eframe::egui;

use crate::auth::AuthManager;
use crate::background::Background;
use crate::launch::LaunchManager;
use crate::theme::ThemePreset;
use crate::ui;

const TITLEBAR_H: f32 = 36.0;
const SIDEBAR_W: f32 = 64.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Play,
    Instances,
    Settings,
}

pub struct TerraLauncherApp {
    pub tab: Tab,
    pub theme: ThemePreset,
    pub settings: ui::settings::SettingsState,
    pub auth: AuthManager,
    pub launch: LaunchManager,
    pub play: ui::play::PlayState,
    background: Background,
    started_at: Instant,
    tab_switched_at: Instant,
}

impl TerraLauncherApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let theme = ThemePreset::default();
        theme.apply(&cc.egui_ctx);
        let background = Background::load(&cc.egui_ctx);
        Self {
            tab: Tab::Play,
            theme,
            settings: Default::default(),
            auth: Default::default(),
            launch: Default::default(),
            play: Default::default(),
            background,
            started_at: Instant::now(),
            tab_switched_at: Instant::now(),
        }
    }

    fn switch_tab(&mut self, tab: Tab) {
        if self.tab != tab {
            self.tab = tab;
            self.tab_switched_at = Instant::now();
        }
    }

    /// Кастомный титлбар: своё окно без системной рамки, перетаскивание,
    /// кнопки свернуть / развернуть / закрыть.
    fn show_titlebar(&mut self, ctx: &egui::Context) {
        let accent = self.theme.accent_color();
        egui::TopBottomPanel::top("titlebar")
            .exact_height(TITLEBAR_H)
            .frame(egui::Frame::none().fill(self.theme.glass_fill()))
            .show(ctx, |ui| {
                let bar_rect = ui.max_rect();
                // Сначала зона перетаскивания, потом кнопки — кнопки выше по
                // z-порядку и получают клики первыми.
                let drag =
                    ui.interact(bar_rect, ui.id().with("drag"), egui::Sense::click_and_drag());
                if drag.drag_started() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                if drag.double_clicked() {
                    let maximized = ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                    ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                }
                ui.horizontal_centered(|ui| {
                    ui.add_space(12.0);
                    ui.label(egui::RichText::new("Terra Launcher").strong().color(accent));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(6.0);
                        if ui
                            .add(egui::Button::new("🗙").frame(false))
                            .on_hover_text("Закрыть")
                            .clicked()
                        {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if ui
                            .add(egui::Button::new("🗖").frame(false))
                            .on_hover_text("Развернуть")
                            .clicked()
                        {
                            let maximized =
                                ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                        }
                        if ui
                            .add(egui::Button::new("🗕").frame(false))
                            .on_hover_text("Свернуть")
                            .clicked()
                        {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                    });
                });
            });
    }

    /// Узкий сайдбар с иконками вместо текстового меню.
    fn show_sidebar(&mut self, ctx: &egui::Context) {
        let accent = self.theme.accent_color();
        let mut clicked: Option<Tab> = None;
        egui::SidePanel::left("nav")
            .resizable(false)
            .exact_width(SIDEBAR_W)
            .frame(egui::Frame::none().fill(self.theme.glass_fill()))
            .show(ctx, |ui| {
                ui.add_space(14.0);
                ui.vertical_centered(|ui| {
                    for (tab, icon, label) in [
                        (Tab::Play, "▶", "Играть"),
                        (Tab::Instances, "📦", "Сборки"),
                        (Tab::Settings, "⚙", "Настройки"),
                    ] {
                        if nav_button(ui, self.tab == tab, icon, label, accent).clicked() {
                            clicked = Some(tab);
                        }
                        ui.add_space(6.0);
                    }
                });
            });
        if let Some(tab) = clicked {
            self.switch_tab(tab);
        }
    }
}

impl eframe::App for TerraLauncherApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let screen = ctx.screen_rect();
        // Зоны «стекла»: под титлбаром и сайдбаром рисуется размытый срез фона.
        let titlebar =
            egui::Rect::from_min_size(screen.min, egui::vec2(screen.width(), TITLEBAR_H));
        let sidebar = egui::Rect::from_min_size(
            egui::pos2(screen.min.x, screen.min.y + TITLEBAR_H),
            egui::vec2(SIDEBAR_W, screen.height() - TITLEBAR_H),
        );
        self.background.paint(ctx, &self.theme, &[titlebar, sidebar]);

        self.show_titlebar(ctx);
        self.show_sidebar(ctx);

        // Плавное появление контента при переключении вкладок.
        let t = (self.tab_switched_at.elapsed().as_secs_f32() / 0.25).min(1.0);
        if t < 1.0 {
            ctx.request_repaint();
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(self.theme.content_tint())
                    .inner_margin(egui::Margin::same(24.0)),
            )
            .show(ctx, |ui| {
                ui.set_opacity(t);
                ui.add_space((1.0 - t) * 12.0);
                match self.tab {
                    Tab::Play => {
                        ui::play::show(ui, &self.theme, &self.auth, &mut self.play, &self.launch)
                    }
                    Tab::Instances => ui::instances::show(ui),
                    Tab::Settings => ui::settings::show(ui, &mut self.settings, &mut self.theme),
                }
            });

        // Фейд-ин всего окна при запуске лаунчера.
        let fade = (self.started_at.elapsed().as_secs_f32() / 0.6).min(1.0);
        if fade < 1.0 {
            ctx.request_repaint();
            let a = ((1.0 - fade) * 255.0) as u8;
            ctx.layer_painter(egui::LayerId::new(
                egui::Order::Foreground,
                egui::Id::new("startup_fade"),
            ))
            .rect_filled(screen, 0.0, egui::Color32::from_black_alpha(a));
        }
    }
}

/// Иконка-кнопка сайдбара: подсветка выбранной вкладки акцентным цветом
/// и полоска-индикатор слева, как в современных лаунчерах.
fn nav_button(
    ui: &mut egui::Ui,
    selected: bool,
    icon: &str,
    label: &str,
    accent: egui::Color32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(46.0, 46.0), egui::Sense::click());
    let rounding = egui::Rounding::same(12.0);
    if selected {
        ui.painter()
            .rect_filled(rect, rounding, accent.gamma_multiply(0.22));
        let bar = egui::Rect::from_min_size(
            egui::pos2(rect.min.x - 9.0, rect.min.y + 12.0),
            egui::vec2(3.0, 22.0),
        );
        ui.painter().rect_filled(bar, egui::Rounding::same(2.0), accent);
    } else if response.hovered() {
        ui.painter().rect_filled(
            rect,
            rounding,
            ui.visuals().widgets.hovered.bg_fill.gamma_multiply(0.6),
        );
    }
    let color = if selected {
        accent
    } else {
        ui.visuals().text_color()
    };
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(20.0),
        color,
    );
    response.on_hover_text(label)
}
