use std::sync::Arc;

use gpui::{
    Context, Div, FocusHandle, KeyDownEvent, PathPromptOptions, RenderImage,
    Stateful, Window, div, img, prelude::*, px, rgb, rgba,
};

use crate::{appearance_settings::{self, AppearanceSettings}, shell::Shell, wallpaper, ui::{self, ButtonKind, Icon}};

#[derive(Clone, Copy)]
pub(crate) enum WallpaperAction {
    Choose,
    Reset,
}

impl WallpaperAction {
    fn index(self) -> usize {
        match self { Self::Choose => 0, Self::Reset => 1 }
    }

    fn label(self) -> &'static str {
        match self { Self::Choose => "Выбрать изображение…", Self::Reset => "Убрать изображение" }
    }

    fn id(self) -> &'static str {
        match self { Self::Choose => "choose-wallpaper", Self::Reset => "reset-wallpaper" }
    }
}

pub(crate) struct Appearance {
    pub(crate) image: Option<Arc<RenderImage>>,
    pub(crate) busy: bool,
    pub(crate) preferences: AppearanceSettings,
    pub(crate) preferences_error: Option<String>,
    pub(crate) preferences_message: Option<String>,
    pub(crate) preferences_focus: [FocusHandle; 7],
    message: Option<String>,
    error: bool,
    focus: [FocusHandle; 2],
}

impl Appearance {
    pub(crate) fn new(cx: &mut Context<Shell>) -> Self {
        Self {
            image: None,
            busy: true,
            preferences: AppearanceSettings::default(),
            preferences_error: None,
            preferences_message: None,
            preferences_focus: std::array::from_fn(|i| {
                cx.focus_handle().tab_index((i + 5) as isize).tab_stop(true)
            }),
            message: Some("Загрузка настроек…".into()),
            error: false,
            focus: [
                cx.focus_handle().tab_index(3).tab_stop(true),
                cx.focus_handle().tab_index(4).tab_stop(true),
            ],
        }
    }
}

fn render_image(mut rgba: image::RgbaImage) -> Arc<RenderImage> {
    // GPUI RenderImage consumes BGRA, not RGBA. One frame stays static.
    for pixel in rgba.chunks_exact_mut(4) {
        pixel.swap(0, 2);
    }
    Arc::new(RenderImage::new(vec![image::Frame::new(rgba)]))
}

impl Shell {
    pub(crate) fn restore_wallpaper(&mut self, window: &Window, cx: &mut Context<Self>) {
        let task = cx.background_executor().spawn(async {
            match wallpaper::directory() {
                Ok(root) => (
                    wallpaper::restore(&root).map(|image| image.map(render_image)),
                    appearance_settings::load(&root),
                ),
                Err(error) => (Err(error.clone()), Err(error)),
            }
        });
        cx.spawn_in(window, async move |this, cx| {
            let (image, preferences) = task.await;
            let _ = this.update_in(cx, |this, window, cx| {
                match preferences {
                    Ok(value) => this.appearance.preferences = value,
                    Err(error) => this.appearance.preferences_error = Some(error),
                }
                this.finish_wallpaper(image, None, window, cx);
            });
        }).detach();
    }

    fn finish_wallpaper(
        &mut self,
        result: Result<Option<Arc<RenderImage>>, String>,
        success: Option<&str>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.appearance.busy = false;
        match result {
            Ok(image) => {
                if let Some(old) = self.appearance.image.take() {
                    let _ = window.drop_image(old);
                }
                self.appearance.image = image;
                self.appearance.message = success.map(str::to_owned);
                self.appearance.error = false;
            }
            Err(error) => {
                // An invalid file / failed save does not replace the visible or saved image.
                self.appearance.message = Some(error);
                self.appearance.error = true;
            }
        }
        cx.notify();
    }

    fn wallpaper_action(&mut self, action: WallpaperAction, window: &mut Window, cx: &mut Context<Self>) {
        if self.appearance.busy { return; }
        self.appearance.busy = true;
        self.appearance.error = false;
        self.appearance.message = Some(match action {
            WallpaperAction::Choose => "Выберите PNG или JPEG…",
            WallpaperAction::Reset => "Сброс фона…",
        }.into());
        cx.notify();
        match action {
            WallpaperAction::Choose => {
                let dialog = cx.prompt_for_paths(PathPromptOptions {
                    files: true,
                    directories: false,
                    multiple: false,
                    prompt: Some("Выбрать обои".into()),
                });
                let executor = cx.background_executor().clone();
                cx.spawn_in(window, async move |this, cx| {
                    let selected = match dialog.await {
                        Ok(Ok(paths)) => Ok(paths.and_then(|paths| paths.into_iter().next())),
                        Ok(Err(error)) => Err(format!("Не удалось открыть выбор файла: {error}")),
                        Err(error) => Err(format!("Окно выбора файла недоступно: {error}")),
                    };
                    if matches!(selected, Ok(None)) {
                        let _ = this.update_in(cx, |this, _, cx| {
                            this.appearance.busy = false;
                            this.appearance.message = None;
                            cx.notify();
                        });
                        return;
                    }
                    let result = match selected {
                        Ok(Some(path)) => executor.spawn(async move {
                            let root = wallpaper::directory()?;
                            wallpaper::choose(&root, &path).map(|image| Some(render_image(image)))
                        }).await,
                        Err(error) => Err(error),
                        Ok(None) => unreachable!(),
                    };
                    let _ = this.update_in(cx, |this, window, cx| {
                        this.finish_wallpaper(result, Some("Обои сохранены."), window, cx);
                    });
                }).detach();
            }
            WallpaperAction::Reset => {
                let task = cx.background_executor().spawn(async {
                    wallpaper::reset(&wallpaper::directory()?)?;
                    Ok(None)
                });
                cx.spawn_in(window, async move |this, cx| {
                    let result = task.await;
                    let _ = this.update_in(cx, |this, window, cx| {
                        this.finish_wallpaper(result, Some("Изображение убрано. Режим и затемнение сохранены."), window, cx);
                    });
                }).detach();
            }
        }
    }

