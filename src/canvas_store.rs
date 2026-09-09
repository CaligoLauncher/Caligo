//! Separate storage: legacy interface-a/b.json are never read, overwritten or deleted.
use super::canvas_model::Scene;
use serde::{Serialize,Deserialize};
use std::{fs,io::{Read,Write},path::Path};
const MAX:u64=1024*1024;
#[derive(Serialize,Deserialize)]
struct Record{generation:u64,scene:Scene}
pub fn load(dir:&Path)->Result<Option<(u64,Scene)>,String>{
    let mut valid=Vec::new();let mut errors=Vec::new();
    for name in ["shell-a.json","shell-b.json"]{
        let path=dir.join(name);
        match fs::File::open(path){
            Err(e) if e.kind()==std::io::ErrorKind::NotFound=>continue,
            Err(e)=>errors.push(e.to_string()),
            Ok(mut f)=>{
                let mut data=Vec::new();Read::by_ref(&mut f).take(MAX+1).read_to_end(&mut data).map_err(|e|e.to_string())?;
                if data.len() as u64>MAX{return Err("Файл интерфейса слишком большой".into())}
                match serde_json::from_slice::<serde_json::Value>(&data){
                    Ok(v)=>{
                        if v.pointer("/scene/schema").and_then(|v|v.as_u64()).is_some_and(|v|v>1){return Err("Новая версия интерфейса. Перезапись запрещена.".into())}
                        match serde_json::from_value::<Record>(v){
                            Ok(r)=>match r.scene.validate(){Ok(())=>valid.push(r),Err(e)=>errors.push(e)},
                            Err(e)=>errors.push(e.to_string())
                        }
                    },Err(e)=>errors.push(e.to_string())
                }
            }
        }
    }
    match valid.into_iter().max_by_key(|r|r.generation){Some(r)=>Ok(Some((r.generation,r.scene))),None if errors.is_empty()=>Ok(None),None=>Err("Оба файла интерфейса недоступны. Оригиналы сохранены.".into())}
}
pub fn save(dir:&Path,expected:u64,scene:&Scene)->Result<u64,String>{
    scene.validate()?;fs::create_dir_all(dir).map_err(|e|e.to_string())?;
    let path=dir.join("shell.lock");
    let lock=fs::OpenOptions::new().write(true).create_new(true).open(&path).map_err(|_|"Сохранение занято. После сбоя закрой все окна Caligo перед удалением shell.lock.")?;
    let result=(||{
        let generation=load(dir)?.map(|r|r.0).unwrap_or(0);
        if generation!=expected{return Err("Интерфейс изменён другим окном. Твои правки не записаны.".into())}
        let next=generation.checked_add(1).ok_or("Слишком много сохранений")?;
        let data=serde_json::to_vec_pretty(&Record{generation:next,scene:scene.clone()}).map_err(|e|e.to_string())?;
        if data.len() as u64>MAX{return Err("Пресет слишком большой".into())}
        // Choose the OTHER slot from the latest valid record, including recovery after corruption.
        let target=dir.join(if next%2==1{"shell-a.json"}else{"shell-b.json"});
        let mut f=fs::OpenOptions::new().write(true).create(true).truncate(true).open(&target).map_err(|e|e.to_string())?;
        f.write_all(&data).and_then(|_|f.sync_all()).map_err(|e|e.to_string())?;Ok(next)
    })();
    drop(lock);let _=fs::remove_file(path);result
}
#[cfg(test)]
mod tests{
    use super::*;
    fn dir()->std::path::PathBuf{std::env::temp_dir().join(format!("caligo-shell-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()))}
    #[test]fn restart_stale_writer_and_legacy_preservation(){
        let p=dir();fs::create_dir_all(&p).unwrap();fs::write(p.join("interface-a.json"),b"old untouched").unwrap();
        let s=Scene::default();assert!(load(&p).unwrap().is_none());assert_eq!(save(&p,0,&s).unwrap(),1);
        assert_eq!(load(&p).unwrap().unwrap(),(1,s.clone()));assert!(save(&p,0,&s).is_err());
        assert_eq!(fs::read(p.join("interface-a.json")).unwrap(),b"old untouched");fs::remove_dir_all(p).unwrap();
    }
    #[test]fn corruption_recovers_and_future_blocks(){
        let p=dir();let s=Scene::default();save(&p,0,&s).unwrap();save(&p,1,&s).unwrap();
        fs::write(p.join("shell-b.json"),b"{broken").unwrap();assert_eq!(load(&p).unwrap().unwrap().0,1);save(&p,1,&s).unwrap();
        fs::write(p.join("shell-b.json"),br#"{"scene":{"schema":99}}"#).unwrap();assert!(save(&p,2,&s).is_err());
        assert!(fs::read(p.join("shell-b.json")).unwrap().windows(2).any(|x|x==b"99"));assert!(!p.join("shell.lock").exists());fs::remove_dir_all(p).unwrap();
    }
}