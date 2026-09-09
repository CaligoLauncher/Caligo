//! Editable scenes. One geometry model for every node, including containers.
//! This document contains presentation only: no credentials or executable paths.
use eframe::egui::{pos2, vec2, Pos2, Rect};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

pub type Id = u64;
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Page { Home, Library, Settings }
impl Page {
    pub const ALL: [Self; 3] = [Self::Home, Self::Library, Self::Settings];
    pub fn label(self) -> &'static str { match self { Self::Home=>"Главная",Self::Library=>"Сборки",Self::Settings=>"Настройки" } }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind { Panel, Text, Character, Selection, Version, Play, Status, Home, Library, Settings, Profile, Search, Create, List, Appearance, Background, Json }
impl Kind {
    pub const ALL: [Self; 17] = [Self::Panel,Self::Text,Self::Character,Self::Selection,Self::Version,Self::Play,Self::Status,Self::Home,Self::Library,Self::Settings,Self::Profile,Self::Search,Self::Create,Self::List,Self::Appearance,Self::Background,Self::Json];
    pub fn label(self)-> &'static str { match self {
        Self::Panel=>"Панель",Self::Text=>"Текст",Self::Character=>"Персонаж",Self::Selection=>"Название сборки",
        Self::Version=>"Версия",Self::Play=>"Играть",Self::Status=>"Состояние игры",Self::Home=>"Главная",
        Self::Library=>"Сборки",Self::Settings=>"Настройки",Self::Profile=>"Профиль",Self::Search=>"Поиск сборок",
        Self::Create=>"Создать сборку",Self::List=>"Список сборок",Self::Appearance=>"Цвет акцента",
        Self::Background=>"Свой фон",Self::Json=>"Экспорт интерфейса" } }
    pub fn button(self)->bool { matches!(self,Self::Play|Self::Version|Self::Home|Self::Library|Self::Settings|Self::Profile|Self::Create|Self::Appearance|Self::Background|Self::Json|Self::Search) }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Style { pub fill:[u8;4], pub ink:[u8;4], pub radius:f32, pub font:f32, pub blur:bool }
