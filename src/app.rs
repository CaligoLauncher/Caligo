use std::time::Instant;

use eframe::egui;

use crate::auth::AuthManager;
use crate::background::Background;
use crate::effects::Mist;
use crate::launch::LaunchManager;
use crate::theme::ThemePreset;
use crate::ui;

const TITLEBAR_H: f32 = 36.0;
/// Ширина зоны сайдбара (панель + отступы вокруг «плавающей» карточки).
const SIDEBAR_W: f32 = 72.0;
const SIDEBAR_CARD_W: f32 = 56.0;
const SIDEBAR_MARGIN: f32 = 8.0;
const SIDEBAR_ROUNDING: f32 = 18.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Play,
    Instances,
    Settings,
}

/// «Плавающая» скруглённая карточка левого меню (не на всю высоту,
/// с отступами от краёв окна — без ровных системных краёв).
fn sidebar_card_rect(screen: egui::Rect) -> egui::Rect {
    egui::Rect::from_min_size(
        egui::pos2(
            screen.min.x + SIDEBAR_MARGIN,
            screen.min.y + TITLEBAR_H + SIDEBAR_MARGIN,
        ),
        egui::vec2(
            SIDEBAR_CARD_W,
            screen.height() - TITLEBAR_H - 2.0 * SIDEBAR_MARGIN,
        ),
    )
}

/// Плавное смешение двух цветов.
fn mix(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let l = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t.clamp(0.0, 1.0)) as u8;
    egui::Color32::from_rgb(l(a.r(), b.r()), l(a.g(), b.g()), l(a.b(), b.b()))
}

pub struct CaligoApp {
    pub tab: Tab,
    pub theme: ThemePreset,
    pub settings: ui::settings::SettingsState,
    pub auth: AuthManager,
    pub launch: LaunchManager,
    pub play: ui::play::PlayState,
    background: Background,
    mist: Mist,
    started_at: Instant,
    tab_switched_at: Instant,
}

impl CaligoApp {
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
            mist: Mist::new(),
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

    /// Кастомный титлбар: полностью прозрачный, без подложки и краёв.
    /// Кнопки окна — «точки», раскрывающие цвет при наведении, а не
    /// системные глифы.
    fn show_titlebar(&mut self, ctx: &egui::Context) {
        let accent = self.theme.accent_color();
        egui::TopBottomPanel::top("titlebar")
            .exact_height(TITLEBAR_H)
            .frame(egui::Frame::none())
            .show_separator_line(false)
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
                    ui.add_space(14.0);
                    // Светящаяся точка-«глаз» — маленький фирменный знак.
                    let (dot, _) =
                        ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                    ui.painter()
                        .circle_filled(dot.center(), 7.0, accent.gamma_multiply(0.25));
                    ui.painter().circle_filled(dot.center(), 3.5, accent);
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("Caligo").strong().color(accent));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(10.0);
                        if window_button(ui, true, "Закрыть").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if window_button(ui, false, "Развернуть").clicked() {
                            let maximized =
                                ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                        }
                        if window_button(ui, false, "Свернуть").clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                    });
                });
            });
    }

    /// Левое меню — «плавающая» скруглённая карточка с иконками.
    fn show_sidebar(&mut self, ctx: &egui::Context) {
        let accent = self.theme.accent_color();
        let mut clicked: Option<Tab> = None;
        egui::SidePanel::left("nav")
            .resizable(false)
            .exact_width(SIDEBAR_W)
            .frame(egui::Frame::none())
            .show_separator_line(false)
            .show(ctx, |ui| {
                let card = sidebar_card_rect(ctx.screen_rect());
                ui.painter().rect_filled(
                    card,
                    egui::Rounding::same(SIDEBAR_ROUNDING),
                    self.theme.glass_fill(),
                );
                let mut card_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(card)
                        .layout(egui::Layout::top_down(egui::Align::Center)),
                );
                card_ui.add_space(14.0);
                for (tab, icon, label) in [
                    (Tab::Play, "▶", "Играть"),
                    (Tab::Instances, "📦", "Сборки"),
                    (Tab::Settings, "⚙", "Настройки"),
                ] {
                    if nav_button(&mut card_ui, self.tab == tab, icon, label, accent).clicked() {
                        clicked = Some(tab);
                    }
                    card_ui.add_space(6.0);
                }
            });
        if let Some(tab) = clicked {
            self.switch_tab(tab);
        }
    }
}

