//! Headless visual review of the *real egui UI*, not a web reimplementation.
//! Software rasterization checks layout and text; it does not replace native
//! Windows compositor, DPI, keyboard or mouse testing.
use std::collections::HashMap;
use std::io::{Cursor,Write};
use base64::Engine;
use eframe::egui::{self,Color32,ColorImage,TextureId};
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
        ("library_1000",1000,620,Tab::Instances,true),
        ("settings_1000",1000,620,Tab::Settings,false),
    ]{
        let ctx=egui::Context::default();
        ctx.set_pixels_per_point(1.0);
        let mut app=CaligoApp::visual_fixture(&ctx,tab,populated);
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