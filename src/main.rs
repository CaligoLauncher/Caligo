#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

mod shell;
mod wallpaper;
mod settings_ui;
mod native_window;

use std::borrow::Cow;

use gpui::{
    App, Application, Bounds, KeyBinding, TitlebarOptions, WindowBounds, WindowOptions, prelude::*, px,
    size,
};
use shell::{FocusNext, FocusPrevious, Shell};

fn main() {
    Application::new().run(|cx: &mut App| {
        cx.text_system()
            .add_fonts(vec![Cow::Borrowed(include_bytes!(
                "../assets/fonts/Manrope.ttf"
            ))])
            .expect("Could not load the embedded Manrope font");

        cx.bind_keys([
            KeyBinding::new("tab", FocusNext, Some("Caligo")),
            KeyBinding::new("shift-tab", FocusPrevious, Some("Caligo")),
        ]);
        cx.on_window_closed(|cx| {
            if cx.windows().is_empty() {
                cx.quit();
            }
        })
        .detach();

        let bounds = Bounds::centered(None, size(px(1000.0), px(620.0)), cx);
        cx.open_window(
            WindowOptions {
                window_bounds: Some(WindowBounds::Windowed(bounds)),
                window_min_size: Some(size(px(720.0), px(440.0))),
                titlebar: Some(TitlebarOptions {
                    title: Some("Caligo".into()),
                    // Keep OS non-client layout/hit testing; native_window restores WS_CAPTION on Windows.
                    appears_transparent: false,
                    ..Default::default()
                }),
                app_id: Some("Caligo".into()),
                ..Default::default()
            },
            |window, cx| cx.new(|cx| Shell::new(window, cx)),
        )
        .expect("Could not open the Caligo window");
        cx.activate(true);
    });
}