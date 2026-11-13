//! The right panel's Files, Project, Usage and Notes seams, over `wormhole-core` (`FFI-CONTRACT.md`,
//! "Files, Project, Usage and Notes"). Swift's `FilesBackend`, `ProjectBackend` and `UsageBackend`
//! protocols and the notes' persistence are implemented over these objects by the `FilesBridge` module.
//!
//! What is decided here and not in Swift: which names a folder lists and how they are cut and
//! ordered, what a file shows (capped, never through a link out of the root), what a rename allows,
//! which parts of a project folder may act (trust, per core's rules), and what notes.md may read and
//! write. Every object is a thin shell over core's sans-IO rules; the blocking ones are meant for a
//! background thread.

mod files;
mod files_records;
mod listener;
pub(crate) mod names;
mod notes_store;
mod project;
mod settings_edit;
#[cfg(test)]
mod tests;
mod usage;
mod watch;

pub use files::FilesService;
pub use files_records::{
    DirectoryRecord, EntryRecord, FileContentRecord, FileEntryKind, FileProblem, FilesFailure, RenameFailure,
};
pub use listener::PanelListener;
pub(crate) use project::worktree_refusal_in;
pub use notes_store::{NoteSaved, NotesSaved, SessionNotesStore, TaskSaved};
pub use project::{
    ApproveOutcome, FolderPaintRecord, NewTerminalRecord, NotesDocRecord, NotesSyncOutcome, OwnerAnswer,
    OwnerAuthenticator, ProjectActionRecord, ProjectChoiceRecord, ProjectDefaultRecord, ProjectEffect,
    ProjectFailure, ProjectFeatureRecord, ProjectFeatureRowRecord, ProjectPageRecord, ProjectPromptRecord,
    ProjectService, ProjectSettingsRecord, ProjectStateRecord, TaskDocRecord, TerminalPlace, TrustLineRecord,
    TrustStatusRecord,
};
pub use usage::{AgentLogoRecord, PlanLimitRecord, UsageReportRecord, UsageService};
pub use watch::FolderWatch;
