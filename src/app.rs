use std::time::Instant;

use eframe::egui;

use crate::auth::{AuthManager, AuthState};
use crate::background::Background;
use crate::effects::Mist;
use crate::launch::LaunchManager;
use crate::skin::{self, SkinManager};
use crate::theme::ThemePreset;
use crate::ui;

const TITLEBAR_H: f32 = 36.0;
/// Ширина «плавающей» карточки сайдбара по умолчанию. Сайдбар — капсула:
/// скругление по умолчанию = половине ширины карточки (Tahoe предпочитает
/// капсульные формы: «чем круглее, тем легче смотреть»).
const SIDEBAR_CARD_W: f32 = 56.0;
const SIDEBAR_MARGIN: f32 = 8.0;
/// Скругление контентных карточек вкладок.
const CARD_ROUNDING: f32 = 18.0;
/// Ширина мини-окна профиля.
const PROFILE_W: f32 = 250.0;

/// Экраны лаунчера. `Home` — главное меню (основной экран лаунчера).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Home,
    Instances,
    Settings,
}

/// «Плавающая» скруглённая карточка левого меню (не на всю высоту,
/// с отступами от краёв окна — без ровных системных краёв).
/// Ширина сайдбара и высота титлбара берутся из настроек модулей.
fn sidebar_card_rect(screen: egui::Rect, theme: &ThemePreset) -> egui::Rect {
    let titlebar_h = theme.modules.titlebar.height_or(TITLEBAR_H);
    let w = theme.modules.sidebar.width_or(SIDEBAR_CARD_W);
    egui::Rect::from_min_size(
        egui::pos2(
            screen.min.x + SIDEBAR_MARGIN,
            screen.min.y + titlebar_h + SIDEBAR_MARGIN,
        ),
        egui::vec2(w, screen.height() - titlebar_h - 2.0 * SIDEBAR_MARGIN),
    )
}

/// Кромка Liquid Glass: тонкий светлый контур по периметру плюс более
/// яркий «блик» по верхней грани — стекло ловит свет сверху (specular
/// highlight из HIG); панель читается краем, а не жёсткой рамкой.
/// Если пользователь задал модулю собственный бортик (цвет/толщину),
/// рисуется только он — без «блика».
fn glass_edge_styled(
    painter: &egui::Painter,
    rect: egui::Rect,
    rounding: egui::Rounding,
    custom: Option<egui::Stroke>,
) {
    if let Some(stroke) = custom {
        if stroke.width > 0.0 {
            painter.rect_stroke(rect, rounding, stroke);
        }
        return;
    }
    painter.rect_stroke(
        rect,
        rounding,
        egui::Stroke::new(1.0_f32, egui::Color32::from_white_alpha(14)),
    );
    let r = rounding.nw.max(rounding.ne);
    painter.line_segment(
        [
            egui::pos2(rect.min.x + r, rect.min.y + 0.5),
            egui::pos2(rect.max.x - r, rect.min.y + 0.5),
        ],
        egui::Stroke::new(1.0_f32, egui::Color32::from_white_alpha(36)),
    );
}

/// Подключает фирменный шрифт Manrope (открытая лицензия OFL, есть
/// кириллица). Штатные шрифты egui остаются фолбэком для иконок/эмодзи.
fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "manrope".to_owned(),
        egui::FontData::from_static(include_bytes!("../assets/fonts/Manrope.ttf")),
    );
    fonts
        .families
        .get_mut(&egui::FontFamily::Proportional)
        .unwrap()
        .insert(0, "manrope".to_owned());
    ctx.set_fonts(fonts);
}

pub struct CaligoApp {
    pub tab: Tab,
    pub theme: ThemePreset,
    pub settings: ui::settings::SettingsState,
    pub auth: AuthManager,
    pub launch: LaunchManager,
    pub play: ui::play::PlayState,
    pub instances: ui::instances::InstancesState,
    pub skin: SkinManager,
    background: Background,
    mist: Mist,
    started_at: Instant,
    tab_switched_at: Instant,
    profile_open: bool,
}

