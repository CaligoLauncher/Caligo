use gpui::{
    Context, Div, FocusHandle, KeyDownEvent, Stateful, Window,
    div, prelude::*, px, rgb,
};

use crate::{
    builds::{BuildControl, BuildPreview, BuildTab},
    shell::Shell,
    ui::{self, ButtonKind, Icon},
};

pub(crate) struct BuildsUi {
    pub(crate) preview: BuildPreview,
    focus: [FocusHandle; 8],
}

impl BuildsUi {
    pub(crate) fn new(cx: &mut Context<Shell>) -> Self {
        Self {
            preview: BuildPreview::default(),
            // 0..2 navigation, 3..11 appearance. Unmounted controls are not tab stops.
            focus: std::array::from_fn(|i| cx.focus_handle().tab_index((12 + i) as isize).tab_stop(true)),
        }
    }
}

fn muted_action(label: &'static str) -> Div {
    // A visual placeholder, never an actionable or focusable button.
    div().h(px(38.0)).px(px(16.0)).flex_shrink_0()
        .flex().items_center().justify_center()
        .rounded(px(7.0)).border_1().border_color(rgb(ui::CONTROL_EDGE))
        .text_size(px(13.0)).text_color(rgb(ui::MUTED)).child(label)
}

fn tab_icon(tab: BuildTab) -> Icon {
    match tab {
        BuildTab::Content => Icon::Folder,
        BuildTab::Files => Icon::File,
        BuildTab::Logs => Icon::Logs,
        BuildTab::Share => Icon::Share,
        BuildTab::Settings => Icon::Settings,
    }
}

