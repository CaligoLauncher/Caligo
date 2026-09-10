//! Caligo's small native visual vocabulary. No animation loop, I/O or renderer patches.
use gpui::{
    Div, FocusHandle, IntoElement, PathBuilder, Stateful, canvas, div,
    linear_color_stop, linear_gradient, point, prelude::*, px, rgb, rgba,
};

pub(crate) const BACKGROUND: u32 = 0x11151c;
pub(crate) const SURFACE: u32 = 0x191f28;
pub(crate) const RAISED: u32 = 0x232c38;
pub(crate) const EDGE: u32 = 0x151b24;
pub(crate) const CONTROL_EDGE: u32 = 0x202936;
pub(crate) const SELECTED: u32 = 0x29394d;
pub(crate) const HOVER: u32 = 0x2d3949;
pub(crate) const PRESSED: u32 = 0x35465d;
pub(crate) const TEXT: u32 = 0xeaf0f8;
pub(crate) const SECONDARY: u32 = 0xb1bece;
pub(crate) const MUTED: u32 = 0x91a1b6;
pub(crate) const ACCENT: u32 = 0xa7c8f5;
pub(crate) const PRIMARY: u32 = 0xaacbf7;
pub(crate) const PRIMARY_TEXT: u32 = 0x142337;
pub(crate) const FOCUS: u32 = 0xd6e7ff;
pub(crate) const ERROR: u32 = 0xffb3ac;
pub(crate) const SUCCESS: u32 = 0xb0d7c2;
pub(crate) const RADIUS: f32 = 14.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum ButtonKind {
    Quiet,
    Selected,
    Primary,
}

fn fill(kind: ButtonKind) -> u32 {
    match kind {
        ButtonKind::Quiet => RAISED,
        ButtonKind::Selected => SELECTED,
        ButtonKind::Primary => PRIMARY,
    }
}

pub(crate) fn control(
    id: &'static str, focus: &FocusHandle, kind: ButtonKind, enabled: bool,
) -> Stateful<Div> {
    div().id(id).track_focus(focus)
        .h(px(40.0)).px(px(13.0)).flex_shrink_0()
        .flex().items_center().justify_center().gap(px(8.0))
        .rounded(px(9.0)).border_1()
        .border_color(rgb(CONTROL_EDGE))
        .bg(rgb(if enabled { fill(kind) } else { SURFACE }))
        .text_size(px(13.0))
        .text_color(rgb(if !enabled { MUTED } else if kind == ButtonKind::Primary {
            PRIMARY_TEXT
        } else if kind == ButtonKind::Selected { TEXT } else { SECONDARY }))
        .when(enabled, |view| view.cursor_pointer()
            .hover(move |style| style.bg(rgb(if kind == ButtonKind::Primary {
                0xc5ddff
            } else { HOVER })))
            .active(move |style| style.bg(rgb(if kind == ButtonKind::Primary {
                0x91b6e9
            } else { PRESSED }))))
        // Focus is deliberately brighter than the resting, darker outlines.
        .focus(move |style| style.border_color(rgb(if kind == ButtonKind::Primary && enabled {
            PRIMARY_TEXT
        } else { FOCUS })))
}

/// Local, almost opaque surfaces: bright wallpaper cannot wash out the labels.
pub(crate) fn panel() -> Div {
    div().rounded(px(RADIUS)).border_1().border_color(rgb(EDGE))
        .bg(rgba(0x191f28fa))
}

pub(crate) fn heading(text: impl IntoElement) -> Div {
    div().text_size(px(25.0)).line_height(px(33.0))
        .text_color(rgb(TEXT)).child(text)
}

pub(crate) fn section_title(text: &'static str) -> Div {
    div().text_size(px(15.0)).text_color(rgb(TEXT)).child(text)
}

pub(crate) fn hint(text: impl IntoElement) -> Div {
    div().text_size(px(12.0)).line_height(px(18.0))
        .text_color(rgb(MUTED)).child(text)
}

pub(crate) fn badge(text: &'static str) -> Div {
    div().px(px(9.0)).py(px(4.0)).rounded(px(6.0))
        .bg(rgb(RAISED)).text_size(px(11.0)).text_color(rgb(SECONDARY)).child(text)
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
                &[(4.,6.),(7.,6.)], &[(11.,6.),(20.,6.)],
                &[(9.,3.),(9.,9.)],
                &[(4.,12.),(13.,12.)], &[(17.,12.),(20.,12.)],
                &[(15.,9.),(15.,15.)],
                &[(4.,18.),(7.,18.)], &[(11.,18.),(20.,18.)],
                &[(9.,15.),(9.,21.)],
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
                &[(14.,3.),(14.,8.),(19.,8.)],
                &[(9.,13.),(15.,13.)], &[(9.,17.),(15.,17.)],
            ],
            Self::Logs => &[
                &[(4.,6.),(9.,11.),(4.,16.)], &[(12.,17.),(20.,17.)],
            ],
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
                if let Ok(path) = path.build() {
                    window.paint_path(path, rgb(color));
                }
            }
        },
    ).size(px(side)).flex_shrink_0()
}

pub(crate) fn emblem(symbol: Icon, side: f32) -> Div {
    div().size(px(side)).flex_shrink_0().rounded(px(12.0))
        .border_1().border_color(rgb(CONTROL_EDGE))
        .bg(linear_gradient(135.0,
            linear_color_stop(rgb(0x30445f), 0.0),
            linear_color_stop(rgb(0x202a39), 1.0)))
        .flex().items_center().justify_center()
        .child(icon(symbol, side * 0.48, ACCENT))
}

pub(crate) fn separator() -> Div {
    div().w_full().h(px(1.0)).flex_shrink_0().bg(rgb(EDGE))
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

    #[test]
    fn resting_borders_are_darker_than_the_previous_palette() {
        assert!(luminance(EDGE) < luminance(0x2c3441));
        assert!(luminance(CONTROL_EDGE) < luminance(0x344154));
    }

    #[test]
    fn body_and_secondary_text_keep_contrast() {
        // Worst-case white wallpaper through the panel's 250/255 alpha.
        let white_backed_panel = 0x1e232c;
        for color in [TEXT, SECONDARY, MUTED] {
            assert!(contrast(color, white_backed_panel) >= 4.5);
        }
        assert!(contrast(SECONDARY, HOVER) >= 4.5);
        assert!(contrast(TEXT, SELECTED) >= 4.5);
    }

    #[test]
    fn primary_button_has_readable_dark_text() {
        for background in [PRIMARY, 0xc5ddff, 0x91b6e9] {
            assert!(contrast(PRIMARY_TEXT, background) >= 4.5);
        }
    }

    #[test]
    fn keyboard_focus_stays_visible_on_dark_controls() {
        for background in [SURFACE, RAISED, SELECTED, HOVER, PRESSED] {
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