impl Default for Style {
    fn default()->Self { Self{fill:[20,29,40,120],ink:[237,243,247,255],radius:12.0,font:15.0,blur:false} }
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Node {
    pub id:Id, pub parent:Option<Id>, pub page:Option<Page>, pub kind:Kind, pub name:String,
    /// Unit rectangle in parent bounds. Same representation during editing and playback.
    pub rect:[f32;4], pub style:Style, pub locked:bool, pub visible:bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Scene { pub schema:u32, pub next_id:Id, pub nodes:Vec<Node>, pub accent:[u8;4] }
impl Default for Scene { fn default()->Self { Self::preset(3) } }
impl Scene {
    pub fn empty()->Self { Self{schema:1,next_id:1,nodes:Vec::new(),accent:[180,219,226,255]} }
    pub fn node(&self,id:Id)->Option<&Node>{self.nodes.iter().find(|n|n.id==id)}
    pub fn node_mut(&mut self,id:Id)->Option<&mut Node>{self.nodes.iter_mut().find(|n|n.id==id)}
    pub fn add(&mut self,kind:Kind,page:Option<Page>,rect:[f32;4],parent:Option<Id>)->Id {
        let id=self.next_id; self.next_id+=1;
        let mut style=Style::default();
        if !kind.button() && kind!=Kind::Panel { style.fill=[0,0,0,0]; }
        if kind==Kind::Play {style.fill=self.accent;style.ink=[20,33,42,255];style.font=18.0;}
        self.nodes.push(Node{id,parent,page,kind,name:kind.label().into(),rect,style,locked:false,visible:true});id
    }
    pub fn descendant(&self,id:Id,ancestor:Id)->bool{
        let mut at=Some(id);
        for _ in 0..=self.nodes.len() {match at {
            Some(x) if x==ancestor=>return true,
            Some(x)=>at=self.node(x).and_then(|n|n.parent),None=>return false
        }}
        false
    }
    pub fn parent_rect(&self,id:Id,bounds:Rect)->Rect{
        self.node(id).and_then(|n|n.parent).and_then(|p|self.screen_rect(p,bounds)).unwrap_or(bounds)
    }
    pub fn screen_rect(&self,id:Id,bounds:Rect)->Option<Rect>{
        let mut chain=Vec::new();let mut at=Some(id);
        while let Some(x)=at{
            if chain.len()>self.nodes.len(){return None}
            let n=self.node(x)?;chain.push(n);at=n.parent;
        }
        let mut r=bounds;
        for n in chain.iter().rev(){let [x,y,w,h]=n.rect;r=Rect::from_min_size(r.min+vec2(x*r.width(),y*r.height()),vec2(w*r.width(),h*r.height()));}
        Some(r)
    }
    pub fn shown(&self,id:Id,page:Page)->bool {
        let mut at=Some(id);
        for _ in 0..=self.nodes.len(){match at{
            Some(x)=>{let Some(n)=self.node(x)else{return false};
                if !n.visible||n.page.is_some_and(|p|p!=page){return false}at=n.parent;},
            None=>return true
        }}
        false
    }
    pub fn frozen(&self,id:Id)->bool{
        let mut at=Some(id);
        for _ in 0..=self.nodes.len(){match at{Some(x)=>{let Some(n)=self.node(x)else{return true};if n.locked{return true}at=n.parent;},None=>return false}}
        true
    }
    /// Parent before children; siblings keep their document ordering.
    pub fn order(&self)->Vec<Id>{
        fn walk(s:&Scene,parent:Option<Id>,out:&mut Vec<Id>){
            if out.len()>s.nodes.len(){return}
            for n in s.nodes.iter().filter(|n|n.parent==parent){out.push(n.id);walk(s,Some(n.id),out);}
        }
        let mut out=Vec::new();walk(self,None,&mut out);out
    }
    pub fn hit(&self,p:Pos2,page:Page,bounds:Rect)->Option<Id>{
        self.order().into_iter().rev().find(|&id|self.shown(id,page)&&!self.frozen(id)&&self.screen_rect(id,bounds).is_some_and(|r|r.contains(p)))
    }
    pub fn place(&mut self,id:Id,r:Rect,bounds:Rect){
        let parent=self.parent_rect(id,bounds);
        if let Some(n)=self.node_mut(id){n.rect=unit_rect(r,parent);}
    }
    pub fn reparent(&mut self,id:Id,parent:Option<Id>,bounds:Rect)->Result<(),String>{
        let Some(r)=self.screen_rect(id,bounds)else{return Err("Элемент не найден".into())};
        if let Some(p)=parent {
            let target=self.node(p).ok_or("Панель не найдена")?;
            if target.kind!=Kind::Panel||self.descendant(p,id){return Err("Нельзя вложить панель в саму себя".into())}
        }
        self.node_mut(id).unwrap().parent=parent;
        self.place(id,r,bounds);Ok(())
    }
    pub fn remove(&mut self,id:Id,bounds:Rect){
        let children:Vec<_>=self.nodes.iter().filter(|n|n.parent==Some(id)).map(|n|(n.id,self.screen_rect(n.id,bounds).unwrap())).collect();
        let parent=self.node(id).and_then(|n|n.parent);
        let page=self.node(id).and_then(|n|n.page);
        for (child,r) in children{let n=self.node_mut(child).unwrap();n.parent=parent;if n.page.is_none(){n.page=page}self.place(child,r,bounds);}
        self.nodes.retain(|n|n.id!=id);
    }
    pub fn duplicate(&mut self,id:Id)->Option<Id>{
        let n=self.node(id)?.clone();
        let children:Vec<_>=self.nodes.iter().filter(|n|self.descendant(n.id,id)).cloned().collect();
        let mut map=std::collections::HashMap::new();
        for child in &children{map.insert(child.id,self.next_id);self.next_id+=1;}
        let root=map[&id];
        for mut child in children{
            child.id=map[&child.id];child.parent=child.parent.map(|p|map.get(&p).copied().unwrap_or(p));
            if child.id==root{child.name=format!("{} · копия",n.name);child.rect[0]=(child.rect[0]+0.02).min(1.0-child.rect[2]);child.rect[1]=(child.rect[1]+0.02).min(1.0-child.rect[3]);}
            self.nodes.push(child);
        }
        Some(root)
    }
    pub fn validate(&self)->Result<(),String>{
        if self.schema!=1{return Err("Неизвестная версия документа. Оригинал не изменён.".into())}
        if self.nodes.len()>256||self.next_id>1_000_000_000{return Err("Превышен лимит документа".into())}
        let mut ids=HashSet::new();
        for n in &self.nodes{
            if n.id==0||n.id>=self.next_id||!ids.insert(n.id){return Err("Повреждены ID".into())}
            let [x,y,w,h]=n.rect;
            if !n.rect.iter().all(|v|v.is_finite())||x<0.0||y<0.0||w<=0.0||h<=0.0||x+w>1.001||y+h>1.001{return Err("Недопустимая геометрия".into())}
            if !n.style.radius.is_finite()||!(0.0..=100.0).contains(&n.style.radius)||!n.style.font.is_finite()||!(10.0..=72.0).contains(&n.style.font)||n.name.len()>500{return Err("Недопустимый стиль".into())}
            if let Some(p)=n.parent{
                if self.node(p).is_none_or(|p|p.kind!=Kind::Panel)||self.descendant(p,n.id){return Err("Недопустимая вложенность".into())}
            }
        }
        Ok(())
    }
    pub fn preset(kind:u8)->Self{
        let mut s=Self::empty();
        let nav=s.add(Kind::Panel,None,if kind==1{[0.018,0.19,0.14,0.52]}else{[0.315,0.865,0.37,0.095]},None);
        s.node_mut(nav).unwrap().name="Навигация".into();
        s.node_mut(nav).unwrap().style.blur=true;
        for (i,k) in [Kind::Home,Kind::Library,Kind::Settings].iter().enumerate(){
            let r=if kind==1{[0.06,0.06+i as f32*0.31,0.88,0.26]}else{[0.02+i as f32*0.327,0.12,0.305,0.76]};
            let id=s.add(*k,None,r,Some(nav));s.node_mut(id).unwrap().style.fill=[0,0,0,0];
            s.node_mut(id).unwrap().style.font=13.0;
        }
        let ch=if kind==6{[0.38,0.11,0.24,0.39]}else{[0.22,0.13,0.27,0.63]};
        s.add(Kind::Character,Some(Page::Home),ch,None);
        let zone=if kind==6{[0.32,0.51,0.36,0.30]}else{[0.57,0.29,0.34,0.45]};
        let g=s.add(Kind::Panel,Some(Page::Home),zone,None);
        {let n=s.node_mut(g).unwrap();n.name="Запуск".into();n.style.fill=[0,0,0,0];}
        let title=s.add(Kind::Selection,None,[0.0,0.0,1.0,0.18],Some(g));s.node_mut(title).unwrap().style.font=24.0;
        let sub=s.add(Kind::Text,None,[0.0,0.19,1.0,0.11],Some(g));s.node_mut(sub).unwrap().name="JAVA EDITION  /  VANILLA".into();s.node_mut(sub).unwrap().style.font=11.0;
        s.add(Kind::Version,None,[0.0,0.38,1.0,0.19],Some(g));
        s.add(Kind::Play,None,[0.0,0.62,1.0,0.24],Some(g));
        s.add(Kind::Status,None,[0.0,0.90,1.0,0.10],Some(g));
        let left=if kind==1{0.20}else{0.08};let width=0.92-left;
        let title=s.add(Kind::Text,Some(Page::Library),[left,0.12,width,0.09],None);
        s.node_mut(title).unwrap().name="Твои сборки".into();s.node_mut(title).unwrap().style.font=29.0;
        s.add(Kind::Search,Some(Page::Library),[left,0.255,width*0.68,0.08],None);
        s.add(Kind::Create,Some(Page::Library),[left+width*0.72,0.255,width*0.28,0.08],None);
        let list=s.add(Kind::List,Some(Page::Library),[left,0.37,width,0.43],None);s.node_mut(list).unwrap().style.fill=[16,27,39,155];s.node_mut(list).unwrap().style.blur=true;
        let title=s.add(Kind::Text,Some(Page::Settings),[left,0.12,width,0.09],None);
        s.node_mut(title).unwrap().name="Твоё пространство".into();s.node_mut(title).unwrap().style.font=29.0;
        for(i,k)in [Kind::Profile,Kind::Appearance,Kind::Background,Kind::Json].iter().enumerate(){
            s.add(*k,Some(Page::Settings),[left,0.27+i as f32*0.115,width,0.085],None);
        }
        s
    }
}
pub fn unit_rect(r:Rect,b:Rect)->[f32;4]{
    let w=(r.width()/b.width()).clamp(0.005,1.0);let h=(r.height()/b.height()).clamp(0.005,1.0);
    [((r.left()-b.left())/b.width()).clamp(0.0,1.0-w),((r.top()-b.top())/b.height()).clamp(0.0,1.0-h),w,h]
}
pub fn bounded(r:Rect,b:Rect)->Rect{
    let size=r.size().min(b.size());Rect::from_min_size(pos2(r.left().clamp(b.left(),b.right()-size.x),r.top().clamp(b.top(),b.bottom()-size.y)),size)
}
#[cfg(test)]
mod tests{
    use super::*;
    #[test]fn all_presets_roundtrip(){for p in [1,3,6]{let s=Scene::preset(p);s.validate().unwrap();assert_eq!(s,serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap());}}
    #[test]fn container_moves_children_and_rejects_cycles(){let mut s=Scene::default();let b=Rect::from_min_size(Pos2::ZERO,vec2(1000.0,550.0));let p=s.nodes.iter().find(|n|n.name=="Запуск").unwrap().id;let c=s.nodes.iter().find(|n|n.kind==Kind::Play).unwrap().id;let old=s.screen_rect(c,b).unwrap();s.node_mut(p).unwrap().rect[0]-=0.1;assert!(s.screen_rect(c,b).unwrap().left()<old.left()-90.0);assert!(s.reparent(p,Some(c),b).is_err());}
    #[test]fn resize_never_writes_scene(){let s=Scene::default();let copy=s.clone();for id in s.order(){s.screen_rect(id,Rect::from_min_size(Pos2::ZERO,vec2(720.0,392.0))).unwrap();}assert_eq!(s,copy);}
    #[test]fn pages_hide_descendants(){let s=Scene::default();let id=s.nodes.iter().find(|n|n.kind==Kind::Play).unwrap().id;assert!(s.shown(id,Page::Home));assert!(!s.shown(id,Page::Library));}
    #[test]fn removing_container_preserves_children(){let mut s=Scene::default();let b=Rect::from_min_size(Pos2::ZERO,vec2(1000.0,550.0));let p=s.nodes.iter().find(|n|n.name=="Запуск").unwrap().id;let c=s.nodes.iter().find(|n|n.kind==Kind::Play).unwrap().id;let old=s.screen_rect(c,b).unwrap();s.remove(p,b);s.validate().unwrap();assert!(s.screen_rect(c,b).unwrap().min.distance(old.min)<0.01);assert!(!s.shown(c,Page::Library));}
}