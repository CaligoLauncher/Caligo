//! Headless visual review of the *real egui UI*, not a web reimplementation.
//! Software rasterization checks layout and text; it does not replace native
//! Windows compositor, DPI, keyboard or mouse testing.
use std::collections::HashMap;
use std::io::{Cursor,Write};
use base64::Engine;
use eframe::egui::{self,ColorImage,TextureId};
use crate::app::{CaligoApp,Tab};

fn apply_delta(textures:&mut HashMap<TextureId,ColorImage>,delta:&egui::TexturesDelta){
    for (id,change) in &delta.set{
        let image=match &change.image{
            egui::ImageData::Color(x)=>(**x).clone(),
            egui::ImageData::Font(x)=>ColorImage{size:x.size,pixels:x.srgba_pixels(None).collect()},
        };
        if let Some([x,y])=change.pos{
            let full=textures.get_mut(id).expect("texture patch requires a base");
            for row in 0..image.size[1]{
                let dst=(row+y)*full.size[0]+x;
                let src=row*image.size[0];
                full.pixels[dst..dst+image.size[0]].copy_from_slice(&image.pixels[src..src+image.size[0]]);
            }
        }else{textures.insert(*id,image);}
    }
}

fn sample(image:&ColorImage,u:f32,v:f32)->[f32;4]{
    let x=(u*image.size[0] as f32-0.5).clamp(0.0,(image.size[0]-1) as f32);
    let y=(v*image.size[1] as f32-0.5).clamp(0.0,(image.size[1]-1) as f32);
    let x0=x.floor() as usize;let y0=y.floor() as usize;
    let x1=(x0+1).min(image.size[0]-1);let y1=(y0+1).min(image.size[1]-1);
    let fx=x-x0 as f32;let fy=y-y0 as f32;
    let cs=[image.pixels[y0*image.size[0]+x0].to_array(),image.pixels[y0*image.size[0]+x1].to_array(),image.pixels[y1*image.size[0]+x0].to_array(),image.pixels[y1*image.size[0]+x1].to_array()];
    let mut out=[0.0;4];
    for k in 0..4{out[k]=((cs[0][k] as f32*(1.0-fx)+cs[1][k] as f32*fx)*(1.0-fy)+(cs[2][k] as f32*(1.0-fx)+cs[3][k] as f32*fx)*fy)/255.0;}
    out
}

fn raster(ctx:&egui::Context,out:egui::FullOutput,textures:&HashMap<TextureId,ColorImage>,w:u32,h:u32)->image::RgbaImage{
    let mut pixels=image::RgbaImage::from_pixel(w,h,image::Rgba([17,20,26,255]));
    let primitives=ctx.tessellate(out.shapes,out.pixels_per_point);
    for clipped in primitives{
        let egui::epaint::Primitive::Mesh(mesh)=clipped.primitive else{continue;};
        let clip=clipped.clip_rect;
        let Some(texture)=textures.get(&mesh.texture_id)else{panic!("missing texture")};
        for tri in mesh.indices.chunks_exact(3){
            let a=mesh.vertices[tri[0] as usize];let b=mesh.vertices[tri[1] as usize];let c=mesh.vertices[tri[2] as usize];
            assert!(a.pos.x.is_finite()&&a.pos.y.is_finite());
            let den=(b.pos.y-c.pos.y)*(a.pos.x-c.pos.x)+(c.pos.x-b.pos.x)*(a.pos.y-c.pos.y);
            if den.abs()<0.00001{continue;}
            let x0=a.pos.x.min(b.pos.x).min(c.pos.x).max(clip.left()).floor().max(0.0) as u32;
            let x1=a.pos.x.max(b.pos.x).max(c.pos.x).min(clip.right()).ceil().min(w as f32) as u32;
            let y0=a.pos.y.min(b.pos.y).min(c.pos.y).max(clip.top()).floor().max(0.0) as u32;
            let y1=a.pos.y.max(b.pos.y).max(c.pos.y).min(clip.bottom()).ceil().min(h as f32) as u32;
            let ca=a.color.to_array();let cb=b.color.to_array();let cc=c.color.to_array();
            for y in y0..y1{for x in x0..x1{
                let px=x as f32+0.5;let py=y as f32+0.5;
                let wa=((b.pos.y-c.pos.y)*(px-c.pos.x)+(c.pos.x-b.pos.x)*(py-c.pos.y))/den;
                let wb=((c.pos.y-a.pos.y)*(px-c.pos.x)+(a.pos.x-c.pos.x)*(py-c.pos.y))/den;
                let wc=1.0-wa-wb;
                if wa<0.0||wb<0.0||wc<0.0{continue;}
                let tex=sample(texture,a.uv.x*wa+b.uv.x*wb+c.uv.x*wc,a.uv.y*wa+b.uv.y*wb+c.uv.y*wc);
                let mut col=[0.0;4];
                for k in 0..4{col[k]=tex[k]*(ca[k] as f32*wa+cb[k] as f32*wb+cc[k] as f32*wc)/255.0;}
                let dst=pixels.get_pixel_mut(x,y);
                for k in 0..3{dst[k]=(col[k]*255.0+dst[k] as f32*(1.0-col[3])).round().clamp(0.0,255.0) as u8;}
                dst[3]=255;
            }}
        }
    }
    pixels
}


