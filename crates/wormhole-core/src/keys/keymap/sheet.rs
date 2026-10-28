//! The keyboard-shortcuts sheet as data: its sections and rows, in the sheet's own
//! sentence-case labels, with the keys read from the action table. A row of an action that is
//! not live yet is left out until it is.

use std::borrow::Cow;
use std::ops;

use super::{ActionId, Stage, entry};
use crate::keys::keyspec::KeySpec;

/// How a row's keys are written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keys {
    /// Each action's key, side by side: "⇧⌘[ ⇧⌘]".
    Each,
    /// The first and the last of a run: "⌥⌘1 … ⌥⌘9".
    Range,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    /// The table's label, or for a row that names each of its actions (" · " between them), the
    /// names of the ones shown.
    pub label: Cow<'static, str>,
    pub actions: Vec<ActionId>,
    /// The keys to show, in order (for a range, the first and the last).
    pub keys: Vec<KeySpec>,
    pub style: Keys,
}

impl Row {
    /// The keys as the sheet writes them, and what its filter matches ("⌘W" finds Close terminal).
    pub fn keys_text(&self) -> String {
        let glyphs: Vec<String> = self.keys.iter().map(KeySpec::glyphs).collect();
        match self.style {
            Keys::Each => glyphs.join(" "),
            Keys::Range => glyphs.join(" … "),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub title: &'static str,
    pub rows: Vec<Row>,
}

use ActionId::*;
use Keys::{Each, Range};

const TERMINALS: [ActionId; 9] = [
    Terminal(1),
    Terminal(2),
    Terminal(3),
    Terminal(4),
    Terminal(5),
    Terminal(6),
    Terminal(7),
    Terminal(8),
    Terminal(9),
];

const GROUPS: [ActionId; 9] =
    [Group(1), Group(2), Group(3), Group(4), Group(5), Group(6), Group(7), Group(8), Group(9)];

/// A row as written below: its label, its actions, how their keys are shown.
type RowSpec = (&'static str, &'static [ActionId], Keys);

/// The sections and rows. The status bar row has ⌘/, so the sheet's own key moved to ⌥⌘/. Find
/// and the prompts' keys were added after that.
const SHEET: &[(&str, &[RowSpec])] = &[
    (
        "Windows & terminals",
        &[
            ("New window", &[NewWindow], Each),
            ("New terminal", &[NewTerminal], Each),
            ("Open folder…", &[OpenFolder], Each),
            ("Close terminal (pane in a split)", &[CloseTerminal], Each),
            ("Close window", &[CloseWindow], Each),
            // The panes.
            ("Split right / down", &[SplitRight, SplitDown], Each),
            ("Zoom pane", &[ZoomPane], Each),
            (
                "Pane left / right / above / below",
                &[FocusPaneLeft, FocusPaneRight, FocusPaneUp, FocusPaneDown],
                Each,
            ),
            ("Previous / next terminal", &[PreviousTerminal, NextTerminal], Each),
            ("Terminals 1–9 (hold ⌘ to see them)", &TERMINALS, Range),
            ("Previous / next group", &[PreviousGroup, NextGroup], Each),
            ("Groups 1–9", &GROUPS, Range),
            ("Go to the terminal that needs input", &[GoToWaiting], Each),
            ("Go to the terminal that finished", &[GoToFinished], Each),
        ],
    ),
    (
        "View & panels",
        &[
            ("Hide / show sidebar", &[ToggleSidebar], Each),
            ("Hide / show right panel", &[ToggleRightPanel], Each),
            ("Hide / show status bar", &[ToggleStatusBar], Each),
            ("Hide / show minimap", &[ToggleMinimap], Each),
            ("Zen mode", &[ToggleZenMode], Each),
            (
                "Changes · History · Notes · Usage · Project · Files",
                &[ShowChanges, ShowHistory, ShowNotes, ShowUsage, ShowProject, ShowFiles],
                Range,
            ),
            ("Focus terminal / panel", &[FocusTerminalOrPanel], Each),
            ("Next / previous area", &[FocusNextArea, FocusPreviousArea], Each),
            ("Bigger / smaller / actual size", &[Bigger, Smaller, ActualSize], Each),
            ("Full screen", &[FullScreen], Each),
        ],
    ),
    (
        "Terminal",
        &[
            ("Copy / paste", &[Copy, Paste], Each),
            ("Select all", &[SelectAll], Each),
            ("Clear buffer", &[ClearBuffer], Each),
            ("Find in scrollback", &[Find], Each),
            ("Find next / previous", &[FindNext, FindPrevious], Each),
            ("Use selection for find", &[UseSelectionForFind], Each),
            ("Previous / next prompt", &[PreviousPrompt, NextPrompt], Each),
        ],
    ),
    (
        "Find anything",
        &[
            ("Command palette", &[CommandPalette], Each),
            ("Keyboard shortcuts (this sheet)", &[KeyboardShortcuts], Each),
            ("Wormhole help", &[Help], Each),
        ],
    ),
    ("Notes", &[("New note", &[NewNote], Each), ("Stash typed prompt", &[StashPrompt], Each)]),
    (
        "App",
        &[
            ("Settings", &[Settings], Each),
            ("Theme Studio", &[ThemeStudio], Each),
            // The Studio's own keys, so they can be found here too.
            ("Save theme (in the Studio)", &[SaveTheme], Each),
            ("Save Copy (in the Studio)", &[SaveThemeCopy], Each),
            ("Hide Wormhole", &[Hide], Each),
            ("Quit", &[Quit], Each),
        ],
    ),
];

/// The sheet's sections with the rows whose keys work today (the system's Help search counts).
pub fn sections() -> Vec<Section> {
    sections_without(&[])
}

/// [`sections`] minus the rows that list any of `hidden` (actions whose feature is off).
pub fn sections_without(hidden: &[ActionId]) -> Vec<Section> {
    SHEET
        .iter()
        .map(|(title, rows)| Section {
            title,
            rows: rows
                .iter()
                .filter_map(|&(label, actions, style)| {
                    let shown: Vec<ActionId> =
                        actions.iter().copied().filter(|a| !hidden.contains(a)).collect();
                    if shown.is_empty() {
                        return None;
                    }
                    if shown.len() == actions.len() {
                        return row(Cow::Borrowed(label), &shown, style);
                    }
                    // A run with a gap is no run: "⌃⌘1 … ⌃⌘4" would promise the ⌃⌘3 that is hidden.
                    row(label_of(label, actions, &shown), &shown, Keys::Each)
                })
                .collect(),
        })
        .filter(|section| !section.rows.is_empty())
        .collect()
}

/// A row's label once some of its actions are hidden: a label that names each action in turn
/// ("Changes · History · Notes") names the ones shown; any other stays as written.
fn label_of(label: &'static str, actions: &[ActionId], shown: &[ActionId]) -> Cow<'static, str> {
    let parts: Vec<&str> = label.split(" · ").collect();
    if parts.len() != actions.len() {
        return Cow::Borrowed(label);
    }
    let named: Vec<&str> = actions
        .iter()
        .zip(parts)
        .filter(|(action, _)| shown.contains(action))
        .map(|(_, part)| part)
        .collect();
    Cow::Owned(named.join(" · "))
}

fn row(label: Cow<'static, str>, actions: &[ActionId], style: Keys) -> Option<Row> {
    let entries: Vec<_> = actions.iter().map(|id| entry(*id)).collect::<Option<_>>()?;
    if entries.iter().any(|e| e.stage != Stage::Now && e.stage != Stage::System) {
        return None;
    }
    let keys: Vec<KeySpec> = entries.iter().filter_map(|e| e.key().cloned()).collect();
    let keys = match style {
        Keys::Range => [keys.first(), keys.last()].into_iter().flatten().cloned().collect(),
        Keys::Each => keys,
    };
    Some(Row { label, actions: actions.to_vec(), keys, style })
}

/// What the filter found in a row: where its label matched (bytes, to mark), if it was the label.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hit {
    pub label: Option<ops::Range<usize>>,
}

/// Whether a row with `label` and `keys` is what the sheet's filter asks for: text is
/// looked for in the label and in the keys as written ("f6"); a query with a modifier glyph is
/// a key, and finds the rows that have that very key ("⌘W" is Close terminal, not Close window's ⇧⌘W),
/// also inside a range ("⌘5" is in Terminals 1–9).
pub fn matches(query: &str, label: &str, keys: &[KeySpec]) -> Option<Hit> {
    let query = query.trim();
    if query.is_empty() {
        return Some(Hit { label: None });
    }
    if query.contains(['⌘', '⌥', '⌃', '⇧']) {
        return match KeySpec::from_glyphs(query) {
            Some(wanted) => keys.iter().any(|k| k.same_key(&wanted)).then_some(Hit { label: None }),
            None => {
                let text: Vec<String> = keys.iter().map(KeySpec::glyphs).collect();
                text.iter().any(|k| k.contains(query)).then_some(Hit { label: None })
            }
        };
    }
    let (lower, wanted) = (label.to_lowercase(), query.to_lowercase());
    if let Some(at) = lower.find(&wanted) {
        // Marked only when lowercasing kept every byte where it was.
        let fits = lower.len() == label.len()
            && label.is_char_boundary(at)
            && label.is_char_boundary(at + wanted.len());
        return Some(Hit { label: fits.then(|| at..at + wanted.len()) });
    }
    let keys_text: Vec<String> = keys.iter().map(|k| k.glyphs().to_lowercase()).collect();
    keys_text.iter().any(|k| k.contains(&wanted)).then_some(Hit { label: None })
}

impl Row {
    /// Every key the row stands for: a range's middle keys too.
    pub fn all_keys(&self) -> Vec<KeySpec> {
        self.actions.iter().filter_map(|id| entry(*id)).flat_map(|e| e.keys.iter().cloned()).collect()
    }
}

/// The sheet as its filter leaves it: the sections that still have rows (in their order, each
/// row with what matched), and how many rows there are in all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Filtered {
    pub sections: Vec<(&'static str, Vec<(Row, Hit)>)>,
    pub shown: usize,
    pub total: usize,
}

/// The rows of `sections` that `query` finds (an empty query: all of them).
pub fn filter(sections: &[Section], query: &str) -> Filtered {
    let total = sections.iter().map(|s| s.rows.len()).sum();
    let found: Vec<(&'static str, Vec<(Row, Hit)>)> = sections
        .iter()
        .map(|section| {
            let rows = section
                .rows
                .iter()
                .filter_map(|row| matches(query, &row.label, &row.all_keys()).map(|hit| (row.clone(), hit)))
                .collect();
            (section.title, rows)
        })
        .filter(|(_, rows): &(_, Vec<_>)| !rows.is_empty())
        .collect();
    let shown = found.iter().map(|(_, rows)| rows.len()).sum();
    Filtered { sections: found, shown, total }
}

/// Splits sections of these heights, in order, into at most `columns` runs whose tallest is as
/// short as it can be (the sheet lays its sections out in three columns, read down then across).
pub fn columns(heights: &[usize], columns: usize) -> Vec<ops::Range<usize>> {
    let n = heights.len();
    if n == 0 || columns == 0 {
        return Vec::new();
    }
    let columns = columns.min(n);
    // best[k][i]: the tallest column when the first i sections go into k columns.
    let prefix: Vec<usize> = std::iter::once(0)
        .chain(heights.iter().scan(0, |sum, h| {
            *sum += h;
            Some(*sum)
        }))
        .collect();
    let mut best = vec![vec![usize::MAX; n + 1]; columns + 1];
    let mut cut = vec![vec![0; n + 1]; columns + 1];
    best[0][0] = 0;
    for k in 1..=columns {
        for i in 1..=n {
            for j in (k - 1)..i {
                if best[k - 1][j] == usize::MAX {
                    continue;
                }
                let tallest = best[k - 1][j].max(prefix[i] - prefix[j]);
                if tallest < best[k][i] {
                    best[k][i] = tallest;
                    cut[k][i] = j;
                }
            }
        }
    }
    let mut runs = Vec::with_capacity(columns);
    let mut end = n;
    for k in (1..=columns).rev() {
        let start = cut[k][end];
        runs.push(start..end);
        end = start;
    }
    runs.reverse();
    runs
}
