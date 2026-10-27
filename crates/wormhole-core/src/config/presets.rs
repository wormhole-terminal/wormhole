//! How much the window shows, as one value ([`Shown`]): the title row's parts, the status bar, the
//! minimap, the lines between the columns, whether the right panel starts open, and which of its
//! optional tools are there. Three presets fill it in at once ([`Preset`]: Zen, Balanced, Everything),
//! the first run's walk changes it one answer at a time (`onboarding::tailor`), and Zen mode draws a
//! window with [`Preset::Zen`] for a while without saving it.
//!
//! Balanced is exactly what a first launch shows, so a person who never chooses sees what they always
//! saw.
//!
//! Each part [`Shown`] reads from the settings comes back as it was when it is written: a title row
//! or a set of tools of the person's own, which no answer describes, is kept as it is (`Own`).

use crate::config::Settings;
use crate::config::feature_table::{FEATURES, FeatureId, FeatureKind};
use crate::config::features::enabled_in;
use crate::view::layout::RememberedPanes;
use crate::view::layout::title_parts::{TitlePart, TitleRow};

/// A way to fill in everything [`Shown`] holds at once.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Preset {
    /// The terminal and little else: the title row down to the name and the splits, no status bar,
    /// minimap or column lines, and the right panel closed.
    Zen,
    /// The defaults.
    Balanced,
    /// All of it: the whole title row, the status bar and the minimap, the panel open.
    Everything,
}

impl Preset {
    pub const ALL: [Preset; 3] = [Preset::Zen, Preset::Balanced, Preset::Everything];
}

/// How much of the title row shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TitleShown {
    /// The terminal's name, and the split buttons at the right end.
    Name,
    /// The breadcrumb (folder › branch and its status › the terminal and its status) and Open in… and
    /// the splits, without the extras.
    Path,
    /// Every part, where it is by default.
    Everything,
    /// An arrangement of the person's own, which none of the above describes: kept as it is.
    Own(TitleRow),
}

impl TitleShown {
    /// The three the walk offers, the least first.
    pub const OFFERED: [TitleShown; 3] = [TitleShown::Name, TitleShown::Path, TitleShown::Everything];

    pub fn row(&self) -> TitleRow {
        use TitlePart::*;
        match self {
            TitleShown::Name => TitleRow::new([Terminal, Space, Splits]),
            TitleShown::Path => {
                TitleRow::new([Folder, Branch, BranchStatus, Terminal, TerminalStatus, Space, OpenIn, Splits])
            }
            TitleShown::Everything => TitleRow::default(),
            TitleShown::Own(row) => row.clone(),
        }
    }

    /// Which of the offered ones `row` is, else the person's own.
    pub fn of(row: &TitleRow) -> Self {
        Self::OFFERED.into_iter().find(|shown| shown.row() == *row).unwrap_or_else(|| Self::Own(row.clone()))
    }
}

/// Which of the right panel's optional tools are there (those this build offers: History, and
/// Notes and Project where their flags are on). The ones that always are (Changes, Usage) are
/// there whatever this says.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ToolsShown {
    /// None of them.
    Few,
    /// All of them.
    All,
    /// Some, as the person switched them: these are on, the rest off. Kept as it is.
    Own(Vec<FeatureId>),
}

/// How much the window shows. `panel_open` is a window's (its layout keeps it), not a setting: the app
/// reads it from the window and applies it there.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Shown {
    pub title: TitleShown,
    pub status_bar: bool,
    pub minimap: bool,
    pub column_dividers: bool,
    pub panel_open: bool,
    pub tools: ToolsShown,
}

impl Shown {
    pub fn of(preset: Preset) -> Self {
        match preset {
            Preset::Zen => Self {
                title: TitleShown::Name,
                status_bar: false,
                minimap: false,
                column_dividers: false,
                panel_open: false,
                tools: ToolsShown::All,
            },
            // The defaults, and the right panel as a first launch has it.
            Preset::Balanced => Self {
                title: TitleShown::Everything,
                status_bar: false,
                minimap: false,
                column_dividers: true,
                panel_open: RememberedPanes::default().panel_open,
                tools: ToolsShown::All,
            },
            Preset::Everything => Self {
                title: TitleShown::Everything,
                status_bar: true,
                minimap: true,
                column_dividers: true,
                panel_open: true,
                tools: ToolsShown::All,
            },
        }
    }

    /// What `settings` show now, with the window's panel `panel_open`.
    pub fn read(settings: &Settings, panel_open: bool) -> Self {
        let offered = offered_tools(settings);
        let on: Vec<FeatureId> =
            offered.iter().copied().filter(|id| settings.feature_switched_on(*id)).collect();
        let tools = match on.len() {
            n if n == offered.len() => ToolsShown::All,
            0 => ToolsShown::Few,
            _ => ToolsShown::Own(on),
        };
        Self {
            title: TitleShown::of(&settings.title_row()),
            status_bar: settings.status_bar,
            minimap: settings.minimap,
            column_dividers: settings.column_dividers,
            panel_open,
            tools,
        }
    }

    /// Write it into `settings` (all but `panel_open`, which is the window's). What is the person's
    /// own goes back as it was read, so reading and writing again changes nothing; a tool this build
    /// doesn't offer is left alone.
    pub fn apply(&self, settings: &mut Settings) {
        settings.title_row = Some(self.title.row());
        settings.status_bar = self.status_bar;
        settings.minimap = self.minimap;
        settings.column_dividers = self.column_dividers;
        self.apply_tools(settings);
    }

