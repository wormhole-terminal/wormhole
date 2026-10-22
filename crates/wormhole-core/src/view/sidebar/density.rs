//! How much room each terminal takes (Settings → Terminals & sidebar → Density), in the sidebar
//! and the tab bar alike.

use serde::{Deserialize, Serialize};

/// The three densities of the terminal list.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Density {
    /// 48 pt rows: a 32 pt tile, the name over a second line.
    #[default]
    Regular,
    /// 32 pt rows: a 20 pt tile and one line.
    Compact,
    /// 26 pt rows: a 16 pt tile and one line, tighter cards.
    Dense,
}

impl Density {
    pub const ALL: [Density; 3] = [Density::Regular, Density::Compact, Density::Dense];

    pub fn label(self) -> &'static str {
        match self {
            Density::Regular => "Regular",
            Density::Compact => "Compact",
            Density::Dense => "Dense",
        }
    }

    /// The row's second line (what the terminal is doing, or where it is) is drawn.
    pub fn two_lines(self) -> bool {
        self == Density::Regular
    }
}

#[cfg(test)]
mod tests {
    use super::Density;

    #[test]
    fn the_names_are_the_words_in_config_json() {
        assert_eq!(serde_json::to_string(&Density::Dense).ok().as_deref(), Some("\"dense\""));
        assert_eq!(serde_json::from_str::<Density>("\"compact\"").ok(), Some(Density::Compact));
        assert_eq!(Density::default(), Density::Regular);
    }

    #[test]
    fn only_the_regular_row_has_a_second_line() {
        assert!(Density::Regular.two_lines());
        assert!(!Density::Compact.two_lines() && !Density::Dense.two_lines());
    }
}
