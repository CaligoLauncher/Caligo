//! Small keyboard-accessible controls for wallpaper presentation.
use gpui::{Context, Div, KeyDownEvent, ObjectFit, Stateful, Window, div, prelude::*, px, rgb};

use crate::{
    appearance_settings::{self, AppearanceSettings, WallpaperMode},
    shell::Shell,
    wallpaper,
    ui::{self, ButtonKind},
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum AppearanceAction {
    Cover, Contain, Dim0, Dim20, Dim40, Dim60, Reset,
}

impl AppearanceAction {
    fn index(self) -> usize {
        match self {
            Self::Cover => 0, Self::Contain => 1, Self::Dim0 => 2,
            Self::Dim20 => 3, Self::Dim40 => 4, Self::Dim60 => 5, Self::Reset => 6,
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Cover => "wallpaper-cover", Self::Contain => "wallpaper-contain",
            Self::Dim0 => "wallpaper-dim-0", Self::Dim20 => "wallpaper-dim-20",
            Self::Dim40 => "wallpaper-dim-40", Self::Dim60 => "wallpaper-dim-60",
            Self::Reset => "reset-appearance",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Cover => "Заполнить", Self::Contain => "Вписать",
            Self::Dim0 => "0%", Self::Dim20 => "20%", Self::Dim40 => "40%",
            Self::Dim60 => "60%", Self::Reset => "Сбросить оформление",
        }
    }

    fn apply(self, mut value: AppearanceSettings) -> AppearanceSettings {
        match self {
            Self::Cover => value.mode = WallpaperMode::Cover,
            Self::Contain => value.mode = WallpaperMode::Contain,
            Self::Dim0 => value.dim_percent = 0,
            Self::Dim20 => value.dim_percent = 20,
            Self::Dim40 => value.dim_percent = 40,
            Self::Dim60 => value.dim_percent = 60,
            Self::Reset => value = AppearanceSettings::default(),
        }
        value
    }

    fn selected(self, value: AppearanceSettings) -> bool {
        self != Self::Reset && self.apply(value) == value
    }
}

impl Shell {
    pub(crate) fn wallpaper_fit(&self) -> ObjectFit {
        match self.appearance.preferences.mode {
            WallpaperMode::Cover => ObjectFit::Cover,
            WallpaperMode::Contain => ObjectFit::Contain,
        }
    }

    fn appearance_action(&mut self, action: AppearanceAction, window: &Window, cx: &mut Context<Self>) {
        if self.appearance.busy
            || (self.appearance.preferences_error.is_some() && action != AppearanceAction::Reset)
        {
            return;
        }
        let next = action.apply(self.appearance.preferences);
        if action != AppearanceAction::Reset && next == self.appearance.preferences {
            return;
        }
        self.appearance.busy = true;
        self.appearance.preferences_message = Some("Сохранение оформления…".into());
        cx.notify();
        let task = cx.background_executor().spawn(async move {
            let root = wallpaper::directory()?;
            if action == AppearanceAction::Reset {
                appearance_settings::reset(&root)?;
            } else {
                appearance_settings::save(&root, next)?;
            }
            Ok::<_, String>(next)
        });
        cx.spawn_in(window, async move |this, cx| {
            let result = task.await;
            let _ = this.update_in(cx, |this, _, cx| {
                this.appearance.busy = false;
                match result {
                    Ok(value) => {
                        // Commit the visible change only after the disk write succeeded.
                        this.appearance.preferences = value;
                        this.appearance.preferences_error = None;
                        this.appearance.preferences_message = Some(
                            if action == AppearanceAction::Reset {
                                "Оформление сброшено. Изображение сохранено."
                            } else {
                                "Оформление сохранено."
                            }.into()
                        );
                    }
                    Err(error) => {
                        this.appearance.preferences_error = Some(error);
                        this.appearance.preferences_message = None;
                    }
                }
                cx.notify();
            });
        }).detach();
    }

    fn appearance_button(&self, action: AppearanceAction, cx: &mut Context<Self>) -> Stateful<Div> {
        let disabled = self.appearance.busy
            || (self.appearance.preferences_error.is_some() && action != AppearanceAction::Reset);
        let selected = action.selected(self.appearance.preferences);
        ui::control(action.id(), &self.appearance.preferences_focus[action.index()],
            if selected { ButtonKind::Selected } else { ButtonKind::Quiet }, !disabled)
            .when(action != AppearanceAction::Reset, |button| button.h(px(36.0)))
            .on_click(cx.listener(move |this, _, window, cx| {
                window.focus(&this.appearance.preferences_focus[action.index()]);
                this.appearance_action(action, window, cx);
            }))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                if !event.is_held && matches!(event.keystroke.key.as_str(), "enter" | "space") {
                    this.appearance_action(action, window, cx);
                    cx.stop_propagation();
                }
            }))
            .child(action.label())
    }

