//! Editor-first interaction: scene geometry never depends on editor chrome.
//! Pointer capture is a transaction; a completed gesture is exactly one undo step.
use eframe::egui::{self,pos2,vec2,Color32,Pos2,Rect,Stroke};
use crate::canvas_model::{self,Id,Kind,Page,Scene};
#[derive(Clone,Copy)]
enum Tool{Select,Add(Kind)}
#[derive(Clone,Copy)]
enum Operation{Move,Resize(i8,i8),Draw(Kind)}
struct Gesture{operation:Operation,start:Pos2,rect:Rect,before:Scene,id:Option<Id>}
pub struct Studio{
    pub scene:Scene,pub page:Page,pub selected:Option<Id>,pub active:bool,
    baseline:Option<Scene>,past:Vec<Scene>,future:Vec<Scene>,gesture:Option<Gesture>,
    tool:Tool,palette:bool,layers:bool,pub inspector:bool,inspector_before:Option<Scene>,
    pub error:Option<String>,generation:u64,blocked:bool,
    pub excluded:Vec<Rect>,pub controls:Vec<(&'static str,Rect)>,
    pub snap:bool,pub guide:Option<Rect>,pub exit_question:bool,property_tab:u8,
}
impl Default for Studio{
    fn default()->Self{Self{scene:Scene::default(),page:Page::Home,selected:None,active:false,baseline:None,past:vec![],future:vec![],gesture:None,tool:Tool::Select,palette:false,layers:false,inspector:false,inspector_before:None,error:None,generation:0,blocked:false,excluded:vec![],controls:vec![],snap:true,guide:None,exit_question:false,property_tab:0}}
}
impl Studio{
    pub fn load()->Self{
        let mut s=Self::default();
        match crate::canvas_store::load(&crate::launch::install::game_dir()){
            Ok(Some((g,d)))=>{s.generation=g;s.scene=d},
            Ok(None)=>{},
            Err(e)=>{s.blocked=true;s.error=Some(e)},
        }s
    }
    pub fn begin(&mut self){
        if self.active{return}
        self.baseline=Some(self.scene.clone());self.active=true;self.past.clear();self.future.clear();self.selected=None;self.excluded.clear();self.error=self.error.take();self.exit_question=false;
    }
    pub fn dirty(&self)->bool{self.baseline.as_ref().is_some_and(|b|b!=&self.scene)}
    pub fn record(&mut self,before:Scene){
        if before==self.scene{return}
        if let Err(e)=self.scene.validate(){self.scene=before;self.error=Some(e);return}
        self.past.push(before);if self.past.len()>80{self.past.remove(0);}self.future.clear();
    }
    fn settle(&mut self){if let Some(b)=self.inspector_before.take(){self.record(b)}}
    pub fn undo(&mut self){
        self.cancel_gesture();self.settle();
        if let Some(s)=self.past.pop(){self.future.push(std::mem::replace(&mut self.scene,s));}
        self.selected=self.selected.filter(|id|self.scene.node(*id).is_some());
    }
    pub fn redo(&mut self){
        self.cancel_gesture();self.settle();
        if let Some(s)=self.future.pop(){self.past.push(std::mem::replace(&mut self.scene,s));}
    }
    pub fn cancel_gesture(&mut self){if let Some(g)=self.gesture.take(){self.scene=g.before;}self.guide=None;}
    pub fn cancel(&mut self){
        if let Some(b)=self.baseline.take(){self.scene=b;}
        self.active=false;self.gesture=None;self.inspector_before=None;self.inspector=false;self.selected=None;self.past.clear();self.future.clear();self.exit_question=false;
    }
    pub fn save(&mut self)->bool{
        self.cancel_gesture();self.settle();
        if self.blocked{self.error=Some("Сохранение заблокировано: исходные файлы требуют восстановления. Можно скопировать JSON.".into());return false}
        match crate::canvas_store::save(&crate::launch::install::game_dir(),self.generation,&self.scene){
            Ok(g)=>{self.generation=g;self.baseline=None;self.active=false;self.inspector=false;self.selected=None;self.exit_question=false;self.error=None;true},
            Err(e)=>{self.error=Some(e);false}
        }
    }
    pub fn set_page(&mut self,page:Page){self.cancel_gesture();self.settle();self.page=page;self.selected=None;self.inspector=false;}
    pub fn choose_preset(&mut self,preset:u8){self.cancel_gesture();self.settle();let b=self.scene.clone();self.scene=Scene::preset(preset);self.record(b);self.selected=None;self.inspector=false;}
    pub fn toolbar_rect(bounds:Rect)->Rect{Rect::from_min_size(bounds.min+vec2(12.0,8.0),vec2((bounds.width()-24.0).min(880.0),44.0))}
    fn ignored(&self,p:Pos2,b:Rect)->bool{Self::toolbar_rect(b).contains(p)||self.excluded.iter().any(|r|r.contains(p))}
    pub fn handle_rects(r:Rect)->Vec<(i8,i8,Rect)>{
        [-1,0,1].into_iter().flat_map(|x|[-1,0,1].into_iter().filter_map(move|y|{
            if x==0&&y==0{return None}
            let p=pos2(if x<0{r.left()}else if x>0{r.right()}else{r.center().x},if y<0{r.top()}else if y>0{r.bottom()}else{r.center().y});
            Some((x,y,Rect::from_center_size(p,vec2(12.0,12.0))))
        })).collect()
    }
    fn edge_at(r:Rect,p:Pos2)->Option<(i8,i8)>{
        if !r.expand(6.0).contains(p){return None}
        let x=if (p.x-r.left()).abs()<=6.0{-1}else if (p.x-r.right()).abs()<=6.0{1}else{0};
        let y=if (p.y-r.top()).abs()<=6.0{-1}else if (p.y-r.bottom()).abs()<=6.0{1}else{0};
        if x==0&&y==0{None}else{Some((x,y))}
    }
    /// Runs BEFORE painting so actual nodes follow the pointer in the same frame.
    pub fn input(&mut self,ctx:&egui::Context,b:Rect){
        if !self.active{return}
        if !ctx.wants_keyboard_input(){
            if ctx.input_mut(|i|i.consume_key(egui::Modifiers::CTRL,egui::Key::Z)){self.undo();}
            if ctx.input_mut(|i|i.consume_key(egui::Modifiers::CTRL,egui::Key::Y)){self.redo();}
            if ctx.input(|i|i.key_pressed(egui::Key::Escape)){
                if self.gesture.is_some(){self.cancel_gesture()}
                else if self.inspector{self.inspector=false;self.settle()}
                else if self.palette||self.layers{self.palette=false;self.layers=false;self.tool=Tool::Select}
                else{self.exit_question=true;}
                return
            }
            if self.gesture.is_none(){
                if let Some(id)=self.selected.filter(|&id|!self.scene.frozen(id)){
                    if ctx.input(|i|i.key_pressed(egui::Key::Delete)){
                        let old=self.scene.clone();self.scene.remove(id,b);self.record(old);self.selected=None;self.inspector=false;
                    }else if ctx.input_mut(|i|i.consume_key(egui::Modifiers::CTRL,egui::Key::D)){
                        let old=self.scene.clone();self.selected=self.scene.duplicate(id);self.record(old);
                    }else{
                        let step=if ctx.input(|i|i.modifiers.shift){10.0}else{1.0};
                        let mut delta=egui::Vec2::ZERO;
                        for(key,d)in[(egui::Key::ArrowLeft,vec2(-step,0.0)),(egui::Key::ArrowRight,vec2(step,0.0)),(egui::Key::ArrowUp,vec2(0.0,-step)),(egui::Key::ArrowDown,vec2(0.0,step))]{if ctx.input(|i|i.key_pressed(key)){delta+=d}}
                        if delta!=egui::Vec2::ZERO{let old=self.scene.clone();if let Some(r)=self.scene.screen_rect(id,b){self.scene.place(id,r.translate(delta),b);}self.record(old);}
                    }
                }
            }
        }
        if self.exit_question||!ctx.input(|i|i.focused){self.cancel_gesture();return}
        let Some(p)=ctx.input(|i|i.pointer.interact_pos())else{
            if self.gesture.is_some(){self.cancel_gesture()}return
        };
        if self.gesture.is_none() && self.ignored(p,b){return}
        if self.gesture.is_none(){
            if let Some((x,y))=self.selected.filter(|&id|!self.scene.frozen(id)).and_then(|id|self.scene.screen_rect(id,b)).and_then(|r|Self::edge_at(r,p)){
                ctx.set_cursor_icon(match (x,y){(0,_)=>egui::CursorIcon::ResizeVertical,(_,0)=>egui::CursorIcon::ResizeHorizontal,(-1,-1)|(1,1)=>egui::CursorIcon::ResizeNwSe,_=>egui::CursorIcon::ResizeNeSw});
            }else if self.scene.hit(p,self.page,b).is_some(){ctx.set_cursor_icon(egui::CursorIcon::Grab);}
        }else{ctx.set_cursor_icon(egui::CursorIcon::Grabbing);}
        let pressed=ctx.input(|i|i.pointer.primary_pressed());
        let released=ctx.input(|i|i.pointer.primary_released());
        if pressed && b.contains(p){
            self.settle();self.inspector=false;
            match self.tool{
                Tool::Add(k)=>{self.gesture=Some(Gesture{operation:Operation::Draw(k),start:p,rect:Rect::from_min_max(p,p),before:self.scene.clone(),id:None});},
                Tool::Select=>{
                    let handle=self.selected.filter(|&id|!self.scene.frozen(id)).and_then(|id|self.scene.screen_rect(id,b).and_then(|r|Self::edge_at(r,p).map(|(x,y)|(id,r,x,y))));
                    if let Some((id,r,x,y))=handle{
                        self.gesture=Some(Gesture{operation:Operation::Resize(x,y),start:p,rect:r,before:self.scene.clone(),id:Some(id)});
                    }else{
                        self.selected=self.scene.hit(p,self.page,b);
                        if let Some(id)=self.selected{
                            self.gesture=Some(Gesture{operation:Operation::Move,start:p,rect:self.scene.screen_rect(id,b).unwrap(),before:self.scene.clone(),id:Some(id)});
                        }
                    }
                }
            }
        }
        if let Some(g)=self.gesture.as_ref(){
            let delta=p-g.start;
            if delta.length()>2.0{
                let r=match g.operation{
                    Operation::Move=>{
                        let mut r=g.rect.translate(delta);
                        if self.snap&&!ctx.input(|i|i.modifiers.alt){
                            let parent=self.scene.parent_rect(g.id.unwrap(),b);
                            let mut xs=vec![parent.left(),parent.center().x,parent.right()];
                            let mut ys=vec![parent.top(),parent.center().y,parent.bottom()];
                            for id in self.scene.order(){
                                if Some(id)!=g.id&&!self.scene.descendant(id,g.id.unwrap())&&self.scene.shown(id,self.page){
                                    if let Some(other)=self.scene.screen_rect(id,b){xs.extend([other.left(),other.center().x,other.right()]);ys.extend([other.top(),other.center().y,other.bottom()]);}
                                }
                            }
                            let near=|values:Vec<f32>,edges:[f32;3]|->f32{
                                values.iter().flat_map(|a|edges.iter().map(move|e|*a-*e)).filter(|d|d.abs()<5.0).min_by(|a,b|a.abs().total_cmp(&b.abs())).unwrap_or(0.0)
                            };
                            let d=vec2(near(xs,[r.left(),r.center().x,r.right()]),near(ys,[r.top(),r.center().y,r.bottom()]));
                            r=r.translate(d);self.guide=if d!=egui::Vec2::ZERO{Some(r)}else{None};
                        }r
                    }
                    Operation::Resize(x,y)=>{
                        let mut r=g.rect;
                        if x<0{r.min.x=(r.min.x+delta.x).min(r.max.x-24.0);}
                        if x>0{r.max.x=(r.max.x+delta.x).max(r.min.x+24.0);}
                        if y<0{r.min.y=(r.min.y+delta.y).min(r.max.y-20.0);}
                        if y>0{r.max.y=(r.max.y+delta.y).max(r.min.y+20.0);}
                        r
                    }
                    Operation::Draw(_)=>Rect::from_two_pos(g.start,p),
                };
                if let Some(id)=g.id{
                    if matches!(g.operation,Operation::Move){
                        // A drag is free across the entire page, not trapped by its old panel.
                        // Preserve inherited page when temporarily detaching.
                        let mut scope=g.before.node(id).and_then(|n|n.page);
                        let mut at=g.before.node(id).and_then(|n|n.parent);
                        while let Some(parent)=at{
                            let n=g.before.node(parent).unwrap();scope=scope.or(n.page);at=n.parent;
                        }
                        let node=self.scene.node_mut(id).unwrap();node.parent=None;node.page=scope;
                        self.scene.place(id,canvas_model::bounded(r,b),b);
                    }else{
                        let parent=self.scene.parent_rect(id,b);
                        self.scene.place(id,canvas_model::bounded(r,parent),b);
                    }
                }else{self.guide=Some(r.intersect(b));}
            }
            if released{
                let g=self.gesture.take().unwrap();
                if !b.contains(p){self.scene=g.before;}
                else{
                    if let Operation::Draw(k)=g.operation{
                        let mut r=Rect::from_two_pos(g.start,p);
                        if r.width()<12.0||r.height()<12.0{r=Rect::from_min_size(g.start,if k==Kind::Panel{vec2(250.0,160.0)}else{vec2(160.0,48.0)});}
                        r=canvas_model::bounded(r,b);
                        let id=self.scene.add(k,Some(self.page),canvas_model::unit_rect(r,b),None);self.selected=Some(id);self.tool=Tool::Select;
                    }
                    if let Some(id)=g.id.filter(|_|matches!(g.operation,Operation::Move)&&p.distance(g.start)>2.0){
                        let rect=self.scene.screen_rect(id,b).unwrap();
                        let target=if ctx.input(|i|i.modifiers.alt){None}else{
                            self.scene.order().into_iter().rev().find(|&other|{
                                other!=id&&!self.scene.descendant(other,id)&&!self.scene.frozen(other)&&self.scene.shown(other,self.page)&&
                                self.scene.node(other).is_some_and(|n|n.kind==Kind::Panel)&&
                                self.scene.screen_rect(other,b).is_some_and(|r|r.contains_rect(rect))
                            })
                        };
                        if let Err(e)=self.scene.reparent(id,target,b){self.error=Some(e);}
                    }
                    self.record(g.before);
                }self.guide=None;
            }
        }
    }
    pub fn paint(&mut self,ctx:&egui::Context,b:Rect){
        if !self.active{return}
        let p=ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground,egui::Id::new("studio-lines")));
        let cyan=Color32::from_rgb(105,212,236);
        if let Some(id)=self.selected{
            if let Some(r)=self.scene.screen_rect(id,b){
                p.rect_stroke(r,0.0,Stroke::new(1.5,cyan));
                if !self.scene.frozen(id){for(_,_,h)in Self::handle_rects(r){p.rect_filled(h.shrink(2.0),2.0,Color32::from_rgb(18,31,43));p.rect_stroke(h.shrink(2.0),2.0,Stroke::new(1.0,cyan));}}
                if self.gesture.is_some(){if let Some(n)=self.scene.node(id){let text=format!("{}  ·  {} × {}",n.name,r.width().round(),r.height().round());
                    p.text(pos2(r.left(),(r.top()-8.0).max(b.top()+64.0)),egui::Align2::LEFT_BOTTOM,text,egui::FontId::proportional(11.0),cyan);}}
            }
        }
        if let Some(r)=self.guide{
            p.rect_stroke(r,0.0,Stroke::new(1.0,cyan));
            if self.gesture.as_ref().is_some_and(|g|matches!(g.operation,Operation::Move)){
                p.line_segment([pos2(r.center().x,b.top()),pos2(r.center().x,b.bottom())],Stroke::new(0.5,cyan.gamma_multiply(0.45)));
                p.line_segment([pos2(b.left(),r.center().y),pos2(b.right(),r.center().y)],Stroke::new(0.5,cyan.gamma_multiply(0.45)));
            }
        }
        self.excluded.clear();self.controls.clear();
        self.toolbar(ctx,b);
        if self.layers{self.layer_list(ctx,b);}
        if self.palette{self.catalog(ctx,b);}
        self.selection_bar(ctx,b);
        if self.inspector{self.properties(ctx,b);}
        if self.exit_question{
            let r=egui::Window::new("Закончить редактирование?").order(egui::Order::Tooltip).collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER,vec2(0.0,0.0)).show(ctx,|ui|{
                ui.label(if self.dirty(){"Есть несохранённые изменения."}else{"Изменений нет."});
                ui.horizontal(|ui|{
                    if ui.button("Сохранить").clicked(){self.save();}
                    if ui.button("Отбросить").clicked(){self.cancel();}
                    if ui.button("Продолжить").clicked(){self.exit_question=false;}
                });
            });
            if let Some(r)=r{self.excluded.push(r.response.rect);}
        }
    }
    fn toolbar(&mut self,ctx:&egui::Context,b:Rect){
        let area=egui::Area::new(egui::Id::new("studio-toolbar")).order(egui::Order::Tooltip).fixed_pos(Self::toolbar_rect(b).min).show(ctx,|ui|{
            egui::Frame::none().fill(Color32::from_rgb(16,25,36)).rounding(12.0).inner_margin(8.0).show(ui,|ui|{
                ui.spacing_mut().interact_size.y=28.0;ui.spacing_mut().item_spacing.x=5.0;
                ui.horizontal(|ui|{
                    ui.label(egui::RichText::new("Редактор").strong());
                    ui.menu_button(self.page.label(),|ui|{for page in Page::ALL{if ui.button(page.label()).clicked(){self.set_page(page);ui.close_menu();}}});
                    let a=ui.button("+ Добавить");self.controls.push(("add",a.rect));if a.clicked(){self.palette=!self.palette;self.layers=false;}
                    if ui.button("Слои").clicked(){self.layers=!self.layers;self.palette=false;}
                    ui.menu_button("Пресет",|ui|{for p in [1,3,6]{if ui.button(format!("{p:02}")).clicked(){self.choose_preset(p);ui.close_menu();}}});
                    let u=ui.add_enabled(!self.past.is_empty(),egui::Button::new("Назад"));self.controls.push(("undo",u.rect));if u.clicked(){self.undo();}
                    if ui.add_enabled(!self.future.is_empty(),egui::Button::new("Вперёд")).clicked(){self.redo();}
                    let done=ui.button("Готово");self.controls.push(("save",done.rect));if done.clicked(){self.save();}
                    if ui.button("Отмена").clicked(){self.exit_question=true;}
                });
            });
        });
        self.excluded.push(area.response.rect);self.controls.push(("toolbar",area.response.rect));
        if let Some(e)=&self.error{
            let r=egui::Area::new(egui::Id::new("studio-error")).fixed_pos(b.min+vec2(12.0,65.0)).order(egui::Order::Tooltip).show(ctx,|ui|{
                egui::Frame::popup(ui.style()).show(ui,|ui|{ui.set_max_width((b.width()-40.0).min(540.0));ui.colored_label(Color32::from_rgb(255,155,145),e);});
            });self.excluded.push(r.response.rect);
        }
    }
    fn catalog(&mut self,ctx:&egui::Context,b:Rect){
        let r=egui::Window::new("Добавить на страницу").id(egui::Id::new("studio-catalog")).order(egui::Order::Tooltip).fixed_pos(b.min+vec2(12.0,64.0)).default_width(210.0).max_height((b.height()-95.0).max(180.0)).vscroll(true).resizable(false).collapsible(false).show(ctx,|ui|{
            ui.label("Выбери элемент и нарисуй его размер на странице.");
            for kind in Kind::ALL{if ui.button(kind.label()).clicked(){self.tool=Tool::Add(kind);self.palette=false;self.inspector=false;}}
            ui.separator();ui.checkbox(&mut self.snap,"Привязка при переносе");ui.small("Alt — временно без привязки");
        });if let Some(r)=r{self.excluded.push(r.response.rect);}
    }
    fn layer_list(&mut self,ctx:&egui::Context,b:Rect){
        let r=egui::Window::new("Слои страницы").id(egui::Id::new("studio-layers")).order(egui::Order::Tooltip).fixed_pos(b.min+vec2(12.0,64.0)).default_width(230.0).max_height((b.height()-95.0).max(180.0)).vscroll(true).resizable(false).collapsible(false).show(ctx,|ui|{
            for id in self.scene.order(){
                let n=self.scene.node(id).unwrap().clone();
                // Hidden and locked nodes stay selectable here for recovery.
                let mut page=n.page;let mut at=n.parent;
                while let Some(p)=at{let parent=self.scene.node(p).unwrap();page=page.or(parent.page);at=parent.parent;}
                if page.is_some_and(|p|p!=self.page){continue}
                let old=self.scene.clone();
                ui.horizontal(|ui|{
                    let mut visible=n.visible;if ui.checkbox(&mut visible,"").on_hover_text("Видимость").changed(){self.scene.node_mut(id).unwrap().visible=visible;}
                    if ui.selectable_label(self.selected==Some(id),format!("{}{}{}",if n.parent.is_some(){"  └ "}else{""},n.name,if n.locked{" [lock]"}else{""})).clicked(){self.settle();self.selected=Some(id);self.inspector=true;}
                });self.record(old);
            }
        });if let Some(r)=r{self.excluded.push(r.response.rect);}
    }
    fn selection_bar(&mut self,ctx:&egui::Context,b:Rect){
        let Some(id)=self.selected else{return};let Some(r)=self.scene.screen_rect(id,b)else{return};
        if self.gesture.is_some(){return}
        let width=155.0;
        let x=r.left().clamp(b.left()+8.0,(b.right()-width-8.0).max(b.left()+8.0));
        let y=if r.top()>b.top()+110.0{r.top()-43.0}else{(r.bottom()+10.0).min(b.bottom()-42.0)};
        let area=egui::Area::new(egui::Id::new("studio-selection-actions")).order(egui::Order::Tooltip).fixed_pos(pos2(x,y)).show(ctx,|ui|{
            egui::Frame::popup(ui.style()).inner_margin(5.0).show(ui,|ui|{
                ui.horizontal(|ui|{
                    let (gear_rect,gear)=ui.allocate_exact_size(vec2(32.0,28.0),egui::Sense::click());
                    let c=gear_rect.center();let st=Stroke::new(1.4,Color32::from_rgb(200,220,232));
                    ui.painter().circle_stroke(c,5.5,st);ui.painter().circle_stroke(c,2.0,st);
                    for i in 0..8{let a=i as f32*std::f32::consts::TAU/8.0;let d=vec2(a.cos(),a.sin());ui.painter().line_segment([c+d*5.0,c+d*8.5],st);}
                    self.controls.push(("gear",gear_rect));
                    if gear.on_hover_text("Настроить этот элемент").clicked(){self.inspector=!self.inspector;self.property_tab=0;if !self.inspector{self.settle();}}
                    if ui.button("Копия").clicked(){let old=self.scene.clone();self.selected=self.scene.duplicate(id);self.record(old);}
                    if ui.button("…").clicked(){self.layers=!self.layers;}
                });
            });
        });self.excluded.push(area.response.rect);
    }
    fn properties(&mut self,ctx:&egui::Context,b:Rect){
        let Some(id)=self.selected else{return};let Some(n)=self.scene.node(id).cloned()else{return};
        let rect=self.scene.screen_rect(id,b).unwrap();
        let x=if rect.right()+280.0<b.right(){rect.right()+14.0}else{(rect.left()-282.0).max(b.left()+12.0)};
        let y=(rect.top()+8.0).clamp(b.top()+65.0,(b.bottom()-250.0).max(b.top()+65.0));
        let before=self.scene.clone();let mut edited=n.clone();let mut parent=edited.parent;let mut remove=false;let mut foreground=false;let mut background=false;
        let mut open=true;
        let r=egui::Window::new(format!("{} · настройки",n.kind.label())).id(egui::Id::new(("studio-properties",id)))
            .order(egui::Order::Tooltip).default_pos(pos2(x,y)).default_width(254.0).max_width(270.0)
            .max_height((b.height()-145.0).max(140.0)).vscroll(true).resizable(false).collapsible(false).open(&mut open)
            .constrain_to(Rect::from_min_max(b.min+vec2(8.0,65.0),b.max-vec2(8.0,8.0))).show(ctx,|ui|{
                ui.horizontal(|ui|{
                    ui.selectable_value(&mut self.property_tab,0,"Внешний вид");
                    ui.selectable_value(&mut self.property_tab,1,"Расположение");
                });
                ui.separator();
                if self.property_tab==0{
                ui.add(egui::TextEdit::singleline(&mut edited.name).char_limit(100));
                ui.horizontal(|ui|{
                    ui.label("Цвет");ui.color_edit_button_srgba_unmultiplied(&mut edited.style.fill);
                    ui.label("Текст");ui.color_edit_button_srgba_unmultiplied(&mut edited.style.ink);
                });
                let mut opacity=edited.style.fill[3] as f32/255.0;
                let a=ui.add(egui::Slider::new(&mut opacity,0.0..=1.0).text("Непрозрачность"));self.controls.push(("opacity",a.rect));if a.changed(){edited.style.fill[3]=(opacity*255.0).round() as u8;}
                let round=ui.add(egui::Slider::new(&mut edited.style.radius,0.0..=50.0).text("Скругление"));self.controls.push(("rounding",round.rect));
                ui.add(egui::Slider::new(&mut edited.style.font,10.0..=40.0).text("Размер текста"));
                ui.checkbox(&mut edited.style.blur,"Размытые обои под элементом").on_hover_text("Кэш обоев, не живое размытие перекрытых элементов.");
                }else{
                egui::ComboBox::from_id_salt("parent").selected_text(parent.and_then(|p|self.scene.node(p)).map(|n|n.name.as_str()).unwrap_or("Без панели")).show_ui(ui,|ui|{
                    ui.selectable_value(&mut parent,None,"Без панели");
                    for p in self.scene.nodes.iter().filter(|p|p.kind==Kind::Panel&&!self.scene.descendant(p.id,id)&&self.scene.shown(p.id,self.page)){ui.selectable_value(&mut parent,Some(p.id),&p.name);}
                });
                let mut global=edited.page.is_none()&&edited.parent.is_none();
                if edited.parent.is_none()&&ui.checkbox(&mut global,"На всех страницах").changed(){edited.page=if global{None}else{Some(self.page)};}
                ui.checkbox(&mut edited.locked,"Закрепить от перемещения");
                ui.horizontal(|ui|{foreground=ui.button("Выше").clicked();background=ui.button("Ниже").clicked();remove=ui.button("Удалить").clicked();});
                ui.small("Размер — за края. Стрелки — 1 px, Shift — 10 px. Ctrl+D — копия.");
                }
            });
        self.inspector=open;
        if edited!=n{*self.scene.node_mut(id).unwrap()=edited;}
        if parent!=n.parent{if let Err(e)=self.scene.reparent(id,parent,b){self.error=Some(e)}}
        if foreground||background{
            let index=self.scene.nodes.iter().position(|n|n.id==id).unwrap();
            let node=self.scene.nodes.remove(index);
            if foreground{self.scene.nodes.push(node)}else{self.scene.nodes.insert(0,node)}
        }
        if remove{self.scene.remove(id,b);self.inspector=false;self.selected=None;}
        if self.scene!=before && self.inspector_before.is_none(){self.inspector_before=Some(before);}
        if !ctx.input(|i|i.pointer.primary_down())&&!ctx.wants_keyboard_input(){self.settle();}
        if !open{self.settle();}
        if let Some(r)=r{self.excluded.push(r.response.rect);self.controls.push(("inspector",r.response.rect));}
    }
}