//! The Project tab's values as Swift reads them (`PanelModel`'s `ProjectState` and its parts,
//! `FolderPaint`), written by core's rules and the app's wording. Every line that depends on trust or
//! on a file's standing arrives already written.

use wormhole_core::project::{TrustLine, TrustStatus};

#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum TrustStatusRecord {
    Untrusted,
    Trusted,
    /// Was trusted, but the files changed since: approved again before use.
    Changed,
}

impl From<TrustStatus> for TrustStatusRecord {
    fn from(status: TrustStatus) -> Self {
        match status {
            TrustStatus::Untrusted => Self::Untrusted,
            TrustStatus::Trusted => Self::Trusted,
            TrustStatus::Changed => Self::Changed,
        }
    }
}

/// One line of what a project wants to run, hidden characters written out.
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct TrustLineRecord {
    pub what: String,
    pub runs: String,
    pub note: String,
}

impl From<&TrustLine> for TrustLineRecord {
    fn from(line: &TrustLine) -> Self {
        Self { what: line.what.clone(), runs: line.runs.clone(), note: line.note.clone() }
    }
}

/// A switch of the tab: one part of the project folder, whether it is on for this person and what the
/// line under it says.
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct ProjectFeatureRecord {
    /// `settings`, `theme`, `actions`, `prompts`, `layout` or `notes`.
    pub key: String,
    pub on: bool,
    pub hint: String,
    /// Held back until the person decides (trust).
    pub warns: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct ProjectActionRecord {
    pub id: String,
    pub title: String,
    pub detail: Option<String>,
    /// The key as glyphs ("⌃⌘T").
    pub key: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct ProjectPromptRecord {
    pub id: String,
    pub title: String,
    pub detail: Option<String>,
}

/// The project at a folder as the tab draws it. Actions, prompts and the layout are listed only where
/// core's rule `enabled` holds (the file is there, the person has it on, and trust allows it): what is
/// not listed cannot be run either.
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct ProjectStateRecord {
    pub root: String,
    pub status: TrustStatusRecord,
    pub commands: Vec<TrustLineRecord>,
    pub applies: String,
    pub asks_for_trust: bool,
    pub features: Vec<ProjectFeatureRecord>,
    pub actions: Vec<ProjectActionRecord>,
    pub prompts: Vec<ProjectPromptRecord>,
    pub layout: Vec<String>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// A folder's colour: the person's own, or the project's.
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct FolderPaintRecord {
    /// One of the eight names (`red` ... `pink`).
    pub chosen: Option<String>,
    /// A colour set by hand, matching none of the names: `#7fb2ff`.
    pub custom: Option<String>,
    /// The project locks the colour, or its config.json can't be read: the colour cannot be changed.
    pub locked: bool,
    /// The line under the row.
    pub note: String,
}

/// Why a project call did nothing.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum ProjectFailure {
    /// Trust or the person's switches do not allow it (`reason` says which).
    #[error("not allowed: {reason}")]
    Refused { reason: String },
    /// The folder has no project, or what was asked for is not in it.
    #[error("not found: {what}")]
    NotFound { what: String },
    /// The disk or a file refused: core's own words.
    #[error("{message}")]
    Failed { message: String },
}

impl ProjectFailure {
    pub(super) fn refused(reason: impl Into<String>) -> Self {
        Self::Refused { reason: reason.into() }
    }

    pub(super) fn not_found(what: impl Into<String>) -> Self {
        Self::NotFound { what: what.into() }
    }

    pub(super) fn failed(error: impl std::fmt::Display) -> Self {
        Self::Failed { message: error.to_string() }
    }
}
