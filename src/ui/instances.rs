use std::path::PathBuf;
use std::io::Write;
use eframe::egui::{self,pos2,vec2,Rect,Stroke};
use serde::{Deserialize,Serialize};
use crate::launch::LaunchManager;
use crate::theme::ThemePreset;
use super::play::PlayState;
use super::components as c;

#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct Instance {pub name:String,pub version:String}

#[derive(Default)]
pub struct InstancesState {
    instances:Vec<Instance>,
    loaded:bool,
    pub creating:bool,
    new_name:String,
    new_version:Option<String>,
    search:String,
    error:Option<String>,
    delete_name:Option<String>,
}
impl InstancesState {
    pub fn ensure_loaded(&mut self){
        if !self.loaded {self.instances=load_all();self.loaded=true;}
    }
    pub fn items(&self)->&[Instance]{&self.instances}
    #[cfg(test)]
    pub fn fixture(items:Vec<Instance>)->Self{Self{instances:items,loaded:true,..Default::default()}}
}
fn instances_dir()->PathBuf{crate::launch::install::game_dir().join("instances")}
fn sanitize(name:&str)->String{
    let s:String=name.chars().map(|c|if c.is_alphanumeric()||c=='-'||c=='_'{c}else{'_'}).collect();
    if s.is_empty(){"instance".into()}else{s}
}
fn load_all()->Vec<Instance>{
    let mut out=Vec::new();
    let Ok(entries)=std::fs::read_dir(instances_dir())else{return out;};
    for entry in entries.flatten(){
        let path=entry.path();
        if path.extension().and_then(|x|x.to_str())!=Some("json"){continue;}
        if let Ok(s)=std::fs::read_to_string(path){
            if let Ok(inst)=serde_json::from_str::<Instance>(&s){out.push(inst);}
        }
    }
    out.sort_by_key(|a|a.name.to_lowercase());out
}
fn save(inst:&Instance)->Result<(),String>{
    let dir=instances_dir();
    std::fs::create_dir_all(&dir).map_err(|e|format!("Папка сборок: {e}"))?;
    let path=dir.join(format!("{}.json",sanitize(&inst.name)));
    let text=serde_json::to_vec_pretty(inst).map_err(|e|e.to_string())?;
    let mut file=std::fs::OpenOptions::new().write(true).create_new(true).open(&path)
        .map_err(|e|if e.kind()==std::io::ErrorKind::AlreadyExists{"Такое имя файла уже занято. Выбери другое имя сборки.".into()}else{format!("Сохранение: {e}")})?;
    if let Err(e)=file.write_all(&text){drop(file);let _=std::fs::remove_file(path);return Err(format!("Сохранение: {e}"));}
    Ok(())
}
fn remove(inst:&Instance)->Result<(),String>{
    std::fs::remove_file(instances_dir().join(format!("{}.json",sanitize(&inst.name)))).map_err(|e|format!("Удаление: {e}"))
}

pub fn compact_row(ui:&mut egui::Ui,theme:&ThemePreset,inst:&Instance,selected:bool,index:usize)->egui::Response{
    ui.push_id(("compact",index),|ui|{
        let (r,response)=ui.allocate_exact_size(vec2(ui.available_width(),64.0),egui::Sense::click());
        let fill=if response.hovered()||selected{theme.surface(3)}else{theme.surface(2)};
        ui.painter().rect_filled(r,10.0,fill);
        c::cube(ui.painter(),pos2(r.left()+28.0,r.center().y),10.0,if selected{theme.accent_color()}else{theme.text_body()});
        c::label(ui.painter(),Rect::from_min_size(r.min+vec2(52.0,8.0),vec2((r.width()-158.0).max(20.0),24.0)),&inst.name,14.0,theme.text_primary());
        c::label(ui.painter(),Rect::from_min_size(r.min+vec2(52.0,33.0),vec2((r.width()-158.0).max(20.0),20.0)),&format!("Minecraft {} · Vanilla",inst.version),12.0,theme.text_tertiary());
        c::label(ui.painter(),Rect::from_min_size(pos2(r.right()-92.0,r.center().y-12.0),vec2(74.0,24.0)),if selected{"Выбрана"}else{"Выбрать"},12.0,if selected{theme.accent_color()}else{theme.text_body()});
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    }).inner
}

