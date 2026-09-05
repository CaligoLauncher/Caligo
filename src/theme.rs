use eframe::egui;
use serde::{Deserialize, Serialize};

/// A serializable UI theme preset.
///
/// Presets are plain JSON, so they can be shared as files and applied at
/// runtime without recompiling. Any missing field falls back to the default,
/// so a preset only needs to specify what it changes.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ThemePreset {
    pub name: String,
    pub dark: bool,
    /// Corner rounding radius for widgets, in pixels.
    pub rounding: f32,
    /// Panel/window background color, RGBA 0-255.
    pub background: [u8; 4],
    /// Accent color for selection/highlights/links, RGBA 0-255.
    pub accent: [u8; 4],
    /// Background opacity, 0.0 (transparent) to 1.0 (opaque).
    pub opacity: f32,
}

impl Default for ThemePreset {
    fn default() -> Self {
        Self {
            name: "Terra Dark".to_string(),
            dark: true,
            rounding: 8.0,
            background: [24, 26, 32, 255],
            accent: [92, 156, 255, 255],
            opacity: 1.0,
        }
    }
}

impl ThemePreset {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn to_visuals(&self) -> egui::Visuals {
        let mut v = if self.dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };

        let alpha = (self.opacity.clamp(0.0, 1.0) * self.background[3] as f32) as u8;
        let bg = egui::Color32::from_rgba_unmultiplied(
            self.background[0],
            self.background[1],
            self.background[2],
            alpha,
        );
        let accent = egui::Color32::from_rgba_unmultiplied(
            self.accent[0],
            self.accent[1],
            self.accent[2],
            self.accent[3],
        );

        v.panel_fill = bg;
        v.window_fill = bg;
        v.selection.bg_fill = accent;
        v.hyperlink_color = accent;

        let rounding = egui::Rounding::same(self.rounding);
        v.widgets.noninteractive.rounding = rounding;
        v.widgets.inactive.rounding = rounding;
        v.widgets.hovered.rounding = rounding;
        v.widgets.active.rounding = rounding;
        v.widgets.open.rounding = rounding;
        v.window_rounding = egui::Rounding::same(self.rounding * 1.5);

        v
    }

    pub fn apply(&self, ctx: &egui::Context) {
        ctx.set_visuals(self.to_visuals());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_theme_json_roundtrip() {
        let theme = ThemePreset::default();
        let json = serde_json::to_string(&theme).unwrap();
        let parsed = ThemePreset::from_json(&json).unwrap();
        assert_eq!(parsed.name, theme.name);
        assert_eq!(parsed.rounding, theme.rounding);
        assert_eq!(parsed.background, theme.background);
    }

    #[test]
    fn partial_json_falls_back_to_defaults() {
        let parsed = ThemePreset::from_json(r#"{"name": "Custom"}"#).unwrap();
        assert_eq!(parsed.name, "Custom");
        assert!(parsed.dark);
        assert_eq!(parsed.rounding, ThemePreset::default().rounding);
    }

    #[test]
    fn invalid_json_is_an_error() {
        assert!(ThemePreset::from_json("not json").is_err());
    }
}
