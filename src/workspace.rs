//! Functional components. Placement belongs to composition; game state is shared.
//! Rendering returns actions. The app alone decides whether actions may execute.
use eframe::egui::{self,vec2};
use crate::{auth::{AuthManager,AuthState},composition::{Action,Widget},launch::LaunchManager,skin::{self,SkinManager},theme::ThemePreset};
use crate::ui::{components as c,instances::{self,InstancesState},play::{self,PlayState},settings::{self,SettingsState}};

#[derive(Clone,Copy)]
pub enum Intent { Launch,Profile,Home,Create }

pub fn show(ui:&mut egui::Ui,w:&Widget,theme:&mut ThemePreset,auth:&AuthManager,
    play:&mut PlayState,launch:&LaunchManager,skin_mgr:&SkinManager,instances:&mut InstancesState,
    settings:&mut SettingsState,background:&crate::background::Background,wallpaper:bool)->Option<Intent>{
    let mut intent=None;
    let look=theme.clone();
    let bare=matches!(w.action,Action::Character|Action::Cover|Action::Selection|Action::Heading|Action::Launch|Action::LibrarySearch|Action::CreateInstance|Action::Status);
    let padding=if bare{0.0}else{16.0};
    let fill=w.style.fill.map(crate::theme::color_arr).unwrap_or(if bare{egui::Color32::TRANSPARENT}else{look.surface(2)});
    let rect=ui.max_rect();
    if w.action!=Action::Launch{background.surface(ui.painter(),rect,w.style.rounding.unwrap_or(look.rounding),fill,w.style.blur&&wallpaper);}
    let padding=if w.action==Action::Version{8.0}else{padding};
    let inner=rect.shrink(padding);
    let mut component=ui.new_child(egui::UiBuilder::new().id_salt(("component_body",w.id)).max_rect(inner));
    component.set_clip_rect(inner.intersect(ui.clip_rect()));
    component.scope(|ui|{
        if w.action==Action::Character {
            skin::paint_paperdoll(ui.painter(),inner,skin_mgr.texture().as_ref(),None,look.accent_color(),0.0);
            return
        }
        if w.action==Action::Cover {
            crate::rpg_scene::cover(ui.painter(),inner,w.style.rounding.unwrap_or(14.0));
            return
        }
        if w.action==Action::Launch {
            if play::launch_button(ui,&look,w,play,launch){intent=Some(Intent::Launch);}
            return
        }
        if w.action==Action::CreateInstance {
            if ui.add_sized(ui.available_size().max(vec2(32.0,32.0)),egui::Button::new(&w.label).fill(w.style.fill.map(crate::theme::color_arr).unwrap_or(look.surface(3))).rounding(w.style.rounding.unwrap_or(look.rounding))).clicked(){intent=Some(Intent::Create);}
            return
        }
        if w.action==Action::LibrarySearch {instances::search(ui,instances);return}
        let scroll=egui::ScrollArea::vertical().id_salt(("component_scroll",w.id)).auto_shrink([false,false]);
        scroll.show(ui,|ui|{
            match w.action {
                Action::Heading=>{
                    c::label(ui.painter(),egui::Rect::from_min_size(ui.cursor().min,vec2(ui.available_width(),40.0)),&w.label,24.0,look.text_primary());
                    ui.allocate_space(vec2(1.0,40.0));
                }
                Action::Selection=>{
                    let title=play.selected_instance.as_deref().unwrap_or("Minecraft: Java Edition");
                    c::label(ui.painter(),egui::Rect::from_min_size(ui.cursor().min,vec2(ui.available_width(),26.0)),title,19.0,look.text_primary());
                    ui.allocate_space(vec2(1.0,26.0));
                    ui.label(egui::RichText::new(if play.selected_instance.is_some(){"Сохранённая конфигурация · Vanilla"}else{"Быстрый запуск · Vanilla"}).size(12.0).color(look.text_tertiary()));
                }
                Action::Version=>{
                    let vs=&look.modules.version_button;
                    let mut style=(**ui.style()).clone();
                    style.visuals.widgets.inactive.weak_bg_fill=vs.fill_or(look.surface(3));
                    style.visuals.widgets.inactive.rounding=egui::Rounding::same(w.style.rounding.unwrap_or(vs.rounding_or(look.rounding)));
                    ui.set_style(style);
                    play::version_picker(ui,play,launch);
                }
                Action::Status=>play::status(ui,&look,launch),
                Action::LibraryList=>{
                    ui.horizontal(|ui|{
                        ui.label(egui::RichText::new(&w.label).strong().color(look.text_primary()));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
                            if ui.small_button("+").on_hover_text("Создать сборку").clicked(){intent=Some(Intent::Create);}
                        });
                    });
                    ui.add_space(4.0);
                    if instances::list(ui,&look,instances,play,w.page!=Some(Action::Library)){intent=Some(Intent::Home);}
                }
                Action::Account=>{
                    let (name,mode)=match auth.state(){
                        AuthState::SignedIn(a)=>(a.username,"Microsoft-аккаунт"),
                        _=>(if play.offline_name.trim().is_empty(){"Player".into()}else{play.offline_name.trim().into()},"Оффлайн-режим"),
                    };
                    ui.horizontal(|ui|{
                        let (r,_)=ui.allocate_exact_size(vec2(30.0,30.0),egui::Sense::hover());
                        skin::paint_head(ui.painter(),r,skin_mgr.texture().as_ref(),6.0);
                        ui.vertical(|ui|{
                            ui.add(egui::Label::new(egui::RichText::new(name).strong()).truncate());
                            ui.label(egui::RichText::new(mode).size(11.0).color(look.text_tertiary()));
                        });
                    });
                    if ui.button("Управлять профилем").clicked(){intent=Some(Intent::Profile);}
                    if !play.offline_name.trim().is_empty()||matches!(auth.state(),AuthState::SignedIn(_)){
                        let (r,_)=ui.allocate_exact_size(vec2(ui.available_width(),116.0),egui::Sense::hover());
                        skin::paint_paperdoll(ui.painter(),r,skin_mgr.texture().as_ref(),None,look.accent_color(),0.0);
                    }else{ui.label(egui::RichText::new("Войди через Microsoft или задай оффлайн-ник.").size(12.0).color(look.text_tertiary()));}
                }
                Action::Appearance=>settings::show_component(ui,settings,theme,0),
                Action::Atmosphere=>settings::show_component(ui,settings,theme,3),
                Action::ThemeJson=>settings::show_component(ui,settings,theme,2),
                Action::LegacyStyles=>settings::show_component(ui,settings,theme,1),
                _=>{}
            }
        });
    });
    intent
}