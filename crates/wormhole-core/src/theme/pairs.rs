//! Dark and light themes that belong together, as the theme picker shows them: one card with the
//! dark theme on its left and the light one on its right ("One theme for both"). Pure: it reads only
//! the catalog.

use super::catalog::ThemeEntry;
use super::tokens::Appearance;

/// One name the picker shows once, with the dark and the light theme that carry it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ThemePair {
    /// What the card says: the family's name ("Catppuccin"), or the theme's own when it has no partner.
    pub name: String,
    /// The dark half, an index into the catalog.
    pub dark: Option<usize>,
    /// The light half.
    pub light: Option<usize>,
}

impl ThemePair {
    /// The halves in the order a card draws them: dark, then light.
    pub fn halves(&self) -> impl Iterator<Item = usize> {
        self.dark.into_iter().chain(self.light)
    }

    /// Both a dark and a light theme.
    pub fn is_whole(&self) -> bool {
        self.dark.is_some() && self.light.is_some()
    }
}

/// Words of a theme's name that say which kind it is (or which variant), and so are not part of what
/// pairs it with another.
const KIND_WORDS: &[&str] = &["Dark", "Light", "Default", "Hard"];

/// Names that share a family without the kind word being in them: the latte and the mocha are
/// Catppuccin's light and dark.
const FAMILIES: &[(&str, &str)] = &[("Catppuccin Mocha", "Catppuccin"), ("Catppuccin Latte", "Catppuccin")];

/// The name two themes must share to be a pair.
fn pair_name(entry: &ThemeEntry) -> String {
    if let Some((family, _)) = &entry.family {
        return family.clone();
    }
    if let Some((_, family)) = FAMILIES.iter().find(|(label, _)| *label == entry.label) {
        return (*family).to_string();
    }
    let words: Vec<&str> = entry.label.split_whitespace().filter(|word| !KIND_WORDS.contains(word)).collect();
    if words.is_empty() { entry.label.clone() } else { words.join(" ") }
}

/// The catalog's themes as pairs, those with both halves first, each group in catalog order. Of
/// several versions of one kind under a name the first is the half; the others are reached by the
/// picker's "Dark and light separately".
pub fn pairs(entries: &[ThemeEntry]) -> Vec<ThemePair> {
    let mut found: Vec<ThemePair> = Vec::new();
    for (ix, entry) in entries.iter().enumerate() {
        let name = pair_name(entry);
        let at = match found.iter().position(|pair| pair.name == name) {
            Some(at) => at,
            None => {
                found.push(ThemePair { name, dark: None, light: None });
                found.len() - 1
            }
        };
        let half = match entry.tokens.appearance {
            Appearance::Dark => &mut found[at].dark,
            Appearance::Light => &mut found[at].light,
        };
        half.get_or_insert(ix);
    }
    let (whole, rest): (Vec<_>, Vec<_>) = found.into_iter().partition(ThemePair::is_whole);
    whole.into_iter().chain(rest).collect()
}
