//! The Show filter and the search of the filter popover: which terminals the list
//! keeps, and the words that say a filter is on.

use std::path::Path;

use serde::{Deserialize, Serialize};

/// What a terminal is doing, as the filter sees it. The app maps its own status into this
/// (an agent's, or for a terminal without one its bell and foreground command).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowState {
    /// An agent asks something, or a terminal without an agent rang its bell.
    NeedsYou,
    /// An agent works, or a command runs.
    Working,
    /// An agent finished and nobody has looked yet.
    Finished,
    /// Nothing happening: an agent waiting for a prompt, a shell at its prompt.
    Idle,
    /// The shell ended.
    Exited,
}

impl RowState {
    /// Idle for the filter: nothing to look at (an ended shell included).
    pub fn is_idle(self) -> bool {
        matches!(self, RowState::Idle | RowState::Exited)
    }
}

/// The Show choice: every terminal, or only those in one state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Show {
    #[default]
    All,
    NeedsYou,
    Working,
    Finished,
    Idle,
}

impl Show {
    pub const ALL: [Show; 5] = [Show::All, Show::NeedsYou, Show::Working, Show::Finished, Show::Idle];

    /// Its segment in the popover.
    pub fn label(self) -> &'static str {
        match self {
            Show::All => "All",
            Show::NeedsYou => "Needs you",
            Show::Working => "Working",
            Show::Finished => "Finished",
            Show::Idle => "Idle",
        }
    }

    /// How the "Filtered" chip and the empty list name it.
    fn word(self) -> &'static str {
        match self {
            Show::All => "all",
            Show::NeedsYou => "needs you",
            Show::Working => "working",
            Show::Finished => "finished",
            Show::Idle => "idle",
        }
    }

    fn admits(self, state: RowState) -> bool {
        match self {
            Show::All => true,
            Show::NeedsYou => state == RowState::NeedsYou,
            Show::Working => state == RowState::Working,
            Show::Finished => state == RowState::Finished,
            Show::Idle => state.is_idle(),
        }
    }
}

/// What the filter looks at in one terminal.
#[derive(Clone, Copy, Debug)]
pub struct Searchable<'a> {
    pub name: &'a str,
    pub state: RowState,
    /// Where it is (its folder or working directory), searched as a path.
    pub place: Option<&'a Path>,
    pub branch: Option<&'a str>,
    /// The command in the foreground of a terminal without an agent.
    pub command: Option<&'a str>,
}

/// A window's filter: saved with the window, so it survives a restart; a new window starts
/// without one.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Filter {
    pub show: Show,
    /// "Hide idle terminals": narrows Show's All (the other choices already leave idle ones out,
    /// or ask for them).
    pub hide_idle: bool,
    pub search: String,
}

impl Filter {
    /// Anything filtered out at all (the funnel's dot, the chip).
    pub fn is_active(&self) -> bool {
        self.show != Show::All || self.hide_idle || !self.search.trim().is_empty()
    }

    /// Whether the list keeps this terminal. The search matches every word, case-insensitively,
    /// each in any of the name, the place, the branch or the command.
    pub fn admits(&self, row: &Searchable) -> bool {
        let hidden_idle = self.hide_idle && self.show == Show::All && row.state.is_idle();
        if !self.show.admits(row.state) || hidden_idle {
            return false;
        }
        let place = row.place.map(|place| place.to_string_lossy());
        let fields = [Some(row.name), place.as_deref(), row.branch, row.command];
        self.search.split_whitespace().all(|word| {
            let word = word.to_lowercase();
            fields.iter().flatten().any(|field| field.to_lowercase().contains(&word))
        })
    }

    /// The chip under the header while a filter is on: "Filtered: working · “deploy”".
    pub fn chip(&self) -> Option<String> {
        if !self.is_active() {
            return None;
        }
        let mut parts = Vec::new();
        if self.show != Show::All {
            parts.push(self.show.word().to_string());
        } else if self.hide_idle {
            parts.push("idle hidden".to_string());
        }
        let search = self.search.trim();
        if !search.is_empty() {
            parts.push(format!("\u{201c}{search}\u{201d}"));
        }
        Some(format!("Filtered: {}", parts.join(" · ")))
    }

    /// Why the list is empty when the filter keeps none of `total` terminals: what was asked
    /// for, then how many it hides.
    pub fn nothing_matches(&self, total: usize) -> String {
        let search = self.search.trim();
        let what = match (self.show, search.is_empty()) {
            (Show::All, false) => format!("Nothing matches \u{201c}{search}\u{201d}."),
            (show, false) => format!("Nothing {} matches \u{201c}{search}\u{201d}.", show.word()),
            // Only "Hide idle terminals" is on.
            (Show::All, true) => "Every terminal is idle.".to_string(),
            (Show::NeedsYou, true) => "No terminal needs you.".to_string(),
            (Show::Working, true) => "No terminal is working.".to_string(),
            (Show::Finished, true) => "No terminal has finished.".to_string(),
            (Show::Idle, true) => "No terminal is idle.".to_string(),
        };
        let hidden = match total {
            1 => "The only terminal is hidden by the filter.".to_string(),
            n => format!("All {n} terminals are hidden by the filter."),
        };
        format!("{what} {hidden}")
    }
}

