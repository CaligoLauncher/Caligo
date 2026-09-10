use gpui::{
    Context, Div, FocusHandle, KeyDownEvent, Stateful, Window,
    div, prelude::*, px, rgb, rgba,
};

use crate::{
    builds::{BuildControl, BuildPreview, BuildTab},
    shell::Shell,
};

pub(crate) struct BuildsUi {
    pub(crate) preview: BuildPreview,
    focus: [FocusHandle; 8],
}

impl BuildsUi {
    pub(crate) fn new(cx: &mut Context<Shell>) -> Self {
        Self {
            preview: BuildPreview::default(),
            // 0..2: navigation; 3..11: appearance. Only rendered controls are tab stops.
            focus: std::array::from_fn(|i| cx.focus_handle().tab_index((12 + i) as isize).tab_stop(true)),
        }
    }
}

/// An original geometric placeholder, not a loader logo or a borrowed modpack asset.
fn build_cover() -> Div {
    div().size(px(52.0)).flex_shrink_0().flex().items_center().justify_center()
        .rounded(px(13.0)).border_1().border_color(rgb(0x4b607c)).bg(rgb(0x283343))
        .child(div().size(px(24.0)).rounded(px(5.0)).border_2()
            .border_color(rgb(0xa7c4ed)).flex().items_center().justify_center()
            .child(div().size(px(8.0)).rounded(px(2.0)).bg(rgb(0xa7c4ed))))
}

fn muted_action(label: &'static str, primary: bool) -> Div {
    // Intentionally not interactive or focusable: no operation is implemented behind it.
    div().h(px(40.0)).px(px(14.0)).flex().items_center().justify_center()
        .rounded(px(9.0)).border_1().border_color(rgb(0x344154))
        .bg(rgb(if primary { 0x283343 } else { 0x1b2029 }))
        .text_color(rgb(0x8995a8)).child(label)
}

impl Shell {
    fn build_action(&mut self, control: BuildControl, window: &mut Window, cx: &mut Context<Self>) {
        // Moving to the empty state removes the header controls. Its initiating
        // sidebar control remains mounted; the empty-state CTA focuses the example.
        let target = if control == BuildControl::TryExample {
            BuildControl::Example
        } else {
            control
        };
        window.focus(&self.builds.focus[target.index()]);
        if self.builds.preview.apply(control) {
            cx.notify();
        }
    }

