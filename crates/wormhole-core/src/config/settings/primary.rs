//! The primary colour a person picks in `primary_color` and the switch that decides whether the theme's
//! own is used instead.

use super::Settings;
use crate::theme::tokens::Color;

impl Settings {
    /// The primary chosen in `primary_color`, while the theme's own is switched off and it is
    /// set to a valid color.
    pub fn primary(&self) -> Option<Color> {
        if self.uses_theme_primary() {
            return None;
        }
        Color::from_hex(&self.primary_color).ok().map(|c| c.with_alpha(1.0))
    }

    /// Whether the theme's own primary is in use: `theme_primary`, or, in a config from before
    /// that switch, whether no color was chosen.
    pub fn uses_theme_primary(&self) -> bool {
        self.theme_primary.unwrap_or_else(|| self.primary_color.trim().is_empty())
    }

    /// Use `hex` as the primary, or, empty, the theme's own (the color last chosen stays, for
    /// the switch).
    pub fn choose_primary(&mut self, hex: String) {
        if hex.is_empty() {
            self.theme_primary = Some(true);
        } else {
            self.primary_color = hex;
            self.theme_primary = Some(false);
        }
    }

    /// A problem with `primary_color` to show the user, while it is in use.
    pub(in crate::config) fn primary_error(&self) -> Option<String> {
        let value = self.primary_color.trim();
        if value.is_empty() || self.uses_theme_primary() {
            return None;
        }
        Color::from_hex(value).err().map(|e| format!("primary_color: {e}"))
    }
}
