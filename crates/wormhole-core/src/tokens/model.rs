//! The resolved values a theme is made of: terminal palette, syntax styles
//! and shape metrics.

use serde::{Deserialize, Serialize};

use crate::color::Color;
use crate::theme;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Appearance {
    Light,
    Dark,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TerminalColors {
    pub foreground: Color,
    pub background: Color,
    pub cursor: Color,
    pub cursor_text: Option<Color>,
    pub selection_background: Color,
    pub selection_foreground: Option<Color>,
    pub ansi: [Color; 16],
}

impl TerminalColors {
    pub fn from_theme(t: &theme::Theme) -> Self {
        Self {
            foreground: Color::from_rgb_u32(t.foreground),
            background: Color::from_rgb_u32(t.background),
            cursor: Color::from_rgb_u32(t.cursor),
            cursor_text: t.cursor_text.map(Color::from_rgb_u32),
            selection_background: Color::from_rgb_u32(t.selection_background),
            selection_foreground: t.selection_foreground.map(Color::from_rgb_u32),
            ansi: t.palette.map(Color::from_rgb_u32),
        }
    }

    pub fn to_theme(&self, name: &str) -> theme::Theme {
        theme::Theme {
            name: name.to_string(),
            foreground: self.foreground.to_rgb_u32(),
            background: self.background.to_rgb_u32(),
            cursor: self.cursor.to_rgb_u32(),
            cursor_text: self.cursor_text.map(Color::to_rgb_u32),
            selection_background: self.selection_background.to_rgb_u32(),
            selection_foreground: self.selection_foreground.map(Color::to_rgb_u32),
            palette: self.ansi.map(Color::to_rgb_u32),
        }
    }

    /// `terminal.*` reference names and values. Unset optional colors resolve
    /// to what the terminal actually draws (background / foreground).
    pub(super) fn named(&self) -> Vec<(String, Color)> {
        let mut v = vec![
            ("terminal.foreground".to_string(), self.foreground),
            ("terminal.background".to_string(), self.background),
            ("terminal.cursor".to_string(), self.cursor),
            ("terminal.cursor_text".to_string(), self.cursor_text.unwrap_or(self.background)),
            ("terminal.selection_background".to_string(), self.selection_background),
            (
                "terminal.selection_foreground".to_string(),
                self.selection_foreground.unwrap_or(self.foreground),
            ),
        ];
        v.extend(self.ansi.iter().enumerate().map(|(i, c)| (format!("terminal.ansi.{i}"), *c)));
        v
    }

    /// A color by its `terminal.*` key, as [`Self::named`] lists it.
    pub(super) fn get(&self, key: &str) -> Option<Color> {
        Some(match key {
            "foreground" => self.foreground,
            "background" => self.background,
            "cursor" => self.cursor,
            "cursor_text" => self.cursor_text.unwrap_or(self.background),
            "selection_background" => self.selection_background,
            "selection_foreground" => self.selection_foreground.unwrap_or(self.foreground),
            _ => {
                let index = key.strip_prefix("ansi.")?;
                // Only the canonical spelling, as listed: `ansi.4`, not `ansi.04`.
                let i = index.parse::<usize>().ok().filter(|i| *i < 16 && i.to_string() == index)?;
                self.ansi[i]
            }
        })
    }

    pub(super) fn set(&mut self, key: &str, c: Color) -> bool {
        match key {
            "foreground" => self.foreground = c,
            "background" => self.background = c,
            "cursor" => self.cursor = c,
            "cursor_text" => self.cursor_text = Some(c),
            "selection_background" => self.selection_background = c,
            "selection_foreground" => self.selection_foreground = Some(c),
            _ => match key.strip_prefix("ansi.").and_then(|i| i.parse::<usize>().ok()) {
                Some(i) if i < 16 => self.ansi[i] = c,
                _ => return false,
            },
        }
        true
    }
}

/// The rule set a palette gets when no theme file says: light for a light
/// background.
pub(super) fn auto_appearance(t: &TerminalColors) -> Appearance {
    if theme::is_light_background(t.background.to_rgb_u32()) { Appearance::Light } else { Appearance::Dark }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SyntaxStyle {
    pub color: Color,
    pub bold: bool,
    pub italic: bool,
}

/// Shape and size. The three `radius*` values are the size scale; `radius_chip`
/// is the one semantic radius, shared by every chip-shaped element (tabs, choice
/// chips, badges, key caps) so they always match. It follows `radius_sm` unless a
/// theme sets it, which lets a theme round its chips into pills without also
/// rounding everything else that is small.
#[derive(Clone, Debug, PartialEq)]
pub struct Metrics {
    /// Tiny elements: swatches, the hover box of a small icon button.
    pub radius_sm: f32,
    /// Tabs, choice chips, badges and key caps. Derived: `radius_sm`.
    pub radius_chip: f32,
    /// Controls and rows: buttons, inputs, selects, list rows.
    pub radius: f32,
    /// Floating surfaces and cards: menus, popovers, cards.
    pub radius_lg: f32,
    pub spacing: f32,
    pub ui_font_family: Option<String>,
    pub ui_font_size: f32,
    pub border_width: f32,
    /// The keyboard focus ring's line.
    pub focus_ring_width: f32,
}

impl Default for Metrics {
    fn default() -> Self {
        // macOS-native proportions: 13pt system font, small radii.
        Self {
            radius_sm: 4.0,
            radius_chip: 4.0,
            radius: 6.0,
            radius_lg: 10.0,
            spacing: 8.0,
            ui_font_family: None,
            ui_font_size: 13.0,
            border_width: 1.0,
            focus_ring_width: 2.0,
        }
    }
}

impl Metrics {
    /// A numeric metric by its key (`radius_chip`) or node name (`metrics.radius_chip`).
    pub fn get(&self, name: &str) -> Option<f32> {
        Some(match name.strip_prefix("metrics.").unwrap_or(name) {
            "radius_sm" => self.radius_sm,
            "radius_chip" => self.radius_chip,
            "radius" => self.radius,
            "radius_lg" => self.radius_lg,
            "spacing" => self.spacing,
            "ui_font_size" => self.ui_font_size,
            "border_width" => self.border_width,
            "focus_ring_width" => self.focus_ring_width,
            _ => return None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::TerminalColors;
    use crate::color::Color;
    use crate::theme;

    #[test]
    fn get_finds_exactly_what_named_lists() {
        let mut colors = TerminalColors::from_theme(&theme::builtins()[0]);
        for set in [false, true] {
            if set {
                colors.cursor_text = Some(Color::WHITE);
                colors.selection_foreground = Some(Color::BLACK);
            }
            for (name, color) in colors.named() {
                assert_eq!(colors.get(&name["terminal.".len()..]), Some(color), "{name}");
            }
        }
        for missing in
            ["", "ansi", "ansi.", "ansi.16", "ansi.04", "ansi.+4", "ansi.x", "Background", "terminal.cursor"]
        {
            assert_eq!(colors.get(missing), None, "{missing}");
        }
    }
}