    fn wallpaper_button(&self, action: WallpaperAction, cx: &mut Context<Self>) -> Stateful<Div> {
        let busy = self.appearance.busy;
        let choose = matches!(action, WallpaperAction::Choose);
        ui::control(action.id(), &self.appearance.focus[action.index()],
            if choose { ButtonKind::Primary } else { ButtonKind::Quiet }, !busy)
            .on_click(cx.listener(move |this, _, window, cx| {
                window.focus(&this.appearance.focus[action.index()]);
                this.wallpaper_action(action, window, cx);
            }))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                if !event.is_held && matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    this.wallpaper_action(action, window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(ui::icon(if choose { Icon::Picture } else { Icon::Remove }, 16.0,
                if busy { ui::MUTED } else if choose { ui::PRIMARY_TEXT } else { ui::SECONDARY }))
            .child(action.label())
    }

    pub(crate) fn settings_page(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        div().id("settings-page").flex_1().min_w_0().h_full().overflow_y_scroll()
            .pr(px(16.0)).py(px(16.0))
            .child(div().w_full().max_w(px(660.0)).flex().flex_col().gap(px(16.0))
                .child(ui::panel().p(px(22.0)).flex().flex_col().gap(px(6.0))
                    .child(ui::heading("Оформление"))
                    .child(ui::hint("Настройки внешнего вида Caligo")))
                .child(ui::panel().p(px(22.0)).flex().flex_col().gap(px(16.0))
                    .child(div().flex().items_center().gap(px(10.0))
                        .child(ui::icon(Icon::Picture, 20.0, ui::ACCENT))
                        .child(ui::section_title("Обои")))
                    .child(div().relative().w_full().h(px(164.0)).flex_shrink_0().overflow_hidden()
                        .rounded(px(10.0)).border_1().border_color(rgb(ui::EDGE))
                        .bg(rgb(ui::BACKGROUND))
                        .when_some(self.appearance.image.clone(), |view, image| {
                            view.child(img(image).absolute().top(px(0.0)).left(px(0.0))
                                .size_full().object_fit(self.wallpaper_fit()))
                                .when(self.appearance.preferences.dim_percent > 0, |view| {
                                    view.child(div().absolute().top(px(0.0)).left(px(0.0)).size_full()
                                        .bg(rgba(self.appearance.preferences.overlay_rgba())))
                                })
                        })
                        .when(self.appearance.image.is_none(), |view| {
                            view.flex().flex_col().items_center().justify_center().gap(px(12.0))
                                .child(ui::emblem(Icon::Picture, 44.0))
                                .child(ui::hint("Выбери изображение для своего пространства"))
                        }))
                    .child(div().flex().flex_wrap().gap(px(8.0))
                        .child(self.wallpaper_button(WallpaperAction::Choose, cx))
                        .child(self.wallpaper_button(WallpaperAction::Reset, cx)))
                    .child(ui::hint("PNG или JPEG, до 32 МБ. Сохраняется отдельная копия — оригинал не меняется."))
                    .when_some(self.appearance.message.clone(), |view, message| {
                        view.child(div().p(px(12.0)).rounded(px(8.0))
                            .bg(rgb(if self.appearance.error { 0x332326 } else { ui::BACKGROUND }))
                            .text_size(px(13.0)).line_height(px(20.0))
                            .text_color(rgb(if self.appearance.error { ui::ERROR } else { ui::SUCCESS }))
                            .child(message))
                    }))
                .child(self.appearance_controls(cx)))
    }

}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn gpui_receives_bgra_single_frame_without_color_swap() {
        let rgba = image::RgbaImage::from_pixel(1, 1, image::Rgba([240, 40, 10, 255]));
        let image = render_image(rgba);
        assert_eq!(image.frame_count(), 1);
        assert_eq!(image.as_bytes(0).unwrap(), &[10, 40, 240, 255]);
    }
}