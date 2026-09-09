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
    for (name,w,h,tab,populated) in [
        ("home_empty_1000",1000,620,Tab::Home,false),
        ("home_min_720",720,440,Tab::Home,false),
        ("home_populated_1000",1000,620,Tab::Home,true),
        ("library_720",720,440,Tab::Instances,true),
        ("settings_720",720,440,Tab::Settings,false),
        ("library_1000",1000,620,Tab::Instances,true),
        ("settings_1000",1000,620,Tab::Settings,false),
    ]{
        let ctx=egui::Context::default();
        ctx.set_pixels_per_point(1.0);
        let mut app=CaligoApp::visual_fixture(&ctx,tab,populated);
        app.editor.document=crate::composition::Document::default();
        let mut textures=HashMap::new();
        let mut last=None;
        for i in 0..4{
            let out=ctx.run(egui::RawInput{
                screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(w as f32,h as f32))),
                time:Some(i as f64/10.0),
                ..Default::default()
            },|ctx|app.render(ctx));
            apply_delta(&mut textures,&out.textures_delta);last=Some(out);
        }
        let png=raster(&ctx,last.unwrap(),&textures,w,h);
        let mut buf=Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(png).write_to(&mut buf,image::ImageFormat::Png).unwrap();
        if std::env::var("CI").is_ok(){
            let data=base64::engine::general_purpose::STANDARD.encode(buf.into_inner());
            let mut stdout=std::io::stdout().lock();
            writeln!(stdout,"CALIGO_VISUAL_BEGIN {name}").unwrap();
            for chunk in data.as_bytes().chunks(6000){writeln!(stdout,"CALIGO_VISUAL_DATA {}",std::str::from_utf8(chunk).unwrap()).unwrap();}
            writeln!(stdout,"CALIGO_VISUAL_END {name}").unwrap();
        }
    }
}

#[test]
fn sidebar_navigation_works_at_both_window_sizes() {
    for (w,h) in [(1000.0,620.0),(720.0,440.0)] {
        let ctx=egui::Context::default();
        let mut app=CaligoApp::visual_fixture(&ctx,Tab::Home,false);
        app.editor.document=crate::composition::Document::legacy();
        let screen=egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(w,h));
        let frame=|app:&mut CaligoApp,events:Vec<egui::Event>|{
            let _=ctx.run(egui::RawInput{screen_rect:Some(screen),events,..Default::default()},|ctx|app.render(ctx));
        };
        frame(&mut app,vec![]); frame(&mut app,vec![]);
        let p=app.editor.rects.iter().find(|(id,_)|*id==3).unwrap().1.center();
        frame(&mut app,vec![egui::Event::PointerMoved(p),egui::Event::PointerButton{pos:p,button:egui::PointerButton::Primary,pressed:true,modifiers:egui::Modifiers::default()}]);
        frame(&mut app,vec![egui::Event::PointerButton{pos:p,button:egui::PointerButton::Primary,pressed:false,modifiers:egui::Modifiers::default()}]);
        assert_eq!(app.tab,Tab::Instances);
        let p=app.editor.rects.iter().find(|(id,_)|*id==2).unwrap().1.center();
        frame(&mut app,vec![egui::Event::PointerMoved(p),egui::Event::PointerButton{pos:p,button:egui::PointerButton::Primary,pressed:true,modifiers:egui::Modifiers::default()}]);
        frame(&mut app,vec![egui::Event::PointerButton{pos:p,button:egui::PointerButton::Primary,pressed:false,modifiers:egui::Modifiers::default()}]);
        assert_eq!(app.tab,Tab::Home);
    }
}

use crate::composition::{Edge,Position};

