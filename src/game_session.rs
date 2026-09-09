//! UI-independent launch selection. No layout knowledge.
use crate::auth::{AuthManager,AuthState};
use crate::launch::{LaunchManager,manifest::ManifestVersion,run::LaunchProfile};
#[derive(Default)]
pub struct Session{
    pub version:Option<String>,pub instance:Option<String>,pub offline_name:String,
}
impl Session{
    pub fn version(&self,launch:&LaunchManager)->Option<ManifestVersion>{
        let versions=launch.versions()?.ok()?;
        match &self.version{
            Some(id)=>versions.into_iter().find(|v|v.kind=="release"&&v.id==*id),
            None=>versions.into_iter().find(|v|v.kind=="release")
        }
    }
    pub fn profile(&self,auth:&AuthManager)->LaunchProfile{
        match auth.state(){
            AuthState::SignedIn(a)=>LaunchProfile{username:a.username,uuid:a.uuid,access_token:a.access_token},
            _=>LaunchProfile{username:if self.offline_name.trim().is_empty(){"Player".into()}else{self.offline_name.trim().into()},uuid:"00000000-0000-0000-0000-000000000000".into(),access_token:"0".into()}
        }
    }
    pub fn skin_key(&self,auth:&AuthManager)->Option<String>{
        match auth.state(){AuthState::SignedIn(a)=>Some(a.uuid),_=>if self.offline_name.trim().is_empty(){None}else{Some(self.offline_name.trim().into())}}
    }
}