pub fn show(ui:&mut egui::Ui,theme:&ThemePreset,state:&mut InstancesState,play:&mut PlayState,launch:&LaunchManager)->bool {
    state.ensure_loaded();
    launch.ensure_versions(ui.ctx().clone());
    c::page_title(ui,"Сборки","Твоя библиотека версий Minecraft",theme);
    let mut go_home=false;
    ui.horizontal(|ui|{
        let w=(ui.available_width()-164.0).max(100.0);
        ui.add_sized(vec2(w,40.0),egui::TextEdit::singleline(&mut state.search).hint_text("Найти сборку…"));
        if c::button(ui,"Создать сборку",vec2(148.0,40.0),theme,true).clicked(){state.creating=true;state.error=None;}
    });
    ui.add_space(16.0);
    if let Some(err)=&state.error{ui.colored_label(egui::Color32::from_rgb(238,145,153),err);ui.add_space(8.0);}
    egui::ScrollArea::vertical().id_salt("library_scroll").auto_shrink([false,false]).show(ui,|ui|{
        if state.creating {create_form(ui,theme,state,play,launch);ui.add_space(16.0);}
        if let Some(name)=state.delete_name.clone(){
            egui::Frame::none().fill(theme.surface(3)).inner_margin(16.0).rounding(10.0).show(ui,|ui|{
                ui.label(egui::RichText::new(format!("Удалить «{name}»?")).strong());
                ui.label("Будет удалена только запись сборки. Файлы игры останутся.");
                ui.add_space(8.0);
                ui.horizontal(|ui|{
                    if c::button(ui,"Удалить запись",vec2(148.0,40.0),theme,false).clicked(){
                        if let Some(i)=state.instances.iter().position(|x|x.name==name){
                            match remove(&state.instances[i]){
                                Ok(())=>{state.instances.remove(i);if play.selected_instance.as_deref()==Some(name.as_str()){play.selected_instance=None;}state.delete_name=None;}
                                Err(e)=>state.error=Some(e),
                            }
                        }
                    }
                    if c::button(ui,"Отмена",vec2(100.0,40.0),theme,false).clicked(){state.delete_name=None;}
                });
            });
            ui.add_space(12.0);
        }
        let search=state.search.to_lowercase();
        let items:Vec<_>=state.instances.iter().enumerate().filter(|(_,x)|x.name.to_lowercase().contains(&search)||x.version.to_lowercase().contains(&search)).map(|(i,x)|(i,x.clone())).collect();
        if items.is_empty(){
            c::empty(ui,if state.instances.is_empty(){"Здесь будут твои сборки"}else{"Ничего не найдено"},if state.instances.is_empty(){"Создай первую сборку: дай ей имя и выбери версию игры."}else{"Попробуй другое имя или версию Minecraft."},theme);
        }
        for (i,inst) in items{
            ui.push_id(("library_item",i),|ui|{
                let selected=play.selected_instance.as_deref()==Some(inst.name.as_str());
                let wide=ui.available_width()>=640.0;
                egui::Frame::none().fill(theme.surface(2)).rounding(12.0).inner_margin(16.0).show(ui,|ui|{
                    ui.set_min_width(ui.available_width());
                    ui.horizontal(|ui|{
                        let (r,_)=ui.allocate_exact_size(vec2(42.0,42.0),egui::Sense::hover());
                        ui.painter().rect_filled(r,10.0,theme.surface(3));
                        c::cube(ui.painter(),r.center(),12.0,if selected{theme.accent_color()}else{theme.text_body()});
                        ui.add_space(8.0);
                        let text_w=(ui.available_width()-if wide{244.0}else{0.0}).max(80.0);
                        ui.allocate_ui_with_layout(vec2(text_w,42.0),egui::Layout::top_down(egui::Align::Min),|ui|{
                            ui.add(egui::Label::new(egui::RichText::new(&inst.name).size(15.0).strong().color(theme.text_primary())).truncate());
                            ui.label(egui::RichText::new(format!("Minecraft {} · Vanilla{}",inst.version,if selected{" · Выбрана"}else{""})).size(12.0).color(theme.text_tertiary()));
                        });
                        if wide {
                            if c::button(ui,if selected{"К запуску"}else{"Выбрать"},vec2(112.0,40.0),theme,false).clicked(){
                                play.selected_instance=Some(inst.name.clone());play.selected_version=Some(inst.version.clone());go_home=true;
                            }
                            if c::button(ui,"Удалить",vec2(92.0,40.0),theme,false).clicked(){state.delete_name=Some(inst.name.clone());}
                        }
                    });
                    if !wide {
                        ui.add_space(12.0);
                        ui.horizontal(|ui|{
                            if c::button(ui,if selected{"К запуску"}else{"Выбрать"},vec2(112.0,40.0),theme,false).clicked(){
                                play.selected_instance=Some(inst.name.clone());play.selected_version=Some(inst.version.clone());go_home=true;
                            }
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
                                if c::button(ui,"Удалить",vec2(92.0,40.0),theme,false).clicked(){state.delete_name=Some(inst.name.clone());}
                            });
                        });
                    }
                });
                ui.add_space(10.0);
            });
        }
        ui.add_space(12.0);
        ui.label(egui::RichText::new("Пока сборка хранит имя и версию. Изолированные папки, моды и загрузчики ещё не реализованы.").size(12.0).color(theme.text_tertiary()));
    });
    go_home
}