fn editor_frame(ctx:&egui::Context,app:&mut CaligoApp,events:Vec<egui::Event>){
    let _=ctx.run(egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(1000.0,620.0))),events,..Default::default()},|ctx|app.render(ctx));
}
fn pointer(ctx:&egui::Context,app:&mut CaligoApp,p:egui::Pos2,pressed:bool){
    editor_frame(ctx,app,vec![egui::Event::PointerMoved(p),egui::Event::PointerButton{pos:p,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()}]);
}
fn click(ctx:&egui::Context,app:&mut CaligoApp,p:egui::Pos2){
    pointer(ctx,app,p,true);pointer(ctx,app,p,false);
}
fn drag_to(ctx:&egui::Context,app:&mut CaligoApp,start:egui::Pos2,end:egui::Pos2){
    pointer(ctx,app,start,true);
    editor_frame(ctx,app,vec![egui::Event::PointerMoved(start+egui::vec2(8.0,8.0))]);
    editor_frame(ctx,app,vec![egui::Event::PointerMoved(end)]);
    pointer(ctx,app,end,false);
    editor_frame(ctx,app,vec![]);
}
#[test]
fn edit_mode_real_pointer_move_undo_cancel_and_navigation(){
    let ctx=egui::Context::default();let mut app=CaligoApp::visual_fixture(&ctx,Tab::Home,false);
        app.editor.document=crate::composition::Document::legacy();
    let original=app.editor.document.clone();
    app.editor.begin();
    for _ in 0..4{editor_frame(&ctx,&mut app,vec![]);}
    let add=app.editor.controls.iter().find(|(s,_)|*s=="add_panel").unwrap().1.center();
    click(&ctx,&mut app,add);editor_frame(&ctx,&mut app,vec![]);
    let top=app.editor.panel_rects.iter().find(|(id,_)|*id==1).unwrap().1.top();
    click(&ctx,&mut app,egui::pos2(600.0,top+8.0));
    for _ in 0..3{editor_frame(&ctx,&mut app,vec![]);}
    assert_eq!(app.editor.document.panels.len(),2);
    let new_id=app.editor.document.panels[1].id;
    assert_eq!(app.editor.document.panels[1].edge,Edge::Top);
    let start=app.editor.rects.iter().find(|(id,_)|*id==3).unwrap().1.center();
    let target=app.editor.panel_rects.iter().find(|(id,_)|*id==new_id).unwrap().1.center();
    drag_to(&ctx,&mut app,start,target);
    assert_eq!(app.editor.document.widgets.iter().find(|w|w.id==3).unwrap().panel,Some(new_id));
    assert_eq!(app.tab,Tab::Home,"edit click must not navigate");
    let undo=app.editor.controls.iter().find(|(s,_)|*s=="undo").unwrap().1.center();
    click(&ctx,&mut app,undo);
    assert_eq!(app.editor.document.widgets.iter().find(|w|w.id==3).unwrap().panel,Some(1));
    let p=app.editor.rects.iter().find(|(id,_)|*id==3).unwrap().1.center();
    click(&ctx,&mut app,p);editor_frame(&ctx,&mut app,vec![]);
    let gear=app.editor.controls.iter().find(|(s,_)|*s=="gear").unwrap().1.center();
    click(&ctx,&mut app,gear);
    for _ in 0..5{editor_frame(&ctx,&mut app,vec![]);}
    let slider=app.editor.controls.iter().find(|(s,_)|*s=="rounding").unwrap().1;
    click(&ctx,&mut app,slider.left_center()+egui::vec2(78.0,0.0));
    editor_frame(&ctx,&mut app,vec![]);
    assert!(app.editor.document.widgets.iter().find(|w|w.id==3).unwrap().style.rounding.unwrap_or(8.0)>15.0);
    assert_eq!(app.editor.document.widgets.iter().find(|w|w.id==2).unwrap().style.rounding,None);
    app.editor.cancel();assert_eq!(app.editor.document,original);
    for _ in 0..3{editor_frame(&ctx,&mut app,vec![]);}
    let p=app.editor.rects.iter().find(|(id,_)|*id==3).unwrap().1.center();
    click(&ctx,&mut app,p);assert_eq!(app.tab,Tab::Instances);
}

#[test]
fn free_position_and_editor_selection_do_not_launch_game(){
    let ctx=egui::Context::default();let mut app=CaligoApp::visual_fixture(&ctx,Tab::Home,false);
        app.editor.document=crate::composition::Document::legacy();
    app.editor.begin();
    for _ in 0..4{editor_frame(&ctx,&mut app,vec![]);}
    let start=app.editor.rects.iter().find(|(id,_)|*id==3).unwrap().1.center();
    drag_to(&ctx,&mut app,start,egui::pos2(760.0,470.0));
    assert_eq!(app.editor.document.widgets.iter().find(|w|w.id==3).unwrap().panel,None);
    click(&ctx,&mut app,egui::pos2(860.0,440.0));
    assert!(matches!(app.launch.state(),crate::launch::LaunchState::Idle));
    assert_eq!(app.tab,Tab::Home);
}

