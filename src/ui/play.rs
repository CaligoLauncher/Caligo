use eframe::egui::{self,vec2};
use crate::auth::{AuthManager,AuthState};
use crate::launch::{LaunchManager,LaunchState};
use crate::launch::manifest::ManifestVersion;
use crate::launch::run::LaunchProfile;
use crate::theme::ThemePreset;
use crate::composition::Widget;
use super::components as c;

#[derive(Default)]
pub struct PlayState {
    pub selected_version:Option<String>,
    pub selected_instance:Option<String>,
    pub offline_name:String,
    pub version_search:String,
}

/// One source of launch truth, shared by all component instances.
pub fn selected_version(play:&PlayState,launch:&LaunchManager)->Option<ManifestVersion>{
    let versions=launch.versions()?.ok()?;
    match &play.selected_version {
        Some(id)=>versions.into_iter().find(|v|v.kind=="release"&&v.id==*id),
        None=>versions.into_iter().find(|v|v.kind=="release"),
    }
}
pub fn profile_for(auth:&AuthManager,play:&PlayState)->LaunchProfile {
    match auth.state(){
        AuthState::SignedIn(a)=>LaunchProfile{username:a.username,uuid:a.uuid,access_token:a.access_token},
        _=>LaunchProfile{
            username:if play.offline_name.trim().is_empty(){"Player".into()}else{play.offline_name.trim().into()},
            uuid:"00000000-0000-0000-0000-000000000000".into(),access_token:"0".into(),
        }
    }
}
pub fn version_picker(ui:&mut egui::Ui,play:&mut PlayState,launch:&LaunchManager) {
    match launch.versions() {
        None=>{ui.horizontal(|ui|{ui.spinner();ui.label("Загрузка версий…");});}
        Some(Err(e))=>{
            ui.label("Не удалось получить версии");
            if ui.button("Повторить").on_hover_text(e).clicked(){launch.retry_versions(ui.ctx().clone());}
        }
        Some(Ok(versions))=>{
            let releases:Vec<_>=versions.iter().filter(|v|v.kind=="release").collect();
            let selected=play.selected_version.clone().or_else(||releases.first().map(|v|v.id.clone()));
            let text=selected.as_ref().map(|s|format!("Minecraft {s}")).unwrap_or_else(||"Нет версий".into());
            let width=ui.available_width().max(40.0);
            egui::ComboBox::from_id_salt("version").width((width-8.0).max(32.0)).selected_text(text).height(230.0).show_ui(ui,|ui|{
                ui.add(egui::TextEdit::singleline(&mut play.version_search).hint_text("Найти версию…").desired_width((width-24.0).max(80.0)));
                let search=play.version_search.to_lowercase();
                for v in releases.iter().filter(|v|v.id.to_lowercase().contains(&search)) {
                    if ui.selectable_label(selected.as_deref()==Some(v.id.as_str()),&v.id).clicked(){
                        play.selected_version=Some(v.id.clone());play.selected_instance=None;ui.close_menu();
                    }
                }
            });
        }
    }
}
pub fn launch_button(ui:&mut egui::Ui,theme:&ThemePreset,w:&Widget,play:&PlayState,launch:&LaunchManager)->bool {
    let state=launch.state();
    let busy=matches!(state,LaunchState::Preparing(_)|LaunchState::Running);
    let enabled=selected_version(play,launch).is_some()&&!busy;
    let label=match state {LaunchState::Preparing(_)=>"Подготовка…",LaunchState::Running=>"Игра запущена",_=>w.label.as_str()};
    let fill=w.style.fill.map(crate::theme::color_arr).unwrap_or(theme.modules.play_button.fill_or(theme.accent_color()));
    ui.add_enabled(enabled,egui::Button::new(egui::RichText::new(label).size(15.0).strong().color(c::contrast(fill)))
        .fill(fill).rounding(w.style.rounding.unwrap_or(theme.modules.play_button.rounding_or(theme.rounding)))
        .stroke(theme.modules.play_button.border_or(egui::Stroke::NONE))
        .min_size(vec2(ui.available_width(),ui.available_height().max(32.0)))).on_hover_text(if enabled{"Запустить выбранную версию Minecraft"}else if busy{"Дождись завершения текущего запуска"}else{"Нужна доступная версия Minecraft"}).clicked()
}
pub fn status(ui:&mut egui::Ui,theme:&ThemePreset,launch:&LaunchManager){
    match launch.state(){
        LaunchState::Idle=>{ui.label(egui::RichText::new("Игра не запущена").color(theme.text_tertiary()));}
        LaunchState::Preparing(step)=>{ui.horizontal_wrapped(|ui|{ui.spinner();ui.label(step);});}
        LaunchState::Running=>{ui.label(egui::RichText::new("Minecraft работает").color(theme.accent_color()));}
        LaunchState::Exited(code)=>{ui.label(format!("Игра завершена · код {code}"));}
        LaunchState::Failed(err)=>{
            ui.colored_label(egui::Color32::from_rgb(240,145,145),"Не удалось запустить игру");
            ui.label(&err);
            if ui.button("Скопировать ошибку").clicked(){ui.ctx().output_mut(|o|o.copied_text=err);}
        }
    }
}