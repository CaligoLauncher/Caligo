use eframe::egui::{self,pos2,vec2,Rect,Stroke};
use crate::auth::{AuthManager,AuthState};
use crate::background::Background;
use crate::effects::Mist;
use crate::launch::LaunchManager;
use crate::skin::{self,SkinManager};
use crate::theme::ThemePreset;
use crate::ui;

const TITLEBAR_H:f32=48.0;
const PROFILE_W:f32=280.0;

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
pub enum Tab{Home,Instances,Settings}

fn install_fonts(ctx:&egui::Context){
    let mut fonts=egui::FontDefinitions::default();
    fonts.font_data.insert("manrope".into(),egui::FontData::from_static(include_bytes!("../assets/fonts/Manrope.ttf")));
    fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0,"manrope".into());
    fonts.font_data.insert("manrope-semibold".into(),egui::FontData::from_static(include_bytes!("../assets/fonts/Manrope-SemiBold.ttf")));
    let mut headings=fonts.families[&egui::FontFamily::Proportional].clone();
    headings.insert(0,"manrope-semibold".into());
    fonts.families.insert(egui::FontFamily::Name("heading".into()),headings);
    ctx.set_fonts(fonts);
}

pub struct CaligoApp{
    pub tab:Tab,
    pub theme:ThemePreset,
    pub settings:ui::settings::SettingsState,
    pub auth:AuthManager,
    pub launch:LaunchManager,
    pub play:ui::play::PlayState,
    pub instances:ui::instances::InstancesState,
    pub skin:SkinManager,
    background:Background,
    mist:Mist,
    profile_open:bool,
    visual_test:bool,
    pub editor:crate::editor::Editor,
}

