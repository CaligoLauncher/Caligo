//! Versioned shell composition. No credentials, game paths or executable content.
//! Rendering constraints never mutate this document.
use eframe::egui::{pos2,vec2,Pos2,Rect,Vec2};
use serde::{Deserialize,Serialize};
use std::{collections::HashSet,fs,io::{Read,Write},path::Path};

pub type NodeId=u64;
pub const VERSION:u32=1;
const MAX_BYTES:u64=256*1024;
#[derive(Clone,Copy,Debug,Serialize,Deserialize,PartialEq,Eq)]
pub enum Action { Home,Library,Settings,Profile }
impl Action {
    pub fn label(self)->&'static str {match self {Self::Home=>"Главная",Self::Library=>"Сборки",Self::Settings=>"Настройки",Self::Profile=>"Профиль"}}
}
#[derive(Clone,Copy,Debug,Serialize,Deserialize,PartialEq,Eq)]
pub enum Edge { Left,Right,Top,Bottom,Float }
#[derive(Clone,Copy,Debug,Serialize,Deserialize,PartialEq,Eq)]
pub enum Anchor { TopLeft,TopRight,BottomLeft,BottomRight }
#[derive(Clone,Copy,Debug,Serialize,Deserialize,PartialEq)]
pub struct Position {pub anchor:Anchor,pub offset:[f32;2]}
impl Default for Position {fn default()->Self {Self{anchor:Anchor::TopLeft,offset:[24.0,24.0]}}}
impl Position {
    pub fn rect(self,bounds:Rect,size:Vec2)->Rect {
        let size=size.min(bounds.size().max(vec2(1.0,1.0)));
        let [x,y]=self.offset;
        let p=match self.anchor {
            Anchor::TopLeft=>bounds.min+vec2(x,y),
            Anchor::TopRight=>pos2(bounds.right()-size.x-x,bounds.top()+y),
            Anchor::BottomLeft=>pos2(bounds.left()+x,bounds.bottom()-size.y-y),
            Anchor::BottomRight=>bounds.max-size-vec2(x,y),
        };
        Rect::from_min_size(pos2(p.x.clamp(bounds.left(),(bounds.right()-size.x).max(bounds.left())),
            p.y.clamp(bounds.top(),(bounds.bottom()-size.y).max(bounds.top()))),size)
    }
    pub fn from_rect(rect:Rect,bounds:Rect,snap:bool)->Self {
        let right=rect.center().x>bounds.center().x;
        let bottom=rect.center().y>bounds.center().y;
        let quant=|v:f32| if snap {(v.max(0.0)/8.0).round()*8.0}else{v.max(0.0)};
        Self{anchor:match(right,bottom){(false,false)=>Anchor::TopLeft,(true,false)=>Anchor::TopRight,(false,true)=>Anchor::BottomLeft,(true,true)=>Anchor::BottomRight},
            offset:[quant(if right{bounds.right()-rect.right()}else{rect.left()-bounds.left()}),
                    quant(if bottom{bounds.bottom()-rect.bottom()}else{rect.top()-bounds.top()})]}
    }
}
#[derive(Clone,Debug,Default,Serialize,Deserialize,PartialEq)]
pub struct Style {pub rounding:Option<f32>,pub fill:Option<[u8;4]>}
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq)]
pub struct Panel {
    pub id:NodeId,pub edge:Edge,pub size:[f32;2],pub position:Position,
    pub vertical:bool,pub style:Style,
}
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq)]
pub struct Widget {
    pub id:NodeId,pub action:Action,pub label:String,pub panel:Option<NodeId>,
    pub position:Position,pub size:[f32;2],pub style:Style,pub bottom:bool,
}
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq)]
pub struct Document {
    pub version:u32,pub next_id:NodeId,pub panels:Vec<Panel>,pub widgets:Vec<Widget>,
    pub background:Option<[u8;4]>,
}
impl Default for Document {
    fn default()->Self {
        Self{version:VERSION,next_id:5,background:None,
            panels:vec![Panel{id:1,edge:Edge::Left,size:[184.0,64.0],position:Position::default(),vertical:true,style:Style::default()}],
            widgets:vec![
                Widget{id:2,action:Action::Home,label:"Главная".into(),panel:Some(1),position:Position::default(),size:[160.0,44.0],style:Style::default(),bottom:false},
                Widget{id:3,action:Action::Library,label:"Сборки".into(),panel:Some(1),position:Position::default(),size:[160.0,44.0],style:Style::default(),bottom:false},
                Widget{id:4,action:Action::Settings,label:"Настройки".into(),panel:Some(1),position:Position::default(),size:[160.0,44.0],style:Style::default(),bottom:true},
            ]}
    }
}
impl Document {
    pub fn validate(&self)->Result<(),String> {
        if self.version!=VERSION {return Err(format!("Неподдерживаемая версия интерфейса: {}. Файлы не изменены.",self.version))}
        if self.panels.len()>12||self.widgets.len()>64 {return Err("Слишком много элементов (до 12 панелей и 64 кнопок)".into())}
        let mut ids=HashSet::new();
        for id in self.panels.iter().map(|p|p.id).chain(self.widgets.iter().map(|w|w.id)) {
            if id==0||id>=self.next_id||!ids.insert(id){return Err("Повреждены ID элементов".into())}
        }
        if self.next_id>1_000_000_000{return Err("Исчерпан диапазон ID".into())}
        fn dimensions(s:[f32;2])->bool {s.iter().all(|n|n.is_finite()&&*n>=32.0&&*n<=2000.0)}
        fn position(p:Position)->bool {p.offset.iter().all(|n|n.is_finite()&&*n>=0.0&&*n<=100_000.0)}
        fn style(s:&Style)->bool {s.rounding.is_none_or(|r|r.is_finite()&&(0.0..=100.0).contains(&r))}
        if self.panels.iter().any(|p|!dimensions(p.size)||!position(p.position)||!style(&p.style)) ||
           self.widgets.iter().any(|w|!dimensions(w.size)||!position(w.position)||!style(&w.style)||w.label.chars().count()>80||w.label.chars().any(char::is_control)||
               w.panel.is_some_and(|id|!self.panels.iter().any(|p|p.id==id))) {
            return Err("Некорректные размеры, стиль, подпись или родитель".into())
        }
        Ok(())
    }
    pub fn add_panel(&mut self,edge:Edge,position:Position)->NodeId {
        let id=self.next_id;self.next_id+=1;
        self.panels.push(Panel{id,edge,position,size:if edge==Edge::Float{[320.0,100.0]}else{[184.0,72.0]},
            vertical:matches!(edge,Edge::Left|Edge::Right),style:Style::default()});id
    }
    pub fn add_widget(&mut self,action:Action)->NodeId {
        let id=self.next_id;self.next_id+=1;
        self.widgets.push(Widget{id,action,label:action.label().into(),panel:None,position:Position::default(),
            size:[160.0,44.0],style:Style::default(),bottom:false});id
    }
    pub fn remove(&mut self,id:NodeId) {
        self.panels.retain(|p|p.id!=id);
        self.widgets.retain(|w|w.id!=id&&w.panel!=Some(id));
    }
    pub fn move_widget(&mut self,id:NodeId,parent:Option<NodeId>,before:Option<NodeId>,position:Position) {
        if parent.is_some_and(|p|!self.panels.iter().any(|x|x.id==p)){return}
        let Some(i)=self.widgets.iter().position(|w|w.id==id)else{return};
        let mut w=self.widgets.remove(i);w.panel=parent;w.position=position;w.bottom=false;
        let at=before.and_then(|b|self.widgets.iter().position(|w|w.id==b&&w.panel==parent)).unwrap_or(self.widgets.len());
        self.widgets.insert(at,w);
    }
}
#[derive(Clone,Debug)]
pub struct Layout {pub panels:Vec<(NodeId,Rect)>,pub content:Rect,pub bounds:Rect}
impl Layout {
    pub fn compute(doc:&Document,bounds:Rect)->Self {
        let mut available=bounds;let mut panels=Vec::new();
        for p in doc.panels.iter().filter(|p|p.edge!=Edge::Float) {
            let horizontal=matches!(p.edge,Edge::Top|Edge::Bottom);
            // Preserve a usable content viewport even with several panels.
            let limit=if horizontal{(available.height()-180.0).max(0.0)}else{(available.width()-320.0).max(0.0)};
            let requested=if horizontal{p.size[1]}else if bounds.width()<880.0{p.size[0].min(68.0)}else{p.size[0]};
            let thickness=requested.min(limit);
            let mut r=available;
            match p.edge {
                Edge::Left=>{r.max.x=r.min.x+thickness;available.min.x=r.max.x;}
                Edge::Right=>{r.min.x=r.max.x-thickness;available.max.x=r.min.x;}
                Edge::Top=>{r.max.y=r.min.y+thickness;available.min.y=r.max.y;}
                Edge::Bottom=>{r.min.y=r.max.y-thickness;available.max.y=r.min.y;}
                _=>{}
            }
            panels.push((p.id,r));
        }
        for p in doc.panels.iter().filter(|p|p.edge==Edge::Float){
            panels.push((p.id,p.position.rect(bounds,vec2(p.size[0],p.size[1]))));
        }
        Self{panels,content:available,bounds}
    }
    pub fn panel(&self,id:NodeId)->Option<Rect>{self.panels.iter().find(|(i,_)|*i==id).map(|(_,r)|*r)}
}