impl Shell {
    fn build_action(&mut self, control: BuildControl, window: &mut Window, cx: &mut Context<Self>) {
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
        let kind = if control == BuildControl::TryExample { ButtonKind::Primary }
            else if selected { ButtonKind::Selected } else { ButtonKind::Quiet };
        ui::control(control.id(), &self.builds.focus[control.index()], kind, true)
            .on_click(cx.listener(move |this, _, window, cx| {
                this.build_action(control, window, cx);
            }))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                if !event.is_held && matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    this.build_action(control, window, cx);
                    cx.stop_propagation();
                }
            }))
            .when(!label.is_empty(), |button| button.child(label))
    }

    pub(crate) fn builds_sidebar(&self, cx: &mut Context<Self>) -> Div {
        div().flex_shrink_0().mt(px(18.0))
            .flex().flex_col().gap(px(10.0))
            .child(ui::separator())
            .child(ui::hint("Библиотека · пример").px(px(8.0)).pt(px(8.0)))
            .child(self.build_button(BuildControl::Example, "", cx)
                .w_full().h(px(62.0)).justify_start().px(px(8.0))
                .child(ui::landscape_cover(30.0))
                .child(div().min_w_0().flex().flex_col().gap(px(3.0))
                    .child(div().text_color(rgb(ui::TEXT)).child("Моя сборка"))
                    .child(ui::hint("1.21.1"))))
            .child(self.build_button(BuildControl::Empty, "Пустая библиотека", cx)
                .w_full().px(px(6.0)))
    }

    fn build_header(&self, cx: &mut Context<Self>) -> Div {
        div().flex_shrink_0().flex().flex_col()
            .child(div().px(px(24.0)).pt(px(24.0)).pb(px(16.0))
                .flex().flex_col().gap(px(14.0))
                .child(div().flex().items_center().flex_wrap().gap(px(16.0))
                    .child(div().flex_1().min_w(px(240.0)).flex().items_center().gap(px(16.0))
                        .child(ui::landscape_cover(52.0))
                        .child(div().min_w_0().flex().flex_col().gap(px(4.0))
                            .child(ui::heading("Моя сборка"))
                            .child(div().text_size(px(13.0)).text_color(rgb(ui::SECONDARY))
                                .child("Minecraft 1.21.1 / NeoForge"))))
                    .child(div().flex().flex_wrap().gap(px(8.0))
                        .child(self.build_button(BuildControl::Tab(BuildTab::Settings), "", cx)
                            .child(ui::icon(Icon::Settings, 16.0, ui::SECONDARY))
                            .child("Параметры"))
                        .child(muted_action("Играть"))))
                .child(ui::hint("Пример интерфейса · сборка не установлена")))
            .child(ui::separator().mx(px(24.0)).w_auto())
            .child(div().px(px(24.0)).flex().flex_wrap().gap(px(8.0)).children(
                BuildTab::TABS.into_iter().map(|tab| {
                    let selected = self.builds.preview.tab == tab;
                    self.build_button(BuildControl::Tab(tab), tab.label(), cx)
                        .relative().h(px(52.0)).px(px(8.0)).rounded(px(4.0))
                        .bg(rgb(ui::SURFACE))
                        .border_color(rgb(ui::SURFACE))
                        .text_color(rgb(if selected { ui::TEXT } else { ui::MUTED }))
                        .when(selected, |button| button.child(
                            div().absolute().bottom(px(0.0)).left(px(8.0)).right(px(8.0))
                                .h(px(2.0)).rounded(px(1.0)).bg(rgb(ui::ACCENT))))
                })))
            .child(ui::separator())
    }

    fn build_empty_detail(&self) -> Div {
        let tab = self.builds.preview.tab;
        div().w_full().max_w(px(440.0))
            .flex().flex_col().items_center().gap(px(16.0)).text_center()
            .child(ui::icon(tab_icon(tab), 30.0, ui::MUTED))
            .child(div().text_size(px(22.0)).line_height(px(30.0)).child(
                if tab == BuildTab::Content { "Пока без содержимого" } else { tab.empty_title() }))
            .child(div().text_size(px(13.0)).line_height(px(21.0))
                .text_color(rgb(ui::SECONDARY)).child(tab.explanation()))
            .when(tab == BuildTab::Content, |view| {
                view.child(div().pt(px(10.0)).flex().flex_wrap().justify_center().gap(px(12.0))
                    .child(muted_action("Добавить файлы"))
                    .child(muted_action("Найти моды")))
                    .child(ui::hint("Добавление и каталог пока недоступны"))
            })
    }

    fn build_parameters(&self) -> Div {
        div().w_full().max_w(px(460.0)).flex().flex_col().gap(px(18.0))
            .child(div().text_size(px(20.0)).child(BuildTab::Settings.empty_title()))
            .children([
                ("Название", "Моя сборка"),
                ("Minecraft", "1.21.1"),
                ("Загрузчик", "NeoForge"),
                ("Игровая папка", "Не создана"),
            ].into_iter().map(|(name, value)| {
                div().pb(px(12.0)).border_b_1().border_color(rgb(ui::EDGE))
                    .flex().flex_wrap().justify_between().gap(px(8.0))
                    .child(div().text_color(rgb(ui::SECONDARY)).child(name))
                    .child(div().child(value))
            }))
            .child(ui::hint(BuildTab::Settings.explanation()))
    }

    pub(crate) fn builds_page(&self, cx: &mut Context<Self>) -> Stateful<Div> {
        let preview = self.builds.preview;
        // One surface; the entire page remains scrollable in short/narrow windows.
        div().id("builds-page").flex_1().min_w_0().h_full().overflow_y_scroll()
            .pr(px(16.0)).py(px(16.0))
            .child(ui::panel().w_full().min_h_full().flex().flex_col()
                .when(preview.example_visible, |view| {
                    view.child(self.build_header(cx))
                        .child(div().flex_1().min_h(px(300.0)).p(px(24.0))
                            .flex().items_center().justify_center()
                            .child(if preview.tab == BuildTab::Settings {
                                self.build_parameters()
                            } else {
                                self.build_empty_detail()
                            }))
                })
                .when(!preview.example_visible, |view| {
                    view.child(div().p(px(24.0)).flex_shrink_0().flex().flex_col().gap(px(4.0))
                        .child(ui::heading("Сборки"))
                        .child(ui::hint("Предпросмотр пустой библиотеки")))
                        .child(ui::separator())
                        .child(div().flex_1().min_h(px(320.0)).p(px(24.0))
                            .flex().items_center().justify_center()
                            .child(div().w_full().max_w(px(440.0))
                                .flex().flex_col().items_center().gap(px(18.0)).text_center()
                                .child(ui::icon(Icon::Folder, 30.0, ui::MUTED))
                                .child(div().text_size(px(22.0)).child("Место для твоих сборок"))
                                .child(div().text_size(px(13.0)).line_height(px(21.0))
                                    .text_color(rgb(ui::SECONDARY))
                                    .child("Создание и импорт ещё не подключены. Пока можно посмотреть пример оформления."))
                                .child(self.build_button(BuildControl::TryExample, "Посмотреть пример", cx))))
                }))
    }
}