//! The three fonts (terminal, interface, code): their slots and size ranges, what they come to
//! ([`ResolvedFonts`]; the settings resolve themselves into it, `config::font_slots`), and the font lists
//! the pickers show. `bundled` is the font that ships with the app, `custom` the user's font files.

pub mod bundled;
pub mod custom;

use std::collections::HashSet;
use std::ops::RangeInclusive;

/// macOS's virtual family for the system UI font (SF Pro); GPUI resolves it.
pub const SYSTEM_FONT: &str = ".SystemUIFont";

/// The sizes the terminal and code fonts come in: Settings keeps them within it, and so does a
/// window's ⌘+ / ⌘- zoom.
pub const TERMINAL_FONT_SIZES: RangeInclusive<f32> = 8.0..=36.0;

/// The size a fresh config has for the interface font. While the setting
/// still has it, a theme's `metrics.ui_font_size` may set another size.
pub const DEFAULT_UI_FONT_SIZE: f32 = 13.0;

/// One of the separately configurable fonts.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FontSlot {
    Terminal,
    Interface,
    Code,
}

impl FontSlot {
    pub const ALL: [FontSlot; 3] = [FontSlot::Interface, FontSlot::Code, FontSlot::Terminal];

    pub fn title(self) -> &'static str {
        match self {
            FontSlot::Terminal => "Terminal",
            FontSlot::Interface => "Interface",
            FontSlot::Code => "Code",
        }
    }

    /// Short id for element ids and devctl.
    pub fn key(self) -> &'static str {
        match self {
            FontSlot::Terminal => "terminal",
            FontSlot::Interface => "interface",
            FontSlot::Code => "code",
        }
    }

    pub fn from_key(key: &str) -> Option<FontSlot> {
        FontSlot::ALL.into_iter().find(|slot| slot.key().eq_ignore_ascii_case(key.trim()))
    }

    /// Terminal and code text line up in columns, so their pickers start with
    /// monospace fonts only.
    pub fn prefers_monospace(self) -> bool {
        matches!(self, FontSlot::Terminal | FontSlot::Code)
    }

    /// The label of the "follow something else" choice an empty family means
    /// (the terminal font always names a family).
    pub fn inherit_label(self) -> Option<&'static str> {
        match self {
            FontSlot::Terminal => None,
            FontSlot::Interface => Some("System font"),
            FontSlot::Code => Some("Same as terminal"),
        }
    }

    pub fn size_range(self) -> (f32, f32) {
        match self {
            FontSlot::Terminal | FontSlot::Code => (*TERMINAL_FONT_SIZES.start(), *TERMINAL_FONT_SIZES.end()),
            // Interface text scales the whole chrome (GPUI Kit sizes in rems),
            // so keep it within what fixed-height rows can hold.
            FontSlot::Interface => (10.0, 20.0),
        }
    }
}

/// The fonts in effect after the settings, the theme and the fallbacks (`Settings::resolved_fonts`).
#[derive(Clone, Debug, PartialEq)]
pub struct ResolvedFonts {
    pub terminal_family: String,
    pub terminal_size: f32,
    pub ui_family: String,
    pub ui_size: f32,
    pub code_family: String,
    pub code_size: f32,
}

impl ResolvedFonts {
    pub fn family(&self, slot: FontSlot) -> &str {
        match slot {
            FontSlot::Terminal => &self.terminal_family,
            FontSlot::Interface => &self.ui_family,
            FontSlot::Code => &self.code_family,
        }
    }

    pub fn size(&self, slot: FontSlot) -> f32 {
        match slot {
            FontSlot::Terminal => self.terminal_size,
            FontSlot::Interface => self.ui_size,
            FontSlot::Code => self.code_size,
        }
    }
}

/// What the user calls a family: the system font's virtual name reads as "System font".
pub fn display_name(family: &str) -> &str {
    if family == SYSTEM_FONT { "System font" } else { family }
}

/// The installed families a picker lists for `query`.
///
/// Hidden families (a leading `.`, macOS's private system faces) are left
/// out. With `monospace_only`, only families in `monospace` are listed –
/// unless detection hasn't finished (`None`), when everything is. Matches on
/// every word of the query; families starting with the query come first,
/// then the rest alphabetically.
pub fn font_choices(
    installed: &[String],
    monospace: Option<&HashSet<String>>,
    query: &str,
    monospace_only: bool,
) -> Vec<String> {
    let query = query.trim().to_lowercase();
    let words: Vec<&str> = query.split_whitespace().collect();
    let mut choices: Vec<(bool, String, &String)> = installed
        .iter()
        .filter(|name| !name.is_empty() && !name.starts_with('.'))
        .filter(|name| !monospace_only || monospace.is_none_or(|mono| mono.contains(*name)))
        .filter_map(|name| {
            let lower = name.to_lowercase();
            words
                .iter()
                .all(|word| lower.contains(word))
                .then(|| (!query.is_empty() && !lower.starts_with(&query), lower, name))
        })
        .collect();
    choices.sort();
    choices.dedup_by(|a, b| a.2 == b.2);
    choices.into_iter().map(|(_, _, name)| name.clone()).collect()
}

