//! The values a provider and a panel trade. All of them are plain, owned data (strings, numbers, ranges,
//! no closures, no handles), so a provider could be written outside this crate and its answers could cross a
//! process boundary as they are: the shape a Probe would need.

use std::ops::Range;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Where the picker was opened: what a provider may use to make sense of the typed text.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Context {
    /// The folder of the terminal the picker is for.
    pub cwd: PathBuf,
    pub home: PathBuf,
    /// The profile the window belongs to, for a provider that keeps something per profile.
    pub profile: String,
}

/// What a provider is asked: the text in the field, as it is.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Query {
    pub text: String,
    pub context: Context,
}

/// One row of the list.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Row {
    /// What the row is, the same whenever the same thing is listed again: decorations, forgetting and the
    /// selection kept across answers go by it, and two rows with one key are one row.
    pub key: String,
    pub label: String,
    /// Where the typed text matched in `label`, as byte ranges, to highlight.
    pub marks: Vec<Range<usize>>,
    /// An SF Symbol's name, which the platform draws as it can (a system without it draws no icon).
    pub icon: String,
    /// A small tag after the label.
    pub badge: Option<String>,
    /// Grey words after the label.
    pub detail: Option<String>,
    /// There is more behind this row (a chevron): a decorator may set it later.
    pub chevron: bool,
    /// The text that goes in the field to go into this row (⇥, →, the ghost completion), when it can be
    /// gone into.
    pub descend: Option<String>,
    /// What picking the row (↩, a click) hands to whoever opened the picker; `None`: it can't be picked.
    pub insert: Option<String>,
    /// The section it is listed in: an index into [`Answer::sections`].
    pub section: usize,
    /// The person can ask to forget this row (⌘⌫).
    pub forgettable: bool,
    /// How well it fits the text; higher first. Rows of one section are ordered by it, stably.
    pub score: i32,
}

impl Row {
    /// A row called `label`, with the key `key`, listed in the first section.
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self { key: key.into(), label: label.into(), ..Self::default() }
    }
}

/// A step of the breadcrumb: its words, and the text that goes in the field when it is clicked.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Crumb {
    pub label: String,
    pub text: String,
}

/// Something the list says besides its rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Note {
    /// Nothing to list. Added by the picker when an answer has no rows.
    Empty,
    /// The provider could not read what the text points at.
    Failed,
    /// The list was cut at this many rows.
    Limited(usize),
    /// Only this many entries were searched, so rows may be missing.
    Searched(usize),
}

/// What a provider answers to a [`Query`].
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Answer {
    /// The titles of the sections, in the order they are listed; an empty title is a section with no header.
    pub sections: Vec<String>,
    pub rows: Vec<Row>,
    pub notes: Vec<Note>,
    /// The text that goes in the field to go one level up from this answer, if there is a level up
    /// (← uses it).
    pub up: Option<String>,
    /// Where the person is, from the outermost step in.
    pub crumbs: Vec<Crumb>,
}

impl Default for Answer {
    fn default() -> Self {
        Self {
            sections: vec![String::new()],
            rows: Vec::new(),
            notes: Vec::new(),
            up: None,
            crumbs: Vec::new(),
        }
    }
}

/// What a decorator found out about a row, by its key. A field left `None` says nothing.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Decoration {
    pub key: String,
    pub badge: Option<String>,
    pub chevron: Option<bool>,
}
