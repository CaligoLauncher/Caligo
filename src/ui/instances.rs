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

/// Selection only; launching is a separate explicit action.
pub fn search(ui:&mut egui::Ui,state:&mut InstancesState) {
    ui.add_sized([ui.available_width(),36.0],egui::TextEdit::singleline(&mut state.search).hint_text("Найти сборку…"));
}
pub fn request_create(state:&mut InstancesState){state.creating=true;state.error=None;}
pub fn list(ui:&mut egui::Ui,theme:&ThemePreset,state:&mut InstancesState,play:&mut PlayState,compact:bool)->bool {
    let mut go_home=false;
    let search=if compact{String::new()}else{state.search.to_lowercase()};
    let items:Vec<_>=state.instances.iter().filter(|x|x.name.to_lowercase().contains(&search)||x.version.to_lowercase().contains(&search)).cloned().collect();
    if items.is_empty(){
        ui.add_space(8.0);
        ui.label(egui::RichText::new(if state.instances.is_empty(){"Нет сохранённых сборок"}else{"Ничего не найдено"}).strong());
        ui.label(egui::RichText::new(if state.instances.is_empty(){"Сохрани имя и версию, чтобы быстро вернуться к ним."}else{"Попробуй другое имя или версию."}).size(12.0).color(theme.text_tertiary()));
        if state.instances.is_empty()&&ui.button("Создать сборку").clicked(){request_create(state);}
    }
    for (i,inst) in items.iter().enumerate(){
        ui.push_id(("instance",i),|ui|{
            let selected=play.selected_instance.as_deref()==Some(inst.name.as_str());
            let width=ui.available_width();
            let (r,response)=ui.allocate_exact_size(vec2(width,60.0),egui::Sense::click());
            let fill=if response.hovered()||selected{theme.surface(3)}else{egui::Color32::TRANSPARENT};
            ui.painter().rect_filled(r,6.0,fill);
            c::cube(ui.painter(),pos2(r.left()+20.0,r.center().y),9.0,if selected{theme.accent_color()}else{theme.text_tertiary()});
            c::label(ui.painter(),Rect::from_min_size(r.min+vec2(42.0,6.0),vec2((width-84.0).max(1.0),24.0)),&inst.name,14.0,theme.text_primary());
            c::label(ui.painter(),Rect::from_min_size(r.min+vec2(42.0,30.0),vec2((width-52.0).max(1.0),22.0)),&format!("{} · Vanilla{}",inst.version,if selected{" · Выбрана"}else{""}),12.0,theme.text_tertiary());
            if response.clicked(){
                play.selected_instance=Some(inst.name.clone());play.selected_version=Some(inst.version.clone());
                go_home=!compact;
            }
            response.context_menu(|ui|{
                if ui.button("Выбрать для запуска").clicked(){play.selected_instance=Some(inst.name.clone());play.selected_version=Some(inst.version.clone());go_home=!compact;ui.close_menu();}
                if ui.button("Удалить запись…").clicked(){state.delete_name=Some(inst.name.clone());ui.close_menu();}
            });
            if !compact {
                let mr=Rect::from_min_size(pos2(r.right()-34.0,r.top()+6.0),vec2(30.0,28.0));
                let mut menu=ui.new_child(egui::UiBuilder::new().id_salt("row_menu").max_rect(mr));
                menu.menu_button("…",|ui|{
                    if ui.button("Удалить запись…").clicked(){state.delete_name=Some(inst.name.clone());ui.close_menu();}
                });
            }
            ui.add_space(4.0);
        });
    }
    go_home
}
pub fn dialogs(ctx:&egui::Context,theme:&ThemePreset,state:&mut InstancesState,play:&mut PlayState,launch:&LaunchManager){
    if ctx.input(|i|i.key_pressed(egui::Key::Escape)){state.creating=false;state.delete_name=None;}
    if state.creating {
        egui::Window::new("Новая сборка").id(egui::Id::new("create_instance")).order(egui::Order::Foreground)
            .collapsible(false).resizable(false).title_bar(false).default_width(400.0)
            .default_pos(egui::pos2((ctx.screen_rect().width()-416.0).max(24.0)/2.0,64.0))
            .constrain_to(egui::Rect::from_min_max(egui::pos2(12.0,56.0),ctx.screen_rect().max-egui::vec2(12.0,12.0))).max_height((ctx.screen_rect().height()-100.0).max(200.0)).vscroll(true).show(ctx,|ui|{
                create_form(ui,theme,state,play,launch);
                if let Some(err)=&state.error{ui.colored_label(egui::Color32::from_rgb(240,145,145),err);}
                ui.label(egui::RichText::new("Сохраняются имя и версия. Игровая папка пока общая; моды и загрузчики не добавляются.").size(12.0).color(theme.text_tertiary()));
            });
    }
    if let Some(name)=state.delete_name.clone(){
        egui::Window::new("Удалить запись сборки?").id(egui::Id::new("delete_instance")).order(egui::Order::Foreground)
            .collapsible(false).resizable(false).default_width(360.0).constrain_to(ctx.screen_rect().shrink(12.0)).show(ctx,|ui|{
                ui.label(egui::RichText::new(&name).strong());
                ui.label("Удалится только JSON-запись. Файлы Minecraft останутся.");
                ui.horizontal(|ui|{
                    if ui.button("Удалить запись").clicked(){
                        if let Some(i)=state.instances.iter().position(|x|x.name==name){
                            match remove(&state.instances[i]){
                                Ok(())=>{state.instances.remove(i);if play.selected_instance.as_deref()==Some(name.as_str()){play.selected_instance=None;}state.delete_name=None;state.error=None;}
                                Err(e)=>state.error=Some(e),
                            }
                        }
                    }
                    if ui.button("Отмена").clicked(){state.delete_name=None;state.error=None;}
                });
                if let Some(err)=&state.error{ui.colored_label(egui::Color32::from_rgb(240,145,145),err);}
            });
    }
}
fn create_form(ui:&mut egui::Ui,theme:&ThemePreset,state:&mut InstancesState,play:&mut PlayState,launch:&LaunchManager){
    egui::Frame::none().inner_margin(12.0).show(ui,|ui|{
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