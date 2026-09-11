//! Small native visual vocabulary: static gradients, local surfaces, no renderer patches.
use gpui::{
    Div, FocusHandle, IntoElement, Stateful, PathBuilder, canvas, div,
    point, prelude::*, px, rgb, rgba, linear_gradient, linear_color_stop,
};

pub(crate) const BACKGROUND: u32 = 0x819da9;
pub(crate) const BACKGROUND_END: u32 = 0xedf2f3;
pub(crate) const SURFACE: u32 = 0xeef4f6;
pub(crate) const RAISED: u32 = 0xe2ecf0;
pub(crate) const EDGE: u32 = 0x526f7f;
pub(crate) const CONTROL_EDGE: u32 = 0x6d8794;
pub(crate) const SELECTED: u32 = 0xd4e3ea;
pub(crate) const HOVER: u32 = 0xe0ebef;
pub(crate) const PRESSED: u32 = 0xc6d9e2;
pub(crate) const TEXT: u32 = 0x183440;
pub(crate) const SECONDARY: u32 = 0x35515f;
pub(crate) const MUTED: u32 = 0x45616e;
pub(crate) const ACCENT: u32 = 0x2f5263;
pub(crate) const PRIMARY: u32 = 0x315566;
pub(crate) const PRIMARY_TEXT: u32 = 0xf5fafc;
pub(crate) const FOCUS: u32 = 0x143d55;
pub(crate) const ERROR: u32 = 0x9a302c;
pub(crate) const SUCCESS: u32 = 0x235f49;
pub(crate) const RADIUS: f32 = 12.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ButtonKind { Quiet, Selected, Primary }

pub(crate) fn control(
    id: &'static str, focus: &FocusHandle, kind: ButtonKind, enabled: bool,
) -> Stateful<Div> {
    let background = if !enabled { 0xe2ecf099 } else {
        match kind {
            ButtonKind::Quiet => 0xffffff60,
            ButtonKind::Selected => (SELECTED << 8) | 0xff,
            ButtonKind::Primary => (PRIMARY << 8) | 0xff,
        }
    };
    div().id(id).track_focus(focus)
        .h(px(40.0)).px(px(13.0)).flex_shrink_0()
        .flex().items_center().justify_center().gap(px(8.0))
        .rounded(px(8.0)).border_1()
        .border_color(rgba(if kind == ButtonKind::Selected { 0x2f5263ff } else { 0x526f7f36 }))
        .bg(rgba(background)).text_size(px(13.0))
        .text_color(rgb(if !enabled { MUTED } else if kind == ButtonKind::Primary {
            PRIMARY_TEXT
        } else { TEXT }))
        .when(enabled, |view| view.cursor_pointer()
            .hover(move |style| style.bg(rgb(if kind == ButtonKind::Primary {
                0x244757
            } else { HOVER })))
            .active(move |style| style.bg(rgb(if kind == ButtonKind::Primary {
                0x193b4b
            } else { PRESSED }))))
        // Reserved border space avoids layout shifts on keyboard focus.
        .focus(move |style| style.border_color(rgb(
            if kind == ButtonKind::Primary && enabled { PRIMARY_TEXT } else { FOCUS })))
}

/// Readable over arbitrary user wallpaper; alpha blending is not backdrop blur.
pub(crate) fn panel() -> Div {
    div().rounded(px(RADIUS)).border_1().border_color(rgba(0x526f7f40))
        .bg(rgba(0xeef4f6f0)).text_color(rgb(TEXT))
}

pub(crate) fn content_panel(has_wallpaper: bool) -> Div {
    panel().bg(rgba(if has_wallpaper { 0xeef4f6f0 } else { 0xffffffb8 }))
}

/// Built-in fallback only. User images are drawn above it; nothing is saved to disk.
pub(crate) fn background() -> gpui::Background {
    linear_gradient(180.,
        linear_color_stop(rgb(BACKGROUND), 0.),
        linear_color_stop(rgb(BACKGROUND_END), 1.))
}

pub(crate) fn heading(text: impl IntoElement) -> Div {
    div().text_size(px(25.0)).line_height(px(33.0)).text_color(rgb(TEXT)).child(text)
}

pub(crate) fn section_title(text: &'static str) -> Div {
    div().text_size(px(15.0)).text_color(rgb(TEXT)).child(text)
}

pub(crate) fn hint(text: impl IntoElement) -> Div {
    div().text_size(px(12.0)).line_height(px(18.0)).text_color(rgb(MUTED)).child(text)
}

#[derive(Clone, Copy, Debug)]
pub(crate) enum Icon {
    Home, Layers, Settings, Picture, Folder, Play, File, Logs, Share, Arrow, Remove,
}

type Polyline = &'static [(f32, f32)];

