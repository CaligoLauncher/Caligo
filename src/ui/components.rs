//! Shared visual primitives. Original Caligo artwork; no third-party assets.
use eframe::egui::{self, pos2, vec2, Color32, FontId, Rect, Stroke};
use crate::theme::ThemePreset;

pub const GAP: f32 = 16.0;

pub fn label(p: &egui::Painter, rect: Rect, text: &str, size: f32, color: Color32) {
    let font = FontId::new(size,if size>=18.0 {egui::FontFamily::Name("heading".into())}else{egui::FontFamily::Proportional});
    let mut job = egui::text::LayoutJob::simple_singleline(text.to_owned(), font, color);
    job.wrap.max_width = rect.width().max(1.0);
    job.wrap.max_rows = 1;
    job.wrap.break_anywhere = true;
    let galley = p.layout_job(job);
    p.galley(pos2(rect.left(), rect.center().y - galley.size().y / 2.0), galley, color);
}

pub fn button(ui: &mut egui::Ui, text: &str, size: egui::Vec2, theme: &ThemePreset, primary: bool) -> egui::Response {
    let fill = if primary { theme.accent_color() } else { theme.surface(3) };
    let fg = if primary { contrast(fill) } else { theme.text_primary() };
    ui.add_sized(size, egui::Button::new(egui::RichText::new(text).size(13.0).color(fg))
        .fill(fill).rounding(egui::Rounding::same(theme.rounding))
        .stroke(Stroke::NONE))
}

pub fn contrast(c: Color32) -> Color32 {
    let linear = |x: u8| { let v = x as f32 / 255.0; if v <= 0.04045 {v/12.92} else {((v+0.055)/1.055).powf(2.4)} };
    let luminance = 0.2126*linear(c.r())+0.7152*linear(c.g())+0.0722*linear(c.b());
    if luminance > 0.179 { Color32::from_rgb(8,18,30) } else { Color32::WHITE }
}

pub fn cube(p: &egui::Painter, c: egui::Pos2, s: f32, col: Color32) {
    let f = |x:f32,y:f32| c + vec2(x*s,y*s);
    let st=Stroke::new(1.5_f32,col);
    for (a,b) in [((-1.0,-0.5),(0.0,-1.0)),((0.0,-1.0),(1.0,-0.5)),((1.0,-0.5),(1.0,0.5)),((1.0,0.5),(0.0,1.0)),((0.0,1.0),(-1.0,0.5)),((-1.0,0.5),(-1.0,-0.5)),((-1.0,-0.5),(0.0,0.0)),((1.0,-0.5),(0.0,0.0)),((0.0,0.0),(0.0,1.0))] {
        p.line_segment([f(a.0,a.1),f(b.0,b.1)],st);
    }
}

fn gradient(p: &egui::Painter,r:Rect,a:Color32,b:Color32,horizontal:bool) {
    let mut m=egui::Mesh::default();
    m.colored_vertex(r.left_top(),a);
    m.colored_vertex(r.right_top(),if horizontal {b} else {a});
    m.colored_vertex(r.right_bottom(),b);
    m.colored_vertex(r.left_bottom(),if horizontal {a} else {b});
    m.add_triangle(0,1,2); m.add_triangle(0,2,3);
    p.add(egui::Shape::mesh(m));
}

fn ridge(p:&egui::Painter,r:Rect,points:&[(f32,f32)],c:Color32) {
    let mut m=egui::Mesh::default();
    for &(x,y) in points {
        m.colored_vertex(pos2(r.left()+r.width()*x,r.top()+r.height()*y),c);
        m.colored_vertex(pos2(r.left()+r.width()*x,r.bottom()),c);
    }
    for i in 0..points.len()-1 {
        let a=(i*2) as u32;
        m.add_triangle(a,a+1,a+2);m.add_triangle(a+1,a+3,a+2);
    }
    p.add(egui::Shape::mesh(m));
}

fn pine(p:&egui::Painter,c:egui::Pos2,h:f32,color:Color32) {
    let w=h*0.26;
    p.rect_filled(Rect::from_center_size(c-vec2(0.0,h*0.20),vec2((h*0.04).max(1.0),h*0.42)),0.0,color);
    for (dy,scale) in [(0.85,0.48),(0.65,0.75),(0.40,1.0)] {
        let top=c-vec2(0.0,h*dy);
        p.add(egui::Shape::convex_polygon(vec![top-vec2(0.0,h*0.15),top+vec2(w*scale,h*0.35),top+vec2(-w*scale,h*0.35)],color,Stroke::NONE));
    }
}