impl CaligoApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        install_fonts(&cc.egui_ctx);
        let theme = ThemePreset::default();
        theme.apply(&cc.egui_ctx);
        let background = Background::load(&cc.egui_ctx);
        Self {
            tab: Tab::Home,
            theme,
            settings: Default::default(),
            auth: Default::default(),
            launch: Default::default(),
            play: Default::default(),
            instances: Default::default(),
            skin: Default::default(),
            background,
            mist: Mist::new(),
            started_at: Instant::now(),
            tab_switched_at: Instant::now(),
            profile_open: false,
        }
    }

    fn switch_tab(&mut self, tab: Tab) {
        if self.tab != tab {
            self.tab = tab;
            self.tab_switched_at = Instant::now();
        }
    }

    /// Кто мы сейчас — ключ для загрузки скина (UUID аккаунта или
    /// оффлайн-ник), чтобы аватар в чипе профиля жил на любом экране.
    fn skin_key(&self) -> Option<String> {
        match self.auth.state() {
            AuthState::SignedIn(account) => Some(account.uuid.clone()),
            _ => {
                let name = self.play.offline_name.trim().to_string();
                if name.is_empty() {
                    None
                } else {
                    Some(name)
                }
            }
        }
    }

    /// Кастомный титлбар: полностью прозрачный, без подложки и краёв.
    /// Слева — знак и имя Caligo, справа — чип профиля (мини-окошко
    /// открывается по клику) и кнопки окна («точки»).
    fn show_titlebar(&mut self, ctx: &egui::Context) {
        let accent = self.theme.accent_color();
        self.skin.ensure(ctx, self.skin_key());
        let mut go_home = false;
        let mut chip_rect: Option<egui::Rect> = None;
        let mut toggle_profile = false;
        let tb_style = self.theme.modules.titlebar.clone();
        let tb_h = tb_style.height_or(TITLEBAR_H);
        egui::TopBottomPanel::top("titlebar")
            .exact_height(tb_h)
            .frame(egui::Frame::none())
            .show_separator_line(false)
            .show(ctx, |ui| {
                let bar_rect = ui.max_rect();
                // По умолчанию титлбар полностью прозрачный; заливка и
                // бортик появляются только если заданы в настройках.
                if tb_style.is_custom() {
                    let rounding = egui::Rounding::same(tb_style.rounding_or(0.0));
                    ui.painter().rect_filled(
                        bar_rect,
                        rounding,
                        tb_style.fill_or(egui::Color32::TRANSPARENT),
                    );
                    if let Some(stroke) = tb_style.border_override() {
                        if stroke.width > 0.0 {
                            ui.painter().rect_stroke(bar_rect, rounding, stroke);
                        }
                    }
                }
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
                    let brand = ui.add(
                        egui::Label::new(egui::RichText::new("Caligo").strong().color(accent))
                            .sense(egui::Sense::click()),
                    );
                    if brand.on_hover_text("На главную").clicked() {
                        go_home = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(10.0);
                        // Кнопки окна справа — как принято в Windows:
                        // закрыть / развернуть / свернуть. Монохромные,
                        // рисованные штрихами, с мягким круглым ховером
                        // (красным — только у «закрыть»).
                        if window_button(ui, WinGlyph::Close, "Закрыть", tb_h).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        if window_button(ui, WinGlyph::Max, "Развернуть", tb_h).clicked() {
                            let maximized =
                                ctx.input(|i| i.viewport().maximized.unwrap_or(false));
                            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
                        }
                        if window_button(ui, WinGlyph::Min, "Свернуть", tb_h).clicked() {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                        ui.add_space(10.0);
                        // Мини-чип профиля: лицо скина + ник; клик — окно.
                        let chip =
                            profile_chip(ui, &self.theme, &self.auth, &self.play, &self.skin);
                        chip_rect = Some(chip.rect);
                        if chip.clicked() {
                            toggle_profile = true;
                        }
                    });
                });
            });
        if go_home {
            self.switch_tab(Tab::Home);
        }
        if toggle_profile {
            self.profile_open = !self.profile_open;
        } else if self.profile_open {
            if let Some(anchor) = chip_rect {
                self.show_profile_popup(ctx, anchor);
            }
        }
    }

    /// Мини-окошко профиля под чипом: вход через Microsoft, оффлайн-ник,
    /// код устройства, выход. Закрывается кликом мимо или Esc.
    fn show_profile_popup(&mut self, ctx: &egui::Context, anchor: egui::Rect) {
        let accent = self.theme.accent_color();
        let bg = self.theme.background_color();
        let fill = egui::Color32::from_rgba_unmultiplied(bg.r(), bg.g(), bg.b(), 246);
        let pos = egui::pos2((anchor.max.x - PROFILE_W).max(8.0), anchor.max.y + 8.0);
        let area = egui::Area::new(egui::Id::new("profile_popup"))
            .fixed_pos(pos)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(fill)
                    .rounding(egui::Rounding::same(14.0))
                    .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_white_alpha(16)))
                    .inner_margin(egui::Margin::same(16.0))
                    .show(ui, |ui| {
                        ui.set_width(PROFILE_W - 32.0);
                        profile_window(ui, accent, &self.auth, &mut self.play, &self.skin);
                    });
            });
        let clicked_away = ctx.input(|i| i.pointer.any_pressed())
            && ctx
                .input(|i| i.pointer.interact_pos())
                .map_or(false, |p| {
                    !area.response.rect.contains(p) && !anchor.expand(4.0).contains(p)
                });
        if clicked_away || ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.profile_open = false;
        }
    }

    /// Левое меню — «плавающая» скруглённая карточка с иконками.
    /// Первая кнопка — главное меню.
    fn show_sidebar(&mut self, ctx: &egui::Context) {
        let accent = self.theme.accent_color();
        let mut clicked: Option<Tab> = None;
        let card = sidebar_card_rect(ctx.screen_rect(), &self.theme);
        let sb_style = self.theme.modules.sidebar.clone();
        egui::SidePanel::left("nav")
            .resizable(false)
            .exact_width(card.width() + 2.0 * SIDEBAR_MARGIN)
            .frame(egui::Frame::none())
            .show_separator_line(false)
            .show(ctx, |ui| {
                // Капсула по умолчанию: скругление = половине ширины;
                // и радиус, и заливка, и бортик настраиваются.
                let rounding =
                    egui::Rounding::same(sb_style.rounding_or(card.width() / 2.0));
                // Regular-стекло: крупный элемент навигации по HIG
                // непрозрачнее мелких — текст и иконки всегда читаемы.
                ui.painter().rect_filled(
                    card,
                    rounding,
                    sb_style.fill_or(self.theme.glass_regular()),
                );
                glass_edge_styled(ui.painter(), card, rounding, sb_style.border_override());
                let mut card_ui = ui.new_child(
                    egui::UiBuilder::new()
                        .max_rect(card)
                        .layout(egui::Layout::top_down(egui::Align::Center)),
                );
                card_ui.add_space(14.0);
                for (tab, icon, label) in [
                    (Tab::Home, NavIcon::Home, "Главная"),
                    (Tab::Instances, NavIcon::Cube, "Сборки"),
                    (Tab::Settings, NavIcon::Sliders, "Настройки"),
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
        let card = sidebar_card_rect(screen, &self.theme);
        let sb_rounding = self
            .theme
            .modules
            .sidebar
            .rounding_or(card.width() / 2.0);
        self.background.paint(
            ctx,
            &self.theme,
            &[(card, egui::Rounding::same(sb_rounding))],
        );
        // Атмосферная «мгла»: светлячки поверх фона, под панелями.
        // Отключается в настройках.
        if self.theme.modules.mist {
            self.mist.paint(ctx, self.theme.accent_color());
        }

        self.show_titlebar(ctx);
        self.show_sidebar(ctx);

        // Плавное появление контента при переключении вкладок.
        let t = (self.tab_switched_at.elapsed().as_secs_f32() / 0.25).min(1.0);
        if t < 1.0 {
            ctx.request_repaint();
        }
        // Главное меню — основной экран: без карточки-подложки, прямо на
        // фоне (мгла остаётся). Вкладки — «плавающая» скруглённая карточка.
        let frame = if self.tab == Tab::Home {
            egui::Frame::none()
                .outer_margin(egui::Margin {
                    left: 0.0,
                    right: SIDEBAR_MARGIN,
                    top: SIDEBAR_MARGIN,
                    bottom: SIDEBAR_MARGIN,
                })
                .inner_margin(egui::Margin::same(24.0))
        } else {
            let tc = &self.theme.modules.tab_card;
            egui::Frame::none()
                .fill(tc.fill_or(self.theme.content_tint()))
                .rounding(egui::Rounding::same(tc.rounding_or(CARD_ROUNDING)))
                .stroke(tc.border_or(self.theme.card_stroke()))
                .outer_margin(egui::Margin {
                    left: 0.0,
                    right: SIDEBAR_MARGIN,
                    top: SIDEBAR_MARGIN,
                    bottom: SIDEBAR_MARGIN,
                })
                .inner_margin(egui::Margin::same(24.0))
        };
        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            ui.set_opacity(t);
            ui.add_space((1.0 - t) * 12.0);
            match self.tab {
                Tab::Home => ui::play::show(
                    ui,
                    &self.theme,
                    &self.auth,
                    &mut self.play,
                    &self.launch,
                    &self.skin,
                ),
                Tab::Instances => ui::instances::show(
                    ui,
                    &self.theme,
                    &mut self.instances,
                    &mut self.play,
                    &self.launch,
                ),
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

/// Компактный чип профиля в титлбаре справа: лицо скина + ник
/// (или «Войти»). Клик открывает мини-окно профиля.
fn profile_chip(
    ui: &mut egui::Ui,
    theme: &ThemePreset,
    auth: &AuthManager,
    play: &crate::ui::play::PlayState,
    skin_mgr: &SkinManager,
) -> egui::Response {
    let label = match auth.state() {
        AuthState::SignedIn(account) => account.username.clone(),
        AuthState::WaitingForUser { .. } | AuthState::InProgress(_) => "Вход…".to_string(),
        _ => {
            let name = play.offline_name.trim();
            if name.is_empty() {
                "Войти".to_string()
            } else {
                name.to_string()
            }
        }
    };
    let style = &theme.modules.profile_chip;
    let font = egui::FontId::proportional(13.0);
    let galley = ui
        .painter()
        .layout_no_wrap(label, font, ui.visuals().text_color());
    let h = style.height_or(26.0);
    let w = style.width_or(galley.size().x + 44.0);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::click());
    let hover = ui
        .ctx()
        .animate_bool(response.id.with("hover"), response.hovered());
    let rounding = egui::Rounding::same(style.rounding_or(h / 2.0));
    let painter = ui.painter();
    painter.rect_filled(rect, rounding, style.fill_or(theme.glass_fill()));
    if hover > 0.0 {
        painter.rect_filled(
            rect,
            rounding,
            egui::Color32::from_white_alpha((10.0 * hover) as u8),
        );
    }
    // Мягкая обводка без «блика»: на тёмном фоне яркая кромка выглядела
    // как белая рамка вокруг «Войти». Бортик настраивается.
    let stroke = style.border_or(egui::Stroke::new(
        1.0_f32,
        egui::Color32::from_white_alpha(10),
    ));
    if stroke.width > 0.0 {
        painter.rect_stroke(rect, rounding, stroke);
    }
    let head = egui::Rect::from_center_size(
        egui::pos2(rect.min.x + 15.0, rect.center().y),
        egui::vec2(16.0, 16.0),
    );
    skin::paint_head(painter, head, skin_mgr.texture().as_ref(), 4.0);
    painter.galley(
        egui::pos2(rect.min.x + 28.0, rect.center().y - galley.size().y / 2.0),
        galley,
        ui.visuals().text_color(),
    );
    response
        .on_hover_text("Профиль")
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Содержимое мини-окна профиля.
fn profile_window(
    ui: &mut egui::Ui,
    accent: egui::Color32,
    auth: &AuthManager,
    play: &mut crate::ui::play::PlayState,
    skin_mgr: &SkinManager,
) {
    match auth.state() {
        AuthState::SignedOut => {
            ui.label(egui::RichText::new("ПРОФИЛЬ").small().weak());
            ui.add_space(8.0);
            ui.add(
                egui::TextEdit::singleline(&mut play.offline_name)
                    .hint_text("Ник (оффлайн)")
                    .desired_width(f32::INFINITY),
            );
            ui.add_space(8.0);
            if ui
                .add_sized(
                    [ui.available_width(), 32.0],
                    egui::Button::new("Войти через Microsoft"),
                )
                .clicked()
            {
                auth.start_login(ui.ctx().clone());
            }
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new("Без входа доступен только оффлайн-режим")
                    .weak()
                    .size(11.0),
            );
        }
        AuthState::WaitingForUser {
            verification_uri,
            user_code,
        } => {
            ui.label(egui::RichText::new("Открой ссылку и введи код:").size(13.0));
            ui.hyperlink(&verification_uri);
            ui.add_space(4.0);
            ui.label(
                egui::RichText::new(&user_code)
                    .size(22.0)
                    .monospace()
                    .strong()
                    .color(accent),
            );
            if ui.button("Скопировать код").clicked() {
                ui.ctx().output_mut(|o| o.copied_text = user_code.clone());
            }
            ui.spinner();
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(500));
        }
        AuthState::InProgress(step) => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(egui::RichText::new(step).size(13.0));
            });
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(500));
        }
        AuthState::SignedIn(account) => {
            ui.horizontal(|ui| {
                let (head, _) =
                    ui.allocate_exact_size(egui::vec2(36.0, 36.0), egui::Sense::hover());
                skin::paint_head(ui.painter(), head, skin_mgr.texture().as_ref(), 8.0);
                ui.add_space(4.0);
                ui.vertical(|ui| {
                    ui.colored_label(
                        accent,
                        egui::RichText::new(&account.username).strong().size(15.0),
                    );
                    ui.label(egui::RichText::new("Microsoft-аккаунт").weak().size(11.0));
                });
            });
            ui.add_space(10.0);
            if ui
                .add_sized([ui.available_width(), 28.0], egui::Button::new("Выйти"))
                .clicked()
            {
                auth.sign_out();
            }
        }
        AuthState::Failed(err) => {
            ui.colored_label(
                egui::Color32::from_rgb(255, 120, 120),
                egui::RichText::new(format!("Ошибка входа: {err}")).size(12.0),
            );
            ui.add_space(6.0);
            if ui.button("Попробовать снова").clicked() {
                auth.start_login(ui.ctx().clone());
            }
        }
    }
}

