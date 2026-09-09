//! Direct editor for navigation and page components. Actions are gated while editing.
use eframe::egui::{self,pos2,vec2,Color32,Id,Pos2,Rect,Sense,Stroke,Vec2};
use crate::{composition::{self,Action,Anchor,Document,Edge,History,Layout,NodeId,Position},theme::ThemePreset};

#[derive(Clone,Copy,PartialEq)]
enum DragKind { Move,Size(i8,i8) }
struct Drag {id:NodeId,kind:DragKind,start:Pos2,rect:Rect,before:Document}
pub struct Editor {
    pub document:Document,
    pub page:Action,
    baseline:Option<Document>,
    history:History,
    selected:Option<NodeId>,
    inspector:bool,
    background_open:bool,
    drag:Option<Drag>,
    adding:bool,
    draw_start:Option<Pos2>,
    pub error:Option<String>,
    generation:u64,
    blocked:bool,
    pub rects:Vec<(NodeId,Rect)>,
    pub panel_rects:Vec<(NodeId,Rect)>,
    gesture_before:Option<Document>,
    pub controls:Vec<(&'static str,Rect)>,
}
impl Default for Editor {
    fn default()->Self {Self{document:Document::default(),page:Action::Home,baseline:None,history:History::default(),selected:None,
        inspector:false,background_open:false,drag:None,adding:false,draw_start:None,error:None,generation:0,blocked:false,
        rects:Vec::new(),panel_rects:Vec::new(),gesture_before:None,controls:Vec::new()}}
}
impl Editor {
    pub fn load()->Self {
        let mut e=Self::default();
        match composition::load(&crate::launch::install::game_dir()) {
            Ok(Some((g,d)))=>{e.generation=g;e.document=d;}
            Ok(None)=>{},
            Err(error)=>{e.error=Some(format!("{error}\nПоказан стандартный интерфейс. Сохранение заблокировано, чтобы не потерять файл."));e.blocked=true;}
        }
        e
    }
    pub fn active(&self)->bool {self.baseline.is_some()}
    pub fn begin(&mut self){if !self.active(){self.baseline=Some(self.document.clone());self.history=History::default();self.selected=None;}}
    pub fn cancel(&mut self){
        if let Some(d)=self.baseline.take(){self.document=d;}
        self.drag=None;self.gesture_before=None;self.history=History::default();self.inspector=false;self.background_open=false;self.adding=false;self.draw_start=None;
    }
    pub fn finish(&mut self)->bool{
        if self.blocked{self.error=Some("Сохранение заблокировано: исходный файл требует восстановления. Изменения можно отменить.".into());return false}
        match composition::save(&crate::launch::install::game_dir(),self.generation,&self.document){
            Ok(g)=>{self.generation=g;self.baseline=None;self.drag=None;self.gesture_before=None;self.history=History::default();self.inspector=false;self.background_open=false;self.adding=false;self.draw_start=None;true}
            Err(e)=>{self.error=Some(format!("Не сохранено: {e}"));false}
        }
    }
    fn settle_local_edit(&mut self){
        if let Some(before)=self.gesture_before.take(){self.history.record(before,&self.document);}
    }
    fn undo(&mut self){self.settle_local_edit();self.history.undo(&mut self.document);}
    fn redo(&mut self){self.settle_local_edit();self.history.redo(&mut self.document);}
    fn change(&mut self,f:impl FnOnce(&mut Document)){
        self.settle_local_edit();
        let before=self.document.clone();f(&mut self.document);
        if let Err(e)=self.document.validate(){self.document=before;self.error=Some(e)}
        else{self.history.record(before,&self.document)}
    }
    pub fn toolbar(&mut self,ctx:&egui::Context,theme:&ThemePreset){
        self.controls.clear();
        if !self.active(){return}
        // No state-mutating keyboard shortcut is active outside edit mode.
        if !ctx.wants_keyboard_input(){
            if ctx.input_mut(|i|i.consume_key(egui::Modifiers::CTRL,egui::Key::Z)){self.undo();}
            if ctx.input_mut(|i|i.consume_key(egui::Modifiers::CTRL,egui::Key::Y)){self.redo();}
            if ctx.input(|i|i.key_pressed(egui::Key::Escape)){
                if let Some(d)=self.drag.take(){self.document=d.before;}else{self.adding=false;self.draw_start=None;self.inspector=false;self.background_open=false;}
            }
        }
        egui::TopBottomPanel::top("composition_toolbar").exact_height(88.0)
            .frame(egui::Frame::none().fill(theme.surface(2)).inner_margin(egui::Margin::symmetric(12.0,8.0)))
            .show(ctx,|ui|{
                ui.horizontal(|ui|{
                    ui.label(egui::RichText::new("Редактор").strong().color(theme.accent_color()));
                    ui.separator();
                    let add=ui.button(if self.adding{"Выбери место…"}else{"+ Панель"});
                    self.controls.push(("add_panel",add.rect));
                    if add.clicked(){self.adding=!self.adding;self.inspector=false;}
                    ui.menu_button("+ Элемент",|ui|{
                        for action in [Action::Home,Action::Library,Action::Settings,Action::Profile]{
                            if ui.button(action.label()).clicked(){
                                self.change(|d|{d.add_widget(action);});ui.close_menu();
                            }
                        }
                        for &kind in Action::components(){
                            if ui.button(kind.label()).clicked(){
                                let page=self.page;
                                self.change(|d|{d.add_content(kind,page,12,160.0);});ui.close_menu();
                            }
                        }
                    });
                    if ui.button("Фон").clicked(){self.background_open=!self.background_open;self.inspector=false;}
                    let undo=ui.add_enabled(self.history.can_undo()||self.gesture_before.is_some(),egui::Button::new("Назад"));
                    self.controls.push(("undo",undo.rect));
                    if undo.on_hover_text("Отменить · Ctrl+Z").clicked(){self.undo();}
                    let redo=ui.add_enabled(self.history.can_redo(),egui::Button::new("Вперёд"));
                    self.controls.push(("redo",redo.rect));
                    if redo.on_hover_text("Повторить · Ctrl+Y").clicked(){self.redo();}
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center),|ui|{
                        let finish=ui.button(egui::RichText::new("Готово").color(theme.accent_color()));
                        self.controls.push(("finish",finish.rect));if finish.clicked(){self.finish();}
                        let cancel=ui.button("Отмена");self.controls.push(("cancel",cancel.rect));
                        if cancel.on_hover_text("Отменить всю сессию").clicked(){self.cancel();}
                    });
                });
                ui.horizontal(|ui|{
                    for page in [Action::Home,Action::Library,Action::Settings] {
                        if ui.selectable_label(self.page==page,page.label()).clicked(){self.page=page;self.selected=None;self.inspector=false;}
                    }
                    ui.separator();
                    ui.menu_button("Пресеты",|ui|{
                        for (kind,label) in [(1,"01 · Боковая панель"),(3,"03 · Нижний док"),(6,"06 · Центральный персонаж")] {
                            if ui.button(label).clicked(){self.change(|d|*d=Document::preset(kind));self.selected=None;self.inspector=false;ui.close_menu();}
                        }
                        ui.label("Заменяет раскладку; можно отменить. Данные игры не меняет.");
                    });
                    ui.menu_button("Объекты",|ui|{
                        let panels=self.document.panels.clone();let widgets=self.document.widgets.clone();
                        for p in panels {if ui.button(format!("Панель {}",p.id)).clicked(){self.selected=Some(p.id);self.inspector=true;ui.close_menu();}}
                        for w in widgets {if ui.button(&w.label).clicked(){self.selected=Some(w.id);self.inspector=true;if let Some(p)=w.page{self.page=p;}ui.close_menu();}}
                        ui.separator();
                        if ui.button("Восстановить стандартную раскладку").clicked(){self.change(|d|*d=Document::default());ui.close_menu();}
                    });
                });
            });
    }
    pub fn layout(&self,bounds:Rect)->Layout {Layout::compute(&self.document,bounds)}