#[test]
fn panel_resize_pointer_and_restart_preserve_navigation(){
    let ctx=egui::Context::default();let mut app=CaligoApp::visual_fixture(&ctx,Tab::Home,false);
        app.editor.document=crate::composition::Document::legacy();
    app.editor.begin();
    for _ in 0..4{editor_frame(&ctx,&mut app,vec![]);}
    click(&ctx,&mut app,egui::pos2(40.0,400.0));
    editor_frame(&ctx,&mut app,vec![]);
    let h=app.editor.controls.iter().find(|(s,_)|*s=="resize").unwrap().1.center();
    drag_to(&ctx,&mut app,h,h+egui::vec2(48.0,0.0));
    assert!((app.editor.document.panels[0].size[0]-124.0).abs()<2.0);
    assert_eq!(app.editor.document.panels[0].size[1],64.0,"resizing width must not corrupt dormant height");
    let saved=app.editor.document.clone();
    let dir=std::env::temp_dir().join(format!("caligo-editor-restart-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    crate::composition::save(&dir,0,&saved).unwrap();
    let (_,loaded)=crate::composition::load(&dir).unwrap().unwrap();
    std::fs::remove_dir_all(dir).unwrap();
    let ctx2=egui::Context::default();let mut restarted=CaligoApp::visual_fixture(&ctx2,Tab::Home,false);
    restarted.editor.document=loaded;
    for _ in 0..4{editor_frame(&ctx2,&mut restarted,vec![]);}
    assert_eq!(restarted.editor.document,saved);
    let p=restarted.editor.rects.iter().find(|(id,_)|*id==3).unwrap().1.center();
    click(&ctx2,&mut restarted,p);assert_eq!(restarted.tab,Tab::Instances);
}

#[test]
fn visual_review_editor_screens(){
    for (name,w,h,editing,custom) in [
        ("editor_default_1000",1000,620,true,false),
        ("editor_local_720",720,440,true,true),
        ("editor_local_1000",1000,620,true,true),
        ("composition_top_1000",1000,620,false,true),
        ("composition_top_720",720,440,false,true),
    ]{
        let ctx=egui::Context::default();ctx.set_pixels_per_point(1.0);
        let mut app=CaligoApp::visual_fixture(&ctx,Tab::Home,false);
        app.editor.document=crate::composition::Document::legacy();
        if custom{
            let p=app.editor.document.add_panel(Edge::Top,Position::default());
            app.editor.document.move_widget(3,Some(p),None,Position::default());
            let w=app.editor.document.widgets.iter_mut().find(|w|w.id==3).unwrap();
            w.style.rounding=Some(20.0);w.style.fill=Some([45,81,126,255]);
        }
        if editing{app.editor.begin();app.editor.test_select(3);}
        let mut textures=HashMap::new();let mut last=None;
        for i in 0..5 {
            let out=ctx.run(egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(w as f32,h as f32))),time:Some(i as f64/10.0),..Default::default()},|ctx|app.render(ctx));
            apply_delta(&mut textures,&out.textures_delta);last=Some(out);
        }
        if editing {
            let inspector=app.editor.controls.iter().find(|(s,_)|*s=="inspector").unwrap().1;
            let available_top=app.editor.panel_rects.iter().map(|(_,r)|r.top()).fold(f32::INFINITY,f32::min);
            assert!(inspector.top()>=available_top-1.0,"local inspector must not cover protected toolbar");
            assert!(inspector.bottom()<=h as f32,"local inspector must remain on screen");
        }
        let png=raster(&ctx,last.unwrap(),&textures,w,h);
        let mut buf=Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(png).write_to(&mut buf,image::ImageFormat::Png).unwrap();
        if std::env::var("CI").is_ok(){
            let data=base64::engine::general_purpose::STANDARD.encode(buf.into_inner());
            let mut stdout=std::io::stdout().lock();
            writeln!(stdout,"CALIGO_VISUAL_BEGIN {name}").unwrap();
            for chunk in data.as_bytes().chunks(6000){writeln!(stdout,"CALIGO_VISUAL_DATA {}",std::str::from_utf8(chunk).unwrap()).unwrap();}
            writeln!(stdout,"CALIGO_VISUAL_END {name}").unwrap();
        }
    }
}
#[test]
fn toolbar_controls_do_not_overlap_at_minimum_width(){
    let ctx=egui::Context::default();let mut app=CaligoApp::visual_fixture(&ctx,Tab::Home,false);
        app.editor.document=crate::composition::Document::legacy();app.editor.begin();
    for _ in 0..4{
        let _=ctx.run(egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(720.0,440.0))),..Default::default()},|ctx|app.render(ctx));
    }
    let controls:Vec<_>=["add_panel","undo","redo","cancel","finish"].iter().map(|name|app.editor.controls.iter().find(|(s,_)|s==name).unwrap().1).collect();
    for (i,a) in controls.iter().enumerate(){
        assert!(a.left()>=0.0&&a.right()<=720.0);
        for b in &controls[i+1..]{assert!(!a.intersects(*b),"toolbar controls overlap");}
    }
}
#[test]
fn workspace_components_are_real_independent_nodes(){
    use crate::composition::Action;
    let ctx=egui::Context::default();let mut app=CaligoApp::visual_fixture(&ctx,Tab::Home,true);
        app.editor.document=crate::composition::Document::legacy();
    for _ in 0..4{editor_frame(&ctx,&mut app,vec![]);}
    let find=|app:&CaligoApp,kind|app.editor.document.widgets.iter().find(|w|w.action==kind&&w.page==Some(Action::Home)).unwrap().id;
    let launch=find(&app,Action::Launch);let version=find(&app,Action::Version);
    assert!(app.editor.rects.iter().any(|(id,_)|*id==launch));
    assert!(app.editor.rects.iter().any(|(id,_)|*id==version));
    app.editor.begin();
    for _ in 0..4{editor_frame(&ctx,&mut app,vec![]);}
    let start=app.editor.rects.iter().find(|(id,_)|*id==launch).unwrap().1.center();
    drag_to(&ctx,&mut app,start,egui::pos2(550.0,330.0));
    let w=app.editor.document.widgets.iter().find(|w|w.id==launch).unwrap();
    assert!(!w.flow&&w.panel.is_none(),"launch component must really leave the layout");
    assert!(app.editor.document.widgets.iter().find(|w|w.id==version).unwrap().flow);
    assert!(matches!(app.launch.state(),crate::launch::LaunchState::Idle),"editing launch must never launch");
    app.editor.test_select(launch);
    for _ in 0..4{editor_frame(&ctx,&mut app,vec![]);}
    let slider=app.editor.controls.iter().find(|(s,_)|*s=="rounding").unwrap().1;
    click(&ctx,&mut app,slider.left_center()+egui::vec2(72.0,0.0));
    editor_frame(&ctx,&mut app,vec![]);
    assert!(app.editor.document.widgets.iter().find(|w|w.id==launch).unwrap().style.rounding.is_some());
    assert_eq!(app.editor.document.widgets.iter().find(|w|w.id==version).unwrap().style.rounding,None);
    app.editor.cancel();assert!(app.editor.document.widgets.iter().find(|w|w.id==launch).unwrap().flow);
}
#[test]
fn component_page_visibility_and_minimum_window_do_not_mutate_layout(){
    use crate::composition::Action;
    let ctx=egui::Context::default();let mut app=CaligoApp::visual_fixture(&ctx,Tab::Home,true);
        app.editor.document=crate::composition::Document::legacy();
    let initial=app.editor.document.clone();
    for tab in [Tab::Home,Tab::Instances,Tab::Settings]{
        app.tab=tab;
        for (w,h) in [(1000.0,620.0),(720.0,440.0),(1000.0,620.0)]{
            for _ in 0..3{
                let _=ctx.run(egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(w,h))),..Default::default()},|ctx|app.render(ctx));
            }
            let active=match tab{Tab::Home=>Action::Home,Tab::Instances=>Action::Library,Tab::Settings=>Action::Settings};
            for (id,rect) in &app.editor.rects {
                let node=app.editor.document.widgets.iter().find(|n|n.id==*id).unwrap();
                assert!(node.page.is_none_or(|p|p==active));
                assert!(rect.left()>=0.0&&rect.right()<=w+1.0&&rect.top()>=48.0&&rect.bottom()<=h+1.0);
            }
        }
    }
    assert_eq!(initial,app.editor.document);
}
#[test]
fn schema_one_migration_preserves_original_files_and_navigation(){
    let dir=std::env::temp_dir().join(format!("caligo-v1-{}-{}",std::process::id(),std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()));
    std::fs::create_dir_all(&dir).unwrap();
    let mut raw=serde_json::to_value(crate::composition::Document::default()).unwrap();
    raw["version"]=1.into();
    raw["widgets"].as_array_mut().unwrap().retain(|w|w["page"].is_null());
    raw["widgets"][1]["label"]="Старые сборки".into();
    for w in raw["widgets"].as_array_mut().unwrap(){let obj=w.as_object_mut().unwrap();obj.remove("page");obj.remove("flow");obj.remove("span");}
    let bytes=serde_json::to_vec(&serde_json::json!({"generation":1,"document":raw})).unwrap();
    std::fs::write(dir.join("interface-a.json"),&bytes).unwrap();
    let (g,doc)=crate::composition::load(&dir).unwrap().unwrap();
    assert_eq!(g,1);assert_eq!(doc.version,crate::composition::VERSION);assert_eq!(doc.widgets[1].label,"Старые сборки");
    assert!(doc.widgets.iter().any(|w|w.action==crate::composition::Action::Launch));
    assert_eq!(std::fs::read(dir.join("interface-a.json")).unwrap(),bytes,"reading must not persist migration");
    crate::composition::save(&dir,g,&doc).unwrap();
    assert_eq!(std::fs::read(dir.join("interface-a.json")).unwrap(),bytes,"first v2 save must preserve v1 slot");
    assert_eq!(crate::composition::load(&dir).unwrap().unwrap().1,doc);
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn flow_resize_changes_actual_width_and_library_selects_actual_version(){
    use crate::composition::Action;
    let ctx=egui::Context::default();let mut app=CaligoApp::visual_fixture(&ctx,Tab::Home,true);
        app.editor.document=crate::composition::Document::legacy();
    app.editor.begin();
    for _ in 0..4{editor_frame(&ctx,&mut app,vec![]);}
    let id=app.editor.document.widgets.iter().find(|w|w.action==Action::Launch).unwrap().id;
    let rect=app.editor.rects.iter().find(|(n,_)|*n==id).unwrap().1;
    click(&ctx,&mut app,rect.center());
    editor_frame(&ctx,&mut app,vec![]);
    let handle=app.editor.controls.iter().find(|(name,_)|*name=="resize").unwrap().1.center();
    drag_to(&ctx,&mut app,handle,handle-egui::vec2(74.0,0.0));
    let node=app.editor.document.widgets.iter().find(|w|w.id==id).unwrap();
    assert!(node.span<4,"horizontal flow resize must change its span");
    for _ in 0..3{editor_frame(&ctx,&mut app,vec![]);}
    let resized=app.editor.rects.iter().find(|(n,_)|*n==id).unwrap().1;
    assert!(resized.width()<rect.width()-30.0);
    app.editor.cancel();app.tab=Tab::Instances;
    for _ in 0..3{editor_frame(&ctx,&mut app,vec![]);}
    let library=app.editor.document.widgets.iter().find(|w|w.action==Action::LibraryList&&w.page==Some(Action::Library)).unwrap().id;
    let r=app.editor.rects.iter().find(|(id,_)|*id==library).unwrap().1;
    click(&ctx,&mut app,r.min+egui::vec2(90.0,68.0));
    assert_eq!(app.play.selected_instance.as_deref(),Some("Выживание"));
    assert_eq!(app.play.selected_version.as_deref(),Some("1.21.1"));
    assert_eq!(app.tab,Tab::Home);
    assert!(matches!(app.launch.state(),crate::launch::LaunchState::Idle));
}

#[test]
fn visual_review_launch_component_and_create_dialog(){
    use crate::composition::{Action,Anchor};
    for (name,w,h,editing) in [
        ("launch_component_local_1000",1000,620,true),
        ("launch_component_local_720",720,440,true),
        ("create_instance_720",720,440,false),
    ] {
        let ctx=egui::Context::default();ctx.set_pixels_per_point(1.0);
        let mut app=CaligoApp::visual_fixture(&ctx,Tab::Home,true);
        app.editor.document=crate::composition::Document::default();
        if editing{
            let node=app.editor.document.widgets.iter_mut().find(|w|w.action==Action::Launch).unwrap();
            node.flow=false;node.size=[200.0,48.0];
            node.position=Position{anchor:Anchor::TopRight,offset:[24.0,28.0]};
            node.style.rounding=Some(22.0);
            let id=node.id;app.editor.begin();app.editor.test_select(id);
        }else{crate::ui::instances::request_create(&mut app.instances);}
        let mut textures=HashMap::new();let mut last=None;
        for i in 0..5{
            let out=ctx.run(egui::RawInput{
                screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(w as f32,h as f32))),
                time:Some(i as f64/10.0),..Default::default()
            },|ctx|app.render(ctx));
            apply_delta(&mut textures,&out.textures_delta);last=Some(out);
        }
        assert!(matches!(app.launch.state(),crate::launch::LaunchState::Idle));
        let png=raster(&ctx,last.unwrap(),&textures,w,h);
        let mut buf=Cursor::new(Vec::new());
        image::DynamicImage::ImageRgba8(png).write_to(&mut buf,image::ImageFormat::Png).unwrap();
        if std::env::var("CI").is_ok(){
            let data=base64::engine::general_purpose::STANDARD.encode(buf.into_inner());
            let mut stdout=std::io::stdout().lock();
            writeln!(stdout,"CALIGO_VISUAL_BEGIN {name}").unwrap();
            for chunk in data.as_bytes().chunks(6000){writeln!(stdout,"CALIGO_VISUAL_DATA {}",std::str::from_utf8(chunk).unwrap()).unwrap();}
            writeln!(stdout,"CALIGO_VISUAL_END {name}").unwrap();
        }
    }
}

