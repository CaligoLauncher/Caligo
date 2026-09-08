use eframe::egui::{self, pos2, vec2, Color32, Rect, Stroke};
use crate::auth::{AuthManager,AuthState};
use crate::launch::{LaunchManager,LaunchState};
use crate::launch::manifest::ManifestVersion;
use crate::launch::run::LaunchProfile;
use crate::skin::{self,SkinManager};
use crate::theme::ThemePreset;
use super::components as c;
use super::instances::InstancesState;

#[derive(Default)]
pub struct PlayState {
    pub selected_version: Option<String>,
    pub selected_instance: Option<String>,
    pub offline_name: String,
    pub version_search: String,
}

#[derive(Clone,Copy)]
pub enum HomeAction { Library, Create, Profile }

pub fn show(ui:&mut egui::Ui,theme:&ThemePreset,auth:&AuthManager,play:&mut PlayState,launch:&LaunchManager,skin_mgr:&SkinManager,instances:&mut InstancesState)->Option<HomeAction> {
    launch.ensure_versions(ui.ctx().clone());
    instances.ensure_loaded();
    let mut action=None;
    let mut library_pick=None;
    c::page_title(ui,"Главная","",theme);
    egui::ScrollArea::vertical().id_salt("home_scroll").auto_shrink([false,false]).show(ui,|ui| {
        let width=ui.available_width();
        let hero_h=if width<650.0 {200.0} else {236.0};
        let (hero,_)=ui.allocate_exact_size(vec2(width,hero_h),egui::Sense::hover());
        // An original in-app landscape, not a remote marketing banner.
        c::landscape(ui.painter(),hero,true);
        let content=Rect::from_min_max(hero.min+vec2(28.0,20.0),pos2(hero.left()+hero.width()*0.65,hero.bottom()-20.0));
        c::label(ui.painter(),Rect::from_min_size(content.min,vec2(content.width(),20.0)),"Minecraft · Java Edition",12.0,Color32::from_rgb(175,195,207));
        let title=play.selected_instance.as_deref().unwrap_or("Твой следующий мир");
        let title_r=Rect::from_min_size(content.min+vec2(0.0,34.0),vec2(content.width(),48.0));
        c::label(ui.painter(),title_r,title,if width<650.0 {27.0}else{33.0},Color32::WHITE);
        c::label(ui.painter(),Rect::from_min_size(content.min+vec2(0.0,88.0),vec2(content.width(),22.0)),"Выбери версию. Всё остальное — за горизонтом.",13.0,Color32::from_rgb(175,195,207));
        let mode=if matches!(auth.state(),AuthState::SignedIn(_)) {"Microsoft-аккаунт"} else {"Оффлайн-режим"};
        c::label(ui.painter(),Rect::from_min_size(pos2(content.left(),hero.bottom()-46.0),vec2(content.width(),20.0)),mode,12.0,Color32::from_rgb(175,195,207));

        launch_bar(ui,theme,auth,play,launch);
        ui.add_space(24.0);
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("Библиотека").size(18.0).strong().color(theme.text_primary()));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui| {
                if c::button(ui,"Все сборки",vec2(118.0,36.0),theme,false).clicked(){action=Some(HomeAction::Library);}
            });
        });
        ui.add_space(8.0);
        let items=instances.items();
        if items.is_empty() {
            c::empty(ui,"Начни со своей сборки","Сохрани имя и версию Minecraft, чтобы не выбирать их каждый раз.",theme);
            if c::button(ui,"Создать сборку",vec2(168.0,40.0),theme,false).clicked(){action=Some(HomeAction::Create);}
        } else {
            for (i,inst) in items.iter().take(3).enumerate() {
                if super::instances::compact_row(ui,theme,inst,play.selected_instance.as_deref()==Some(inst.name.as_str()),i).clicked() {
                    library_pick=Some((inst.name.clone(),inst.version.clone()));
                }
                ui.add_space(6.0);
            }
        }
        ui.add_space(24.0);
        ui.separator();
        ui.add_space(12.0);
        let name=match auth.state(){AuthState::SignedIn(a)=>a.username,_=>play.offline_name.trim().to_string()};
        ui.horizontal(|ui| {
            let (r,_)=ui.allocate_exact_size(vec2(36.0,36.0),egui::Sense::hover());
            skin::paint_head(ui.painter(),r,skin_mgr.texture().as_ref(),6.0);
            ui.vertical(|ui|{
                ui.label(egui::RichText::new(if name.is_empty(){"Твой профиль"}else{&name}).color(theme.text_primary()));
                ui.label(egui::RichText::new(if name.is_empty(){"Войди через Microsoft или укажи ник"}else{"Управление входом и персонажем"}).size(12.0).color(theme.text_tertiary()));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
                if c::button(ui,"Открыть",vec2(92.0,40.0),theme,false).clicked(){action=Some(HomeAction::Profile);}
            });
        });
        // The static player belongs to the profile, not an empty social panel.
        if !name.is_empty() {
            let (r,_)=ui.allocate_exact_size(vec2(width,200.0),egui::Sense::hover());
            skin::paint_paperdoll(ui.painter(),r,skin_mgr.texture().as_ref(),None,theme.accent_color(),0.0);
        }
        ui.add_space(12.0);
    });
    if let Some((name,version))=library_pick {
        play.selected_instance=Some(name);play.selected_version=Some(version);
    }
    action
}

