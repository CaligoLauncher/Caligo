use gpui::{
    Context, Div, FocusHandle, KeyDownEvent, Stateful, Window, actions, div, img, prelude::*, px, rgb, rgba,
};

use crate::{native_window, settings_ui::Appearance};

actions!(caligo, [FocusNext, FocusPrevious]);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Page {
    #[default]
    Home,
    Settings,
}

impl Page {
    const ALL: [Self; 2] = [Self::Home, Self::Settings];

    fn index(self) -> usize {
        match self {
            Self::Home => 0,
            Self::Settings => 1,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Home => "Главная",
            Self::Settings => "Настройки",
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::Settings => "settings",
        }
    }
}

#[derive(Default)]
struct Navigation {
    selected: Page,
}

impl Navigation {
    /// Return whether navigation actually changed.
    fn select(&mut self, page: Page) -> bool {
        if self.selected == page {
            return false;
        }
        self.selected = page;
        true
    }
}

pub struct Shell {
    navigation: Navigation,
    pub(crate) appearance: Appearance,
    native_error: Option<String>,
    root_focus: FocusHandle,
    button_focus: [FocusHandle; 2],
}

impl Shell {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let root_focus = cx.focus_handle();
        window.focus(&root_focus);
        let native_titlebar = native_window::prepare_native_titlebar(window);
        cx.spawn_in(window, async move |this, cx| {
            if this.upgrade().is_none() { return; }
            // Outside an App/Window borrow: Win32 frame changes send resize callbacks.
            let error = native_titlebar.and_then(|apply| apply()).err();
            let _ = this.update_in(cx, |this, window, cx| {
                this.native_error = error;
                window.refresh();
                cx.notify();
            });
        }).detach();
        let mut shell = Self {
            appearance: Appearance::new(cx),
            native_error: None,
            navigation: Navigation::default(),
            root_focus,
            button_focus: [
                cx.focus_handle().tab_index(0).tab_stop(true),
                cx.focus_handle().tab_index(1).tab_stop(true),
            ],
        };
        shell.restore_wallpaper(window, cx);
        shell
    }

    fn select(&mut self, page: Page, cx: &mut Context<Self>) {
        if self.navigation.select(page) {
            cx.notify();
        }
    }

    fn button(&self, page: Page, cx: &mut Context<Self>) -> Stateful<Div> {
        let selected = self.navigation.selected == page;
        div()
            .id(page.id())
            .track_focus(&self.button_focus[page.index()])
            .w_full()
            .h(px(44.0))
            .flex()
            .items_center()
            .gap(px(12.0))
            .px(px(14.0))
            .rounded(px(10.0))
            .border_1()
            .border_color(rgb(if selected { 0x344154 } else { 0x1b2029 }))
            .bg(rgb(if selected { 0x283343 } else { 0x1b2029 }))
            .text_color(rgb(if selected { 0xedf3fc } else { 0xb5bfce }))
            .cursor_pointer()
            .hover(move |style| {
                style.bg(rgb(if selected { 0x303e51 } else { 0x252d39 }))
            })
            .active(|style| style.bg(rgb(0x354358)))
            .focus(|style| style.border_color(rgb(0x9dbce6)))
            .on_click(cx.listener(move |this, _, window, cx| {
                window.focus(&this.button_focus[page.index()]);
                this.select(page, cx);
            }))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                if !event.is_held
                    && matches!(event.keystroke.key.as_str(), "enter" | "space")
                {
                    this.select(page, cx);
                    cx.stop_propagation();
                }
            }))
            .child(
                div()
                    .w(px(3.0))
                    .h(px(16.0))
                    .rounded(px(2.0))
                    .bg(rgb(if selected { 0xa7c4ed } else { 0x667489 })),
            )
            .child(page.label())
    }
}

impl Render for Shell {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("caligo")
            .key_context("Caligo")
            .track_focus(&self.root_focus)
            .on_action(|_: &FocusNext, window, _| window.focus_next())
            .on_action(|_: &FocusPrevious, window, _| window.focus_prev())
            .size_full()
            .relative().overflow_hidden()
            .flex()
            .bg(rgb(0x12161d))
            .text_color(rgb(0xedf3fc))
            .font_family("Manrope")
            .text_size(px(14.0))
            .when_some(self.appearance.image.clone(), |root, image| {
                root.child(img(image).absolute().top(px(0.0)).left(px(0.0)).size_full().object_fit(self.wallpaper_fit()))
            })
            .when(self.appearance.image.is_some() && self.appearance.preferences.dim_percent > 0, |root| {
                root.child(div().absolute().top(px(0.0)).left(px(0.0)).size_full()
                    .bg(rgba(self.appearance.preferences.overlay_rgba())))
            })
            .child(
                div()
                    .id("left-panel")
                    .w(px(208.0))
                    .flex_shrink_0()
                    .m(px(16.0))
                    .p(px(12.0))
                    .flex()
                    .flex_col()
                    .gap(px(8.0))
                    .rounded(px(16.0))
                    .border_1()
                    .border_color(rgb(0x2c3441))
                    .bg(rgb(0x1b2029))
                    .child(
                        div()
                            .px(px(14.0))
                            .pt(px(10.0))
                            .pb(px(24.0))
                            .text_size(px(20.0))
                            .child("Caligo"),
                    )
                    .children(Page::ALL.into_iter().map(|page| self.button(page, cx))),
            )
            .when(self.navigation.selected == Page::Home, |root| {
                root.child(div().flex_1().h_full())
            })
            .when(self.navigation.selected == Page::Settings, |root| {
                root.child(self.settings_page(cx))
            })
            .when_some(self.native_error.clone(), |root, error| {
                root.child(div().absolute().bottom(px(12.0)).right(px(16.0))
                    .max_w(px(430.0)).p(px(12.0)).rounded(px(8.0))
                    .bg(rgb(0x47282b)).text_color(rgb(0xffd2ce))
                    .child(format!("Не удалось включить системный заголовок: {error}")))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::{Navigation, Page};

    #[test]
    fn home_is_selected_initially() {
        assert_eq!(Navigation::default().selected, Page::Home);
    }

    #[test]
    fn navigation_switches_both_ways() {
        let mut navigation = Navigation::default();
        assert!(navigation.select(Page::Settings));
        assert_eq!(navigation.selected, Page::Settings);
        assert!(navigation.select(Page::Home));
        assert_eq!(navigation.selected, Page::Home);
    }

    #[test]
    fn reselecting_does_not_request_a_state_change() {
        let mut navigation = Navigation::default();
        assert!(!navigation.select(Page::Home));
        assert!(navigation.select(Page::Settings));
        assert!(!navigation.select(Page::Settings));
    }

    #[test]
    fn only_home_and_settings_are_available() {
        assert_eq!(Page::ALL, [Page::Home, Page::Settings]);
        assert_eq!(Page::Home.index(), 0);
        assert_eq!(Page::Settings.index(), 1);
        assert_ne!(Page::Home.id(), Page::Settings.id());
    }
}