impl Icon {
    // Original simple line symbols in a 24-unit view box; not branding or emoji.
    fn paths(self) -> &'static [Polyline] {
        match self {
            Self::Home => &[
                &[(3.,10.),(12.,3.),(21.,10.)],
                &[(5.,9.),(5.,21.),(10.,21.),(10.,14.),(14.,14.),(14.,21.),(19.,21.),(19.,9.)],
            ],
            Self::Layers => &[
                &[(3.,7.),(12.,3.),(21.,7.),(12.,11.),(3.,7.)],
                &[(3.,12.),(12.,16.),(21.,12.)],
                &[(3.,17.),(12.,21.),(21.,17.)],
            ],
            Self::Settings => &[
                &[(4.,6.),(7.,6.)], &[(11.,6.),(20.,6.)], &[(9.,3.),(9.,9.)],
                &[(4.,12.),(13.,12.)], &[(17.,12.),(20.,12.)], &[(15.,9.),(15.,15.)],
                &[(4.,18.),(7.,18.)], &[(11.,18.),(20.,18.)], &[(9.,15.),(9.,21.)],
            ],
            Self::Picture => &[
                &[(4.,3.),(20.,3.),(20.,21.),(4.,21.),(4.,3.)],
                &[(4.,17.),(10.,11.),(14.,15.),(17.,12.),(20.,15.)],
                &[(15.,7.),(17.,7.)],
            ],
            Self::Folder => &[
                &[(3.,7.),(3.,5.),(10.,5.),(12.,8.),(21.,8.),(21.,20.),(3.,20.),(3.,7.)],
            ],
            Self::Play => &[&[(7.,4.),(20.,12.),(7.,20.),(7.,4.)]],
            Self::File => &[
                &[(5.,3.),(14.,3.),(19.,8.),(19.,21.),(5.,21.),(5.,3.)],
                &[(14.,3.),(14.,8.),(19.,8.)], &[(9.,13.),(15.,13.)], &[(9.,17.),(15.,17.)],
            ],
            Self::Logs => &[&[(4.,6.),(9.,11.),(4.,16.)], &[(12.,17.),(20.,17.)]],
            Self::Share => &[
                &[(12.,15.),(12.,3.)], &[(7.,8.),(12.,3.),(17.,8.)],
                &[(4.,13.),(4.,21.),(20.,21.),(20.,13.)],
            ],
            Self::Arrow => &[&[(4.,12.),(20.,12.)], &[(14.,6.),(20.,12.),(14.,18.)]],
            Self::Remove => &[&[(6.,6.),(18.,18.)], &[(18.,6.),(6.,18.)]],
        }
    }
}

pub(crate) fn icon(symbol: Icon, side: f32, color: u32) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let scale = side / 24.0;
            for points in symbol.paths() {
                let mut path = PathBuilder::stroke(px(1.6));
                for (i, (x, y)) in points.iter().enumerate() {
                    let position = bounds.origin + point(px(x * scale), px(y * scale));
                    if i == 0 { path.move_to(position); } else { path.line_to(position); }
                }
                if let Ok(path) = path.build() { window.paint_path(path, rgb(color)); }
            }
        },
    ).size(px(side)).flex_shrink_0()
}

pub(crate) fn separator() -> Div {
    div().w_full().h(px(1.0)).flex_shrink_0().bg(rgba(0x526f7f30))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn luminance(color: u32) -> f64 {
        let linear = |value: u32| {
            let c = value as f64 / 255.0;
            if c <= 0.04045 { c / 12.92 } else { ((c + 0.055) / 1.055).powf(2.4) }
        };
        0.2126 * linear((color >> 16) & 255)
            + 0.7152 * linear((color >> 8) & 255)
            + 0.0722 * linear(color & 255)
    }

    fn contrast(a: u32, b: u32) -> f64 {
        let (a, b) = (luminance(a), luminance(b));
        (a.max(b) + 0.05) / (a.min(b) + 0.05)
    }

    fn over_black(color: u32, alpha: u32) -> u32 {
        [16, 8, 0].into_iter().fold(0, |value, shift| {
            value | ((((color >> shift) & 255) * alpha / 255) << shift)
        })
    }

    #[test]
    fn outlines_are_darker_than_the_light_surfaces() {
        assert!(luminance(EDGE) < luminance(SURFACE));
        assert!(luminance(CONTROL_EDGE) < luminance(RAISED));
    }

    #[test]
    fn local_surface_text_survives_black_wallpaper() {
        let darkest_panel = over_black(SURFACE, 240);
        for text in [TEXT, SECONDARY, MUTED, ERROR, SUCCESS] {
            assert!(contrast(text, darkest_panel) >= 4.5);
        }
        assert!(contrast(TEXT, SELECTED) >= 4.5);
    }

    #[test]
    fn open_navigation_and_hero_read_on_entire_default_gradient() {
        for background in [BACKGROUND, BACKGROUND_END] {
            assert!(contrast(TEXT, background) >= 4.5);
        }
    }

    #[test]
    fn primary_text_reads_in_all_interaction_states() {
        for background in [PRIMARY, 0x244757, 0x193b4b] {
            assert!(contrast(PRIMARY_TEXT, background) >= 4.5);
        }
    }

    #[test]
    fn keyboard_focus_is_visible_on_light_surfaces() {
        for background in [SURFACE, RAISED, SELECTED, HOVER, PRESSED, BACKGROUND] {
            assert!(contrast(FOCUS, background) >= 3.0);
        }
    }

    #[test]
    fn symbols_stay_within_their_viewbox() {
        for symbol in [Icon::Home, Icon::Layers, Icon::Settings, Icon::Picture,
            Icon::Folder, Icon::Play, Icon::File, Icon::Logs, Icon::Share,
            Icon::Arrow, Icon::Remove] {
            for line in symbol.paths() {
                assert!(line.len() >= 2);
                assert!(line.iter().all(|(x,y)| (0.0..=24.0).contains(x) && (0.0..=24.0).contains(y)));
            }
        }
    }
}