pub fn shell(&mut self,ctx:&egui::Context,layout:&Layout,theme:&ThemePreset,active:Action,background:&crate::background::Background,
        mut content:impl FnMut(&mut egui::Ui,&composition::Widget))->Option<Action> {
        if self.active()&&self.drag.is_some(){
            if let Some(p)=ctx.pointer_interact_pos(){self.apply_drag(p,layout,false,false);}
        }
        let current_layout=Layout::compute(&self.document,layout.bounds);
        let layout=&current_layout;
        self.rects.clear();self.panel_rects=layout.panels.clone();
        let mut action=None;
        let visible=|w:&composition::Widget|w.page.is_none_or(|p|p==active);
        let flow:Vec<_>=self.document.widgets.iter().filter(|w|visible(w)&&w.panel.is_none()&&w.flow).cloned().collect();
        egui::CentralPanel::default().frame(egui::Frame::none()).show(ctx,|ui|{
            let mut area=layout.content.shrink(20.0);
            for p in &self.document.panels {
                if p.relative.is_some_and(|r|r[1]>0.8) {
                    if let Some(r)=layout.panel(p.id){area.max.y=area.max.y.min(r.top()-12.0);}
                }
            }
            let mut page=ui.new_child(egui::UiBuilder::new().id_salt(("workspace_page",active)).max_rect(area));
            page.set_clip_rect(area);
            egui::ScrollArea::vertical().id_salt(("workspace_scroll",active)).auto_shrink([false,false]).show(&mut page,|ui|{
                let bounds=Rect::from_min_size(ui.cursor().min,vec2(ui.available_width(),1.0));
                let rects=composition::flow_rects(&flow,bounds);
                for (w,(_,r)) in flow.iter().zip(rects.iter()) {
                    let mut child=ui.new_child(egui::UiBuilder::new().id_salt(("workspace_node",w.id)).max_rect(*r));
                    child.set_clip_rect(r.intersect(ui.clip_rect()));
                    if let Some(a)=self.widget(&mut child,w,r.size(),theme,active,&mut content){action=Some(a);}
                }
                if let Some((_,last))=rects.last(){
                    let bottom=rects.iter().map(|(_,r)|r.bottom()).fold(last.bottom(),f32::max);
                    ui.allocate_space(vec2(bounds.width(),(bottom-bounds.top()).max(1.0)));
                }
            });
        });
        for p in self.document.panels.clone(){
            let Some(r)=layout.panel(p.id)else{continue};
            if r.width()<24.0||r.height()<24.0{continue}
            egui::Area::new(Id::new(("compose_panel",p.id))).fixed_pos(r.min).order(egui::Order::Middle).fade_in(false).show(ctx,|ui|{
                ui.set_min_size(r.size());ui.set_max_size(r.size());ui.set_clip_rect(r);
                let fill=p.style.fill.map(rgba).unwrap_or(theme.modules.sidebar.fill_or(theme.surface(2)));
                background.surface(ui.painter(),r,p.style.rounding.unwrap_or(theme.modules.sidebar.rounding_or(0.0)),fill,p.style.blur&&self.document.background.is_none());
                let inner=r.shrink(10.0);
                let mut child=ui.new_child(egui::UiBuilder::new().id_salt(("panel_items",p.id)).max_rect(inner));
                child.set_clip_rect(inner);
                let widgets:Vec<_>=self.document.widgets.iter().filter(|w|w.panel==Some(p.id)&&visible(w)).cloned().collect();
                let total:f32=widgets.iter().map(|w|if p.vertical{w.size[1]+8.0}else{w.size[0]+8.0}).sum();
                let view=if p.vertical{inner.height()}else{inner.width()};
                let scroll=if p.vertical{egui::ScrollArea::vertical()}else{egui::ScrollArea::horizontal()};
                scroll.id_salt(("panel_scroll",p.id)).auto_shrink([false,false]).show(&mut child,|ui|{
                    if p.vertical {
                        let mut used=0.0;
                        for w in &widgets {
                            if w.bottom&&total<view {ui.add_space((view-used-w.size[1]-12.0).max(0.0));}
                            if let Some(a)=self.widget(ui,w,vec2(inner.width().max(24.0),w.size[1]),theme,active,&mut content){action=Some(a)}
                            ui.add_space(8.0);used+=w.size[1]+8.0;
                        }
                    }else{
                        ui.horizontal(|ui|{for w in &widgets{
                            if let Some(a)=self.widget(ui,w,vec2(if p.relative.is_some(){((inner.width()-16.0)/widgets.len().max(1) as f32).max(32.0)}else{w.size[0]},w.size[1].min(inner.height().max(24.0))),theme,active,&mut content){action=Some(a)}
                        }});
                    }
                });
            });
        }
        for w in self.document.widgets.clone().into_iter().filter(|w|visible(w)&&w.panel.is_none()&&!w.flow){
            let r=composition::placed_rect(w.position,w.size,w.relative,layout.bounds);
            egui::Area::new(Id::new(("compose_free",w.id))).fixed_pos(r.min).fade_in(false).order(egui::Order::Middle).show(ctx,|ui|{
                ui.set_clip_rect(r);if let Some(a)=self.widget(ui,&w,r.size(),theme,active,&mut content){action=Some(a)}
            });
        }
        if self.active(){None}else{action}
    }
    fn widget(&mut self,ui:&mut egui::Ui,w:&composition::Widget,size:Vec2,theme:&ThemePreset,active:Action,
        content:&mut impl FnMut(&mut egui::Ui,&composition::Widget))->Option<Action>{
        let (r,_)=ui.allocate_exact_size(size,Sense::hover());
        let rclip=r.intersect(ui.clip_rect());if rclip.is_positive(){self.rects.push((w.id,rclip));}
        if !w.action.is_navigation(){
            let mut child=ui.new_child(egui::UiBuilder::new().id_salt(("component",w.id)).max_rect(r));
            child.set_clip_rect(r.intersect(ui.clip_rect()));
            child.add_enabled_ui(!self.active(),|ui|content(ui,w));
            return None
        }
        let response=ui.interact(r,Id::new(("composition_action",w.id)),Sense::click());
        let selected=active==w.action;
        let fill=w.style.fill.map(rgba).unwrap_or(if selected{theme.surface(4)}else{Color32::TRANSPARENT});
        ui.painter().rect_filled(r,w.style.rounding.unwrap_or(theme.rounding),fill);
        if response.hovered(){ui.painter().rect_filled(r,w.style.rounding.unwrap_or(theme.rounding),theme.accent_bg());}
        let color=if w.style.fill.is_some(){crate::ui::components::contrast(fill)}else if selected{theme.accent_color()}else{theme.text_body()};
        let center=pos2(if r.width()<90.0{r.center().x}else{r.left()+20.0},r.center().y);
        let icon=match w.action{Action::Home=>crate::app::NavIcon::Home,Action::Library=>crate::app::NavIcon::Cube,_=>crate::app::NavIcon::Sliders};
        crate::app::paint_nav_icon(ui.painter(),center,icon,color,0.0);
        if r.width()>=90.0{
            crate::ui::components::label(ui.painter(),Rect::from_min_max(pos2(r.left()+40.0,r.top()),r.max-vec2(6.0,0.0)),&w.label,13.0,color);
        }
        if response.on_hover_text(&w.label).clicked()&&!self.active(){Some(w.action)}else{None}
    }    pub fn overlay(&mut self,ctx:&egui::Context,layout:&Layout,theme:&ThemePreset){
        if !self.active(){self.show_error(ctx);return}
        let bounds=layout.bounds;
        let panel_rects=self.panel_rects.clone();let widget_rects=self.rects.clone();
        let mut overlay_rect=Rect::NOTHING;
        egui::Area::new(Id::new("composition_overlay")).fixed_pos(bounds.min).order(egui::Order::Foreground).fade_in(false).show(ctx,|ui|{
            ui.set_min_size(bounds.size());ui.set_max_size(bounds.size());ui.set_clip_rect(bounds);
            overlay_rect=bounds;
            // The overlay consumes clicks; app additionally disables every functional subtree.
            let bg=ui.interact(bounds,Id::new("compose_background"),Sense::click());
            if bg.clicked()&&!self.adding{self.selected=None;self.inspector=false;}
            let accent=theme.accent_color();
            for (id,r) in panel_rects.iter().chain(widget_rects.iter()){
                if self.adding{continue}
                if r.width()<24.0||r.height()<24.0{continue}
                let response=ui.interact(*r,Id::new(("edit_target",id)),Sense::click_and_drag());
                if response.clicked(){self.selected=Some(*id);self.inspector=false;}
                if response.drag_started()&&self.drag.is_none(){
                    if let Some(start)=ctx.input(|i|i.pointer.press_origin()){
                        self.settle_local_edit();
                        self.selected=Some(*id);self.inspector=false;
                        self.drag=Some(Drag{id:*id,kind:DragKind::Move,start,rect:*r,before:self.document.clone()});
                    }
                }
                if self.selected==Some(*id){
                    ui.painter().rect_stroke(r.shrink(1.0),6.0,Stroke::new(1.5_f32,accent));
                }else if response.hovered(){ui.painter().rect_stroke(r.shrink(1.0),4.0,Stroke::new(1.0_f32,accent.gamma_multiply(0.5)));}
            }
            if let Some(id)=self.selected {
                if let Some((_,r))=widget_rects.iter().chain(panel_rects.iter()).find(|(i,_)|*i==id){
                    let gear=Rect::from_min_size(pos2((r.right()+4.0).min(bounds.right()-30.0),r.top().max(bounds.top())),vec2(28.0,28.0));
                    ui.painter().rect_filled(gear,7.0,theme.surface(3));
                    gear_icon(ui.painter(),gear.center(),accent);
                    if ui.interact(gear,Id::new("local_gear"),Sense::click()).on_hover_text("Настройки выбранного элемента").clicked(){self.inspector=!self.inspector;self.background_open=false;}
                    self.controls.push(("gear",gear));
                    let edge=self.document.panels.iter().find(|p|p.id==id).map(|p|p.edge).unwrap_or(Edge::Float);
                    let handles:Vec<(i8,i8,Pos2)>=match edge {
                        Edge::Left=>vec![(1,0,r.right_center())],Edge::Right=>vec![(-1,0,r.left_center())],
                        Edge::Top=>vec![(0,1,r.center_bottom())],Edge::Bottom=>vec![(0,-1,r.center_top())],
                        Edge::Float=>vec![(1,1,r.right_bottom()),(-1,-1,r.left_top()),(1,-1,r.right_top()),(-1,1,r.left_bottom()),
                            (-1,0,r.left_center()),(1,0,r.right_center()),(0,-1,r.center_top()),(0,1,r.center_bottom())],
                    };
                    for (index,(x,y,center)) in handles.into_iter().enumerate(){
                        let center=pos2(center.x.clamp(bounds.left()+6.0,bounds.right()-6.0),center.y.clamp(bounds.top()+6.0,bounds.bottom()-6.0));
                        let handle=Rect::from_center_size(center,vec2(16.0,16.0));
                        ui.painter().rect_filled(handle.shrink(4.0),2.0,accent);
                        if index==0{self.controls.push(("resize",handle));}
                        let cursor=if x==0{egui::CursorIcon::ResizeVertical}else if y==0{egui::CursorIcon::ResizeHorizontal}else if x==y{egui::CursorIcon::ResizeNwSe}else{egui::CursorIcon::ResizeNeSw};
                        let h=ui.interact(handle,Id::new(("resize_handle",id,index)),Sense::drag()).on_hover_cursor(cursor);
                        if h.drag_started(){if let Some(start)=ctx.input(|i|i.pointer.press_origin()){
                            self.settle_local_edit();self.drag=Some(Drag{id,kind:DragKind::Size(x,y),start,rect:*r,before:self.document.clone()});
                        }}
                    }
                }
            }
            if self.adding {
                if let Some(p)=ctx.pointer_interact_pos().or_else(||ctx.pointer_hover_pos()).filter(|p|bounds.contains(*p)){
                    if ctx.input(|i|i.pointer.primary_pressed()){self.draw_start=Some(p);}
                    let preview=self.draw_start.map(|s|Rect::from_two_pos(s,p)).unwrap_or_else(||panel_preview(near_edge(p,bounds),p,bounds));
                    ui.painter().rect_filled(preview,12.0,accent.gamma_multiply(0.12));
                    ui.painter().rect_stroke(preview,12.0,Stroke::new(1.5_f32,accent));
                    if ctx.input(|i|i.pointer.primary_released()){
                        if let Some(start)=self.draw_start.take(){
                            let drawn=Rect::from_two_pos(start,p);
                            let edge=if drawn.size().length()>20.0{Edge::Float}else{near_edge(p,bounds)};
                            let r=if drawn.width()>=44.0&&drawn.height()>=44.0{drawn}else{panel_preview(edge,p,bounds)};
                            let position=Position::from_rect(r,bounds,true);
                            self.change(|d|{
                                d.add_panel(edge,position);
                                let panel=d.panels.last_mut().unwrap();
                                if edge==Edge::Float{panel.size=[r.width().clamp(44.0,1000.0),r.height().clamp(44.0,1000.0)];}
                                panel.style.fill=Some([17,30,46,170]);panel.style.rounding=Some(14.0);panel.style.blur=true;
                            });
                            self.selected=self.document.panels.last().map(|p|p.id);self.adding=false;
                        }
                    }
                }
            }
            if let Some(d)=&self.drag {
                if d.kind==DragKind::Move {
                    if let Some(p)=ctx.pointer_interact_pos(){
                        if let Some((_,r))=panel_rects.iter().rev().find(|(id,r)|*id!=d.id&&r.contains(p)){
                            ui.painter().rect_stroke(r.shrink(3.0),8.0,Stroke::new(2.0_f32,accent));
                        }
                    }
                }
            }
        });
        if ctx.input(|i|i.pointer.any_released())&&self.drag.is_some(){
            if let Some(p)=ctx.pointer_interact_pos().filter(|p|overlay_rect.contains(*p)){
                self.apply_drag(p,layout,true,!ctx.input(|i|i.modifiers.alt));
                let d=self.drag.take().unwrap();
                if let Err(e)=self.document.validate(){self.error=Some(e);self.document=d.before;}
                else{self.history.record(d.before,&self.document);}
            }else if let Some(d)=self.drag.take(){self.document=d.before;}
        }
        let before_frame=self.document.clone();
        self.inspector_ui(ctx,layout,theme);
        // Coalesce a slider/color drag or a text focus session into one undo operation.
        if before_frame!=self.document&&self.drag.is_none()&&self.gesture_before.is_none(){self.gesture_before=Some(before_frame);}
        if self.gesture_before.is_some()&&!ctx.input(|i|i.pointer.any_down())&&!ctx.wants_keyboard_input(){
            let before=self.gesture_before.take().unwrap();
            self.history.record(before,&self.document);
        }
        self.show_error(ctx);
    }