/// Layered, angular valley at blue hour. Vector artwork always ships with the app.
/// The left side remains quiet for readable launch information; no external download.
pub fn landscape(p:&egui::Painter,r:Rect,quiet_left:bool) {
    let p=p.with_clip_rect(r.intersect(p.clip_rect()));
    gradient(&p,r,Color32::from_rgb(25,44,61),Color32::from_rgb(62,91,103),false);
    let moon=pos2(r.left()+r.width()*0.78,r.top()+r.height()*0.22);
    p.circle_filled(moon,r.height()*0.078,Color32::from_rgb(173,195,191));
    ridge(&p,r,&[(0.0,0.62),(0.10,0.40),(0.23,0.57),(0.40,0.25),(0.51,0.46),(0.58,0.39),(0.72,0.59),(0.90,0.33),(1.0,0.42)],Color32::from_rgb(56,78,90));
    ridge(&p,r,&[(0.0,0.61),(0.20,0.48),(0.33,0.74),(0.43,0.57),(0.57,0.79),(0.69,0.56),(0.76,0.60),(0.92,0.46),(1.0,0.52)],Color32::from_rgb(35,65,77));
    gradient(&p,Rect::from_min_max(pos2(r.left(),r.top()+r.height()*0.67),r.max),Color32::from_rgba_unmultiplied(122,162,162,0),Color32::from_rgb(78,119,127),false);
    ridge(&p,r,&[(0.0,0.70),(0.12,0.66),(0.28,0.73),(0.49,0.85),(0.60,1.0)],Color32::from_rgb(16,43,53));
    ridge(&p,r,&[(0.68,1.0),(0.61,0.89),(0.76,0.79),(0.82,0.76),(1.0,0.77)],Color32::from_rgb(20,49,57));
    for i in 0..19 {
        let x=0.01+i as f32*0.022;
        let y=0.70+x*0.28;
        let h=r.height()*(0.10+((i*7)%9) as f32*0.014);
        pine(&p,pos2(r.left()+x*r.width(),r.top()+y*r.height()),h,Color32::from_rgb(18,47,57));
    }
    for i in 0..14 {
        let x=0.79+i as f32*0.019;
        let h=r.height()*(0.14+((i*3)%7) as f32*0.02);
        pine(&p,pos2(r.left()+x*r.width(),r.top()+r.height()*0.84),h,Color32::from_rgb(13,37,45));
    }
    // Horizontal water reflections, not concentric glow bands.
    for i in 0..7 {
        let y=r.top()+r.height()*(0.83+i as f32*0.025);
        let cx=r.left()+r.width()*(0.62+i as f32*0.005);
        let half=r.width()*(0.016+i as f32*0.006);
        p.line_segment([pos2(cx-half,y),pos2(cx+half,y)],Stroke::new(1.0_f32,Color32::from_rgba_unmultiplied(180,213,209,35)));
    }
    if quiet_left {
        gradient(&p,r,Color32::from_rgba_unmultiplied(13,21,29,248),Color32::from_rgba_unmultiplied(13,21,29,0),true);
    }
}

pub fn page_title(ui:&mut egui::Ui,title:&str,subtitle:&str,theme:&ThemePreset) {
    ui.label(egui::RichText::new(title).font(FontId::new(26.0,egui::FontFamily::Name("heading".into()))).color(theme.text_primary()));
    if !subtitle.is_empty() {
        ui.label(egui::RichText::new(subtitle).size(13.0).color(theme.text_tertiary()));
    }
    ui.add_space(GAP);
}

pub fn empty(ui:&mut egui::Ui,title:&str,subtitle:&str,theme:&ThemePreset) {
    let (r,_)=ui.allocate_exact_size(vec2(ui.available_width(),78.0),egui::Sense::hover());
    cube(ui.painter(),pos2(r.left()+24.0,r.top()+35.0),12.0,theme.text_tertiary());
    label(ui.painter(),Rect::from_min_size(r.min+vec2(52.0,16.0),vec2(r.width()-64.0,24.0)),title,15.0,theme.text_primary());
    let mut sub=ui.new_child(egui::UiBuilder::new().max_rect(Rect::from_min_max(r.min+vec2(52.0,46.0),r.max)));
    sub.label(egui::RichText::new(subtitle).size(13.0).color(theme.text_tertiary()));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accent_text_is_dark_on_caligo_blue() {
        assert_ne!(contrast(ThemePreset::default().accent_color()),Color32::WHITE);
    }
}