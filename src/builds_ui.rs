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

fn muted_action(label: &'static str, symbol: Icon) -> Div {
    // Not a button: no input handlers, pointer cursor, or keyboard focus.
    div().h(px(40.0)).px(px(13.0)).flex_shrink_0()
        .flex().items_center().justify_center().gap(px(8.0))
        .rounded(px(9.0)).border_1().border_color(rgb(ui::CONTROL_EDGE))
        .bg(rgb(ui::SURFACE)).text_size(px(13.0)).text_color(rgb(ui::MUTED))
        .child(ui::icon(symbol, 16.0, ui::MUTED)).child(label)
}

fn tab_icon(tab: BuildTab) -> Icon {
    match tab {
        BuildTab::Content => Icon::Layers,
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
        div().flex_shrink_0().mt(px(14.0))
            .flex().flex_col().gap(px(10.0))
            .child(ui::separator())
            .child(ui::hint("ПРЕВЬЮ БИБЛИОТЕКИ").px(px(9.0)).pt(px(8.0)))
            .child(self.build_button(BuildControl::Example, "", cx)
                .w_full().h(px(68.0)).justify_start().px(px(9.0))
                .child(ui::emblem(Icon::Layers, 32.0))
                .child(div().min_w_0().flex().flex_col().gap(px(3.0))
                    .child(div().text_color(rgb(ui::TEXT)).child("Моя сборка"))
                    .child(ui::hint("1.21.1 · пример"))))
            .child(self.build_button(BuildControl::Empty, "Пустая библиотека", cx).w_full())
    }

    fn build_header(&self, cx: &mut Context<Self>) -> Div {
        ui::panel().flex().flex_col().overflow_hidden()
            .child(div().p(px(22.0)).flex().flex_col().gap(px(18.0))
                .child(div().flex().items_center().flex_wrap().gap(px(16.0))
                    .child(ui::emblem(Icon::Layers, 64.0))
                    .child(div().flex_1().min_w(px(180.0)).flex().flex_col().gap(px(8.0))
                        .child(ui::heading("Моя сборка"))
                        .child(div().flex().flex_wrap().gap(px(6.0))
                            .child(ui::badge("Minecraft 1.21.1"))
                            .child(ui::badge("NeoForge")))))
                .child(div().flex().flex_wrap().items_center().justify_between().gap(px(12.0))
                    .child(ui::hint("Пример интерфейса · не установлен"))
                    .child(div().flex().flex_wrap().gap(px(8.0))
                        .child(muted_action("Играть", Icon::Play))
                        .child(self.build_button(BuildControl::Tab(BuildTab::Settings), "", cx)
                            .child(ui::icon(Icon::Settings, 16.0, ui::SECONDARY))
                            .child("Параметры")))))
            .child(ui::separator())
            .child(div().p(px(8.0)).flex().flex_wrap().gap(px(4.0)).children(
                BuildTab::TABS.into_iter().map(|tab| {
                    let selected = self.builds.preview.tab == tab;
                    self.build_button(BuildControl::Tab(tab), tab.label(), cx)
                        .relative()
                        .bg(rgb(if selected { ui::SELECTED } else { ui::SURFACE }))
                        .border_color(rgb(ui::SURFACE))
                        .when(selected, |button| button.child(
                            div().absolute().bottom(px(2.0)).left(px(13.0)).right(px(13.0))
                                .h(px(2.0)).rounded(px(1.0)).bg(rgb(ui::ACCENT))))
                })))
    }

    fn build_empty_detail(&self) -> Div {
        let tab = self.builds.preview.tab;
        ui::panel().w_full().max_w(px(460.0)).p(px(26.0))
            .flex().flex_col().items_center().gap(px(14.0)).text_center()
            .child(ui::emblem(tab_icon(tab), 56.0))
            .child(div().text_size(px(20.0)).line_height(px(28.0)).child(tab.empty_title()))
            .child(div().text_size(px(13.0)).line_height(px(20.0))
                .text_color(rgb(ui::SECONDARY)).child(tab.explanation()))
            .when(tab == BuildTab::Content, |view| {
                view.child(div().pt(px(4.0)).flex().flex_wrap().justify_center().gap(px(8.0))
                    .child(muted_action("Добавить файлы", Icon::Folder))
                    .child(muted_action("Найти моды", Icon::Layers)))
                    .child(ui::hint("Добавление и каталог пока недоступны"))
            })
    }

    fn build_parameters(&self) -> Div {
        ui::panel().w_full().max_w(px(460.0)).p(px(24.0))
            .flex().flex_col().gap(px(18.0))
            .child(div().flex().items_center().gap(px(10.0))
                .child(ui::icon(Icon::Settings, 20.0, ui::ACCENT))
                .child(div().text_size(px(20.0)).child(BuildTab::Settings.empty_title())))
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
        // Scroll the entire page, including header/actions, on short or high-DPI windows.
        // The body uses local surfaces, not an opaque fullscreen sheet.
        div().id("builds-page").flex_1().min_w_0().h_full().overflow_y_scroll()
            .pr(px(16.0)).py(px(16.0))
            .child(div().w_full().min_h_full().flex().flex_col().gap(px(18.0))
                .when(preview.example_visible, |view| {
                    view.child(self.build_header(cx))
                        .child(div().flex_1().min_h(px(300.0)).py(px(12.0))
                            .flex().items_center().justify_center()
                            .child(if preview.tab == BuildTab::Settings {
                                self.build_parameters()
                            } else {
                                self.build_empty_detail()
                            }))
                })
                .when(!preview.example_visible, |view| {
                    view.child(ui::panel().p(px(22.0)).flex().flex_col().gap(px(6.0))
                        .child(ui::heading("Сборки"))
                        .child(ui::hint("Предпросмотр пустой библиотеки")))
                        .child(div().flex_1().min_h(px(320.0)).py(px(12.0))
                            .flex().items_center().justify_center()
                            .child(ui::panel().w_full().max_w(px(460.0)).p(px(26.0))
                                .flex().flex_col().items_center().gap(px(16.0)).text_center()
                                .child(ui::emblem(Icon::Folder, 56.0))
                                .child(div().text_size(px(22.0)).child("Место для твоих сборок"))
                                .child(div().text_size(px(13.0)).line_height(px(20.0))
                                    .text_color(rgb(ui::SECONDARY))
                                    .child("Библиотека пока пуста. Создание и импорт ещё не подключены — пока можно посмотреть пример оформления."))
                                .child(self.build_button(BuildControl::TryExample, "Посмотреть пример", cx)
                                    .child(ui::icon(Icon::Arrow, 16.0, ui::PRIMARY_TEXT)))))
                }))
    }
}