/// Two-slot journal: a save never truncates the last valid slot. No rename gap on Windows.
#[derive(Clone,Debug,Serialize,Deserialize)]
struct Snapshot {generation:u64,document:Document}
pub fn load(dir:&Path)->Result<Option<(u64,Document)>,String>{
    let mut valid=Vec::new();let mut errors=Vec::new();
    for name in ["interface-a.json","interface-b.json"]{
        let path=dir.join(name);if !path.exists(){continue}
        let result=(||->Result<Snapshot,String>{
            let file=fs::File::open(&path).map_err(|e|e.to_string())?;
            let mut text=String::new();file.take(MAX_BYTES+1).read_to_string(&mut text).map_err(|e|e.to_string())?;
            if text.len() as u64>MAX_BYTES{return Err("Файл интерфейса слишком велик".into())}
            let snap:Snapshot=serde_json::from_str(&text).map_err(|e|e.to_string())?;
            // Future schema must not silently fall back and overwrite new data.
            if snap.document.version!=VERSION{return Err(format!("FUTURE: версия {}",snap.document.version))}
            snap.document.validate()?;Ok(snap)
        })();
        match result {Ok(s)=>valid.push(s),Err(e)=>{if e.starts_with("FUTURE:"){return Err(e)}errors.push(e);}}
    }
    if let Some(s)=valid.into_iter().max_by_key(|s|s.generation){return Ok(Some((s.generation,s.document)))}
    if errors.is_empty(){Ok(None)}else{Err(format!("Не удалось прочитать интерфейс; оригиналы сохранены: {}",errors.join("; ")))}
}
pub fn save(dir:&Path,expected:u64,document:&Document)->Result<u64,String>{
    document.validate()?;
    let current=load(dir)?.map(|(g,_)|g).unwrap_or(0);
    if current!=expected{return Err("Интерфейс изменён другим окном. Перезапусти Caligo перед сохранением.".into())}
    let generation=current.checked_add(1).ok_or("Слишком много сохранений")?;
    let text=serde_json::to_vec_pretty(&Snapshot{generation,document:document.clone()}).map_err(|e|e.to_string())?;
    if text.len() as u64>MAX_BYTES{return Err("Пресет превышает допустимый размер".into())}
    fs::create_dir_all(dir).map_err(|e|e.to_string())?;
    // Cooperative lock prevents two Caligo processes racing between load and write.
    let lock_path=dir.join("interface.lock");
    let lock=fs::OpenOptions::new().create_new(true).write(true).open(&lock_path)
        .map_err(|_|"Сохранение занято. Если Caligo аварийно завершился, закрой его окна и удали interface.lock из каталога данных.")?;
    let result=(||->Result<u64,String>{
        let current=load(dir)?.map(|(g,_)|g).unwrap_or(0);
        if current!=expected{return Err("Обнаружено одновременное изменение интерфейса".into())}
        let path=dir.join(if generation%2==1{"interface-a.json"}else{"interface-b.json"});
        let mut f=fs::OpenOptions::new().create(true).write(true).truncate(true).open(&path).map_err(|e|e.to_string())?;
        if let Err(e)=f.write_all(&text).and_then(|_|f.sync_all()){
            drop(f);let _=fs::remove_file(path);return Err(e.to_string())
        }
        Ok(generation)
    })();
    drop(lock);let _=fs::remove_file(lock_path);result
}