fn apply_drag(&mut self,p:Pos2,layout:&Layout,finalize:bool,snap:bool){
        let Some(d)=self.drag.as_ref()else{return};
        let id=d.id;let kind=d.kind;let start=d.start;let original=d.rect;
        let before=d.before.clone();let delta=p-start;
        let mut r=original;
        if kind==DragKind::Move {r=r.translate(delta);}
        else if let DragKind::Size(x,y)=kind {
            if x<0{r.min.x=(r.min.x+delta.x).min(r.max.x-44.0);}
            if x>0{r.max.x=(r.max.x+delta.x).max(r.min.x+44.0);}
            if y<0{r.min.y=(r.min.y+delta.y).min(r.max.y-44.0);}
            if y>0{r.max.y=(r.max.y+delta.y).max(r.min.y+44.0);}
        }
        self.document=before.clone();
        if let Some(panel)=self.document.panels.iter_mut().find(|n|n.id==id){
            panel.relative=None;
            if kind==DragKind::Move {
                panel.edge=if finalize{near_edge(p,layout.bounds)}else{Edge::Float};
                if finalize&&panel.edge!=Edge::Float{panel.vertical=matches!(panel.edge,Edge::Left|Edge::Right);}
            }
            panel.size=[r.width().clamp(44.0,1000.0),r.height().clamp(44.0,1000.0)];
            // Preserve dormant axis for edge panels.
            if let Some(old)=before.panels.iter().find(|n|n.id==id){
                if matches!(panel.edge,Edge::Left|Edge::Right){panel.size[1]=old.size[1];}
                if matches!(panel.edge,Edge::Top|Edge::Bottom){panel.size[0]=old.size[0];}
            }
            panel.position=Position::from_rect(r,layout.bounds,snap);
        }else if kind==DragKind::Move{
            let parent=if finalize{layout.panels.iter().rev().find(|(_,b)|b.contains(p)).map(|(i,_)|*i)}else{None};
            let vertical=parent.and_then(|i|self.document.panels.iter().find(|n|n.id==i)).is_some_and(|n|n.vertical);
            let target=if parent.is_some(){self.rects.iter().filter(|(i,_)|*i!=id).find(|(i,b)|self.document.widgets.iter().any(|w|w.id==*i&&w.panel==parent)&&if vertical{p.y<b.center().y}else{p.x<b.center().x}).map(|(i,_)|*i)}else{None};
            self.document.move_widget(id,parent,target,Position::from_rect(r,layout.bounds,snap));
            if let Some(w)=self.document.widgets.iter_mut().find(|n|n.id==id){w.size=[r.width(),r.height()];}
        }else if let Some(w)=self.document.widgets.iter_mut().find(|n|n.id==id) {
            w.relative=None;
            if w.flow{
                let width=(layout.content.width()-40.0).max(1.0);
                w.span=((r.width()+12.0)/(width+12.0)*12.0).round().clamp(1.0,12.0) as u8;
                w.size[1]=r.height().clamp(44.0,1000.0);
            }else{
                w.size=[r.width().clamp(44.0,1000.0),r.height().clamp(44.0,1000.0)];
                w.position=Position::from_rect(r,layout.bounds,snap);
            }
        }
    }
    fn inspector_ui(&mut self,ctx:&egui::Context,layout:&Layout,theme:&ThemePreset){
        if self.background_open {
            egui::Window::new("Фон рабочего пространства").id(Id::new("composition_background_inspector")).order(egui::Order::Foreground).resizable(false).collapsible(false).default_width(250.0).constrain_to(layout.bounds.shrink(8.0)).vscroll(true).max_height((layout.bounds.height()-32.0).max(120.0)).show(ctx,|ui|{
                let mut col=self.document.background.unwrap_or(theme.background);
                if ui.color_edit_button_srgba_unmultiplied(&mut col).changed(){self.document.background=Some(col);}
                if ui.button("Использовать фон темы").clicked(){self.document.background=None;}
                ui.label("Цвет перекрывает изображение только в этом пресете.");
            });
        }
        if !self.inspector{return}
        let Some(id)=self.selected else{return};
        let is_panel=self.document.panels.iter().any(|p|p.id==id);
        let name=self.document.widgets.iter().find(|w|w.id==id).map(|w|format!("{} · {}",w.action.label(),w.id)).unwrap_or_else(||format!("Панель {id}"));
        let rect=self.rects.iter().chain(self.panel_rects.iter()).find(|(i,_)|*i==id).map(|(_,r)|*r).unwrap_or(layout.content);
        let pos=pos2((rect.right()+12.0).min(layout.bounds.right()-280.0).max(layout.bounds.left()+8.0),(rect.top()+34.0).min(layout.bounds.bottom()-290.0).max(layout.bounds.top()));
        let mut remove=false;let mut parent_select=None;let mut close=false;
        let window=egui::Window::new(name.clone()).id(Id::new(("local_inspector",id))).order(egui::Order::Foreground).fixed_pos(pos).default_width(252.0).default_height(340.0).min_height(120.0).resizable(false).collapsible(false).title_bar(false).constrain_to(layout.bounds.shrink(8.0)).max_height((layout.bounds.height()-32.0).max(120.0)).vscroll(true).show(ctx,|ui|{
            ui.label(egui::RichText::new(name).size(15.0).strong());
            ui.label(egui::RichText::new("Только выбранный элемент").size(11.0).color(theme.text_tertiary()));
            if let Some(w)=self.document.widgets.iter_mut().find(|w|w.id==id){
                if let Some(parent)=w.panel{if ui.small_button("Выбрать родительскую панель").clicked(){parent_select=Some(parent);}}
                ui.add(egui::TextEdit::singleline(&mut w.label).char_limit(80).desired_width(236.0));
                if w.panel.and_then(|id|self.document.panels.iter().find(|p|p.id==id)).is_some_and(|p|p.vertical){
                    ui.checkbox(&mut w.bottom,"Прижать вниз");
                }
                egui::CollapsingHeader::new("Размещение и страницы").id_salt(("placement",id)).show(ui,|ui|{
                if w.panel.is_none(){
                    ui.checkbox(&mut w.flow,"В потоке страницы");
                    if w.flow {ui.add(egui::Slider::new(&mut w.span,1..=12).text("Доля строки"));}else{anchor_ui(ui,&mut w.position);}
                }
                egui::ComboBox::from_id_salt("visibility").selected_text(w.page.map(|p|p.label()).unwrap_or("Все страницы")).show_ui(ui,|ui|{
                    for page in [None,Some(Action::Home),Some(Action::Library),Some(Action::Settings)]{
                        ui.selectable_value(&mut w.page,page,page.map(|p|p.label()).unwrap_or("Все страницы"));
                    }
                });
                });
            }
            if let Some(p)=self.document.panels.iter_mut().find(|p|p.id==id){
                ui.checkbox(&mut p.vertical,"Вертикальное расположение");
                ui.label("Перетяни панель к краю или на свободное место.");
                if p.edge==Edge::Float{anchor_ui(ui,&mut p.position);}
            }
            let is_launch=self.document.widgets.iter().any(|w|w.id==id&&w.action==Action::Launch);
            let style=if is_panel{self.document.panels.iter_mut().find(|p|p.id==id).map(|p|&mut p.style)}else{self.document.widgets.iter_mut().find(|p|p.id==id).map(|p|&mut p.style)};
            if let Some(style)=style{
                let mut radius=style.rounding.unwrap_or(if is_panel{0.0}else{8.0});
                let slider=ui.add(egui::Slider::new(&mut radius,0.0..=40.0).text("Скругление").show_value(false));
                self.controls.push(("rounding",slider.rect));
                if slider.changed(){style.rounding=Some(radius);}
                let mut fill=style.fill.unwrap_or(if is_launch{theme.modules.play_button.fill_or(theme.accent_color()).to_array()}else{theme.surface(2).to_array()});
                ui.horizontal(|ui|{ui.label("Заливка");if ui.color_edit_button_srgba_unmultiplied(&mut fill).changed(){style.fill=Some(fill);}});
                let mut opacity=fill[3] as f32/255.0;
                if ui.add(egui::Slider::new(&mut opacity,0.0..=1.0).text("Непрозрачность")).changed(){fill[3]=(opacity*255.0) as u8;style.fill=Some(fill);}
                ui.checkbox(&mut style.blur,"Размытый фон под элементом");
                ui.label(egui::RichText::new("Кэш фона, не live-blur соседних элементов.").size(10.0).color(theme.text_tertiary()));
                if ui.button("Вернуть стиль темы").clicked(){*style=Default::default();}
            }
            ui.separator();
            ui.horizontal(|ui|{
                if ui.button("Удалить").clicked(){remove=true;}
                if ui.button("Закрыть").clicked(){close=true;}
            });
            ui.label(egui::RichText::new(if is_panel{"Удаляет панель и её кнопки, не данные игры. Можно отменить."}else{"Удаляет компонент, не игровые данные. Можно отменить."}).size(11.0).color(theme.text_tertiary()));
        });
        if let Some(window)=window{self.controls.push(("inspector",window.response.rect));}
        if remove{self.document.remove(id);self.selected=None;self.inspector=false;}
        if close{self.inspector=false;}
        if let Some(p)=parent_select{self.selected=Some(p);}
    }
    fn show_error(&mut self,ctx:&egui::Context){
        if let Some(error)=self.error.clone(){
            egui::Window::new("Интерфейс: требуется внимание").id(Id::new("layout_error")).order(egui::Order::Foreground).collapsible(false).resizable(false).show(ctx,|ui|{
                ui.set_max_width(480.0);ui.label(error);
                if ui.button("Закрыть сообщение").clicked(){self.error=None;}
            });
        }
    }
}
fn rgba(c:[u8;4])->Color32{Color32::from_rgba_unmultiplied(c[0],c[1],c[2],c[3])}
fn near_edge(p:Pos2,r:Rect)->Edge {
    let candidates=[(p.x-r.left(),Edge::Left),(r.right()-p.x,Edge::Right),(p.y-r.top(),Edge::Top),(r.bottom()-p.y,Edge::Bottom)];
    candidates.into_iter().filter(|(d,_)|*d<40.0).min_by(|a,b|a.0.total_cmp(&b.0)).map(|(_,e)|e).unwrap_or(Edge::Float)
}
fn panel_preview(edge:Edge,p:Pos2,r:Rect)->Rect {
    match edge{
        Edge::Top=>Rect::from_min_size(r.min,vec2(r.width(),72.0)),
        Edge::Bottom=>Rect::from_min_size(pos2(r.left(),r.bottom()-72.0),vec2(r.width(),72.0)),
        Edge::Left=>Rect::from_min_size(r.min,vec2(184.0,r.height())),
        Edge::Right=>Rect::from_min_size(pos2(r.right()-184.0,r.top()),vec2(184.0,r.height())),
        Edge::Float=>Position::from_rect(Rect::from_min_size(p,vec2(320.0,100.0)),r,true).rect(r,vec2(320.0,100.0)),
    }
}
fn gear_icon(p:&egui::Painter,c:Pos2,color:Color32){
    p.circle_stroke(c,5.0,Stroke::new(1.5_f32,color));p.circle_stroke(c,1.5,Stroke::new(1.2_f32,color));
    for i in 0..8 {let a=i as f32*std::f32::consts::TAU/8.0;let d=vec2(a.cos(),a.sin());p.line_segment([c+d*5.0,c+d*8.0],Stroke::new(2.0_f32,color));}
}
fn anchor_ui(ui:&mut egui::Ui,p:&mut Position){
    egui::ComboBox::from_id_salt("anchor").selected_text(match p.anchor{Anchor::TopLeft=>"Слева сверху",Anchor::TopRight=>"Справа сверху",Anchor::BottomLeft=>"Слева снизу",Anchor::BottomRight=>"Справа снизу"}).show_ui(ui,|ui|{
        for (a,label) in [(Anchor::TopLeft,"Слева сверху"),(Anchor::TopRight,"Справа сверху"),(Anchor::BottomLeft,"Слева снизу"),(Anchor::BottomRight,"Справа снизу")]{ui.selectable_value(&mut p.anchor,a,label);}
    });
}
#[cfg(test)]
impl Editor {
    pub fn test_select(&mut self,id:NodeId){self.selected=Some(id);self.inspector=true;}
}
#[cfg(test)]
mod transaction_tests {
    use super::*;
    #[test]
    fn pending_text_edit_undo_does_not_eat_previous_operation(){
        let mut e=Editor::default();e.begin();
        e.change(|d|{d.add_panel(Edge::Top,Position::default());});
        let before=e.document.clone();
        e.gesture_before=Some(before.clone());
        e.document.widgets[1].label="Мои сборки".into();
        e.undo();
        assert_eq!(e.document,before);
        assert_eq!(e.document.panels.len(),2);
        e.redo();assert_eq!(e.document.widgets[1].label,"Мои сборки");
        e.cancel();assert_eq!(e.document,Document::default());
        assert!(!e.history.can_undo());assert!(!e.history.can_redo());
    }
}