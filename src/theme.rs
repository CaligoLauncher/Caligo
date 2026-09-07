use eframe::egui;
use serde::{Deserialize, Serialize};

/// A serializable UI theme preset.
///
/// Presets are plain JSON, so they can be shared as files and applied at
/// runtime without recompiling. Any missing field falls back to the default,
/// so a preset only needs to specify what it changes.
///
/// Помимо базовых полей пресет порождает целую систему токенов
/// (по образцу дизайн-системы Modrinth/Omorphia):
/// - «лестница поверхностей» `surface(1..=5)` — глубина строится ступенями
///   всё более светлых поверхностей, а не только прозрачностью;
/// - трёхуровневая иерархия текста (`text_primary/body/tertiary`);
/// - акцентные подсветки (`accent_highlight`, `accent_bg`);
/// - рецепт карточек (`card_fill` + `card_stroke`).
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
    /// Panel opacity over the background image, 0.0 to 1.0.
    pub opacity: f32,
}

impl Default for ThemePreset {
    fn default() -> Self {
        Self {
            name: "Caligo Dark".to_string(),
            dark: true,
            rounding: 10.0,
            background: [15, 17, 21, 255],
            // Светлый голубой для тёмной темы: насыщенный «средний» синий
            // на тёмном фоне тускнеет — тёмные темы (Modrinth) используют
            // осветлённый тон акцента.
            accent: [79, 156, 255, 255],
            opacity: 0.92,
        }
    }
}

