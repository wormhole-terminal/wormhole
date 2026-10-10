//! The three font settings (terminal, interface, code): how each resolves
//! against the theme and the others, and the font lists the pickers show.
//! `bundled` is the font that ships with the app, `custom` the user's font files.

pub mod bundled;
pub mod custom;

use std::collections::HashSet;
use std::ops::RangeInclusive;

use crate::config::Settings;
use crate::tokens::Metrics;

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

    /// The family as stored (empty = inherit).
    pub fn family(self, settings: &Settings) -> &str {
        match self {
            FontSlot::Terminal => &settings.font_family,
            FontSlot::Interface => &settings.ui_font_family,
            FontSlot::Code => &settings.code_font_family,
        }
    }

    pub fn set_family(self, settings: &mut Settings, family: String) {
        match self {
            FontSlot::Terminal if family.trim().is_empty() => {}
            FontSlot::Terminal => settings.font_family = family,
            FontSlot::Interface => settings.ui_font_family = family,
            FontSlot::Code => settings.code_font_family = family,
        }
    }

    pub fn size(self, settings: &Settings) -> f32 {
        match self {
            FontSlot::Terminal => settings.font_size,
            FontSlot::Interface => settings.ui_font_size,
            FontSlot::Code => settings.code_font_size,
        }
    }

    /// Change the size by `delta` points, within the slot's range.
    pub fn step_size(self, settings: &mut Settings, delta: f32) {
        let (min, max) = self.size_range();
        let size = match self {
            FontSlot::Terminal => &mut settings.font_size,
            FontSlot::Interface => &mut settings.ui_font_size,
            FontSlot::Code => &mut settings.code_font_size,
        };
        *size = (*size + delta).clamp(min, max);
    }
}

/// The fonts in effect after the settings, the theme and the fallbacks.
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
    /// The user's choice wins over the theme: a non-empty family setting, or
    /// a size changed from the default. Otherwise the theme's metrics apply,
    /// then the system font (interface) or the terminal font (code).
    pub fn resolve(settings: &Settings, metrics: &Metrics) -> Self {
        let non_empty = |s: &str| (!s.trim().is_empty()).then(|| s.trim().to_string());
        let ui_family = non_empty(&settings.ui_font_family)
            .or_else(|| metrics.ui_font_family.as_deref().and_then(non_empty))
            .unwrap_or_else(|| SYSTEM_FONT.to_string());
        let ui_size = if settings.ui_font_size != DEFAULT_UI_FONT_SIZE {
            settings.ui_font_size
        } else {
            metrics.ui_font_size
        };
        let (min, max) = FontSlot::Interface.size_range();
        Self {
            terminal_family: settings.font_family.clone(),
            terminal_size: settings.font_size,
            ui_family,
            ui_size: ui_size.clamp(min, max),
            code_family: non_empty(&settings.code_font_family)
                .unwrap_or_else(|| settings.font_family.clone()),
            code_size: settings.code_font_size,
        }
    }

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
    fn empty_families_inherit() {
        let settings = Settings::default();
        let fonts = ResolvedFonts::resolve(&settings, &Metrics::default());
        assert_eq!(fonts.ui_family, SYSTEM_FONT);
        assert_eq!(fonts.code_family, settings.font_family);
        assert_eq!(fonts.ui_size, 13.0);
    }

    #[test]
    fn a_fresh_config_uses_the_bundled_font_for_terminal_and_code() {
        let fonts = ResolvedFonts::resolve(&Settings::default(), &Metrics::default());
        assert_eq!(fonts.terminal_family, bundled::FAMILY);
        assert_eq!(fonts.code_family, bundled::FAMILY);
        assert_eq!(fonts.ui_family, SYSTEM_FONT);
    }

    #[test]
    fn theme_metrics_apply_unless_the_user_chose() {
        let metrics =
            Metrics { ui_font_family: Some("Inter".into()), ui_font_size: 14.0, ..Metrics::default() };
        let fonts = ResolvedFonts::resolve(&Settings::default(), &metrics);
        assert_eq!((fonts.ui_family.as_str(), fonts.ui_size), ("Inter", 14.0));

        let settings = Settings {
            ui_font_family: "Helvetica Neue".into(),
            ui_font_size: 15.0,
            code_font_family: "SF Mono".into(),
            ..Settings::default()
        };
        let fonts = ResolvedFonts::resolve(&settings, &metrics);
        assert_eq!((fonts.ui_family.as_str(), fonts.ui_size), ("Helvetica Neue", 15.0));
        assert_eq!(fonts.code_family, "SF Mono");
    }

    #[test]
    fn slots_read_write_and_clamp() {
        let mut settings = Settings::default();
        FontSlot::Code.set_family(&mut settings, "Fira Code".into());
        FontSlot::Terminal.set_family(&mut settings, "  ".into());
        assert_eq!(FontSlot::Code.family(&settings), "Fira Code");
        assert_eq!(FontSlot::Terminal.family(&settings), Settings::default().font_family);
        FontSlot::Interface.step_size(&mut settings, 100.0);
        assert_eq!(FontSlot::Interface.size(&settings), 20.0);
        FontSlot::Code.step_size(&mut settings, -100.0);
        assert_eq!(FontSlot::Code.size(&settings), 8.0);
        assert_eq!(FontSlot::from_key("Interface"), Some(FontSlot::Interface));
    }
}