/// Where a family in a picker comes from; pickers list them in this order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum FontSource {
    /// Ships with the app (JetBrains Mono).
    Bundled,
    /// From the user's fonts folder.
    Custom,
    Installed,
}

impl FontSource {
    pub fn label(self) -> &'static str {
        match self {
            FontSource::Bundled => "Bundled",
            FontSource::Custom => "Custom",
            FontSource::Installed => "Installed",
        }
    }
}

/// [`font_choices`] over the installed, bundled and custom families, grouped
/// by [`FontSource`] (search order within each group). A family that is both
/// installed and bundled or custom is listed once, under the latter, since the
/// app draws with that copy. `monospace` must include the bundled and custom
/// monospace families.
pub fn grouped_font_choices(
    installed: &[String],
    bundled: &[String],
    custom: &[String],
    monospace: Option<&HashSet<String>>,
    query: &str,
    monospace_only: bool,
) -> Vec<(FontSource, String)> {
    let all: Vec<String> = installed.iter().chain(bundled).chain(custom).cloned().collect();
    let mut grouped: Vec<(FontSource, String)> = font_choices(&all, monospace, query, monospace_only)
        .into_iter()
        .map(|name| {
            let source = if bundled.contains(&name) {
                FontSource::Bundled
            } else if custom.contains(&name) {
                FontSource::Custom
            } else {
                FontSource::Installed
            };
            (source, name)
        })
        .collect();
    grouped.sort_by_key(|(source, _)| *source);
    grouped
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn choices_hide_private_fonts_and_sort_case_insensitively() {
        let installed = names(&["menlo mod", ".SF NS", "Zapfino", "Arial", "JetBrains Mono"]);
        assert_eq!(
            font_choices(&installed, None, "", false),
            names(&["Arial", "JetBrains Mono", "menlo mod", "Zapfino"])
        );
    }

    #[test]
    fn monospace_filter_waits_for_detection() {
        let installed = names(&["Arial", "Menlo", "SF Mono"]);
        let mono: HashSet<String> = names(&["Menlo", "SF Mono"]).into_iter().collect();
        assert_eq!(font_choices(&installed, Some(&mono), "", true), names(&["Menlo", "SF Mono"]));
        assert_eq!(font_choices(&installed, Some(&mono), "", false).len(), 3);
        assert_eq!(font_choices(&installed, None, "", true).len(), 3);
    }

    #[test]
    fn search_matches_all_words_and_ranks_prefixes_first() {
        let installed = names(&["Andale Mono", "Monaco", "PT Mono", "Menlo", "Mona Sans"]);
        assert_eq!(
            font_choices(&installed, None, "mon", false),
            names(&["Mona Sans", "Monaco", "Andale Mono", "PT Mono"])
        );
        assert_eq!(font_choices(&installed, None, "  pt MONO ", false), names(&["PT Mono"]));
        assert!(font_choices(&installed, None, "nope", false).is_empty());
    }

    #[test]
    fn bundled_and_custom_families_come_first_once() {
        let installed = names(&["Menlo", "JetBrains Mono", "Arial", "Fira Code", "Monaspace Neon"]);
        let bundled = names(&["JetBrains Mono"]);
        let custom = names(&["Fira Code", "Iosevka"]);
        let choices = grouped_font_choices(&installed, &bundled, &custom, None, "", false);
        let expect = [
            (FontSource::Bundled, "JetBrains Mono"),
            (FontSource::Custom, "Fira Code"),
            (FontSource::Custom, "Iosevka"),
            (FontSource::Installed, "Arial"),
            (FontSource::Installed, "Menlo"),
            (FontSource::Installed, "Monaspace Neon"),
        ];
        assert_eq!(choices, expect.map(|(source, name)| (source, name.to_string())));

        let mono: HashSet<String> = names(&["JetBrains Mono", "Iosevka", "Menlo"]).into_iter().collect();
        let choices = grouped_font_choices(&installed, &bundled, &custom, Some(&mono), "o", true);
        let listed: Vec<&str> = choices.iter().map(|(_, name)| name.as_str()).collect();
        assert_eq!(listed, ["JetBrains Mono", "Iosevka", "Menlo"]);
    }

    #[test]
    fn slots_read_from_their_keys() {
        assert_eq!(FontSlot::from_key("Interface"), Some(FontSlot::Interface));
    }
}
