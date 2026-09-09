//! Application shell rebuilt around a single scene, not fixed pages under an editor.
use eframe::egui::{self,pos2,vec2,Color32,Rect,Stroke};
use crate::{auth::{AuthManager,AuthState},launch::{LaunchManager,LaunchState},skin::{self,SkinManager}};
use crate::{canvas_model::{Id,Kind,Page},canvas_editor::Studio,canvas_paint::{self as paint,Wallpaper},game_session::Session,library::Library};
pub type Tab=Page;
#[derive(Clone,Copy,PartialEq)]
enum Popup{Profile,Versions,Search,Create,Appearance,Background,Json,Status,Delete(usize)}
pub struct CaligoApp {
    pub tab:Page,pub studio:Studio,pub auth:AuthManager,pub launch:LaunchManager,pub session:Session,
    pub library:Library,skin:SkinManager,wallpaper:Wallpaper,popup:Option<Popup>,
    search:String,version_search:String,new_name:String,new_version:Option<String>,json:String,error:Option<String>,
    test:bool,pub node_rects:Vec<(Id,Rect)>,pub controls:Vec<(&'static str,Rect)>,
    pub canvas_bounds:Rect,
}
impl CaligoApp{
    pub fn new(cc:&eframe::CreationContext<'_>)->Self{let mut a=Self::create(&cc.egui_ctx,false);a.studio=Studio::load();a}
    fn create(ctx:&egui::Context,test:bool)->Self{
        paint::setup(ctx);
        Self{tab:Page::Home,studio:Studio::default(),auth:Default::default(),launch:Default::default(),session:Default::default(),library:Default::default(),skin:Default::default(),wallpaper:Wallpaper::load(ctx,test),popup:None,search:String::new(),version_search:String::new(),new_name:String::new(),new_version:None,json:String::new(),error:None,test,node_rects:vec![],controls:vec![],canvas_bounds:Rect::NOTHING}
    }
    pub fn render(&mut self,ctx:&egui::Context){
        if !self.test {self.skin.ensure(ctx,self.session.skin_key(&self.auth));self.launch.ensure_versions(ctx.clone());self.library.ensure_loaded();}
        if ctx.input_mut(|i|i.consume_key(egui::Modifiers::CTRL|egui::Modifiers::SHIFT,egui::Key::E)){
            self.studio.page=self.tab;self.studio.begin();self.popup=None;
        }
        self.wallpaper.paint(ctx);self.controls.clear();
        self.titlebar(ctx);
        let bounds=ctx.available_rect();self.canvas_bounds=bounds;
        let editing=self.studio.active;
        if editing{self.popup=None;self.studio.input(ctx,bounds);}
        else{self.studio.page=self.tab;}
        let page=if editing{self.studio.page}else{self.tab};
        self.node_rects.clear();
        let mut clicked=None;let mut picked=None;
        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx,|ui|{
            for id in self.studio.scene.order(){
                if !self.studio.scene.shown(id,page){continue}
                let n=self.studio.scene.node(id).unwrap().clone();
                let Some(r)=self.studio.scene.screen_rect(id,bounds)else{continue};
                self.node_rects.push((id,r));
                let p=ui.painter().with_clip_rect(r.intersect(bounds));
                self.wallpaper.surface(&p,r,&n.style);
                let active_nav=matches!((n.kind,page),(Kind::Home,Page::Home)|(Kind::Library,Page::Library)|(Kind::Settings,Page::Settings));
                let enabled=match n.kind{Kind::Play=>self.session.version(&self.launch).is_some()&&!matches!(self.launch.state(),LaunchState::Preparing(_)|LaunchState::Running),_=>true};
                // In editor mode no widget is built, focused or fired below the overlay.
                // Exactly the same painter/style is used, without disabled-grey fading.
                let resp=if !editing&&self.popup.is_none()&&n.kind.button(){
                    Some(ui.interact(r,egui::Id::new(("shell-node",id)),egui::Sense::click()))
                }else{None};
                if active_nav{p.rect_filled(r,n.style.radius,Color32::from_white_alpha(20));}
                if resp.as_ref().is_some_and(|r|r.hovered())&&enabled{p.rect_filled(r,n.style.radius,Color32::from_white_alpha(15));ctx.set_cursor_icon(egui::CursorIcon::PointingHand);}
                let ink=paint::rgba(n.style.ink);
                let label=match n.kind{
                    Kind::Selection=>self.session.instance.clone().unwrap_or("Minecraft".into()),
                    Kind::Version=>self.session.version(&self.launch).map(|v|format!("Minecraft {}",v.id)).unwrap_or("Выбрать версию".into()),
                    Kind::Search=>if self.search.is_empty(){"Найти сборку…".into()}else{self.search.clone()},
                    Kind::Status=>match self.launch.state(){LaunchState::Idle=>"Готов к игре".into(),LaunchState::Preparing(s)=>s,LaunchState::Running=>"Minecraft работает".into(),LaunchState::Exited(c)=>format!("Игра завершена · {c}"),LaunchState::Failed(_)=>"Ошибка запуска — открыть подробности".into()},
                    Kind::Play=>match self.launch.state(){LaunchState::Preparing(_)=>"Подготовка…".into(),LaunchState::Running=>"Игра запущена".into(),_=>n.name.clone()},
                    _=>n.name.clone()
                };
                match n.kind{
                    Kind::Character=>{
                        let draw=Rect::from_center_size(r.center(),vec2(r.width()*1.6,r.height()));
                        skin::paint_paperdoll(&p,draw,self.skin.texture().as_ref(),None,paint::rgba(self.studio.scene.accent),0.0);
                    },
                    Kind::Panel=>{},
                    Kind::List=>{
                        let inner=r.shrink(14.0);
                        if editing{
                            let items:Vec<_>=self.library.items.iter().filter(|x|x.name.to_lowercase().contains(&self.search.to_lowercase())||x.version.contains(&self.search)).collect();
                            if items.is_empty(){
                                paint::text(&p,Rect::from_min_size(inner.min,vec2(inner.width(),28.0)),if self.library.items.is_empty(){"Здесь будут твои сборки"}else{"Ничего не найдено"},18.0,ink,false);
                                paint::text(&p,Rect::from_min_size(inner.min+vec2(0.0,36.0),vec2(inner.width(),26.0)),"Сборка пока сохраняет имя и версию Minecraft.",13.0,ink,false);
                            }
                            for (i,item) in items.iter().take((inner.height()/64.0).ceil() as usize).enumerate(){
                                let row=Rect::from_min_size(inner.min+vec2(0.0,i as f32*64.0),vec2(inner.width(),60.0));
                                paint::text(&p,Rect::from_min_size(row.min+vec2(10.0,2.0),vec2((row.width()-60.0).max(1.0),30.0)),&item.name,16.0,ink,false);
                                paint::text(&p,Rect::from_min_size(row.min+vec2(10.0,31.0),vec2((row.width()-60.0).max(1.0),22.0)),&format!("{}  ·  Vanilla",item.version),12.0,Color32::from_rgb(168,191,208),false);
                            }
                        }else{
                            let mut child=ui.new_child(egui::UiBuilder::new().id_salt(("library-list",id)).max_rect(inner));
                            child.set_clip_rect(inner.intersect(bounds));
                            child.add_enabled_ui(self.popup.is_none(),|ui|{
                                let matching:Vec<_>=self.library.items.iter().enumerate().filter(|(_,x)|x.name.to_lowercase().contains(&self.search.to_lowercase())||x.version.contains(&self.search)).map(|(i,x)|(i,x.clone())).collect();
                                if matching.is_empty(){
                                    ui.label(egui::RichText::new(if self.library.items.is_empty(){"Здесь будут твои сборки"}else{"Ничего не найдено"}).size(18.0));
                                    ui.label("Сборка пока сохраняет имя и версию Minecraft.");
                                    if self.library.items.is_empty()&&ui.button("Создать первую").clicked(){self.popup=Some(Popup::Create);}
                                }else{
                                    egui::ScrollArea::vertical().id_salt(("library-scroll",id)).max_height(inner.height()).show_rows(ui,64.0,matching.len(),|ui,range|{
                                        for index in range{
                                            let (source,item)=&matching[index];
                                            let(w,h)=(ui.available_width(),60.0);
                                            let (row,response)=ui.allocate_exact_size(vec2(w,h),egui::Sense::click());
                                            if response.hovered(){ui.painter().rect_filled(row,8.0,Color32::from_white_alpha(12));}
                                            paint::text(ui.painter(),Rect::from_min_size(row.min+vec2(10.0,2.0),vec2((w-60.0).max(1.0),30.0)),&item.name,16.0,ink,false);
                                            paint::text(ui.painter(),Rect::from_min_size(row.min+vec2(10.0,31.0),vec2((w-60.0).max(1.0),22.0)),&format!("{}  ·  Vanilla",item.version),12.0,Color32::from_rgb(168,191,208),false);
                                            if response.clicked(){picked=Some(*source);}
                                            response.context_menu(|ui|{if ui.button("Удалить запись…").clicked(){self.popup=Some(Popup::Delete(*source));ui.close_menu();}});
                                            let more=ui.put(Rect::from_min_size(pos2(row.right()-38.0,row.top()+12.0),vec2(30.0,28.0)),egui::Button::new("…"));
                                            if more.clicked(){self.popup=Some(Popup::Delete(*source));picked=None;}
                                        }
                                    });
                                }
                            });
                        }
                    },
                    _=>{
                        let font=if n.kind==Kind::Status{n.style.font.min(12.0)}else{n.style.font};
                        paint::text(&p,r.shrink2(vec2(if n.kind.button(){10.0}else{0.0},0.0)),&label,font,if enabled||editing{ink}else{ink.gamma_multiply(0.55)},n.kind.button());
                    }
                }
                if resp.is_some_and(|r|r.clicked())&&enabled{clicked=Some(n.kind);}
                if n.kind==Kind::Status&&!editing&&self.popup.is_none()&&ui.interact(r,egui::Id::new(("status",id)),egui::Sense::click()).clicked(){clicked=Some(Kind::Status);}
            }
        });
        if !editing{
            if let Some(index)=picked{if let Some(x)=self.library.items.get(index){self.session.instance=Some(x.name.clone());self.session.version=Some(x.version.clone());self.tab=Page::Home;}}
            if let Some(kind)=clicked{self.dispatch(ctx,kind);}
            self.dialogs(ctx);
            if let Some(error)=&self.studio.error{
                egui::Window::new("Интерфейс не загружен").collapsible(false).show(ctx,|ui|{ui.label(error);ui.label("Сохранение заблокировано. Исходные файлы не изменены.");});
            }
        }else{self.studio.paint(ctx,bounds);}
    }
    fn dispatch(&mut self,ctx:&egui::Context,k:Kind){
        if self.studio.active{return}
        match k{
            Kind::Home=>self.tab=Page::Home,Kind::Library=>self.tab=Page::Library,Kind::Settings=>self.tab=Page::Settings,
            Kind::Play=>if !self.test&&!matches!(self.launch.state(),LaunchState::Preparing(_)|LaunchState::Running){if let Some(v)=self.session.version(&self.launch){self.launch.launch(ctx.clone(),v,self.session.profile(&self.auth));}},
            Kind::Profile=>self.popup=Some(Popup::Profile),Kind::Version=>self.popup=Some(Popup::Versions),
            Kind::Search=>self.popup=Some(Popup::Search),Kind::Create=>self.popup=Some(Popup::Create),
            Kind::Appearance=>self.popup=Some(Popup::Appearance),Kind::Background=>self.popup=Some(Popup::Background),
            Kind::Status=>self.popup=Some(Popup::Status),
            Kind::Json=>{self.json=serde_json::to_string_pretty(&self.studio.scene).unwrap_or_default();self.popup=Some(Popup::Json);},
            _=>{}
        }
    }
    fn titlebar(&mut self,ctx:&egui::Context){
        egui::TopBottomPanel::top("shell-window-bar").exact_height(44.0).show_separator_line(false).frame(egui::Frame::none()).show(ctx,|ui|{
            let r=ui.max_rect();ui.painter().rect_filled(r,0.0,Color32::from_black_alpha(24));
            let drag=Rect::from_min_max(r.min,pos2((r.right()-460.0).max(r.left()+100.0),r.bottom()));
            let response=ui.interact(drag,egui::Id::new("window-drag"),egui::Sense::click_and_drag());
            if response.drag_started(){ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);}
            if response.double_clicked(){ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!ctx.input(|i|i.viewport().maximized.unwrap_or(false))));}
            paint::text(ui.painter(),Rect::from_min_size(r.min+vec2(20.0,0.0),vec2(100.0,44.0)),"Caligo",17.0,Color32::WHITE,false);
            for(i,label)in["×","□","−"].iter().enumerate(){
                let rect=Rect::from_min_size(pos2(r.right()-46.0*(i as f32+1.0),r.top()),vec2(46.0,44.0));
                let res=ui.interact(rect,egui::Id::new(("window",i)),egui::Sense::click());
                if res.hovered(){ui.painter().rect_filled(rect,0.0,if i==0{Color32::from_rgb(232,17,35)}else{Color32::from_white_alpha(18)});}
                paint::text(ui.painter(),rect,label,19.0,Color32::WHITE,true);
                if res.clicked(){match i{0=>if self.studio.active&&self.studio.dirty(){self.studio.exit_question=true}else{ctx.send_viewport_cmd(egui::ViewportCommand::Close)},1=>ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!ctx.input(|i|i.viewport().maximized.unwrap_or(false)))),_=>ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true))}}
            }
            let profile=Rect::from_min_size(pos2(r.right()-274.0,r.top()+7.0),vec2(122.0,30.0));
            let name=match self.auth.state(){AuthState::SignedIn(a)=>a.username,_=>if self.session.offline_name.is_empty(){"Профиль".into()}else{self.session.offline_name.clone()}};
            if ui.put(profile,egui::Button::new(name).wrap_mode(egui::TextWrapMode::Truncate).fill(Color32::from_white_alpha(10)).stroke(Stroke::NONE).rounding(8.0)).clicked()&&!self.studio.active{self.popup=Some(Popup::Profile);}
            let edit=Rect::from_min_size(pos2(r.right()-390.0,r.top()+7.0),vec2(104.0,30.0));
            self.controls.push(("editor",edit));
            if ui.put(edit,egui::Button::new("Редактор").wrap_mode(egui::TextWrapMode::Truncate).fill(Color32::TRANSPARENT)).clicked()&&!self.studio.active{self.studio.page=self.tab;self.studio.begin();self.popup=None;}
        });
    }
    fn dialogs(&mut self,ctx:&egui::Context){
        let Some(popup)=self.popup else{return};
        if ctx.input(|i|i.key_pressed(egui::Key::Escape)){self.popup=None;self.error=None;return}
        let title=match popup{Popup::Profile=>"Профиль",Popup::Versions=>"Версия Minecraft",Popup::Search=>"Поиск сборок",Popup::Create=>"Новая сборка",Popup::Appearance=>"Акцент интерфейса",Popup::Background=>"Свой фон",Popup::Json=>"Документ интерфейса",Popup::Status=>"Состояние Minecraft",Popup::Delete(_)=>"Удалить запись?"};
        let mut open=true;
        let w=egui::Window::new(title).id(egui::Id::new("shell-dialog")).order(egui::Order::Foreground).open(&mut open)
            .collapsible(false).resizable(false).default_width(360.0).max_height((ctx.screen_rect().height()-140.0).max(180.0)).vscroll(true).constrain_to(ctx.screen_rect().shrink(20.0));
        w.show(ctx,|ui|{
            match popup{
                Popup::Profile=>self.profile(ui),
                Popup::Versions=>{
                    ui.add(egui::TextEdit::singleline(&mut self.version_search).hint_text("Поиск версии"));
                    match self.launch.versions(){
                        None=>{ui.spinner();ui.label("Получаем список версий…");},
                        Some(Err(e))=>{ui.label(e);if ui.button("Повторить").clicked(){self.launch.retry_versions(ctx.clone());}},
                        Some(Ok(v))=>{for v in v.iter().filter(|v|v.kind=="release"&&v.id.contains(&self.version_search)){
                            if ui.selectable_label(self.session.version.as_ref()==Some(&v.id),&v.id).clicked(){self.session.version=Some(v.id.clone());self.session.instance=None;self.popup=None;}
                        }},
                    }
                },
                Popup::Search=>{ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Имя или версия"));if ui.button("Показать").clicked(){self.popup=None;}},
                Popup::Create=>{
                    ui.label("Имя");ui.add(egui::TextEdit::singleline(&mut self.new_name).char_limit(80).hint_text("Например, Выживание"));
                    ui.label("Версия");
                    match self.launch.versions(){
                        Some(Ok(v))=>{
                            let releases:Vec<_>=v.iter().filter(|v|v.kind=="release").collect();
                            if self.new_version.is_none(){self.new_version=releases.first().map(|v|v.id.clone());}
                            egui::ComboBox::from_id_salt("create-version").selected_text(self.new_version.as_deref().unwrap_or("Нет версий")).show_ui(ui,|ui|{for v in releases{ui.selectable_value(&mut self.new_version,Some(v.id.clone()),&v.id);}});
                        },
                        _=>{ui.label("Список версий недоступен");if ui.button("Загрузить версии").clicked(){self.launch.retry_versions(ctx.clone());}},
                    }
                    ui.label("Сохраняются имя и версия. Игровая папка общая; моды и загрузчики пока не добавляются.");
                    if ui.add_enabled(!self.new_name.trim().is_empty()&&self.new_version.is_some(),egui::Button::new("Создать")).clicked(){
                        if let Some(v)=&self.new_version{match self.library.create(&self.new_name,v){
                            Ok(())=>{self.session.instance=Some(self.new_name.trim().into());self.session.version=Some(v.clone());self.new_name.clear();self.error=None;self.popup=None;self.tab=Page::Home;},
                            Err(e)=>self.error=Some(e),
                        }}
                    }
                },
                Popup::Delete(index)=>{
                    if let Some(item)=self.library.items.get(index).cloned(){
                        ui.label(egui::RichText::new(&item.name).strong());ui.label("Удалится только JSON-запись. Файлы Minecraft останутся.");
                        if ui.button("Удалить запись").clicked(){match self.library.delete(index){Ok(())=>{if self.session.instance.as_ref()==Some(&item.name){self.session.instance=None;}self.popup=None;self.error=None;},Err(e)=>self.error=Some(e)}}
                    }else{ui.label("Запись уже отсутствует");}
                    if ui.button("Отмена").clicked(){self.popup=None;self.error=None;}
                },
                Popup::Appearance=>{
                    ui.color_edit_button_srgba_unmultiplied(&mut self.studio.scene.accent);
                    ui.label("Акцент редактора и новых элементов. Уже настроенные элементы не перекрашиваются.");
                    if ui.button("Сохранить").clicked(){if self.studio.save(){self.popup=None;}}
                },
                Popup::Background=>{
                    ui.label("Положи background.png, background.jpg или background.jpeg в каталог данных Caligo.");
                    ui.monospace(crate::launch::install::game_dir().display().to_string());
                    if ui.button("Перечитать фон").clicked(){self.wallpaper=Wallpaper::load(ctx,self.test);}
                    ui.label("Размытие отдельных поверхностей — кэш обоев, не размытие всего окна.");
                },
                Popup::Json=>{
                    ui.label("Раскладка, страницы и локальные стили. Без аккаунтов и токенов.");
                    ui.add(egui::TextEdit::multiline(&mut self.json).code_editor().desired_rows(8).desired_width(340.0));
                    if ui.button("Скопировать JSON").clicked(){ctx.output_mut(|o|o.copied_text=self.json.clone());}
                    if ui.button("Открыть как черновик в редакторе").clicked(){
                        match serde_json::from_str::<crate::canvas_model::Scene>(&self.json){
                            Ok(s)=>match s.validate(){Ok(())=>{self.studio.page=self.tab;self.studio.begin();let old=self.studio.scene.clone();self.studio.scene=s;self.studio.record(old);self.popup=None;self.error=None;},Err(e)=>self.error=Some(e)},
                            Err(e)=>self.error=Some(e.to_string()),
                        }
                    }
                },
                Popup::Status=>{ui.label(match self.launch.state(){LaunchState::Idle=>"Игра не запущена".into(),LaunchState::Preparing(s)=>s,LaunchState::Running=>"Minecraft работает".into(),LaunchState::Exited(c)=>format!("Код завершения: {c}"),LaunchState::Failed(e)=>e});}
            }
            if let Some(e)=&self.error{ui.colored_label(Color32::from_rgb(255,155,145),e);}
        });
        if !open{self.popup=None;self.error=None;}
    }
    fn profile(&mut self,ui:&mut egui::Ui){
        match self.auth.state(){
            AuthState::SignedOut=>{
                ui.add(egui::TextEdit::singleline(&mut self.session.offline_name).hint_text("Ник для оффлайн-режима"));
                if ui.button("Войти через Microsoft").clicked(){self.auth.start_login(ui.ctx().clone());}
                ui.small("Без входа доступны только оффлайн-возможности.");
            },
            AuthState::WaitingForUser{verification_uri,user_code}=>{
                ui.label("Открой ссылку и введи код:");ui.hyperlink(verification_uri);
                ui.label(egui::RichText::new(&user_code).size(24.0).strong());
                if ui.button("Скопировать код").clicked(){ui.ctx().output_mut(|o|o.copied_text=user_code);}
                ui.spinner();
            },
            AuthState::InProgress(s)=>{ui.spinner();ui.label(s);},
            AuthState::SignedIn(a)=>{ui.heading(a.username);if ui.button("Выйти").clicked(){self.auth.sign_out();}},
            AuthState::Failed(e)=>{ui.colored_label(Color32::from_rgb(255,155,145),e);if ui.button("Повторить вход").clicked(){self.auth.start_login(ui.ctx().clone());}}
        }
        if let Some(e)=self.skin.error(){ui.label(format!("Скин: {e}"));}
    }
    #[cfg(test)]
    pub fn visual_fixture(ctx:&egui::Context,tab:Tab,populated:bool)->Self{
        let mut a=Self::create(ctx,true);a.tab=tab;a.launch=LaunchManager::visual_fixture();
        if populated{a.library=Library::fixture();}a
    }
}
impl eframe::App for CaligoApp{fn update(&mut self,ctx:&egui::Context,_:&mut eframe::Frame){self.render(ctx)}}