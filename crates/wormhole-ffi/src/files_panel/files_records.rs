//! The Files tab's values as Swift reads them (`PanelModel`'s `Directory`, `FileContent`,
//! `DirectoryFailure`, `RenameFailure`), made from core's.

use wormhole_core::storage::files::listing::EntryKind as CoreKind;
use wormhole_core::storage::files::{Loaded, Problem, RenameError};
use wormhole_core::view::file_tree::{Dir, Failure};

/// What an entry is, by its own type: a link is never looked through.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum FileEntryKind {
    Folder,
    File,
    Link,
}

/// An entry of a folder. A name that is not UTF-8 reads with U+FFFD in its place, so a path made from it
/// names nothing and every call on it answers "gone": hostile bytes never become another path.
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct EntryRecord {
    /// What a path is made of.
    pub name: String,
    /// As drawn: control and bidirectional characters replaced.
    pub label: String,
    pub kind: FileEntryKind,
}

/// One folder as the tab lists it: folders first, then files, each in natural order, cut at core's cap.
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Record)]
pub struct DirectoryRecord {
    pub entries: Vec<EntryRecord>,
    /// Entries left out: past the cap, or past what was read at all.
    pub more: u64,
    /// The reader stopped before the end of a huge folder, so `more` counts only what it saw.
    pub at_least: bool,
}

impl From<&Dir> for DirectoryRecord {
    fn from(dir: &Dir) -> Self {
        let entries = dir
            .entries
            .iter()
            .map(|entry| EntryRecord {
                name: entry.name.to_string_lossy().into_owned(),
                label: entry.label.clone(),
                kind: match entry.kind {
                    CoreKind::Folder => FileEntryKind::Folder,
                    CoreKind::File => FileEntryKind::File,
                    CoreKind::Link => FileEntryKind::Link,
                },
            })
            .collect();
        Self { entries, more: dir.more as u64, at_least: dir.at_least }
    }
}

/// Why a folder could not be listed.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum FilesFailure {
    #[error("the folder is gone, or is not one any more")]
    Gone,
    #[error("the folder may not be read")]
    Denied,
    #[error("the folder could not be read")]
    Other,
}

impl From<Failure> for FilesFailure {
    fn from(failure: Failure) -> Self {
        match failure {
            Failure::Gone => Self::Gone,
            Failure::Denied => Self::Denied,
            Failure::Other => Self::Other,
        }
    }
}

/// Why a file can't be shown.
#[derive(Clone, Copy, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum FileProblem {
    Gone,
    /// With links resolved it is outside the folder the tree is of.
    Outside,
    NotAFile,
    Denied,
    Other,
}

impl From<Problem> for FileProblem {
    fn from(problem: Problem) -> Self {
        match problem {
            Problem::Gone => Self::Gone,
            Problem::Outside => Self::Outside,
            Problem::NotAFile => Self::NotAFile,
            Problem::Denied => Self::Denied,
            Problem::Other => Self::Other,
        }
    }
}

/// What the viewer has to show of a file.
#[derive(Clone, Debug, PartialEq, Eq, uniffi::Enum)]
pub enum FileContentRecord {
    /// Text, whole or its first bytes (`truncated`); `size` is the file's.
    Text {
        text: String,
        size: u64,
        truncated: bool,
    },
    Binary {
        size: u64,
    },
    /// A picture, its bytes as the file has them.
    Image {
        bytes: Vec<u8>,
        size: u64,
    },
    Unavailable {
        problem: FileProblem,
    },
}

impl From<Loaded> for FileContentRecord {
    fn from(loaded: Loaded) -> Self {
        match loaded {
            Loaded::Text { text, size, truncated } => Self::Text { text, size, truncated },
            Loaded::Binary { size } => Self::Binary { size },
            Loaded::Image { bytes, size, .. } => Self::Image { bytes, size },
            Loaded::Unavailable(problem) => Self::Unavailable { problem: problem.into() },
        }
    }
}

/// Why a rename was refused: the field stays open and the notice says why.
#[derive(Clone, Copy, Debug, PartialEq, Eq, thiserror::Error, uniffi::Error)]
pub enum RenameFailure {
    #[error("not a usable name")]
    Name,
    #[error("something is called that already")]
    Exists,
    #[error("not allowed")]
    Denied,
    #[error("it is not there any more")]
    Gone,
    #[error("the rename failed")]
    Other,
}

impl From<&RenameError> for RenameFailure {
    /// The words of the Files tab's notice: a path that leaves the folder or goes through a link reads as gone.
    fn from(error: &RenameError) -> Self {
        match error {
            RenameError::Name(_) => Self::Name,
            RenameError::Exists => Self::Exists,
            RenameError::Denied => Self::Denied,
            RenameError::Missing | RenameError::Outside => Self::Gone,
            RenameError::Failed(_) => Self::Other,
        }
    }
}