#[derive(Default)]
pub struct History {past:Vec<Document>,future:Vec<Document>}
impl History {
    pub fn record(&mut self,before:Document,after:&Document){
        if &before==after{return}
        self.past.push(before);if self.past.len()>80{self.past.remove(0);}
        self.future.clear();
    }
    pub fn undo(&mut self,doc:&mut Document){if let Some(prev)=self.past.pop(){self.future.push(std::mem::replace(doc,prev));}}
    pub fn redo(&mut self,doc:&mut Document){if let Some(next)=self.future.pop(){self.past.push(std::mem::replace(doc,next));}}
    pub fn can_undo(&self)->bool{!self.past.is_empty()}
    pub fn can_redo(&self)->bool{!self.future.is_empty()}
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn document_roundtrip_and_validation(){
        let d=Document::default();assert_eq!(serde_json::from_str::<Document>(&serde_json::to_string(&d).unwrap()).unwrap(),d);
        assert!(d.validate().is_ok());let mut bad=d.clone();bad.widgets[0].id=1;assert!(bad.validate().is_err());
        bad=d.clone();bad.widgets[0].panel=Some(100);assert!(bad.validate().is_err());
        bad=d.clone();bad.widgets[0].size[0]=f32::NAN;assert!(bad.validate().is_err());
        bad=d;bad.version=999;assert!(bad.validate().is_err());
    }
    #[test] fn move_is_not_copy_and_undo_keeps_identity(){
        let mut d=Document::default();let initial=d.clone();let mut h=History::default();
        let panel=d.add_panel(Edge::Top,Position::default());
        d.move_widget(3,Some(panel),None,Position::default());h.record(initial.clone(),&d);
        assert_eq!(d.widgets.len(),3);assert_eq!(d.widgets.iter().find(|w|w.id==3).unwrap().action,Action::Library);
        h.undo(&mut d);assert_eq!(d,initial);h.redo(&mut d);assert_eq!(d.widgets.iter().find(|w|w.id==3).unwrap().panel,Some(panel));
    }
    #[test] fn resize_layout_does_not_rewrite_document(){
        let mut d=Document::default();d.widgets[1].panel=None;d.widgets[1].position=Position{anchor:Anchor::BottomRight,offset:[16.0,16.0]};
        let original=d.clone();
        for size in [vec2(1000.0,572.0),vec2(720.0,392.0),vec2(1000.0,572.0)] {
            let l=Layout::compute(&d,Rect::from_min_size(Pos2::ZERO,size));
            assert!(l.content.width()>=320.0);
            assert!(l.bounds.contains_rect(d.widgets[1].position.rect(l.bounds,vec2(160.0,44.0))));
        }
        assert_eq!(d,original);
    }
    #[test] fn local_style_and_panel_removal_are_independent(){
        let mut d=Document::default();d.widgets[1].style.rounding=Some(20.0);
        assert_eq!(d.widgets[0].style.rounding,None);
        d.remove(1);assert!(d.panels.is_empty()&&d.widgets.is_empty());assert!(d.validate().is_ok());
    }
    #[test] fn persistence_recovers_last_slot_and_rejects_stale_writer(){
        let dir=std::env::temp_dir().join(format!("caligo-layout-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let d=Document::default();assert_eq!(save(&dir,0,&d).unwrap(),1);
        let mut newer=d.clone();newer.background=Some([1,2,3,255]);assert_eq!(save(&dir,1,&newer).unwrap(),2);
        assert!(save(&dir,1,&d).is_err());assert_eq!(load(&dir).unwrap().unwrap().1,newer);
        fs::write(dir.join("interface-b.json"),"{broken").unwrap();assert_eq!(load(&dir).unwrap().unwrap(),(1,d));
        fs::remove_dir_all(dir).unwrap();
    }
}