    /// Write into `settings` only what differs from `from`, which is what they showed: an answer that
    /// changes the minimap writes the minimap and nothing else (not the title row's saved arrangement,
    /// not each tool's switch). Like [`Shown::apply`], all but `panel_open`.
    pub fn apply_changes(&self, from: &Shown, settings: &mut Settings) {
        if self.title != from.title {
            settings.title_row = Some(self.title.row());
        }
        if self.status_bar != from.status_bar {
            settings.status_bar = self.status_bar;
        }
        if self.minimap != from.minimap {
            settings.minimap = self.minimap;
        }
        if self.column_dividers != from.column_dividers {
            settings.column_dividers = self.column_dividers;
        }
        if self.tools != from.tools {
            self.apply_tools(settings);
        }
    }

    fn apply_tools(&self, settings: &mut Settings) {
        for id in offered_tools(settings) {
            let on = match &self.tools {
                ToolsShown::Few => false,
                ToolsShown::All => true,
                ToolsShown::Own(on) => on.contains(&id),
            };
            settings.switch_feature(id, on);
        }
    }

    /// The preset this is, if it is one.
    pub fn preset(&self) -> Option<Preset> {
        Preset::ALL.into_iter().find(|preset| Self::of(*preset) == *self)
    }
}

/// The right panel's optional tools this build offers with `settings`: the optional features with a tab
/// whose flag (while they have one) is on.
pub fn offered_tools(settings: &Settings) -> Vec<FeatureId> {
    let mode = settings.flag_mode();
    FEATURES
        .iter()
        .filter(|info| matches!(info.kind, FeatureKind::Optional { .. }) && !info.tabs.is_empty())
        .filter(|info| info.gate.is_none_or(|flag| enabled_in(flag, &settings.feature_flags, mode)))
        .map(|info| info.id)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Preset, Shown, TitleShown, ToolsShown, offered_tools};
    use crate::config::Settings;
    use crate::config::feature_table::{FeatureId, FeatureMask};
    use crate::view::layout::RememberedPanes;
    use crate::view::layout::title_parts::{TitlePart, TitleRow};

    /// What a first launch shows: the defaults, the right panel as a first launch has it.
    fn first_launch() -> Shown {
        Shown::read(&Settings::default(), RememberedPanes::default().panel_open)
    }

    #[test]
    fn balanced_is_a_first_launch_and_each_preset_reads_back_as_itself() {
        assert_eq!(first_launch(), Shown::of(Preset::Balanced));
        for preset in Preset::ALL {
            let mut settings = Settings::default();
            let shown = Shown::of(preset);
            shown.apply(&mut settings);
            assert_eq!(Shown::read(&settings, shown.panel_open).preset(), Some(preset), "{preset:?}");
        }
    }

    #[test]
    fn zen_keeps_the_name_and_the_splits_and_hides_the_chrome() {
        let mut settings = Settings::default();
        Shown::of(Preset::Zen).apply(&mut settings);
        assert_eq!(settings.title_row().parts(), [TitlePart::Terminal, TitlePart::Space, TitlePart::Splits]);
        assert!(!settings.status_bar && !settings.minimap && !settings.column_dividers);
    }

    #[test]
    fn the_tools_switch_the_panels_optional_features_only() {
        let mut settings = Settings::default();
        Shown { tools: ToolsShown::Few, ..Shown::of(Preset::Balanced) }.apply(&mut settings);
        let mask = FeatureMask::resolve(&settings);
        for id in offered_tools(&settings) {
            assert!(!mask.has(id), "{id:?}");
        }
        assert!(mask.has(FeatureId::Changes) && mask.has(FeatureId::Usage), "always there");
        assert_eq!(Shown::read(&settings, false).tools, ToolsShown::Few);
    }

    /// What no answer describes (a title row, a set of tools of one's own) reads as the person's own,
    /// and writing it back changes nothing.
    #[test]
    fn what_is_ones_own_reads_and_writes_back_as_it_was() {
        let mine = TitleRow::new([TitlePart::Splits, TitlePart::Space, TitlePart::Folder]);
        let mut settings = Settings { title_row: Some(mine.clone()), ..Settings::default() };
        settings.switch_feature(FeatureId::History, false);
        let before = settings.clone();
        let shown = Shown::read(&settings, false);
        assert_eq!(shown.title, TitleShown::Own(mine.clone()));
        assert!(
            matches!(&shown.tools, ToolsShown::Own(on) if !on.contains(&FeatureId::History)),
            "{shown:?}"
        );
        assert_eq!(shown.preset(), None);
        // Another preset, and then back to what was read: as it was.
        Shown::of(Preset::Zen).apply(&mut settings);
        shown.apply(&mut settings);
        assert_eq!(settings, before);
    }

    #[test]
    fn applying_the_changes_writes_only_what_differs() {
        let mut settings = Settings::default();
        let before = settings.clone();
        let balanced = Shown::of(Preset::Balanced);
        // Nothing differs: nothing is written, not even the title row's arrangement.
        balanced.apply_changes(&balanced, &mut settings);
        assert_eq!(settings, before);
        // One answer, one field.
        Shown { minimap: true, ..balanced.clone() }.apply_changes(&balanced, &mut settings);
        assert_eq!(settings, Settings { minimap: true, ..before.clone() });
        // And the result is what a full application of the same window makes.
        let mut whole = before;
        Shown { minimap: true, ..balanced }.apply(&mut whole);
        assert_eq!(whole.minimap, settings.minimap);
    }

    /// A release offers only the tools it ships (no flag of theirs is on there).
    #[test]
    fn the_tools_offered_are_the_ones_this_build_has() {
        let all = offered_tools(&Settings::default());
        assert!(all.contains(&FeatureId::History));
        let off = Settings {
            feature_flags: [("project_folders".to_string(), false)].into(),
            ..Settings::default()
        };
        assert!(!offered_tools(&off).contains(&FeatureId::Project));
    }
}