/// Значки кнопок окна (Windows-стиль, справа).
#[derive(Clone, Copy, PartialEq, Eq)]
enum WinGlyph {
    Min,
    Max,
    Close,
}

/// Кнопка окна: монохромный штриховой значок, при наведении — мягкий
/// круг подсветки («закрыть» подсвечивается красным, как в Windows).
fn window_button(ui: &mut egui::Ui, glyph: WinGlyph, tooltip: &str, bar_h: f32) -> egui::Response {
    let (rect, response) =
        ui.allocate_exact_size(egui::vec2(30.0, bar_h), egui::Sense::click());
    let hover = ui
        .ctx()
        .animate_bool(response.id.with("hover"), response.hovered());
    let danger = glyph == WinGlyph::Close;
    if hover > 0.0 {
        let fill = if danger {
            egui::Color32::from_rgba_unmultiplied(232, 17, 35, (200.0 * hover) as u8)
        } else {
            egui::Color32::from_white_alpha((16.0 * hover) as u8)
        };
        ui.painter().circle_filled(rect.center(), 12.0, fill);
    }
    let color = if danger && hover > 0.4 {
        egui::Color32::WHITE
    } else {
        ui.visuals().text_color()
    };
    let c = rect.center();
    let stroke = egui::Stroke::new(1.2_f32, color);
    match glyph {
        WinGlyph::Min => {
            ui.painter()
                .line_segment([c + egui::vec2(-4.5, 0.0), c + egui::vec2(4.5, 0.0)], stroke);
        }
        WinGlyph::Max => {
            ui.painter().rect_stroke(
                egui::Rect::from_center_size(c, egui::vec2(9.0, 9.0)),
                egui::Rounding::same(2.0),
                stroke,
            );
        }
        WinGlyph::Close => {
            ui.painter().line_segment(
                [c + egui::vec2(-4.5, -4.5), c + egui::vec2(4.5, 4.5)],
                stroke,
            );
            ui.painter().line_segment(
                [c + egui::vec2(-4.5, 4.5), c + egui::vec2(4.5, -4.5)],
                stroke,
            );
        }
    }
    response
        .on_hover_text(tooltip)
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Иконка-кнопка сайдбара: мягкое свечение выбранной вкладки,
/// плавная подсветка при наведении.
fn nav_button(
    ui: &mut egui::Ui,
    selected: bool,
    icon: NavIcon,
    label: &str,
    accent: egui::Color32,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(46.0, 46.0), egui::Sense::click());
    let hover = ui
        .ctx()
        .animate_bool(response.id.with("hover"), response.hovered() && !selected);
    // Круглые иконки-кнопки: круг концентричен капсуле сайдбара
    // (Tahoe: иконочные кнопки — круги, текстовые — капсулы).
    if selected {
        // Свечение вокруг активной иконки вместо жёсткой рамки.
        ui.painter()
            .circle_filled(rect.center(), 27.0, accent.gamma_multiply(0.10));
        ui.painter()
            .circle_filled(rect.center(), 21.0, accent.gamma_multiply(0.25));
    } else if hover > 0.0 {
        ui.painter().circle_filled(
            rect.center(),
            21.0,
            egui::Color32::from_white_alpha((12.0 * hover) as u8),
        );
    }
    let color = if selected {
        accent
    } else {
        ui.visuals().text_color()
    };
    paint_nav_icon(ui.painter(), rect.center(), icon, color, hover);
    response.on_hover_text(label)
}