fn create_form(ui:&mut egui::Ui,theme:&ThemePreset,state:&mut InstancesState,play:&mut PlayState,launch:&LaunchManager){
    egui::Frame::none().fill(theme.surface(3)).stroke(Stroke::new(1.0_f32,theme.surface(4))).rounding(12.0).inner_margin(20.0).show(ui,|ui|{
        ui.label(egui::RichText::new("Новая сборка").size(18.0).strong());
        ui.add_space(12.0);ui.label("Название");
        ui.add_sized(vec2(ui.available_width(),40.0),egui::TextEdit::singleline(&mut state.new_name).hint_text("Например, Выживание").char_limit(80));
        ui.add_space(8.0);ui.label("Версия Minecraft");
        match launch.versions(){
            None=>{ui.spinner();}
            Some(Err(_))=>{if ui.button("Повторить загрузку версий").clicked(){launch.retry_versions(ui.ctx().clone());}}
            Some(Ok(v))=>{
                let releases:Vec<_>=v.iter().filter(|x|x.kind=="release").collect();
                if state.new_version.is_none(){state.new_version=releases.first().map(|x|x.id.clone());}
                egui::ComboBox::from_id_salt("create_version").width(200.0).height(240.0).selected_text(state.new_version.as_deref().unwrap_or("Нет версий")).show_ui(ui,|ui|{
                    for v in releases{ui.selectable_value(&mut state.new_version,Some(v.id.clone()),&v.id);}
                });
            }
        }
        ui.add_space(16.0);
        ui.horizontal(|ui|{
            let ready=!state.new_name.trim().is_empty()&&state.new_version.is_some();
            ui.add_enabled_ui(ready,|ui|{
                if c::button(ui,"Создать",vec2(120.0,40.0),theme,true).clicked(){
                    let name=state.new_name.trim().to_string();
                    if state.instances.iter().any(|x|x.name.to_lowercase()==name.to_lowercase()){
                        state.error=Some("Сборка с таким именем уже есть.".into());
                    }else if let Some(version)=state.new_version.clone(){
                        let inst=Instance{name:name.clone(),version:version.clone()};
                        match save(&inst){
                            Ok(())=>{state.instances.push(inst);state.instances.sort_by_key(|x|x.name.to_lowercase());play.selected_instance=Some(name);play.selected_version=Some(version);state.creating=false;state.new_name.clear();state.error=None;}
                            Err(e)=>state.error=Some(e),
                        }
                    }
                }
            });
            if c::button(ui,"Отмена",vec2(100.0,40.0),theme,false).clicked(){state.creating=false;state.error=None;}
        });
    });
}

#[cfg(test)]
mod tests{
    use super::*;
    #[test]
    fn filename_cannot_escape_instances(){assert!(!sanitize("../a/b").contains('/'));assert!(!sanitize("../a/b").contains('.'));}
}