impl CaligoApp{
    pub fn new(cc:&eframe::CreationContext<'_>)->Self{
        let mut app=Self::with_context(&cc.egui_ctx);
        app.editor=crate::editor::Editor::load();
        app
    }
    fn with_context(ctx:&egui::Context)->Self{
        install_fonts(ctx);
        let mut theme=ThemePreset::default();theme.modules.mist=false;theme.modules.background.vignette=0.0;theme.apply(ctx);
        Self{tab:Tab::Home,theme,settings:Default::default(),auth:Default::default(),launch:Default::default(),play:Default::default(),instances:Default::default(),skin:Default::default(),background:Background::load(ctx),mist:Mist::new(),profile_open:false,visual_test:false,editor:Default::default()}
    }
    fn skin_key(&self)->Option<String>{
        match self.auth.state(){
            AuthState::SignedIn(a)=>Some(a.uuid),
            _=>{let n=self.play.offline_name.trim();if n.is_empty(){None}else{Some(n.into())}}
        }
    }
    pub fn render(&mut self,ctx:&egui::Context){
        let active=match self.tab{Tab::Home=>crate::composition::Action::Home,Tab::Instances=>crate::composition::Action::Library,Tab::Settings=>crate::composition::Action::Settings};
        if !self.editor.active(){self.editor.page=active;}
        if ctx.input_mut(|i|i.consume_key(egui::Modifiers::CTRL|egui::Modifiers::SHIFT,egui::Key::E)){self.editor.begin();self.profile_open=false;}
        if let Some(c)=self.editor.document.background {
            ctx.layer_painter(egui::LayerId::background()).rect_filled(ctx.screen_rect(),0.0,crate::theme::color_arr(c));
        }else{self.background.paint(ctx,&self.theme,&[]);}
        if !self.visual_test{self.skin.ensure(ctx,self.skin_key());}
        self.launch.ensure_versions(ctx.clone());self.instances.ensure_loaded();
        self.titlebar(ctx,76.0);
        self.editor.toolbar(ctx,&self.theme);
        let editing=self.editor.active();
        if editing{self.profile_open=false;}
        let active=if editing{self.editor.page}else{active};
        let layout=self.editor.layout(ctx.available_rect());
        let look=self.theme.clone();
        let mut intents=Vec::new();
        let wallpaper=self.editor.document.background.is_none();
        let action=self.editor.shell(ctx,&layout,&look,active,&self.background,|ui,w|{
            if let Some(intent)=crate::workspace::show(ui,w,&mut self.theme,&self.auth,&mut self.play,&self.launch,&self.skin,&mut self.instances,&mut self.settings,&self.background,wallpaper){
                intents.push(intent);
            }
        });
        if !editing {
            if let Some(action)=action {self.dispatch_shell_action(action);}
            for intent in intents {match intent{
                crate::workspace::Intent::Launch=>{
                    if let Some(version)=ui::play::selected_version(&self.play,&self.launch){
                        self.launch.launch(ctx.clone(),version,ui::play::profile_for(&self.auth,&self.play));
                    }
                }
                crate::workspace::Intent::Profile=>self.profile_open=true,
                crate::workspace::Intent::Home=>self.tab=Tab::Home,
                crate::workspace::Intent::Create=>ui::instances::request_create(&mut self.instances),
            }}
            ui::instances::dialogs(ctx,&self.theme,&mut self.instances,&mut self.play,&self.launch);
        }
        if self.theme.modules.mist&&!self.visual_test&&!editing{self.mist.paint(ctx,self.theme.accent_color());}
        self.editor.overlay(ctx,&layout,&self.theme);
    }
    fn dispatch_shell_action(&mut self,action:crate::composition::Action){
        if self.editor.active(){return}
        match action{
            crate::composition::Action::Home=>self.tab=Tab::Home,
            crate::composition::Action::Library=>self.tab=Tab::Instances,
            crate::composition::Action::Settings=>self.tab=Tab::Settings,
            crate::composition::Action::Profile=>self.profile_open=true,
            _=>{}
        }
    }
    fn titlebar(&mut self,ctx:&egui::Context,sb_w:f32){
        let tb=self.theme.modules.titlebar.clone();
        let h=tb.height_or(TITLEBAR_H).clamp(40.0,96.0);
        let mut anchor=None;
        egui::TopBottomPanel::top("titlebar").exact_height(h).show_separator_line(false).frame(egui::Frame::none()).show(ctx,|ui|{
            let r=ui.max_rect();
            ui.painter().rect_filled(r,tb.rounding_or(0.0),tb.fill_or(egui::Color32::from_black_alpha(20)));
            if let Some(s)=tb.border_override(){ui.painter().rect_stroke(r,tb.rounding_or(0.0),s);}
            let drag=ui.interact(r,ui.id().with("drag"),egui::Sense::click_and_drag());
            if drag.drag_started(){ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);}
            if drag.double_clicked(){ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!ctx.input(|i|i.viewport().maximized.unwrap_or(false))));}
            ui.horizontal_centered(|ui|{
                ui.add_space(22.0);
                if ui.add(egui::Label::new(egui::RichText::new("Caligo").size(17.0).strong().color(self.theme.text_primary())).sense(egui::Sense::click())).clicked()&&!self.editor.active(){self.tab=Tab::Home;}
                ui.add_space((sb_w-133.0).max(0.0));
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
                    ui.spacing_mut().item_spacing.x=0.0;
                    if window_button(ui,WinGlyph::Close,"Закрыть",h).clicked(){ctx.send_viewport_cmd(egui::ViewportCommand::Close);}
                    if window_button(ui,WinGlyph::Max,"Развернуть",h).clicked(){ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!ctx.input(|i|i.viewport().maximized.unwrap_or(false))));}
                    if window_button(ui,WinGlyph::Min,"Свернуть",h).clicked(){ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));}
                    ui.add_space(18.0);
                    let chip=profile_chip(ui,&self.theme,&self.auth,&self.play,&self.skin);
                    anchor=Some(chip.rect);
                    if chip.clicked()&&!self.editor.active(){self.profile_open=!self.profile_open;}
                    ui.add_space(10.0);
                    if ui.button("Редактор").on_hover_text("Редактировать интерфейс · Ctrl+Shift+E").clicked(){self.editor.begin();self.profile_open=false;}
                });
            });
        });
        if self.profile_open&&!self.editor.active() {if let Some(r)=anchor{self.profile_popup(ctx,r);}}
    }
    fn profile_popup(&mut self,ctx:&egui::Context,anchor:Rect){
        let pos=pos2((anchor.right()-PROFILE_W).max(8.0),anchor.bottom()+8.0);
        let area=egui::Area::new(egui::Id::new("profile_popup")).fixed_pos(pos).order(egui::Order::Foreground).show(ctx,|ui|{
            egui::Frame::none().fill(self.theme.surface(2)).rounding(12.0).stroke(Stroke::new(1.0_f32,self.theme.surface(4))).inner_margin(20.0).show(ui,|ui|{
                ui.set_width(PROFILE_W-40.0);
                egui::ScrollArea::vertical().max_height((ctx.screen_rect().height()-120.0).max(160.0)).show(ui,|ui|{
                profile_window(ui,self.theme.accent_color(),&self.auth,&mut self.play,&self.skin);
                if self.skin.loading(){ui.label("Загрузка скина…");}
                if let Some(error)=self.skin.error(){ui.label(egui::RichText::new(format!("Скин недоступен: {error}")).size(12.0));}
                if !self.play.offline_name.trim().is_empty(){
                    let (r,_)=ui.allocate_exact_size(vec2(PROFILE_W-40.0,160.0),egui::Sense::hover());
                    skin::paint_paperdoll(ui.painter(),r,self.skin.texture().as_ref(),None,self.theme.accent_color(),0.0);
                }
                });
            });
        });
        let outside=ctx.input(|i|i.pointer.any_pressed())&&ctx.input(|i|i.pointer.interact_pos()).is_some_and(|p|!area.response.rect.contains(p)&&!anchor.contains(p));
        if outside||ctx.input(|i|i.key_pressed(egui::Key::Escape)){self.profile_open=false;}
    }
}
impl eframe::App for CaligoApp{
    fn update(&mut self,ctx:&egui::Context,_frame:&mut eframe::Frame){self.render(ctx);}
}
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
    let h = style.height_or(32.0).clamp(28.0, 64.0);
    let w = style.width_or(galley.size().x + 44.0).clamp(72.0, 190.0);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::click());
    let hover = ui
        .ctx()
        .animate_bool(response.id.with("hover"), response.hovered());
    let rounding = egui::Rounding::same(style.rounding_or(8.0));
    let painter = ui.painter();
    painter.rect_filled(rect, rounding, style.fill_or(egui::Color32::from_rgba_unmultiplied(17,30,46,150)));
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
            ui.label(egui::RichText::new("Профиль").small().weak());
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
        ui.allocate_exact_size(egui::vec2(40.0, bar_h), egui::Sense::click());
    let hover = ui
        .ctx()
        .animate_bool(response.id.with("hover"), response.hovered());
    let danger = glyph == WinGlyph::Close;
    if hover > 0.0 {
        let fill = if danger {
            egui::Color32::from_rgba_unmultiplied(232, 17, 35, (255.0 * hover) as u8)
        } else {
            egui::Color32::from_white_alpha((16.0 * hover) as u8)
        };
        ui.painter().rect_filled(rect, 0.0, fill);
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


#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum NavIcon {
    Home,
    Cube,
    Sliders,
}

/// Рисует иконку навигации штрихами. При наведении слегка растёт.
pub(crate) fn paint_nav_icon(
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
#[cfg(test)]
impl CaligoApp {
    pub fn visual_fixture(ctx: &egui::Context, tab: Tab, populated: bool) -> Self {
        let mut app=Self::with_context(ctx);
        app.tab=tab;
        app.visual_test=true;
        app.launch=LaunchManager::visual_fixture();
        app.theme.modules.mist=false;
        app.instances=ui::instances::InstancesState::fixture(if populated {
            vec![
                ui::instances::Instance{name:"Выживание".into(),version:"1.21.1".into()},
                ui::instances::Instance{name:"Творческий мир".into(),version:"1.20.4".into()},
            ]
        }else{Vec::new()});
        app
    }
}