#[test]
fn responsive_flow_has_no_overlap_and_keeps_requested_geometry(){
    let doc=crate::composition::Document::default();
    let nodes:Vec<_>=doc.widgets.iter().filter(|w|w.flow&&w.page==Some(crate::composition::Action::Home)).cloned().collect();
    for width in [280.0,600.0,884.0]{
        let bounds=egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(width,1000.0));
        let rects=crate::composition::flow_rects(&nodes,bounds);
        for (i,(_,a)) in rects.iter().enumerate(){
            assert!(a.width()>0.0&&a.left()>=0.0&&a.right()<=width+1.0);
            for (_,b) in &rects[i+1..]{assert!(!a.intersects(*b));}
        }
    }
    assert_eq!(doc,crate::composition::Document::default());
}#[test]
fn rpg_actual_launch_moves_live_and_resizes_without_running() {
    use crate::composition::{Action,Document};
    let ctx=egui::Context::default();
    let mut app=CaligoApp::visual_fixture(&ctx,Tab::Home,false);
    app.editor.document=Document::preset(3);
    app.editor.begin();
    for _ in 0..4{editor_frame(&ctx,&mut app,vec![]);}
    let id=app.editor.document.widgets.iter().find(|w|w.action==Action::Launch).unwrap().id;
    let initial=app.editor.document.clone();
    let start=app.editor.rects.iter().find(|(i,_)|*i==id).unwrap().1.center();
    pointer(&ctx,&mut app,start,true);
    editor_frame(&ctx,&mut app,vec![egui::Event::PointerMoved(start+egui::vec2(9.0,0.0))]);
    let dest=start-egui::vec2(110.0,40.0);
    editor_frame(&ctx,&mut app,vec![egui::Event::PointerMoved(dest)]);
    editor_frame(&ctx,&mut app,vec![egui::Event::PointerMoved(dest)]);
    let during=app.editor.rects.iter().find(|(i,_)|*i==id).unwrap().1.center();
    assert!(during.distance(start)>80.0,"actual button must move before release");
    assert!(matches!(app.launch.state(),crate::launch::LaunchState::Idle));
    pointer(&ctx,&mut app,dest,false);
    for _ in 0..3{editor_frame(&ctx,&mut app,vec![]);}
    let node=app.editor.document.widgets.iter().find(|w|w.id==id).unwrap();
    assert!(node.relative.is_none()&&node.panel.is_none());
    let before_size=node.size;
    let handle=app.editor.controls.iter().find(|(n,_)|*n=="resize").unwrap().1.center();
    drag_to(&ctx,&mut app,handle,handle+egui::vec2(30.0,20.0));
    let node=app.editor.document.widgets.iter().find(|w|w.id==id).unwrap();
    assert!(node.size[0]>before_size[0]+20.0&&node.size[1]>before_size[1]+10.0);
    app.editor.test_select(id);
    for _ in 0..4{editor_frame(&ctx,&mut app,vec![]);}
    let radius=app.editor.controls.iter().find(|(n,_)|*n=="rounding").unwrap().1;
    click(&ctx,&mut app,radius.left_center()+egui::vec2(70.0,0.0));
    assert!(app.editor.document.widgets.iter().find(|w|w.id==id).unwrap().style.rounding.is_some());
    assert_eq!(app.editor.document.widgets.iter().find(|w|w.action==Action::Version).unwrap().style.rounding,None);
    app.editor.cancel();assert_eq!(app.editor.document,initial);
}
#[test]
fn rpg_draws_panel_with_pointer_rectangle() {
    let ctx=egui::Context::default();
    let mut app=CaligoApp::visual_fixture(&ctx,Tab::Home,false);
    app.editor.begin();
    for _ in 0..4{editor_frame(&ctx,&mut app,vec![]);}
    let add=app.editor.controls.iter().find(|(n,_)|*n=="add_panel").unwrap().1.center();
    click(&ctx,&mut app,add);
    drag_to(&ctx,&mut app,egui::pos2(60.0,180.0),egui::pos2(320.0,290.0));
    assert_eq!(app.editor.document.panels.len(),2);
    let panel=app.editor.document.panels.last().unwrap();
    assert_eq!(panel.edge,Edge::Float);
    assert!((panel.size[0]-260.0).abs()<2.0&&(panel.size[1]-110.0).abs()<2.0);
    assert!(app.editor.document.validate().is_ok());
}
#[test]
fn rpg_navigation_replaces_character_and_launch_at_all_sizes() {
    use crate::composition::{Action,Document};
    for preset in [1,3,6]{
        for (width,height) in [(1000.0,620.0),(720.0,440.0)]{
            let ctx=egui::Context::default();
            let mut app=CaligoApp::visual_fixture(&ctx,Tab::Home,false);
            app.editor.document=Document::preset(preset);
            let screen=egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(width,height));
            let frame=|app:&mut CaligoApp,events:Vec<egui::Event>|{
                let _=ctx.run(egui::RawInput{screen_rect:Some(screen),events,..Default::default()},|ctx|app.render(ctx));
            };
            for _ in 0..4{frame(&mut app,vec![]);}
            let home:Vec<_>=app.editor.document.widgets.iter().filter(|w|matches!(w.action,Action::Character|Action::Launch)).map(|w|w.id).collect();
            assert!(home.iter().all(|id|app.editor.rects.iter().any(|(i,_)|i==id)));
            let p=app.editor.rects.iter().find(|(i,_)|*i==3).unwrap().1.center();
            for pressed in [true,false]{frame(&mut app,vec![egui::Event::PointerMoved(p),egui::Event::PointerButton{pos:p,button:egui::PointerButton::Primary,pressed,modifiers:Default::default()}]);}
            for _ in 0..2{frame(&mut app,vec![]);}
            assert_eq!(app.tab,Tab::Instances);
            assert!(home.iter().all(|id|!app.editor.rects.iter().any(|(i,_)|i==id)));
        }
    }
}
#[test]
fn visual_review_rpg_presets(){
    for preset in [1,3,6]{
        for (w,h) in [(1000,620),(720,440)]{
            let ctx=egui::Context::default();
            let mut app=CaligoApp::visual_fixture(&ctx,Tab::Home,false);
            app.editor.document=crate::composition::Document::preset(preset);
            let mut textures=HashMap::new();let mut last=None;
            for i in 0..4 {
                let out=ctx.run(egui::RawInput{screen_rect:Some(egui::Rect::from_min_size(egui::Pos2::ZERO,egui::vec2(w as f32,h as f32))),time:Some(i as f64/10.0),..Default::default()},|ctx|app.render(ctx));
                apply_delta(&mut textures,&out.textures_delta);last=Some(out);
            }
            let png=raster(&ctx,last.unwrap(),&textures,w,h);
            let mut buf=Cursor::new(Vec::new());
            image::DynamicImage::ImageRgba8(png).write_to(&mut buf,image::ImageFormat::Png).unwrap();
            if std::env::var("CI").is_ok(){
                let data=base64::engine::general_purpose::STANDARD.encode(buf.into_inner());
                let mut stdout=std::io::stdout().lock();
                writeln!(stdout,"CALIGO_VISUAL_BEGIN rpg_{preset:02}_{w}").unwrap();
                for chunk in data.as_bytes().chunks(6000){writeln!(stdout,"CALIGO_VISUAL_DATA {}",std::str::from_utf8(chunk).unwrap()).unwrap();}
                writeln!(stdout,"CALIGO_VISUAL_END rpg_{preset:02}_{w}").unwrap();
            }
        }
    }
}