impl eframe::App for CaligoApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let screen = ctx.screen_rect();
        // «Стекло» только под карточкой сайдбара: титлбар полностью
        // прозрачный, размытый срез фона рисуется со скруглением карточки.
        let card = sidebar_card_rect(screen);
        self.background.paint(
            ctx,
            &self.theme,
            &[(card, egui::Rounding::same(SIDEBAR_ROUNDING))],
        );
        // Атмосферная «мгла»: светлячки поверх фона, под панелями.
        self.mist.paint(ctx, self.theme.accent_color());

        self.show_titlebar(ctx);
        self.show_sidebar(ctx);

        // Плавное появление контента при переключении вкладок.
        let t = (self.tab_switched_at.elapsed().as_secs_f32() / 0.25).min(1.0);
        if t < 1.0 {
            ctx.request_repaint();
        }
        // Контент — тоже «плавающая» скруглённая карточка, в пару к сайдбару.
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(self.theme.content_tint())
                    .rounding(egui::Rounding::same(SIDEBAR_ROUNDING))
                    .outer_margin(egui::Margin {
                        left: 0.0,
                        right: SIDEBAR_MARGIN,
                        top: SIDEBAR_MARGIN,
                        bottom: SIDEBAR_MARGIN,
                    })
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

/// Кнопка окна в титлбаре: спокойная точка, которая при наведении плавно
/// разгорается (красным — для закрытия) и чуть увеличивается.
fn window_button(ui: &mut egui::Ui, danger: bool, tooltip: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::click());
    let hover = ui.ctx().animate_bool(response.id.with("hover"), response.hovered());
    let base = ui.visuals().weak_text_color().gamma_multiply(0.7);
    let target = if danger {
        egui::Color32::from_rgb(235, 87, 87)
    } else {
        ui.visuals().text_color()
    };
    let color = mix(base, target, hover);
    ui.painter()
        .circle_filled(rect.center(), 5.0 + hover * 1.5, color);
    response.on_hover_text(tooltip)
}

/// Иконка-кнопка сайдбара: мягкое свечение выбранной вкладки,
/// плавная подсветка при наведении.
fn nav_button(
    ui: &mut egui::Ui,
    selected: bool,
    icon: &str,
    label: &str,
    accent: egui::Color32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(46.0, 46.0), egui::Sense::click());
    let hover = ui
        .ctx()
        .animate_bool(response.id.with("hover"), response.hovered() && !selected);
    let rounding = egui::Rounding::same(12.0);
    if selected {
        // Свечение вокруг активной иконки вместо жёсткой рамки.
        ui.painter()
            .circle_filled(rect.center(), 27.0, accent.gamma_multiply(0.10));
        ui.painter()
            .rect_filled(rect, rounding, accent.gamma_multiply(0.22));
        let bar = egui::Rect::from_min_size(
            egui::pos2(rect.min.x - 5.0, rect.min.y + 12.0),
            egui::vec2(3.0, 22.0),
        );
        ui.painter().rect_filled(bar, egui::Rounding::same(2.0), accent);
    } else if hover > 0.0 {
        ui.painter().rect_filled(
            rect,
            rounding,
            egui::Color32::from_white_alpha((12.0 * hover) as u8),
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
        egui::FontId::proportional(20.0 + hover * 1.5),
        color,
    );
    response.on_hover_text(label)
}