pub(crate) fn appearance_controls(&self, cx: &mut Context<Self>) -> Div {
        div().flex_shrink_0().flex().flex_col().gap(px(16.0))
            .child(div().flex().flex_wrap().items_center().justify_between().gap(px(14.0))
                .child(div().flex_1().min_w(px(180.0)).flex().flex_col().gap(px(4.0))
                    .child(ui::section_title("Размещение"))
                    .child(ui::hint(match self.appearance.preferences.mode {
                        WallpaperMode::Cover => "Заполнение с обрезкой краёв.",
                        WallpaperMode::Contain => "Целиком, с полями по краям.",
                    })))
                .child(div().p(px(3.0)).rounded(px(8.0)).bg(rgb(ui::BACKGROUND))
                    .flex().gap(px(2.0))
                    .child(self.appearance_button(AppearanceAction::Cover, cx))
                    .child(self.appearance_button(AppearanceAction::Contain, cx))))
            .child(ui::separator())
            .child(div().flex().flex_wrap().items_center().justify_between().gap(px(14.0))
                .child(div().flex_1().min_w(px(180.0)).flex().flex_col().gap(px(4.0))
                    .child(ui::section_title("Затемнение"))
                    .child(ui::hint("Только фон, без затемнения текста.")))
                .child(div().p(px(3.0)).rounded(px(8.0)).bg(rgb(ui::BACKGROUND))
                    .flex().gap(px(2.0)).children(
                        [AppearanceAction::Dim0, AppearanceAction::Dim20, AppearanceAction::Dim40, AppearanceAction::Dim60]
                            .into_iter().map(|action| self.appearance_button(action, cx)))))
            .child(ui::separator())
            .when_some(self.appearance.preferences_message.clone(), |view, message| {
                view.child(div().text_size(px(13.0)).line_height(px(20.0))
                    .text_color(rgb(ui::SUCCESS)).child(message))
            })
            .when_some(self.appearance.preferences_error.clone(), |view, error| {
                view.child(div().text_size(px(13.0)).line_height(px(20.0)).text_color(rgb(ui::ERROR))
                    .child(format!("{error} Обычные изменения приостановлены. Исправьте файл и перезапустите приложение либо явно сбросьте оформление.")))
            })
            .child(div().flex().flex_wrap().items_center().justify_between().gap(px(12.0))
                .child(self.appearance_button(AppearanceAction::Reset, cx))
                .child(ui::hint("«Заполнить» и 0%. Обои останутся.")
                    .flex_1().min_w(px(190.0))))
    }

}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gpui_fit_keeps_portrait_image_inside_landscape_area() {
        let area = gpui::Bounds {
            origin: gpui::point(px(0.0), px(0.0)),
            size: gpui::size(px(1000.0), px(600.0)),
        };
        let result = ObjectFit::Contain.get_bounds(
            area, gpui::size(100_u32.into(), 200_u32.into()));
        assert_eq!(result.size, gpui::size(px(300.0), px(600.0)));
        assert_eq!(result.origin, gpui::point(px(350.0), px(0.0)));
    }

    #[test]
    fn gpui_cover_crops_without_stretching() {
        let area = gpui::Bounds {
            origin: gpui::point(px(0.0), px(0.0)),
            size: gpui::size(px(1000.0), px(600.0)),
        };
        let result = ObjectFit::Cover.get_bounds(
            area, gpui::size(100_u32.into(), 200_u32.into()));
        assert_eq!(result.size, gpui::size(px(1000.0), px(2000.0)));
        assert_eq!(result.origin, gpui::point(px(0.0), px(-700.0)));
    }

    #[test]
    fn mode_change_preserves_dimming_and_dim_change_preserves_mode() {
        let mut value = AppearanceAction::Dim40.apply(AppearanceSettings::default());
        value = AppearanceAction::Contain.apply(value);
        assert_eq!(value.mode, WallpaperMode::Contain);
        assert_eq!(value.dim_percent, 40);
        value = AppearanceAction::Dim60.apply(value);
        assert_eq!(value.mode, WallpaperMode::Contain);
        assert_eq!(value.dim_percent, 60);
    }

    #[test]
    fn selection_is_exclusive_within_each_group() {
        let value = AppearanceAction::Contain.apply(
            AppearanceAction::Dim20.apply(AppearanceSettings::default()));
        for (action, selected) in [
            (AppearanceAction::Cover, false), (AppearanceAction::Contain, true),
            (AppearanceAction::Dim0, false), (AppearanceAction::Dim20, true),
            (AppearanceAction::Dim40, false), (AppearanceAction::Dim60, false),
            (AppearanceAction::Reset, false),
        ] {
            assert_eq!(action.selected(value), selected);
        }
    }

    #[test]
    fn reset_and_reselect_are_predictable() {
        let defaults = AppearanceSettings::default();
        assert_eq!(AppearanceAction::Cover.apply(defaults), defaults);
        assert_eq!(AppearanceAction::Dim0.apply(defaults), defaults);
        assert_eq!(
            AppearanceAction::Reset.apply(AppearanceAction::Contain.apply(defaults)),
            defaults,
        );
    }

    #[test]
    fn focus_indices_and_element_ids_do_not_collide() {
        let actions = [
            AppearanceAction::Cover, AppearanceAction::Contain, AppearanceAction::Dim0,
            AppearanceAction::Dim20, AppearanceAction::Dim40, AppearanceAction::Dim60,
            AppearanceAction::Reset,
        ];
        for (i, action) in actions.iter().enumerate() {
            assert_eq!(action.index(), i);
            for other in &actions[i + 1..] {
                assert_ne!(action.id(), other.id());
            }
        }
    }
}