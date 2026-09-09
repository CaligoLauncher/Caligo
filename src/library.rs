//! Name/version library persistence retained independently of its views.
use std::path::PathBuf;
use std::io::Write;
use serde::{Deserialize,Serialize};
#[derive(Debug,Clone,Serialize,Deserialize)]
pub struct Instance{pub name:String,pub version:String}
#[derive(Default)]
pub struct Library{pub items:Vec<Instance>,loaded:bool}
impl Library{
    pub fn ensure_loaded(&mut self){if !self.loaded{self.items=load_all();self.loaded=true;}}
    pub fn create(&mut self,name:&str,version:&str)->Result<(),String>{
        let name=name.trim();
        if name.is_empty()||name.chars().count()>80{return Err("Введи имя до 80 символов".into())}
        if self.items.iter().any(|x|x.name.to_lowercase()==name.to_lowercase()){return Err("Такое имя уже есть".into())}
        let inst=Instance{name:name.into(),version:version.into()};save(&inst)?;self.items.push(inst);
        self.items.sort_by_key(|x|x.name.to_lowercase());Ok(())
    }
    pub fn delete(&mut self,index:usize)->Result<(),String>{
        let inst=self.items.get(index).ok_or("Запись больше не существует")?;remove(inst)?;self.items.remove(index);Ok(())
    }
    #[cfg(test)]
    pub fn fixture()->Self{Self{items:vec![Instance{name:"Выживание".into(),version:"1.21.1".into()},Instance{name:"Творческий мир".into(),version:"1.20.4".into()}],loaded:true}}
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


#[cfg(test)]mod tests{use super::*;#[test]fn names_cannot_escape(){assert!(!sanitize("../a/b").contains('/'));assert!(!sanitize("../a/b").contains('.'));}}