/// The line under a filtered list: "8 terminals hidden".
pub fn hidden_label(hidden: usize) -> String {
    match hidden {
        1 => "1 terminal hidden".to_string(),
        n => format!("{n} terminals hidden"),
    }
}

#[cfg(test)]
mod tests {
    use super::{Filter, RowState, Searchable, Show, hidden_label};
    use std::path::Path;

    fn row(name: &'static str, state: RowState) -> Searchable<'static> {
        Searchable { name, state, place: None, branch: None, command: None }
    }

    fn filter(show: Show, hide_idle: bool, search: &str) -> Filter {
        Filter { show, hide_idle, search: search.into() }
    }

    #[test]
    fn show_keeps_one_state_and_idle_includes_an_ended_shell() {
        let states =
            [RowState::NeedsYou, RowState::Working, RowState::Finished, RowState::Idle, RowState::Exited];
        let kept =
            |show| states.iter().filter(|&&state| filter(show, false, "").admits(&row("x", state))).count();
        assert_eq!(kept(Show::All), 5);
        for show in [Show::NeedsYou, Show::Working, Show::Finished] {
            assert_eq!(kept(show), 1, "{show:?}");
        }
        assert_eq!(kept(Show::Idle), 2);
        // Hiding idle terminals under All keeps the three that have something going on.
        let busy = states.iter().filter(|&&s| filter(Show::All, true, "").admits(&row("x", s))).count();
        assert_eq!(busy, 3);
        // ...and only there: under Idle it would hide everything Show asked for.
        assert!(filter(Show::Idle, true, "").admits(&row("x", RowState::Idle)));
        assert!(!filter(Show::Working, true, "").admits(&row("x", RowState::Idle)));
    }

    #[test]
    fn search_finds_every_word_in_any_field_ignoring_case() {
        let tests = Searchable {
            name: "tests",
            state: RowState::Working,
            place: Some(Path::new("/Users/me/Projects/terminal")),
            branch: Some("ui-unify"),
            command: Some("cargo test -p wormhole-core"),
        };
        for query in ["tests", "TERMINAL", "unify", "cargo", "cargo terminal", "  wormhole-core  ", ""] {
            assert!(filter(Show::All, false, query).admits(&tests), "{query:?}");
        }
        for query in ["deploy", "cargo deploy", "main"] {
            assert!(!filter(Show::All, false, query).admits(&tests), "{query:?}");
        }
        // The search and the Show choice both have to agree.
        assert!(!filter(Show::NeedsYou, false, "tests").admits(&tests));
        assert!(filter(Show::Working, false, "tests").admits(&tests));
    }

    #[test]
    fn a_filter_is_on_when_anything_narrows_the_list() {
        assert!(!Filter::default().is_active());
        assert!(!filter(Show::All, false, "   ").is_active(), "blank search");
        assert!(filter(Show::Idle, false, "").is_active());
        assert!(filter(Show::All, true, "").is_active());
        assert!(filter(Show::All, false, "x").is_active());
    }

    #[test]
    fn the_chip_says_what_is_filtered() {
        assert_eq!(Filter::default().chip(), None);
        assert_eq!(filter(Show::NeedsYou, false, "").chip().as_deref(), Some("Filtered: needs you"));
        assert_eq!(
            filter(Show::Working, false, " deploy ").chip().as_deref(),
            Some("Filtered: working · \u{201c}deploy\u{201d}")
        );
        assert_eq!(filter(Show::All, true, "").chip().as_deref(), Some("Filtered: idle hidden"));
        assert_eq!(filter(Show::All, false, "api").chip().as_deref(), Some("Filtered: \u{201c}api\u{201d}"));
    }

    #[test]
    fn an_empty_list_says_why_and_how_much_is_hidden() {
        assert_eq!(
            filter(Show::Working, false, "deploy").nothing_matches(10),
            "Nothing working matches \u{201c}deploy\u{201d}. All 10 terminals are hidden by the filter."
        );
        assert_eq!(
            filter(Show::NeedsYou, false, "").nothing_matches(1),
            "No terminal needs you. The only terminal is hidden by the filter."
        );
        assert_eq!(hidden_label(8), "8 terminals hidden");
        assert_eq!(hidden_label(1), "1 terminal hidden");
    }

    #[test]
    fn the_filter_round_trips_through_json_and_defaults_what_is_missing() {
        let saved = filter(Show::NeedsYou, true, "api");
        let json = serde_json::to_string(&saved).unwrap_or_default();
        assert_eq!(json, r#"{"show":"needs_you","hide_idle":true,"search":"api"}"#);
        assert_eq!(serde_json::from_str::<Filter>(&json).ok(), Some(saved));
        assert_eq!(serde_json::from_str::<Filter>("{}").ok(), Some(Filter::default()));
    }
}