    fn build_button(
        &self, control: BuildControl, label: &'static str, cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let selected = control.selected(&self.builds.preview);
        div().id(control.id()).track_focus(&self.builds.focus[control.index()])
            .h(px(40.0)).px(px(12.0)).flex_shrink_0()
            .flex().items_center().justify_center()
            .rounded(px(9.0)).border_1()
            .border_color(rgb(if selected { 0x526984 } else { 0x344154 }))
            .bg(rgb(if selected { 0x283343 } else { 0x1b2029 }))
            .text_color(rgb(if selected { 0xedf3fc } else { 0xb5bfce }))
            .cursor_pointer()
            .hover(|style| style.bg(rgb(0x303e51)))
            .active(|style| style.bg(rgb(0x354358)))
            .focus(|style| style.border_color(rgb(0xe2edff)))
            .on_click(cx.listener(move |this, _, window, cx| {
                this.build_action(control, window, cx);
            }))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                if !event.is_held && matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    this.build_action(control, window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(label)
    }

    pub(crate) fn builds_sidebar(&self, cx: &mut Context<Self>) -> Div {
        div().flex_shrink_0().mt(px(16.0)).pt(px(16.0)).border_t_1().border_color(rgb(0x2c3441))
            .flex().flex_col().gap(px(8.0))
            .child(div().px(px(8.0)).pb(px(4.0)).text_size(px(11.0))
                .text_color(rgb(0x8995a8)).child("ПРЕВЬЮ СБОРОК"))
            .child(self.build_button(BuildControl::Example, "Моя сборка", cx).w_full())
            .child(div().px(px(8.0)).text_size(px(11.0))
                .text_color(rgb(0x8995a8)).child("NeoForge · 1.21.1 · пример"))
            .child(self.build_button(BuildControl::Empty, "Пустая библиотека", cx).w_full())
    }

    fn build_header(&self, cx: &mut Context<Self>) -> Div {
        div().p(px(20.0)).flex().flex_col().gap(px(16.0))
            .rounded(px(16.0)).border_1().border_color(rgb(0x2c3441))
            .bg(rgba(0x1b2029f5))
            .child(div().flex().items_center().flex_wrap().gap(px(16.0))
                .child(div().flex_1().min_w(px(185.0)).flex().items_center().gap(px(12.0))
                    .child(build_cover())
                    .child(div().flex_1().min_w_0().flex().flex_col().gap(px(5.0))
                        .child(div().text_size(px(23.0)).child("Моя сборка"))
                        .child(div().text_size(px(12.0)).text_color(rgb(0xb5bfce))
                            .child("NeoForge · Minecraft 1.21.1"))))
                .child(div().flex().flex_wrap().gap(px(8.0))
                    .child(muted_action("Играть", true))
                    .child(self.build_button(BuildControl::Tab(BuildTab::Settings), "Параметры", cx))))
            .child(div().pt(px(12.0)).border_t_1().border_color(rgb(0x2c3441))
                .text_size(px(12.0)).text_color(rgb(0xb5bfce))
                .child("Пример интерфейса · не установлен. Запуск пока недоступен."))
    }

    fn build_empty_detail(&self) -> Div {
        let tab = self.builds.preview.tab;
        div().w_full().max_w(px(470.0)).p(px(24.0))
            .flex().flex_col().items_center().gap(px(14.0))
            .rounded(px(16.0)).border_1().border_color(rgb(0x2c3441))
            .bg(rgba(0x1b2029f5)).text_center()
            .child(div().w(px(58.0)).h(px(44.0)).mb(px(4.0))
                .rounded(px(10.0)).border_2().border_color(rgb(0x526984))
                .flex().items_center().justify_center()
                .child(div().w(px(22.0)).h(px(2.0)).bg(rgb(0x8995a8))))
            .child(div().text_size(px(21.0)).child(tab.empty_title()))
            .child(div().text_size(px(13.0)).text_color(rgb(0xb5bfce)).child(tab.explanation()))
            .when(tab == BuildTab::Content, |view| {
                view.child(div().flex().flex_wrap().justify_center().gap(px(8.0))
                    .child(muted_action("Добавить файлы", false))
                    .child(muted_action("Найти моды", true)))
                    .child(div().text_size(px(11.0)).text_color(rgb(0x8995a8))
                        .child("Добавление и каталог пока недоступны."))
            })
    }

    fn build_parameters(&self) -> Div {
        div().w_full().max_w(px(470.0)).p(px(24.0))
            .rounded(px(16.0)).border_1().border_color(rgb(0x2c3441))
            .bg(rgba(0x1b2029f5)).flex().flex_col().gap(px(16.0))
            .child(div().text_size(px(21.0)).child(BuildTab::Settings.empty_title()))
            .children([
                ("Название", "Моя сборка"),
                ("Minecraft", "1.21.1"),
                ("Загрузчик", "NeoForge"),
                ("Игровая папка", "Не создана"),
            ].into_iter().map(|(name, value)| {
                div().flex().flex_wrap().justify_between().gap(px(8.0))
                    .child(div().text_color(rgb(0xb5bfce)).child(name))
                    .child(div().child(value))
            }))
            .child(div().pt(px(12.0)).border_t_1().border_color(rgb(0x2c3441))
                .text_size(px(12.0)).text_color(rgb(0x8995a8))
                .child(BuildTab::Settings.explanation()))
    }

    pub(crate) fn builds_page(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        // The outer scroller also keeps header/actions reachable at 720x440 / high DPI.
        // No large fixed-height main card: wallpaper remains visible around local surfaces.
        let preview = self.builds.preview;
        div().id("builds-page").flex_1().min_w_0().h_full().overflow_y_scroll()
            .pr(px(16.0)).py(px(16.0))
            .child(div().w_full().min_h_full().flex().flex_col().gap(px(16.0))
                .when(preview.example_visible, |view| {
                    view.child(self.build_header(cx))
                        .child(div().flex().flex_wrap().gap(px(6.0)).children(
                            BuildTab::TABS.into_iter().map(|tab| {
                                self.build_button(BuildControl::Tab(tab), tab.label(), cx)
                            })))
                        .child(div().flex_1().min_h(px(270.0)).py(px(12.0))
                            .flex().items_center().justify_center()
                            .child(if preview.tab == BuildTab::Settings {
                                self.build_parameters()
                            } else {
                                self.build_empty_detail()
                            }))
                })
                .when(!preview.example_visible, |view| {
                    view.child(div().p(px(20.0)).rounded(px(16.0)).border_1()
                        .border_color(rgb(0x2c3441)).bg(rgba(0x1b2029f5))
                        .child(div().text_size(px(23.0)).child("Сборки")))
                        .child(div().flex_1().min_h(px(300.0)).flex().items_center().justify_center()
                            .child(div().w_full().max_w(px(430.0)).p(px(24.0))
                                .rounded(px(16.0)).border_1().border_color(rgb(0x2c3441))
                                .bg(rgba(0x1b2029f5)).flex().flex_col().items_center()
                                .gap(px(16.0)).text_center()
                                .child(div().text_size(px(21.0)).child("Библиотека пока пуста"))
                                .child(div().text_color(rgb(0xb5bfce))
                                    .child("Создание и импорт сборок ещё не подключены. Можно посмотреть пример нового экрана."))
                                .child(self.build_button(BuildControl::TryExample, "Посмотреть пример", cx))))
                }))
    }
}