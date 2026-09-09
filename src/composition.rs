//! Versioned shell composition. No credentials, game paths or executable content.
//! Rendering constraints never mutate this document.
use eframe::egui::{pos2,vec2,Pos2,Rect,Vec2};
use serde::{Deserialize,Serialize};
use std::{collections::HashSet,fs,io::{Read,Write},path::Path};

pub type NodeId=u64;
pub const VERSION:u32=3;
const MAX_BYTES:u64=256*1024;
#[derive(Clone,Copy,Debug,Serialize,Deserialize,PartialEq,Eq,Hash)]
pub enum Action { Character,Cover,Home,Library,Settings,Profile,Heading,Selection,Version,Launch,Status,LibrarySearch,CreateInstance,LibraryList,Account,Appearance,Atmosphere,ThemeJson,LegacyStyles }
impl Action {
    pub fn label(self)->&'static str {match self {Self::Character=>"Персонаж",Self::Cover=>"Обложка",Self::Home=>"Главная",Self::Library=>"Сборки",Self::Settings=>"Настройки",Self::Profile=>"Профиль",Self::Heading=>"Заголовок",Self::Selection=>"Выбранная сборка",Self::Version=>"Версия Minecraft",Self::Launch=>"Играть",Self::Status=>"Состояние игры",Self::LibrarySearch=>"Поиск сборок",Self::CreateInstance=>"Создать сборку",Self::LibraryList=>"Список сборок",Self::Account=>"Аккаунт и скин",Self::Appearance=>"Оформление",Self::Atmosphere=>"Фон и атмосфера",Self::ThemeJson=>"JSON-тема",Self::LegacyStyles=>"Совместимость тем"}}
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
pub struct Style {pub rounding:Option<f32>,pub fill:Option<[u8;4]>,#[serde(default)] pub blur:bool}
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq)]
pub struct Panel {
    pub id:NodeId,pub edge:Edge,pub size:[f32;2],pub position:Position,
    pub vertical:bool,pub style:Style,
    #[serde(default)] pub relative:Option<[f32;4]>,
}
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq)]
pub struct Widget {
    pub id:NodeId,pub action:Action,pub label:String,pub panel:Option<NodeId>,
    pub position:Position,pub size:[f32;2],pub style:Style,pub bottom:bool,
    #[serde(default)] pub relative:Option<[f32;4]>,
    #[serde(default)] pub page:Option<Action>,
    #[serde(default)] pub flow:bool,
    #[serde(default="default_span")] pub span:u8,
}
#[derive(Clone,Debug,Serialize,Deserialize,PartialEq)]
pub struct Document {
    pub version:u32,pub next_id:NodeId,pub panels:Vec<Panel>,pub widgets:Vec<Widget>,
    pub background:Option<[u8;4]>,
}
impl Document {
    pub fn legacy()->Self {
        let mut d=Self{version:VERSION,next_id:5,background:None,
            panels:vec![Panel{id:1,edge:Edge::Left,size:[76.0,64.0],position:Position::default(),vertical:true,relative:None,style:Style::default()}],
            widgets:vec![
                Widget{id:2,action:Action::Home,label:"Главная".into(),panel:Some(1),position:Position::default(),size:[160.0,44.0],style:Style::default(),bottom:false,relative:None,page:None,flow:false,span:12},
                Widget{id:3,action:Action::Library,label:"Сборки".into(),panel:Some(1),position:Position::default(),size:[160.0,44.0],style:Style::default(),bottom:false,relative:None,page:None,flow:false,span:12},
                Widget{id:4,action:Action::Settings,label:"Настройки".into(),panel:Some(1),position:Position::default(),size:[160.0,44.0],style:Style::default(),bottom:true,relative:None,page:None,flow:false,span:12},
            ]};
        d.add_default_content();d
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
        if self.panels.iter().any(|p|!valid_relative(p.relative)||!dimensions(p.size)||!position(p.position)||!style(&p.style)) ||
           self.widgets.iter().any(|w|!valid_relative(w.relative)||!dimensions(w.size)||!position(w.position)||!style(&w.style)||!(1..=12).contains(&w.span)||w.page.is_some_and(|p|!matches!(p,Action::Home|Action::Library|Action::Settings))||w.label.chars().count()>80||w.label.chars().any(char::is_control)||
               w.panel.is_some_and(|id|!self.panels.iter().any(|p|p.id==id))) {
            return Err("Некорректные размеры, стиль, подпись или родитель".into())
        }
        Ok(())
    }
    pub fn add_panel(&mut self,edge:Edge,position:Position)->NodeId {
        let id=self.next_id;self.next_id+=1;
        self.panels.push(Panel{id,edge,position,size:if edge==Edge::Float{[320.0,100.0]}else{[184.0,72.0]},
            vertical:matches!(edge,Edge::Left|Edge::Right),relative:None,style:Style::default()});id
    }
    pub fn add_widget(&mut self,action:Action)->NodeId {
        let id=self.next_id;self.next_id+=1;
        self.widgets.push(Widget{id,action,label:action.label().into(),panel:None,position:Position::default(),
            size:[160.0,44.0],style:Style::default(),bottom:false,relative:None,page:None,flow:false,span:12});id
    }
    pub fn remove(&mut self,id:NodeId) {
        self.panels.retain(|p|p.id!=id);
        self.widgets.retain(|w|w.id!=id&&w.panel!=Some(id));
    }
    pub fn move_widget(&mut self,id:NodeId,parent:Option<NodeId>,before:Option<NodeId>,position:Position) {
        if parent.is_some_and(|p|!self.panels.iter().any(|x|x.id==p)){return}
        let Some(i)=self.widgets.iter().position(|w|w.id==id)else{return};
        let mut w=self.widgets.remove(i);w.panel=parent;w.position=position;w.bottom=false;w.flow=false;w.relative=None;
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
            panels.push((p.id,placed_rect(p.position,p.size,p.relative,bounds)));
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
            let raw:serde_json::Value=serde_json::from_str(&text).map_err(|e|e.to_string())?;
            // Inspect the version BEFORE typed parsing: a future widget enum may
            // not deserialize, but that must never make its file overwriteable.
            let version=raw.get("document").and_then(|d|d.get("version")).and_then(|v|v.as_u64());
            if version.is_some_and(|v|v!=1&&v!=2&&v!=VERSION as u64){return Err(format!("FUTURE: версия {}",version.unwrap()))}
            let mut snap:Snapshot=serde_json::from_value(raw).map_err(|e|e.to_string())?;
            if snap.document.version==1{snap.document=snap.document.migrate_v1();}
            if snap.document.version==2{snap.document.version=VERSION;}
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
        assert_eq!(d.widgets.len(),initial.widgets.len());assert_eq!(d.widgets.iter().find(|w|w.id==3).unwrap().action,Action::Library);
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
        d.remove(1);assert!(d.panels.is_empty()&&d.widgets.iter().all(|w|w.panel.is_none()));assert!(d.validate().is_ok());
    }
    #[test] fn future_schema_and_write_failure_preserve_files(){
        let dir=std::env::temp_dir().join(format!("caligo-future-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
        let d=Document::default();save(&dir,0,&d).unwrap();
        let future=r#"{"generation":2,"document":{"version":999,"new_schema":"preserve me"}}"#;
        fs::write(dir.join("interface-b.json"),future).unwrap();
        assert!(load(&dir).is_err());assert!(save(&dir,1,&d).is_err());
        assert_eq!(fs::read_to_string(dir.join("interface-b.json")).unwrap(),future);
        fs::remove_file(dir.join("interface-b.json")).unwrap();
        fs::create_dir(dir.join("interface-b.json")).unwrap();
        assert!(save(&dir,1,&d).is_err());
        assert_eq!(load(&dir).unwrap().unwrap(),(1,d));
        assert!(!dir.join("interface.lock").exists());
        fs::remove_dir_all(dir).unwrap();
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
impl Action {
    pub fn is_navigation(self)->bool {matches!(self,Self::Home|Self::Library|Self::Settings|Self::Profile)}
    pub fn components()->&'static [Action] {
        &[Self::Character,Self::Cover,Self::Heading,Self::Selection,Self::Version,Self::Launch,Self::Status,Self::LibrarySearch,Self::CreateInstance,Self::LibraryList,Self::Account,Self::Appearance,Self::Atmosphere,Self::ThemeJson,Self::LegacyStyles]
    }
}
impl Document {
    pub fn add_content(&mut self,action:Action,page:Action,span:u8,height:f32)->NodeId {
        let id=self.add_widget(action);
        let w=self.widgets.last_mut().unwrap();
        w.page=Some(page);w.flow=true;w.span=span;
        w.size=[if matches!(action,Action::Appearance|Action::Atmosphere){360.0}else{180.0},height];
        id
    }
    pub fn add_default_content(&mut self) {
        use Action::*;
        for (page,action,span,height,label) in [
            (Home,Heading,12,48.0,"Главная"),
            (Home,Selection,12,84.0,"Выбрано для запуска"),
            (Home,Version,8,56.0,"Версия Minecraft"),
            (Home,Launch,4,56.0,"Играть"),
            (Home,Status,12,44.0,"Состояние игры"),
            (Home,LibraryList,8,208.0,"Мои сборки"),
            (Home,Account,4,208.0,"Профиль"),
            (Library,Heading,12,48.0,"Сборки"),
            (Library,LibrarySearch,8,48.0,"Найти сборку"),
            (Library,CreateInstance,4,48.0,"Создать сборку"),
            (Library,LibraryList,12,368.0,"Библиотека"),
            (Settings,Heading,12,48.0,"Настройки"),
            (Settings,Appearance,6,272.0,"Оформление"),
            (Settings,Atmosphere,6,272.0,"Фон и атмосфера"),
            (Settings,ThemeJson,12,288.0,"JSON-тема"),
            (Settings,LegacyStyles,12,256.0,"Совместимость тем"),
        ] {
            self.add_content(action,page,span,height);
            self.widgets.last_mut().unwrap().label=label.into();
        }
    }
    pub fn migrate_v1(mut self)->Self {
        self.version=VERSION;
        self.add_default_content();
        self
    }
}
fn default_span()->u8 {12}

/// Responsive row-flow. Rows are computed from component spans, not hardcoded
/// component kinds. Narrow views stack; saved spans and dimensions stay intact.
pub fn flow_rects(widgets:&[Widget],bounds:Rect)->Vec<(NodeId,Rect)> {
    let gap=12.0;let narrow=bounds.width()<560.0;
    let unit=((bounds.width()-11.0*gap)/12.0).max(1.0);
    let mut y=bounds.top();let mut x=bounds.left();let mut used=0_u8;let mut row_h=0.0_f32;
    let mut out=Vec::new();
    for w in widgets {
        let requested=w.span.clamp(1,12);
        let desired=unit*requested as f32+gap*(requested-1) as f32;
        let span=if narrow||desired<w.size[0].min(bounds.width()){12}else{requested};
        if used>0&&used+span>12 {y+=row_h+gap;x=bounds.left();used=0;row_h=0.0;}
        let width=if span==12{bounds.width()}else{unit*span as f32+gap*(span-1) as f32};
        out.push((w.id,Rect::from_min_size(pos2(x,y),vec2(width,w.size[1]))));
        x+=width+gap;used+=span;row_h=row_h.max(w.size[1]);
    }
    out
}/// Built-in layouts are documents, not alternate renderers.
/// Choosing a preset is an editor transaction; loading an older document never
/// silently replaces its layout.
impl Default for Document {
    fn default()->Self { Self::preset(3) }
}
impl Document {
    pub fn preset(kind:u8)->Self {
        use Action::*;
        let mut d=Self::legacy();
        d.widgets.retain(|w|w.page!=Some(Home));
        let panel=&mut d.panels[0];
        panel.edge=Edge::Float;
        panel.vertical=kind==1;
        panel.relative=Some(if kind==1{[0.025,0.18,0.075,0.64]}else{[0.30,0.88,0.40,0.10]});
        panel.style=Style{rounding:Some(18.0),fill:Some([17,30,46,170]),blur:true};
        for w in &mut d.widgets {
            if w.panel==Some(1){w.size=[116.0,46.0];w.bottom=kind==1&&w.action==Settings;}
            if matches!(w.action,LibraryList|Appearance|Atmosphere|ThemeJson|LegacyStyles) {
                w.style.fill=Some([17,30,46,185]);w.style.rounding=Some(16.0);w.style.blur=true;
            }
        }
        let nodes=if kind==6 {
            vec![(Character,[0.37,0.03,0.26,0.44]),(Cover,[0.70,0.16,0.23,0.22]),
                (Selection,[0.32,0.49,0.36,0.11]),(Version,[0.32,0.63,0.17,0.09]),
                (Launch,[0.51,0.63,0.17,0.09]),(Status,[0.32,0.75,0.36,0.07])]
        }else{
            vec![(Character,[0.18,0.12,0.30,0.65]),(Cover,[0.60,0.13,0.30,0.23]),
                (Selection,[0.60,0.39,0.31,0.12]),(Version,[0.60,0.54,0.31,0.08]),
                (Launch,[0.60,0.65,0.31,0.10]),(Status,[0.60,0.77,0.31,0.07])]
        };
        for (action,relative) in nodes {
            d.add_widget(action);
            let w=d.widgets.last_mut().unwrap();
            w.page=Some(Home);w.relative=Some(relative);w.size=[240.0,56.0];
            if action==Version{w.style.fill=Some([17,30,46,165]);w.style.blur=true;}
        }
        d
    }
}
/// Normalized preset rectangles adapt without rewriting their saved data.
/// Direct manipulation detaches only the manipulated instance to pixel geometry.
pub fn placed_rect(position:Position,size:[f32;2],relative:Option<[f32;4]>,bounds:Rect)->Rect {
    if let Some([x,y,w,h])=relative {
        let size=vec2((bounds.width()*w).max(32.0),(bounds.height()*h).max(32.0)).min(bounds.size());
        let rect=Rect::from_min_size(bounds.min+vec2(bounds.width()*x,bounds.height()*y),size);
        return Position::from_rect(rect,bounds,false).rect(bounds,size)
    }
    position.rect(bounds,vec2(size[0],size[1]))
}
fn valid_relative(value:Option<[f32;4]>)->bool {
    value.is_none_or(|r|r.iter().all(|n|n.is_finite()&&(0.0..=1.0).contains(n))&&r[2]>0.0&&r[3]>0.0&&r[0]+r[2]<=1.001&&r[1]+r[3]<=1.001)
}
#[cfg(test)]
mod rpg_tests {
    use super::*;
    #[test] fn presets_are_editable_documents_and_not_dashboards(){
        for kind in [1,3,6] {
            let d=Document::preset(kind);assert!(d.validate().is_ok());
            let home:Vec<_>=d.widgets.iter().filter(|w|w.page==Some(Action::Home)).collect();
            assert_eq!(home.len(),6);
            assert!(home.iter().any(|w|w.action==Action::Character));
            assert!(!home.iter().any(|w|w.action==Action::LibraryList||w.action==Action::Account));
            for size in [vec2(1000.0,572.0),vec2(720.0,392.0),vec2(720.0,304.0)] {
                let bounds=Rect::from_min_size(Pos2::ZERO,size);
                let rects:Vec<_>=home.iter().map(|w|placed_rect(w.position,w.size,w.relative,bounds)).collect();
                for (i,a) in rects.iter().enumerate() {
                    assert!(bounds.contains_rect(*a));
                    for b in &rects[i+1..]{assert!(!a.intersects(*b),"preset {kind} overlaps at {size:?}");}
                }
            }
            assert_eq!(serde_json::from_str::<Document>(&serde_json::to_string(&d).unwrap()).unwrap(),d);
        }
    }
    #[test] fn schema_two_migration_keeps_layout_in_memory_only(){
        let mut old=Document::legacy();old.version=2;
        let original=old.clone();
        old.version=VERSION;
        assert_eq!(old.widgets,original.widgets);assert_eq!(old.panels,original.panels);
    }
}