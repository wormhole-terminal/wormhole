//! The seam between the Swift UI experiment and the Rust terminal: a [`Terminal`] on the real
//! engine (`wormhole-term`, started through `wormhole-core`), its screen as packed bytes, its
//! events behind one coalesced wake.
//!
//! This crate is the only `unsafe` boundary the experiment has in Rust, and it contains none that
//! is written by hand: the allowance below is for the scaffolding UniFFI's proc macros generate
//! (`extern "C"` entry points taking raw pointers and buffers). Nothing here names the UI.

// UniFFI's generated scaffolding is `unsafe extern "C"` code; no hand-written `unsafe` exists in
// this crate (`apple/scripts/rules.py` checks that).
#![allow(unsafe_code, reason = "UniFFI's generated scaffolding")]

mod app_actions;
mod aux_backends;
mod contents;
mod blocks;
mod cli_tool;
mod error;
mod events;
mod files_panel;
mod find;
pub mod folder_picker;
mod frame;
mod git;
mod hook_server;
mod inline_images;
mod hosting;
mod interaction;
mod isolation;
pub mod keymap;
mod minimap;
pub mod notify;
pub mod palette_search;
mod settings;
pub mod settings_dump;
mod terminal;
pub mod theme_dump;
mod theme_json;
mod theme_resolve;
mod themes;
mod title_fit;
pub mod words;
mod workspace;
mod workspace_feed;
mod workspace_git;
mod workspace_pump;
mod workspace_types;
mod workspace_watch;

pub use app_actions::{
    default_terminal_failed, default_terminal_script_types, dismissed_tips_count, profile_copy_option,
    profile_delete_question, reset_tips,
};
pub use aux_backends::*;
pub use contents::ContentsScope;
pub use blocks::{BlockRecord, BlockSpanRecord, BlockStatus, PutResult};
pub use cli_tool::{
    CliToolError, CliToolPathRecord, CliToolPathVerdict, CliToolRecord, CliToolState, cli_tool_install,
    cli_tool_keep_current, cli_tool_path_note, cli_tool_status, cli_tool_uninstall,
};
pub use error::FfiError;
pub use events::{TermEvent, TerminalListener};
pub use files_panel::{
    ApproveOutcome, DirectoryRecord, EntryRecord, FileContentRecord, FileEntryKind, FileProblem,
    FilesFailure, FilesService, FolderPaintRecord, FolderWatch, NewTerminalRecord, NoteSaved, NotesDocRecord,
    NotesSaved, NotesSyncOutcome, OwnerAnswer, OwnerAuthenticator, PanelListener, PlanLimitRecord,
    ProjectActionRecord, ProjectEffect, ProjectFailure, ProjectFeatureRecord, ProjectPromptRecord,
    ProjectService, ProjectSettingsRecord, ProjectStateRecord, RenameFailure, SessionNotesStore,
    TaskDocRecord, TaskSaved, TerminalPlace, TrustLineRecord, TrustStatusRecord, UsageReportRecord,
    UsageService,
};
pub use find::{FindOutcome, FoundSpan};
pub use git::{
    Git, GitBackendError, GitChange, GitChangeKind, GitCommitDetails, GitDiff, GitEnvVar, GitListener,
    GitLog, GitLogCommit, GitRef, GitRefKind, GitRev, GitRevKind, GitSignature, GitStatus, GitWatch,
    GitWorktree, GitWorktreePlan, WorktreeService,
};
pub use hook_server::{HookError, HookEvent, HookListener, HookServer, cli_exit_code};
pub use inline_images::InlineImage;
pub use hosting::{
    HostedStart, HostedTerminals, HostingError, HostingProgram, LaunchPlan, OrphanHost, SavedTerminal,
    TerminalHosting,
};
pub use interaction::{
    CellSide, LinkHit, OpenTarget, PointerAction, PointerButton, PointerEvent, RowSpan, SelectionKind,
    WheelStep,
};
pub use minimap::{MinimapGeometryRecord, MinimapRunRecord};
pub use settings::{
    SettingsAgentRow, SettingsChangedRow, SettingsError, SettingsHandle, SettingsHealth, SettingsHit,
    SettingsHookState, SettingsImportRow, SettingsListener, SettingsOptionRow, SettingsPaths,
    SettingsProfileRow, SettingsTextSpan, ShortcutVerdict, check_shortcut, settings_agent_rows,
    settings_create_profile, settings_delete_profile, settings_import_rows, settings_profiles,
    settings_schema_json, settings_set_agent_hooks, settings_switch_profile,
};
pub use terminal::{FrameTiming, KeyPress, Terminal};
pub use themes::{list_user_themes, resolve_theme};
pub use title_fit::{TitlePathShown, TitleRowFit, TitleRowWants, title_fit_row};
pub use workspace::{TerminalFactsRecord, Workspace, WorkspaceConfig, WorkspaceEvent, WorkspaceListener};

uniffi::setup_scaffolding!();

/// The packed frame's layout version, for the Swift reader to check against its own.
#[uniffi::export]
pub fn frame_format_version() -> u32 {
    frame::VERSION
}

/// The contract's version (`FFI-CONTRACT.md`), for the Swift side to check against its own at start-up:
/// 2 added `Workspace`. A minor addition (a new event or action case) does not change it.
#[uniffi::export]
pub fn contract_version() -> u32 {
    CONTRACT_VERSION
}

/// See [`contract_version`].
pub(crate) const CONTRACT_VERSION: u32 = 2;

/// A call that does nothing, for measuring what crossing the seam costs by itself.
#[uniffi::export]
pub fn ping(value: u64) -> u64 {
    value
}

/// Tests start real terminals, which refuse to run on real data: give the test process the
/// experiment's own data folder, once, before any test thread reads the variable.
#[cfg(test)]
pub(crate) fn isolate_for_tests() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let dir = wormhole_core::platform::paths::temp().join("Wormhole-swift-ffi-tests");
        // SAFETY: called once, from the first test that needs it, before it starts a thread of its own.
        // Other test threads only read the variable through `isolation::require`, after this `Once`.
        unsafe { std::env::set_var("WORMHOLE_DATA_DIR", dir) };
    });
}

#[cfg(test)]
mod workspace_feed_tests;
#[cfg(test)]
mod workspace_tests;
