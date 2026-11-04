//! The git branch's color a person picks in `branch_color` (the theme's, the primary or their
//! own `branch_color_custom`), and the picks the theme is drawn with.

use super::{BranchColor, Settings};
use crate::theme::tokens::{BranchTint, Color, ColorChoices};

impl Settings {
    /// The branch color chosen over the theme's: the primary, or the custom color once one is
    /// set to a valid color. `None` keeps the theme's own.
    pub fn branch_tint(&self) -> Option<BranchTint> {
        match self.branch_color {
            BranchColor::Theme => None,
            BranchColor::Primary => Some(BranchTint::Primary),
            BranchColor::Custom => Color::from_hex(self.branch_color_custom.trim())
                .ok()
                .map(|c| BranchTint::Color(c.with_alpha(1.0))),
        }
    }

    /// Use `hex` as the branch's color.
    pub fn choose_branch_color(&mut self, hex: String) {
        self.branch_color_custom = hex;
        self.branch_color = BranchColor::Custom;
    }

    /// Every color picked over the theme's, for [`ThemeTokens::with_choices`].
    ///
    /// [`ThemeTokens::with_choices`]: crate::theme::tokens::ThemeTokens::with_choices
    pub fn color_choices(&self) -> ColorChoices {
        ColorChoices { primary: self.primary(), branch: self.branch_tint() }
    }

    /// A problem with `branch_color_custom` to show the user, while it is in use.
    pub(in crate::config) fn branch_color_error(&self) -> Option<String> {
        let value = self.branch_color_custom.trim();
        if value.is_empty() || self.branch_color != BranchColor::Custom {
            return None;
        }
        Color::from_hex(value).err().map(|e| format!("branch_color_custom: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::super::{BranchColor, Settings};
    use crate::theme::tokens::{BranchTint, Color, ColorChoices};

    #[test]
    fn the_branch_color_is_the_themes_until_one_is_chosen() {
        let mut s = Settings::default();
        assert_eq!(s.branch_tint(), None);
        assert!(s.color_choices().is_empty());

        s.branch_color = BranchColor::Primary;
        assert_eq!(s.branch_tint(), Some(BranchTint::Primary));

        // Custom with no color picked yet still shows the theme's.
        s.branch_color = BranchColor::Custom;
        assert_eq!(s.branch_tint(), None);
        assert_eq!(s.branch_color_error(), None);

        s.choose_branch_color("#ff8800".into());
        let orange = Color::from_hex("#ff8800").ok();
        assert_eq!(s.branch_tint(), orange.map(BranchTint::Color));
        assert_eq!(s.color_choices(), ColorChoices { primary: None, branch: orange.map(BranchTint::Color) });

        // Back to the theme's, the color stays for next time.
        s.branch_color = BranchColor::Theme;
        assert_eq!(s.branch_tint(), None);
        assert_eq!(s.branch_color_custom, "#ff8800");
    }

    #[test]
    fn a_custom_branch_color_is_a_valid_hex_or_nothing() {
        let s = Settings {
            branch_color: BranchColor::Custom,
            branch_color_custom: "orange".into(),
            ..Settings::default()
        };
        assert_eq!(s.branch_tint(), None);
        assert!(s.branch_color_error().is_some_and(|e| e.starts_with("branch_color_custom")));
        // Not in use, it is not a problem.
        let s = Settings { branch_color: BranchColor::Theme, ..s };
        assert_eq!(s.branch_color_error(), None);
    }

    #[test]
    fn the_branch_color_reads_and_writes_by_its_name() {
        let s: Settings = serde_json::from_str(r##"{"branch_color": "primary"}"##).unwrap();
        assert_eq!(s.branch_color, BranchColor::Primary);
        let json = serde_json::to_value(&s).unwrap();
        assert_eq!(json["branch_color"], "primary");
        assert_eq!(json["branch_color_custom"], "");
    }
}
