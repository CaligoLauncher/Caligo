//! Original procedural scenery; no third-party wallpaper or launcher artwork.
use eframe::egui::{self,pos2,vec2,Color32,Rect,Stroke};
pub fn wallpaper()->image::RgbaImage {
    let (w,h)=(1280,800);
    image::RgbaImage::from_fn(w,h,|x,y|{
        let u=x as f32/w as f32;let v=y as f32/h as f32;
        let top=[35.0,68.0,101.0];let bottom=[104.0,144.0,161.0];
        let mut c=[0_u8;4];
        for k in 0..3{c[k]=(top[k]+(bottom[k]-top[k])*v) as u8;}c[3]=255;
        let ridge=0.38+0.085*(u*13.0).sin()+0.025*(u*47.0).sin();
        if v>ridge{c=[61,95,121,255];}
        let ridge2=0.52+0.095*(u*10.0+1.3).sin()+0.018*(u*56.0).sin();
        if v>ridge2{c=[40,72,99,255];}
        if v>0.70 {
            let t=(v-0.70)/0.30; c=[(39.0-19.0*t) as u8,(69.0-31.0*t) as u8,(94.0-40.0*t) as u8,255];
            let ripple=(u*220.0+v*80.0).sin()*(v*310.0).sin();
            for k in 0..3{c[k]=(c[k] as f32+ripple*1.8).clamp(0.0,255.0) as u8;}
        }
        let shore=0.91+0.08*(u*6.0+0.4).sin();
        if v>shore{c=[17,33,48,255];}
        c.into()
    })
}
pub fn cover(p:&egui::Painter,r:Rect,radius:f32){
    p.rect_filled(r,radius,Color32::from_rgb(31,55,74));
    let clip=p.with_clip_rect(r.shrink(8.0));
    for (y,color) in [(0.40,Color32::from_rgb(57,90,110)),(0.65,Color32::from_rgb(39,73,89))] {
        let points=vec![pos2(r.left()+8.0,r.bottom()-8.0),pos2(r.left()+8.0,r.top()+r.height()*y),
            pos2(r.left()+r.width()*0.29,r.top()+r.height()*(y-0.22)),
            pos2(r.left()+r.width()*0.52,r.top()+r.height()*(y+0.09)),
            pos2(r.left()+r.width()*0.78,r.top()+r.height()*(y-0.13)),
            pos2(r.right()-8.0,r.top()+r.height()*y),pos2(r.right()-8.0,r.bottom()-8.0)];
        // Concave silhouette built as a triangle fan along a lower baseline.
        for pair in points[1..points.len()-1].windows(2) {
            let base=pos2(pair[0].x,r.bottom()-8.0);
            let end=pos2(pair[1].x,r.bottom()-8.0);
            clip.add(egui::Shape::convex_polygon(vec![base,pair[0],pair[1],end],color,Stroke::NONE));
        }
    }
    let label=Rect::from_min_size(r.left_bottom()+vec2(14.0,-30.0),vec2(r.width()-28.0,22.0));
    crate::ui::components::label(p,label,"VANILLA",10.0,Color32::from_rgb(208,224,233));
}