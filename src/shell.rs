use gpui::{
    Context, Div, FocusHandle, KeyDownEvent, Stateful, Window, actions, div, img, prelude::*, px, rgb, rgba,
};

use crate::{native_window, settings_ui::Appearance, ui::{self, ButtonKind, Icon}};

actions!(caligo, [FocusNext, FocusPrevious]);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Page {
    #[default]
    Home,
    Builds,
    Settings,
}

impl Page {
    const ALL: [Self; 3] = [Self::Home, Self::Builds, Self::Settings];

    fn index(self) -> usize {
        match self {
            Self::Home => 0,
            Self::Builds => 1,
            Self::Settings => 2,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Home => "Главная",
            Self::Builds => "Сборки",
            Self::Settings => "Настройки",
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Home => "home",
            Self::Builds => "builds",
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
    pub(crate) builds: crate::builds_ui::BuildsUi,
    native_error: Option<String>,
    root_focus: FocusHandle,
    button_focus: [FocusHandle; 3],
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
            builds: crate::builds_ui::BuildsUi::new(cx),
            native_error: None,
            navigation: Navigation::default(),
            root_focus,
            button_focus: [
                cx.focus_handle().tab_index(0).tab_stop(true),
                cx.focus_handle().tab_index(1).tab_stop(true),
                cx.focus_handle().tab_index(2).tab_stop(true),
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
        let symbol = match page {
            Page::Home => Icon::Home, Page::Builds => Icon::Layers, Page::Settings => Icon::Settings,
        };
        ui::control(page.id(), &self.button_focus[page.index()],
            if selected { ButtonKind::Selected } else { ButtonKind::Quiet }, true)
            .relative().w_full().h(px(44.0)).justify_start().gap(px(12.0))
            .bg(rgb(if selected { ui::SELECTED } else { ui::SURFACE }))
            .border_color(rgb(ui::SURFACE))
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
            .when(selected, |button| button.child(div().absolute().left(px(0.0))
                .top(px(13.0)).w(px(3.0)).h(px(16.0))
                .rounded(px(2.0)).bg(rgb(ui::ACCENT))))
            .child(ui::icon(symbol, 19.0, if selected { ui::ACCENT } else { ui::MUTED }))
            .child(page.label())
    }
}

impl Render for Shell {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .id("caligo")
            .key_context("Caligo")
            .track_focus(&self.root_focus)
            .on_action(|_: &FocusNext, window, _| window.focus_next())
            .on_action(|_: &FocusPrevious, window, _| window.focus_prev())
            .size_full()
            .relative().overflow_hidden()
            .flex()
            .bg(rgb(ui::BACKGROUND))
            .text_color(rgb(ui::TEXT))
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
                ui::panel()
                    .id("left-panel").overflow_y_scroll()
                    .w(px(180.0)).flex_shrink_0()
                    .m(px(16.0)).p(px(10.0))
                    .flex().flex_col().gap(px(6.0))
                    .bg(rgb(ui::SURFACE))
                    .child(div().flex_shrink_0().px(px(10.0)).pt(px(10.0)).pb(px(24.0))
                        .text_size(px(23.0)).line_height(px(30.0)).child("Caligo"))
                    .children([Page::Home, Page::Builds].into_iter().map(|page| self.button(page, cx)))
                    .when(self.navigation.selected == Page::Builds, |panel| {
                        panel.child(self.builds_sidebar(cx))
                    })
                    .child(div().flex_1().min_h(px(24.0)))
                    .child(self.button(Page::Settings, cx)),
            )
            .when(self.navigation.selected == Page::Home, |root| {
                root.child(div().flex_1().h_full())
            })
            .when(self.navigation.selected == Page::Builds, |root| {
                root.child(self.builds_page(cx))
            })
            .when(self.navigation.selected == Page::Settings, |root| {
                root.child(self.settings_page(window.viewport_size().width >= px(960.0), cx))
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
    fn home_builds_and_settings_have_unique_navigation_slots() {
        assert_eq!(Page::ALL, [Page::Home, Page::Builds, Page::Settings]);
        assert_eq!(Page::Home.index(), 0);
        assert_eq!(Page::Builds.index(), 1);
        assert_eq!(Page::Settings.index(), 2);
        assert_ne!(Page::Builds.id(), Page::Settings.id());
        assert_ne!(Page::Builds.id(), Page::Home.id());
        assert_ne!(Page::Home.id(), Page::Settings.id());
    }
}