impl ThemePreset {
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }

    pub fn background_color(&self) -> egui::Color32 {
        egui::Color32::from_rgb(self.background[0], self.background[1], self.background[2])
    }

    pub fn accent_color(&self) -> egui::Color32 {
        egui::Color32::from_rgba_unmultiplied(
            self.accent[0],
            self.accent[1],
            self.accent[2],
            self.accent[3],
        )
    }

    /// «Лестница поверхностей»: уровень 1 — фон окна, 3 — карточки,
    /// 4 — кнопки и обводки карточек, 5 — разделители/ховеры.
    /// Каждая ступень чуть светлее предыдущей (в тёмной теме — в сторону
    /// холодного белого), что даёт глубину без резких рамок.
    pub fn surface(&self, level: u8) -> egui::Color32 {
        let t = match level {
            0 | 1 => 0.0,
            2 => 0.035,
            3 => 0.08,
            4 => 0.135,
            _ => 0.19,
        };
        let toward: [f32; 3] = if self.dark {
            [205.0, 214.0, 228.0]
        } else {
            [10.0, 12.0, 16.0]
        };
        let l = |i: usize| {
            (self.background[i] as f32 + (toward[i] - self.background[i] as f32) * t) as u8
        };
        egui::Color32::from_rgb(l(0), l(1), l(2))
    }

    /// Заголовки и самое важное — максимально контрастный текст.
    pub fn text_primary(&self) -> egui::Color32 {
        if self.dark {
            egui::Color32::from_rgb(255, 255, 255)
        } else {
            egui::Color32::from_rgb(26, 32, 44)
        }
    }

    /// Основной текст — приглушённый сине-серый, НЕ чисто белый:
    /// так заголовки выигрывают контрастом и интерфейс не «звенит».
    pub fn text_body(&self) -> egui::Color32 {
        if self.dark {
            egui::Color32::from_rgb(176, 186, 197)
        } else {
            egui::Color32::from_rgb(44, 46, 49)
        }
    }

    /// Вторичный текст: подписи, подсказки, метаданные.
    pub fn text_tertiary(&self) -> egui::Color32 {
        if self.dark {
            egui::Color32::from_rgb(150, 162, 176)
        } else {
            egui::Color32::from_rgb(72, 77, 84)
        }
    }

    /// Акцентная подсветка (~25% альфы) — фон выбранных элементов.
    pub fn accent_highlight(&self) -> egui::Color32 {
        egui::Color32::from_rgba_unmultiplied(self.accent[0], self.accent[1], self.accent[2], 64)
    }

    /// Слабая акцентная заливка (~15% альфы) — тонированные области.
    pub fn accent_bg(&self) -> egui::Color32 {
        egui::Color32::from_rgba_unmultiplied(self.accent[0], self.accent[1], self.accent[2], 38)
    }

    /// Заливка карточек: surface-3 с учётом прозрачности темы.
    pub fn card_fill(&self) -> egui::Color32 {
        let s = self.surface(3);
        let a = (self.opacity.clamp(0.0, 1.0) * 235.0) as u8;
        egui::Color32::from_rgba_unmultiplied(s.r(), s.g(), s.b(), a)
    }

    /// Обводка карточек 1px цветом surface-4: карточка читается границей
    /// чуть светлее собственной заливки, а не тенью или жирной рамкой.
    pub fn card_stroke(&self) -> egui::Stroke {
        egui::Stroke::new(1.0_f32, self.surface(4))
    }

    /// --- Материал Liquid Glass (macOS Tahoe) ---
    ///
    /// «Обычное» стекло (regular): для крупных элементов — панелей,
    /// сайдбаров. По HIG крупные элементы делаются заметно более
    /// непрозрачными, чтобы текст оставался читаемым на сложном фоне.
    pub fn glass_regular(&self) -> egui::Color32 {
        let a = (self.opacity.clamp(0.0, 1.0) * 228.0) as u8;
        egui::Color32::from_rgba_unmultiplied(
            self.background[0],
            self.background[1],
            self.background[2],
            a,
        )
    }

    /// «Прозрачное» стекло (clear): почти прозрачный материал для мелких
    /// контролов над насыщенным фоном; читаемость обеспечивает
    /// затемняющий слой `glass_dim`.
    pub fn glass_clear(&self) -> egui::Color32 {
        egui::Color32::from_rgba_unmultiplied(
            self.background[0],
            self.background[1],
            self.background[2],
            96,
        )
    }

    /// Затемняющий слой под clear-стеклом — ровно 35% непрозрачности,
    /// как предписывает HIG («dark dimming layer at 35% opacity»).
    pub fn glass_dim(&self) -> egui::Color32 {
        egui::Color32::from_black_alpha(89)
    }

    /// Концентрические скругления (Tahoe): радиус вложенного элемента =
    /// радиус контейнера минус отступ — углы «дышат» согласованно.
    pub fn concentric(outer: f32, inset: f32) -> f32 {
        (outer - inset).max(4.0)
    }

    /// Заливка «стеклянных» панелей поверх размытого фона.
    pub fn glass_fill(&self) -> egui::Color32 {
        let a = (self.opacity.clamp(0.0, 1.0) * 210.0) as u8;
        egui::Color32::from_rgba_unmultiplied(
            self.background[0],
            self.background[1],
            self.background[2],
            a,
        )
    }

    /// Лёгкое затемнение центральной области — текст читается на любом фоне.
    pub fn content_tint(&self) -> egui::Color32 {
        let a = (self.opacity.clamp(0.0, 1.0) * 140.0) as u8;
        egui::Color32::from_rgba_unmultiplied(
            self.background[0],
            self.background[1],
            self.background[2],
            a,
        )
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
        let accent = self.accent_color();

        v.panel_fill = bg;
        v.window_fill = bg;
        // Выделение — акцентная подсветка (25% альфы), как в дизайн-системах.
        v.selection.bg_fill = self.accent_highlight();
        v.hyperlink_color = accent;

        // Трёхуровневая иерархия текста: обычные виджеты — «телесным»
        // цветом, наведённые/активные — контрастным.
        v.override_text_color = None;
        v.widgets.noninteractive.fg_stroke.color = self.text_body();
        v.widgets.inactive.fg_stroke.color = self.text_body();
        v.widgets.hovered.fg_stroke.color = self.text_primary();
        v.widgets.active.fg_stroke.color = self.text_primary();
        v.widgets.open.fg_stroke.color = self.text_primary();

        // Кнопки и поля — ступень surface-4, при наведении — surface-5:
        // осветление вместо рамок.
        v.widgets.inactive.weak_bg_fill = self.surface(4).gamma_multiply(0.9);
        v.widgets.inactive.bg_fill = self.surface(4).gamma_multiply(0.9);
        v.widgets.hovered.weak_bg_fill = self.surface(5);
        v.widgets.hovered.bg_fill = self.surface(5);
        v.widgets.hovered.bg_stroke = egui::Stroke::new(1.0_f32, self.surface(5));
        v.widgets.active.weak_bg_fill = self.surface(5);

        let rounding = egui::Rounding::same(self.rounding);
        v.widgets.noninteractive.rounding = rounding;
        v.widgets.inactive.rounding = rounding;
        v.widgets.hovered.rounding = rounding;
        v.widgets.active.rounding = rounding;
        v.widgets.open.rounding = rounding;
        v.window_rounding = egui::Rounding::same(self.rounding * 1.5);

        v
    }

    /// Применяет тему целиком: не только цвета, но и типографику с ритмом
    /// отступов. Шкала размеров — системная лестница macOS (Tahoe):
    /// Large Title 26, Body/Headline 13, Subheadline 11 (иерархия
    /// строится весом и цветом, а не крупными кеглями).
    pub fn apply(&self, ctx: &egui::Context) {
        let mut style = (*ctx.style()).clone();
        style.text_styles = [
            (
                egui::TextStyle::Heading,
                egui::FontId::new(26.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Body,
                egui::FontId::new(13.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Button,
                egui::FontId::new(13.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Small,
                egui::FontId::new(11.0, egui::FontFamily::Proportional),
            ),
            (
                egui::TextStyle::Monospace,
                egui::FontId::new(12.0, egui::FontFamily::Monospace),
            ),
        ]
        .into();
        // Ритм отступов по шкале 4/8/12/16/24.
        style.spacing.item_spacing = egui::vec2(10.0, 8.0);
        style.spacing.button_padding = egui::vec2(14.0, 7.0);
        style.visuals = self.to_visuals();
        ctx.set_style(style);
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

    #[test]
    fn concentric_radius_shrinks_with_inset() {
        assert_eq!(ThemePreset::concentric(20.0, 8.0), 12.0);
        // Никогда не схлопывается в прямые углы.
        assert_eq!(ThemePreset::concentric(10.0, 12.0), 4.0);
    }

    #[test]
    fn surfaces_get_lighter_in_dark_theme() {
        let theme = ThemePreset::default();
        let s1 = theme.surface(1);
        let s5 = theme.surface(5);
        assert!(s5.r() > s1.r() && s5.g() > s1.g() && s5.b() > s1.b());
    }
}