fn launch_bar(ui:&mut egui::Ui,theme:&ThemePreset,auth:&AuthManager,play:&mut PlayState,launch:&LaunchManager) {
    let width=ui.available_width();
    let vh=theme.modules.version_button.height_or(48.0).clamp(40.0,120.0);
    let ph=theme.modules.play_button.height_or(48.0).clamp(40.0,120.0);
    let h=vh.max(ph)+32.0;
    let (rect,_)=ui.allocate_exact_size(vec2(width,h),egui::Sense::hover());
    ui.painter().rect_filled(rect,egui::Rounding{nw:0.0,ne:0.0,sw:14.0,se:14.0},theme.card_fill());
    let pw=theme.modules.play_button.width_or(172.0).clamp(132.0,(width*0.42).max(132.0));
    let vw=theme.modules.version_button.width_or(260.0).clamp(120.0,(width-pw-48.0).max(120.0));
    let vr=Rect::from_min_size(pos2(rect.left()+16.0,rect.center().y-vh/2.0),vec2(vw,vh));
    let pr=Rect::from_min_size(pos2(rect.right()-pw-16.0,rect.center().y-ph/2.0),vec2(pw,ph));
    let mut v_ui=ui.new_child(egui::UiBuilder::new().max_rect(vr).layout(egui::Layout::top_down(egui::Align::Min)));
    let vs=&theme.modules.version_button;
    let mut style=(*v_ui.style()).clone();
    style.spacing.interact_size.y=vh;
    style.visuals.widgets.inactive.weak_bg_fill=vs.fill_or(theme.surface(2));
    style.visuals.widgets.inactive.rounding=egui::Rounding::same(vs.rounding_or(8.0));
    style.visuals.widgets.inactive.bg_stroke=vs.border_or(Stroke::NONE);
    v_ui.set_style(style);
    match launch.versions() {
        None=>{v_ui.horizontal(|ui|{ui.spinner();ui.label("Загрузка версий…");});}
        Some(Err(e))=>{
            if v_ui.button("Повторить загрузку").on_hover_text(e).clicked(){launch.retry_versions(ui.ctx().clone());}
        }
        Some(Ok(versions))=>{
            let releases:Vec<_>=versions.iter().filter(|v|v.kind=="release").collect();
            let selected=play.selected_version.clone().or_else(||releases.first().map(|v|v.id.clone()));
            let text=selected.as_ref().map(|s|format!("Minecraft  {s}")).unwrap_or_else(||"Нет версий".into());
            egui::ComboBox::from_id_salt("launch_version").width(vw).selected_text(text).height(280.0).show_ui(&mut v_ui,|ui|{
                ui.add(egui::TextEdit::singleline(&mut play.version_search).hint_text("Найти версию…").desired_width(vw-24.0));
                let search=play.version_search.to_lowercase();
                for v in releases.iter().filter(|v|v.id.to_lowercase().contains(&search)) {
                    if ui.selectable_label(selected.as_deref()==Some(v.id.as_str()),&v.id).clicked(){
                        play.selected_version=Some(v.id.clone());play.selected_instance=None;
                        ui.close_menu();
                    }
                }
            });
        }
    }
    let current=launch.state();
    let version=selected_version(play,launch);
    let enabled=version.is_some()&&!matches!(current,LaunchState::Preparing(_)|LaunchState::Running);
    let label=match &current {LaunchState::Preparing(_)=>"Подготовка…",LaunchState::Running=>"Игра запущена",_=>"Играть"};
    let ps=&theme.modules.play_button;
    let fill=ps.fill_or(theme.accent_color());
    let mut b_ui=ui.new_child(egui::UiBuilder::new().max_rect(pr));
    let response=b_ui.add_enabled(enabled,egui::Button::new(egui::RichText::new(label).size(15.0).strong().color(c::contrast(fill)))
        .fill(fill).rounding(egui::Rounding::same(ps.rounding_or(10.0)))
        .stroke(ps.border_or(Stroke::NONE)).min_size(pr.size()));
    if response.clicked() {
        if let Some(v)=version {launch.launch(ui.ctx().clone(),v,profile_for(auth,play));}
    }
    match current {
        LaunchState::Preparing(step)=>{ui.add_space(8.0);ui.horizontal(|ui|{ui.spinner();ui.label(step);});}
        LaunchState::Failed(err)=>{
            ui.add_space(8.0);
            egui::Frame::none().fill(Color32::from_rgb(50,25,29)).rounding(8.0).inner_margin(12.0).show(ui,|ui|{
                ui.label(egui::RichText::new("Не удалось запустить игру").strong().color(Color32::from_rgb(255,166,171)));
                ui.label(&err);
                if ui.button("Скопировать ошибку").clicked(){ui.ctx().output_mut(|o|o.copied_text=err.clone());}
            });
        }
        LaunchState::Exited(code)=>{ui.add_space(8.0);ui.label(format!("Игра завершена · код {code}"));}
        _=>{}
    }
}

fn profile_for(auth:&AuthManager,play:&PlayState)->LaunchProfile {
    match auth.state(){
        AuthState::SignedIn(a)=>LaunchProfile{username:a.username,uuid:a.uuid,access_token:a.access_token},
        _=>LaunchProfile{
            username:if play.offline_name.trim().is_empty(){"Player".into()}else{play.offline_name.trim().into()},
            uuid:"00000000-0000-0000-0000-000000000000".into(),access_token:"0".into(),
        }
    }
}

fn selected_version(play:&PlayState,launch:&LaunchManager)->Option<ManifestVersion>{
    let versions=launch.versions()?.ok()?;
    match &play.selected_version {
        Some(id)=>versions.into_iter().find(|v|v.kind=="release"&&v.id==*id),
        None=>versions.into_iter().find(|v|v.kind=="release"),
    }
}