#[test]
fn visual_review_screens(){
    for preset in [1,3,6] {
        for(w,h)in[(1000,620),(720,440)]{
            for (name,tab,edit,local) in [
                ("home",Tab::Home,false,false),
                ("library",Tab::Library,false,false),
                ("settings",Tab::Settings,false,false),
                ("editor",Tab::Home,true,false),
                ("local",Tab::Home,true,true),
            ]{
                let ctx=egui::Context::default();ctx.set_pixels_per_point(1.0);
                let mut app=CaligoApp::visual_fixture(&ctx,tab,true);
                app.studio.scene=crate::canvas_model::Scene::preset(preset);
                if edit{
                    app.studio.begin();
                    app.studio.selected=app.studio.scene.nodes.iter().find(|n|n.kind==crate::canvas_model::Kind::Play).map(|n|n.id);
                    app.studio.inspector=local;
                }
                let mut textures=HashMap::new();let mut last=None;
                for i in 0..5{
                    let out=ctx.run(egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(w as f32,h as f32))),focused:true,time:Some(i as f64/10.0),..Default::default()},|ctx|app.render(ctx));
                    apply_delta(&mut textures,&out.textures_delta);last=Some(out);
                }
                let png=raster(&ctx,last.unwrap(),&textures,w,h);let mut buf=Cursor::new(Vec::new());
                image::DynamicImage::ImageRgba8(png).write_to(&mut buf,image::ImageFormat::Png).unwrap();
                if std::env::var("CI").is_ok(){
                    let data=base64::engine::general_purpose::STANDARD.encode(buf.into_inner());
                    let mut stdout=std::io::stdout().lock();
                    writeln!(stdout,"CALIGO_VISUAL_BEGIN clean_{preset:02}_{name}_{w}").unwrap();
                    for chunk in data.as_bytes().chunks(6000){writeln!(stdout,"CALIGO_VISUAL_DATA {}",std::str::from_utf8(chunk).unwrap()).unwrap();}
                    writeln!(stdout,"CALIGO_VISUAL_END clean_{preset:02}_{name}_{w}").unwrap();
                }
            }
        }
    }
}
mod interaction {
//! Real egui input tests. No auth, network, native GPU or Windows claims.
use super::*;
use crate::canvas_model::{Kind,Page};
fn frame(ctx:&egui::Context,a:&mut CaligoApp,w:f32,h:f32,events:Vec<egui::Event>){
    let _=ctx.run(egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(w,h))),events,focused:true,..Default::default()},|ctx|a.render(ctx));
}
fn point(ctx:&egui::Context,a:&mut CaligoApp,w:f32,h:f32,p:egui::Pos2,down:bool){
    frame(ctx,a,w,h,vec![egui::Event::PointerMoved(p),egui::Event::PointerButton{pos:p,button:egui::PointerButton::Primary,pressed:down,modifiers:Default::default()}]);
}
fn click(ctx:&egui::Context,a:&mut CaligoApp,w:f32,h:f32,p:egui::Pos2){point(ctx,a,w,h,p,true);point(ctx,a,w,h,p,false);}
fn get(a:&CaligoApp,k:Kind)->(u64,egui::Rect){
    let id=a.studio.scene.nodes.iter().find(|n|n.kind==k).unwrap().id;
    (id,a.studio.scene.screen_rect(id,a.canvas_bounds).unwrap())
}
#[test]fn enter_editor_does_not_shift_canvas(){
    for(w,h)in[(1000.0,620.0),(720.0,440.0)]{
        let ctx=egui::Context::default();let mut a=CaligoApp::visual_fixture(&ctx,Page::Home,false);
        frame(&ctx,&mut a,w,h,vec![]);frame(&ctx,&mut a,w,h,vec![]);
        let before=get(&a,Kind::Play).1;let bounds=a.canvas_bounds;
        a.studio.begin();for _ in 0..4{frame(&ctx,&mut a,w,h,vec![]);}
        assert_eq!(a.canvas_bounds,bounds);assert_eq!(get(&a,Kind::Play).1,before);
        let toolbar=a.studio.controls.iter().find(|x|x.0=="toolbar").unwrap().1;
        assert!(a.canvas_bounds.contains_rect(toolbar),"editor toolbar outside minimum window: {toolbar:?}");
    }
}
#[test]fn direct_move_resize_undo_and_cancel(){
    let ctx=egui::Context::default();let mut a=CaligoApp::visual_fixture(&ctx,Page::Home,false);
    frame(&ctx,&mut a,1000.0,620.0,vec![]);a.studio.begin();a.studio.snap=false;
    for _ in 0..4{frame(&ctx,&mut a,1000.0,620.0,vec![]);}
    let baseline=a.studio.scene.clone();let(id,r)=get(&a,Kind::Play);
    let start=r.center();let end=start+egui::vec2(-12.0,-18.0);
    point(&ctx,&mut a,1000.0,620.0,start,true);
    frame(&ctx,&mut a,1000.0,620.0,vec![egui::Event::PointerMoved(end)]);
    assert!(get(&a,Kind::Play).1.min.distance(r.min)>10.0,"real node should follow before release");
    point(&ctx,&mut a,1000.0,620.0,end,false);
    assert_eq!(a.studio.selected,Some(id));
    assert!(matches!(a.launch.state(),crate::launch::LaunchState::Idle));
    a.studio.undo();assert_eq!(a.studio.scene,baseline);
    a.studio.redo();assert_ne!(a.studio.scene,baseline);
    let moved=get(&a,Kind::Play).1;
    frame(&ctx,&mut a,1000.0,620.0,vec![]);
    let start=moved.right_bottom();let end=start-egui::vec2(18.0,10.0);
    point(&ctx,&mut a,1000.0,620.0,start,true);
    frame(&ctx,&mut a,1000.0,620.0,vec![egui::Event::PointerMoved(end)]);
    point(&ctx,&mut a,1000.0,620.0,end,false);
    assert!(get(&a,Kind::Play).1.width()<moved.width()-10.0);
    a.studio.cancel();assert_eq!(a.studio.scene,baseline);assert!(!a.studio.active);
}
#[test]fn click_without_motion_is_not_a_layout_edit(){
    let ctx=egui::Context::default();let mut a=CaligoApp::visual_fixture(&ctx,Page::Home,false);
    frame(&ctx,&mut a,1000.0,620.0,vec![]);a.studio.begin();
    for _ in 0..4{frame(&ctx,&mut a,1000.0,620.0,vec![]);}
    let old=a.studio.scene.clone();let p=get(&a,Kind::Play).1.center();
    click(&ctx,&mut a,1000.0,620.0,p);assert_eq!(a.studio.scene,old);
}
#[test]fn navigation_replaces_lobby_in_all_presets(){
    for preset in [1,3,6]{for(w,h)in[(1000.0,620.0),(720.0,440.0)]{
        let ctx=egui::Context::default();let mut a=CaligoApp::visual_fixture(&ctx,Page::Home,true);a.studio.scene=crate::canvas_model::Scene::preset(preset);
        for _ in 0..3{frame(&ctx,&mut a,w,h,vec![]);}
        let p=get(&a,Kind::Library).1.center();click(&ctx,&mut a,w,h,p);frame(&ctx,&mut a,w,h,vec![]);
        assert_eq!(a.tab,Page::Library);
        assert!(!a.node_rects.iter().any(|(id,_)|matches!(a.studio.scene.node(*id).unwrap().kind,Kind::Play|Kind::Character)));
        let p=get(&a,Kind::Settings).1.center();click(&ctx,&mut a,w,h,p);assert_eq!(a.tab,Page::Settings);
    }}
}
#[test]fn local_settings_dont_move_scene_or_edit_sibling(){
    let ctx=egui::Context::default();let mut a=CaligoApp::visual_fixture(&ctx,Page::Home,false);
    frame(&ctx,&mut a,720.0,440.0,vec![]);a.studio.begin();
    for _ in 0..4{frame(&ctx,&mut a,720.0,440.0,vec![]);}
    let(id,r)=get(&a,Kind::Play);let old=a.studio.scene.clone();
    click(&ctx,&mut a,720.0,440.0,r.center());
    for _ in 0..3{frame(&ctx,&mut a,720.0,440.0,vec![]);}
    let gear=a.studio.controls.iter().find(|x|x.0=="gear").unwrap().1.center();
    click(&ctx,&mut a,720.0,440.0,gear);
    for _ in 0..4{frame(&ctx,&mut a,720.0,440.0,vec![]);}
    assert!(a.studio.inspector);
    let window=a.studio.controls.iter().find(|x|x.0=="inspector").unwrap().1;
    assert!(a.canvas_bounds.contains_rect(window));
    let slider=a.studio.controls.iter().find(|x|x.0=="rounding").unwrap().1;
    click(&ctx,&mut a,720.0,440.0,egui::pos2(slider.left()+70.0,slider.center().y));
    frame(&ctx,&mut a,720.0,440.0,vec![]);
    assert_eq!(get(&a,Kind::Play).1,r);
    for n in old.nodes.iter().filter(|n|n.id!=id){assert_eq!(a.studio.scene.node(n.id).unwrap(),n);}
    assert_ne!(a.studio.scene.node(id).unwrap().style.radius,old.node(id).unwrap().style.radius);
    a.studio.cancel();assert_eq!(a.studio.scene,old);
}
#[test]fn escape_aborts_drag(){
    let ctx=egui::Context::default();let mut a=CaligoApp::visual_fixture(&ctx,Page::Home,false);
    frame(&ctx,&mut a,1000.0,620.0,vec![]);a.studio.begin();
    for _ in 0..3{frame(&ctx,&mut a,1000.0,620.0,vec![]);}
    let old=a.studio.scene.clone();let p=get(&a,Kind::Play).1.center();
    point(&ctx,&mut a,1000.0,620.0,p,true);
    frame(&ctx,&mut a,1000.0,620.0,vec![egui::Event::PointerMoved(p-egui::vec2(20.0,20.0))]);
    frame(&ctx,&mut a,1000.0,620.0,vec![egui::Event::Key{key:egui::Key::Escape,physical_key:None,pressed:true,repeat:false,modifiers:Default::default()}]);
    assert_eq!(a.studio.scene,old);
}
#[test]fn drag_play_out_of_container_and_back(){
    let ctx=egui::Context::default();let mut a=CaligoApp::visual_fixture(&ctx,Page::Home,false);
    frame(&ctx,&mut a,1000.0,620.0,vec![]);a.studio.begin();a.studio.snap=false;
    for _ in 0..4{frame(&ctx,&mut a,1000.0,620.0,vec![]);}
    let old=a.studio.scene.clone();let(id,r)=get(&a,Kind::Play);let parent=a.studio.scene.node(id).unwrap().parent;
    let end=r.center()-egui::vec2(350.0,60.0);
    point(&ctx,&mut a,1000.0,620.0,r.center(),true);
    frame(&ctx,&mut a,1000.0,620.0,vec![egui::Event::PointerMoved(end)]);
    point(&ctx,&mut a,1000.0,620.0,end,false);
    assert_eq!(a.studio.scene.node(id).unwrap().parent,None);
    assert_eq!(a.studio.scene.node(id).unwrap().page,Some(Page::Home));
    assert!((get(&a,Kind::Play).1.center()-end).length()<0.1);
    frame(&ctx,&mut a,1000.0,620.0,vec![]);
    point(&ctx,&mut a,1000.0,620.0,end,true);
    frame(&ctx,&mut a,1000.0,620.0,vec![egui::Event::PointerMoved(r.center())]);
    point(&ctx,&mut a,1000.0,620.0,r.center(),false);
    assert_eq!(a.studio.scene.node(id).unwrap().parent,parent);
    a.studio.cancel();assert_eq!(a.studio.scene,old);
}
#[test]fn release_outside_and_focus_loss_restore(){
    for lose_focus in [false,true]{
        let ctx=egui::Context::default();let mut a=CaligoApp::visual_fixture(&ctx,Page::Home,false);
        frame(&ctx,&mut a,1000.0,620.0,vec![]);a.studio.begin();
        for _ in 0..3{frame(&ctx,&mut a,1000.0,620.0,vec![]);}
        let old=a.studio.scene.clone();let p=get(&a,Kind::Play).1.center();
        point(&ctx,&mut a,1000.0,620.0,p,true);
        frame(&ctx,&mut a,1000.0,620.0,vec![egui::Event::PointerMoved(p-egui::vec2(100.0,60.0))]);
        if lose_focus{
            let _=ctx.run(egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(1000.0,620.0))),focused:false,..Default::default()},|ctx|a.render(ctx));
        }else{point(&ctx,&mut a,1000.0,620.0,egui::pos2(-20.0,-20.0),false);}
        assert_eq!(a.studio.scene,old);
    }
}
#[test]fn deleting_parent_keeps_real_controls_and_undo(){
    let ctx=egui::Context::default();let mut a=CaligoApp::visual_fixture(&ctx,Page::Home,false);
    frame(&ctx,&mut a,1000.0,620.0,vec![]);a.studio.begin();
    let old=a.studio.scene.clone();let(id,r)=get(&a,Kind::Play);a.studio.selected=a.studio.scene.node(id).unwrap().parent;
    frame(&ctx,&mut a,1000.0,620.0,vec![egui::Event::Key{key:egui::Key::Delete,physical_key:None,pressed:true,repeat:false,modifiers:Default::default()}]);
    assert_eq!(a.studio.scene.node(id).unwrap().parent,None);
    assert!(get(&a,Kind::Play).1.min.distance(r.min)<0.01);
    a.studio.undo();assert_eq!(a.studio.scene,old);
}
#[test]fn locked_tree_cannot_be_grabbed(){
    let ctx=egui::Context::default();let mut a=CaligoApp::visual_fixture(&ctx,Page::Home,false);
    frame(&ctx,&mut a,1000.0,620.0,vec![]);
    let(id,r)=get(&a,Kind::Play);let parent=a.studio.scene.node(id).unwrap().parent.unwrap();
    a.studio.scene.node_mut(parent).unwrap().locked=true;a.studio.begin();
    for _ in 0..3{frame(&ctx,&mut a,1000.0,620.0,vec![]);}
    let old=a.studio.scene.clone();point(&ctx,&mut a,1000.0,620.0,r.center(),true);
    frame(&ctx,&mut a,1000.0,620.0,vec![egui::Event::PointerMoved(r.center()-egui::vec2(100.0,0.0))]);
    point(&ctx,&mut a,1000.0,620.0,r.center()-egui::vec2(100.0,0.0),false);
    assert_eq!(a.studio.scene,old);assert!(a.studio.selected.is_none());
}
#[test]fn palette_draw_panel_mouse_flow_and_cancel(){
    for(w,h)in[(1000.0,620.0),(720.0,440.0)]{
        let ctx=egui::Context::default();let mut a=CaligoApp::visual_fixture(&ctx,Page::Home,false);
        frame(&ctx,&mut a,w,h,vec![]);a.studio.begin();
        for _ in 0..4{frame(&ctx,&mut a,w,h,vec![]);}
        let old=a.studio.scene.clone();
        let add=a.studio.controls.iter().find(|x|x.0=="add").unwrap().1.center();
        click(&ctx,&mut a,w,h,add);
        for _ in 0..4{frame(&ctx,&mut a,w,h,vec![]);}
        let panel=a.studio.controls.iter().find(|x|x.0=="add_panel").unwrap().1.center();
        click(&ctx,&mut a,w,h,panel);
        for _ in 0..4{frame(&ctx,&mut a,w,h,vec![]);}
        assert_eq!(a.studio.scene,old,"opening and picking from catalog must not change scene");
        let start=egui::pos2(60.0,190.0);let end=egui::pos2(220.0,310.0);
        point(&ctx,&mut a,w,h,start,true);
        frame(&ctx,&mut a,w,h,vec![egui::Event::PointerMoved(end)]);
        point(&ctx,&mut a,w,h,end,false);
        assert_eq!(a.studio.scene.nodes.len(),old.nodes.len()+1,"drawing should create exactly one panel");
        let id=a.studio.selected.unwrap();assert_eq!(a.studio.scene.node(id).unwrap().kind,Kind::Panel);
        let rect=a.studio.scene.screen_rect(id,a.canvas_bounds).unwrap();
        assert!(rect.min.distance(start)<0.1&&rect.max.distance(end)<0.1);
        a.studio.undo();assert_eq!(a.studio.scene,old);a.studio.redo();assert_ne!(a.studio.scene,old);a.studio.cancel();assert_eq!(a.studio.scene,old);
    }
}
#[test]fn save_load_edited_scene_not_only_defaults(){
    let ctx=egui::Context::default();let mut a=CaligoApp::visual_fixture(&ctx,Page::Home,false);
    frame(&ctx,&mut a,1000.0,620.0,vec![]);a.studio.begin();a.studio.snap=false;
    for _ in 0..3{frame(&ctx,&mut a,1000.0,620.0,vec![]);}
    let(id,r)=get(&a,Kind::Play);let dest=r.center()-egui::vec2(310.0,30.0);
    point(&ctx,&mut a,1000.0,620.0,r.center(),true);
    frame(&ctx,&mut a,1000.0,620.0,vec![egui::Event::PointerMoved(dest)]);
    point(&ctx,&mut a,1000.0,620.0,dest,false);
    a.studio.scene.node_mut(id).unwrap().style.radius=23.0;
    let dir=std::env::temp_dir().join(format!("caligo-gesture-save-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    crate::canvas_store::save(&dir,0,&a.studio.scene).unwrap();
    let (_,loaded)=crate::canvas_store::load(&dir).unwrap().unwrap();
    assert_eq!(a.studio.scene,loaded);assert_eq!(loaded.node(id).unwrap().style.radius,23.0);
    assert!((loaded.screen_rect(id,a.canvas_bounds).unwrap().center()-dest).length()<0.1);
    std::fs::remove_dir_all(dir).unwrap();
}
}
