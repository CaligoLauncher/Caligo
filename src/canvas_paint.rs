use eframe::egui::{self,Color32,Rect,pos2,vec2};
pub fn rgba(c:[u8;4])->Color32{Color32::from_rgba_unmultiplied(c[0],c[1],c[2],c[3])}
pub fn text(p:&egui::Painter,r:Rect,s:&str,size:f32,c:Color32,center:bool){
    let font=egui::FontId::new(size,if size>=20.0{egui::FontFamily::Name("heading".into())}else{egui::FontFamily::Proportional});
    let mut job=egui::text::LayoutJob::simple_singleline(s.into(),font,c);
    job.wrap.max_width=r.width().max(1.0);job.wrap.max_rows=1;job.wrap.break_anywhere=true;
    let galley=p.layout_job(job);
    let x=if center{r.center().x-galley.size().x/2.0}else{r.left()};
    p.galley(pos2(x,r.center().y-galley.size().y/2.0),galley,c);
}
pub fn setup(ctx:&egui::Context){
    let mut fonts=egui::FontDefinitions::default();
    fonts.font_data.insert("manrope".into(),egui::FontData::from_static(include_bytes!("../assets/fonts/Manrope.ttf")));
    fonts.font_data.insert("manrope-semibold".into(),egui::FontData::from_static(include_bytes!("../assets/fonts/Manrope-SemiBold.ttf")));
    fonts.families.get_mut(&egui::FontFamily::Proportional).unwrap().insert(0,"manrope".into());
    let mut heading=fonts.families[&egui::FontFamily::Proportional].clone();heading.insert(0,"manrope-semibold".into());
    fonts.families.insert(egui::FontFamily::Name("heading".into()),heading);ctx.set_fonts(fonts);
    let mut v=egui::Visuals::dark();v.window_fill=Color32::from_rgb(22,32,44);v.panel_fill=Color32::TRANSPARENT;
    v.widgets.inactive.weak_bg_fill=Color32::from_rgb(38,52,66);
    v.widgets.hovered.weak_bg_fill=Color32::from_rgb(52,73,89);
    v.selection.bg_fill=Color32::from_rgb(49,97,115);
    ctx.set_visuals(v);
    ctx.style_mut(|s|{s.spacing.interact_size=vec2(36.0,30.0);s.spacing.item_spacing=vec2(8.0,8.0);s.spacing.button_padding=vec2(10.0,5.0);});
}
pub struct Wallpaper{normal:egui::TextureHandle,blurred:egui::TextureHandle}
impl Wallpaper{
    pub fn load(ctx:&egui::Context,test:bool)->Self{
        let custom=if test{None}else{
            ["background.png","background.jpg","background.jpeg"].iter().find_map(|n|image::open(crate::launch::install::game_dir().join(n)).ok())
        };
        let img=custom.unwrap_or_else(||image::DynamicImage::ImageRgba8(Self::fallback())).thumbnail(1600,1000).to_rgba8();
        let blur=image::imageops::fast_blur(&img,18.0);
        let size=[img.width()as usize,img.height()as usize];
        Self{normal:ctx.load_texture("shell-wallpaper",egui::ColorImage::from_rgba_unmultiplied(size,img.as_raw()),egui::TextureOptions::LINEAR),blurred:ctx.load_texture("shell-wallpaper-soft",egui::ColorImage::from_rgba_unmultiplied(size,blur.as_raw()),egui::TextureOptions::LINEAR)}
    }
    fn uv(&self,screen:Rect,part:Rect)->Rect{
        let t=self.normal.size_vec2();let sa=screen.width()/screen.height();let ta=t.x/t.y;
        let (x,y,w,h)=if ta>sa{let w=sa/ta;((1.0-w)/2.0,0.0,w,1.0)}else{let h=ta/sa;(0.0,(1.0-h)/2.0,1.0,h)};
        let at=|p:egui::Pos2|pos2(x+(p.x-screen.left())/screen.width()*w,y+(p.y-screen.top())/screen.height()*h);
        Rect::from_min_max(at(part.min),at(part.max))
    }
    pub fn paint(&self,ctx:&egui::Context){let r=ctx.screen_rect();ctx.layer_painter(egui::LayerId::background()).image(self.normal.id(),r,self.uv(r,r),Color32::WHITE);}
    pub fn surface(&self,p:&egui::Painter,r:Rect,style:&crate::canvas_model::Style){
        if style.blur{
            let mut shape=egui::epaint::RectShape::new(r,egui::Rounding::same(style.radius),Color32::WHITE,egui::Stroke::NONE);
            shape.fill_texture_id=self.blurred.id();shape.uv=self.uv(p.ctx().screen_rect(),r);p.add(shape);
        }
        p.rect_filled(r,style.radius,rgba(style.fill));
    }
    /// Original calm landscape; custom artwork is never downloaded or copied.
    fn fallback()->image::RgbaImage{
        let(w,h)=(1000,620);
        image::RgbaImage::from_fn(w,h,|x,y|{
            let fx=x as f32/w as f32;let fy=y as f32/h as f32;
            let sun=((-((fx-0.23).powi(2)*9.0+(fy-0.24).powi(2)*7.0)).exp()*0.55).clamp(0.0,1.0);
            let ridge=0.59+0.07*(fx*9.0).sin()+0.035*(fx*21.0).cos();
            let back=if fy>ridge {0.50+0.28*fy}else{1.0};
            let front=0.87+0.055*(fx*11.0).cos();
            let shade=if fy>front{0.53}else{back};
            let base=[30.0+28.0*sun,46.0+37.0*sun,62.0+37.0*sun];
            let noise=(((x.wrapping_mul(73856093)^y.wrapping_mul(19349663))%7)as f32-3.0)*0.14;
            image::Rgba([(base[0]*shade+noise)as u8,(base[1]*shade+noise)as u8,(base[2]*shade+noise)as u8,255])
        })
    }
}