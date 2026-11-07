//! The Project tab over core's `project` domain: the folder's state as the tab draws it, trust, the
//! person's switches, folder colours, the effects of actions, prompts and the layout, and the notes.md
//! sync. See `service` for where trust is enforced.

mod effects;
mod hub;
mod notes_sync;
mod overrides;
mod paint;
mod records;
mod service;
mod settings_page;
mod state;
mod trust;

pub use effects::{NewTerminalRecord, ProjectEffect, TerminalPlace};
pub use notes_sync::{NotesDocRecord, NotesSyncOutcome, TaskDocRecord};
pub use overrides::ProjectSettingsRecord;
pub use records::{
    FolderPaintRecord, ProjectActionRecord, ProjectFailure, ProjectFeatureRecord, ProjectPromptRecord,
    ProjectStateRecord, TrustLineRecord, TrustStatusRecord,
};
pub use service::ProjectService;
pub use settings_page::{
    ProjectChoiceRecord, ProjectDefaultRecord, ProjectFeatureRowRecord, ProjectPageRecord,
};
pub use trust::{ApproveOutcome, OwnerAnswer, OwnerAuthenticator};
