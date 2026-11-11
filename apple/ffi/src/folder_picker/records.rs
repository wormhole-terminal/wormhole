//! The folder picker's values (`view::picker::types`) as Swift reads them. Plain records; where the typed text
//! matched is in **byte** ranges of the label (the Swift side turns them into character offsets).

use std::path::PathBuf;

use wormhole_core::view::picker::{Answer, Context, Crumb, Decoration, Note, Query, Row};

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PickerContextRecord {
    /// The folder of the terminal the picker is for.
    pub cwd: String,
    pub home: String,
    pub profile: String,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PickerQueryRecord {
    pub text: String,
    pub context: PickerContextRecord,
}

impl PickerQueryRecord {
    pub(super) fn to_core(&self) -> Query {
        Query {
            text: self.text.clone(),
            context: Context {
                cwd: PathBuf::from(&self.context.cwd),
                home: PathBuf::from(&self.context.home),
                profile: self.context.profile.clone(),
            },
        }
    }
}

/// A half-open range of bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct ByteRangeRecord {
    pub start: u32,
    pub end: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PickerRowRecord {
    pub key: String,
    pub label: String,
    pub marks: Vec<ByteRangeRecord>,
    /// An SF Symbol's name.
    pub icon: String,
    pub badge: Option<String>,
    pub detail: Option<String>,
    pub chevron: bool,
    pub descend: Option<String>,
    pub insert: Option<String>,
    pub section: u32,
    pub forgettable: bool,
    pub kind: String,
    pub score: i32,
}

impl From<&Row> for PickerRowRecord {
    fn from(row: &Row) -> Self {
        Self {
            key: row.key.clone(),
            label: row.label.clone(),
            marks: row
                .marks
                .iter()
                .map(|r| ByteRangeRecord { start: r.start as u32, end: r.end as u32 })
                .collect(),
            icon: row.icon.clone(),
            badge: row.badge.clone(),
            detail: row.detail.clone(),
            chevron: row.chevron,
            descend: row.descend.clone(),
            insert: row.insert.clone(),
            section: row.section as u32,
            forgettable: row.forgettable,
            kind: row.kind.clone(),
            score: row.score,
        }
    }
}

/// Something the list says besides its rows.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum PickerNoteRecord {
    Empty,
    Failed,
    Limited { count: u32 },
    Searched { count: u32 },
    Reading,
    Custom { text: String },
}

impl From<&Note> for PickerNoteRecord {
    fn from(note: &Note) -> Self {
        match note {
            Note::Empty => Self::Empty,
            Note::Failed => Self::Failed,
            Note::Limited(count) => Self::Limited { count: *count as u32 },
            Note::Searched(count) => Self::Searched { count: *count as u32 },
            Note::Reading => Self::Reading,
            Note::Custom(text) => Self::Custom { text: text.clone() },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PickerCrumbRecord {
    pub label: String,
    /// The text that goes in the field when it is clicked.
    pub text: String,
}

impl From<&Crumb> for PickerCrumbRecord {
    fn from(crumb: &Crumb) -> Self {
        Self { label: crumb.label.clone(), text: crumb.text.clone() }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PickerAnswerRecord {
    pub sections: Vec<String>,
    pub rows: Vec<PickerRowRecord>,
    pub notes: Vec<PickerNoteRecord>,
    pub up: Option<String>,
    pub up_at_end: bool,
    pub crumbs: Vec<PickerCrumbRecord>,
}

impl From<&Answer> for PickerAnswerRecord {
    fn from(answer: &Answer) -> Self {
        Self {
            sections: answer.sections.clone(),
            rows: answer.rows.iter().map(PickerRowRecord::from).collect(),
            notes: answer.notes.iter().map(PickerNoteRecord::from).collect(),
            up: answer.up.clone(),
            up_at_end: answer.up_at_end,
            crumbs: answer.crumbs.iter().map(PickerCrumbRecord::from).collect(),
        }
    }
}

impl PickerAnswerRecord {
    /// What a job that was given up on, or that could not run, answers: nothing.
    pub(super) fn empty() -> Self {
        Self::from(&Answer::default())
    }
}

/// What a decorator found out about a row, by key; a field left `None` says nothing.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct PickerDecorationRecord {
    pub key: String,
    pub badge: Option<String>,
    pub chevron: Option<bool>,
}

impl From<&Decoration> for PickerDecorationRecord {
    fn from(decoration: &Decoration) -> Self {
        Self { key: decoration.key.clone(), badge: decoration.badge.clone(), chevron: decoration.chevron }
    }
}