/// Значки навигации сайдбара. Векторные, рисованные штрихами — в одном
/// стиле с кнопками окна (единая толщина штриха на уровне иерархии,
/// без эмодзи в роли структурных иконок).
#[derive(Clone, Copy, PartialEq, Eq)]
enum NavIcon {
    Home,
    Cube,
    Sliders,
}

/// Рисует иконку навигации штрихами. При наведении слегка растёт.
fn paint_nav_icon(
    painter: &egui::Painter,
    center: egui::Pos2,
    icon: NavIcon,
    color: egui::Color32,
    hover: f32,
) {
    let s = 1.0 + hover * 0.08;
    let stroke = egui::Stroke::new(1.5_f32, color);
    let p = |x: f32, y: f32| center + egui::vec2(x * s, y * s);
    match icon {
        NavIcon::Home => {
            // Домик: крыша, стены и дверной проём.
            painter.line_segment([p(-7.0, 0.5), p(0.0, -6.5)], stroke);
            painter.line_segment([p(0.0, -6.5), p(7.0, 0.5)], stroke);
            painter.line_segment([p(-5.0, -0.5), p(-5.0, 6.5)], stroke);
            painter.line_segment([p(5.0, -0.5), p(5.0, 6.5)], stroke);
            painter.line_segment([p(-5.0, 6.5), p(-1.8, 6.5)], stroke);
            painter.line_segment([p(1.8, 6.5), p(5.0, 6.5)], stroke);
            painter.line_segment([p(-1.8, 6.5), p(-1.8, 2.8)], stroke);
            painter.line_segment([p(1.8, 6.5), p(1.8, 2.8)], stroke);
            painter.line_segment([p(-1.8, 2.8), p(1.8, 2.8)], stroke);
        }
        NavIcon::Cube => {
            // Изометрический куб — «сборка» как блок Minecraft.
            let top = p(0.0, -7.5);
            let ne = p(6.5, -3.75);
            let se = p(6.5, 3.75);
            let bottom = p(0.0, 7.5);
            let sw = p(-6.5, 3.75);
            let nw = p(-6.5, -3.75);
            let mid = p(0.0, 0.0);
            for seg in [
                [top, ne],
                [ne, se],
                [se, bottom],
                [bottom, sw],
                [sw, nw],
                [nw, top],
                [mid, nw],
                [mid, ne],
                [mid, bottom],
            ] {
                painter.line_segment(seg, stroke);
            }
        }
        NavIcon::Sliders => {
            // Три дорожки с бегунками на разных позициях.
            for (dy, knob_x) in [(-5.0_f32, -2.0_f32), (0.0, 3.0), (5.0, -3.5)] {
                painter.line_segment([p(-7.0, dy), p(7.0, dy)], stroke);
                painter.circle_filled(p(knob_x, dy), 2.4 * s, color);
            }
        }
    }
}