//! The seam between the Swift UI experiment and the Rust terminal: a [`Terminal`] on the real
//! engine (`wormhole-term`, started through `wormhole-core`), its screen as packed bytes, its
//! events behind one coalesced wake.
//!
//! This crate is the only `unsafe` boundary the experiment has in Rust, and it contains none that
//! is written by hand: the allowance below is for the scaffolding UniFFI's proc macros generate
//! (`extern "C"` entry points taking raw pointers and buffers). Nothing here names the UI.

// UniFFI's generated scaffolding is `unsafe extern "C"` code; no hand-written `unsafe` exists in
// this crate (`macos/scripts/rules.py` checks that).
#![allow(unsafe_code, reason = "UniFFI's generated scaffolding")]

mod app_actions;
mod aux_backends;
mod blocks;
mod byte_size;
mod cell;
mod cli_tool;
mod clipboard;
mod contents;
mod drop_part;
mod engine_look;
mod error;
mod events;
mod files_panel;
mod find;
mod focus;
pub mod folder_picker;
mod fonts;
mod frame;
mod frame_colors;
mod frame_extras;
mod git;
mod hook_server;
mod hosting;
mod icon_theme;
mod inline_images;
mod interaction;
mod isolation;
pub mod keymap;
mod launch;
mod legacy;
mod live_resize;
mod logging;
mod minimap;
mod motion_scenes;
mod notes_text;
pub mod notify;
mod open_file;
pub mod palette_search;
mod panel_spoken;
mod perf_counters;
mod probes;
mod profile;
mod row_motion;
mod settings;
pub mod settings_dump;
mod terminal;
pub mod theme_dump;
mod theme_json;
mod theme_resolve;
mod studio_files;
mod syntax;
mod themes;
mod title_fit;
mod visible;
mod window_ops;
mod windows;
pub mod words;
mod workspace;
mod workspace_access;
mod workspace_extras;
mod workspace_feed;
mod workspace_git;
mod workspace_notices;
mod workspace_pump;
mod workspace_spoken;
mod workspace_tabs;
mod workspace_texts;
mod workspace_types;
mod workspace_waited;
mod workspace_watch;

pub use app_actions::{
    default_terminal_failed, default_terminal_script_types, dismiss_no_agents_tip, dismissed_tips_count,
    no_agents_tip_dismissed, profile_copy_option, profile_delete_question, reset_tips,
};
pub use aux_backends::*;
pub use blocks::{BlockRecord, BlockSpanRecord, BlockStatus, PutResult};
pub use byte_size::byte_size;
pub use cell::{
    CellRecord, CellShapeRecord, CellShapesRecord, GridFitRecord, cell_shapes, terminal_cell,
    terminal_grid_fit,
};
pub use cli_tool::{
    CliToolError, CliToolPathRecord, CliToolPathVerdict, CliToolRecord, CliToolState, cli_tool_install,
    cli_tool_keep_current, cli_tool_path_note, cli_tool_status, cli_tool_uninstall,
};
pub use contents::ContentsScope;
pub use drop_part::drop_part_at;
pub use engine_look::set_terminal_colors;
pub use error::FfiError;
pub use events::{TermEvent, TerminalListener};
pub use files_panel::{
    AgentLogoRecord, ApproveOutcome, DirectoryRecord, EntryRecord, FileContentRecord, FileEntryKind,
    FileProblem, FilesFailure, FilesService, FolderPaintRecord, FolderWatch, NewTerminalRecord, NoteSaved,
    NotesDocRecord, NotesSaved, NotesSyncOutcome, OwnerAnswer, OwnerAuthenticator, PanelListener,
    PlanLimitRecord, ProjectActionRecord, ProjectChoiceRecord, ProjectDefaultRecord, ProjectEffect,
    ProjectFailure, ProjectFeatureRecord, ProjectFeatureRowRecord, ProjectPageRecord, ProjectPromptRecord,
    ProjectService, ProjectSettingsRecord, ProjectStateRecord, RenameFailure, SessionNotesStore,
    TaskDocRecord, TaskSaved, TerminalPlace, TrustLineRecord, TrustStatusRecord, UsageReportRecord,
    UsageService,
};
pub use find::{FindOutcome, FoldWords, FoundSpan};
pub use focus::{
    AreaStops, EscapeRung, EscapeState, FocusArea, FocusStop, ListItem, ListKey, ListMove, PresetShown,
    WindowPreset, focus_escape, focus_list_walk, focus_press_folds_band, focus_step, focus_tab_step,
    focus_type_ahead, window_preset,
};
pub use fonts::{
    CustomFontFamilyRecord, CustomFontFileRecord, CustomFontsRecord, FontChoiceRecord, FontOrigin,
    font_bundled_family, font_choices, font_custom_scan,
};
pub use git::{
    Git, GitBackendError, GitChange, GitChangeKind, GitCommitDetails, GitCompact, GitDiff, GitDiffFile,
    GitDiffLine, GitEnvVar, GitFact, GitHunk, GitLineKind, GitListener, GitLog, GitLogCommit, GitParent,
    GitRef, GitRefKind, GitRev, GitRevKind, GitSpan, GitStatus, GitTone, GitWatch, GitWorktree,
    GitWorktreePlan, WorktreeService,
};
pub use git::{
    GitDiffNotes, GitProblemWords, GitSideNote, GitSubmoduleCard, git_next_age_change, git_problem,
};
pub use hook_server::{
    HookError, HookEvent, HookListener, HookServer, WelcomeDemoRow, WelcomeDemoState, WelcomeDock,
    WelcomePlan, cli_exit_code, welcome_demo_label, welcome_demo_next_ms, welcome_demo_rows,
    welcome_demo_title, welcome_dock,
};
pub use hosting::{
    HostUnavailableKind, HostedStart, HostedTerminals, HostingError, HostingProgram, LaunchPlan, OrphanHost,
    SavedTerminal, TerminalHosting,
};
pub use icon_theme::IconThemeHandle;
pub use inline_images::InlineImage;
pub use interaction::{
    CellSide, LinkHit, OpenTarget, PointerAction, PointerButton, PointerEvent, RowSpan, SelectionKind,
    WheelStep,
};
pub use live_resize::{
    LiveResizeGate, ResizeStepRecord, SizeRecord, WindowFrameRecord, resize_settle, resize_settled_frame,
    resize_snapped,
};
pub use minimap::{MinimapFoldRecord, MinimapGeometryRecord, MinimapRunRecord};
pub use open_file::{open_file_record_to, open_file_run, open_file_tooltip, repo_file_resolve};
pub use panel_spoken::panel_tab_spoken;
pub use perf_counters::{PerfCounters, PerfLockUse, perf_counters_start, perf_counters_stop};
pub use probes::{ProbeStatusRecord, ResolvedFile};
pub use profile::{ActiveProfile, active_profile};
pub use row_motion::{
    RowMotionRecord, RowPoseRecord, StatusPoseRecord, fold_row_motion, fold_row_pose, insert_row_motion, insert_row_pose,
    status_motion_duration, status_motion_pose,
};
pub use settings::{
    FontFamilyResolver, SettingsImportDone, SettingsImportItem, SettingsImportReview, SettingsImportSkip,
    SettingsLoginField, SettingsProfileForm, SettingsShellLine, settings_import_review,
    settings_import_selected_line, settings_others_head, settings_preview_note, settings_profile_form,
    settings_profile_save, settings_reset_question, settings_update_user_agent,
};
pub use settings::{
    ProbeApprovalOutcome, ProbeChangeRecord, ProbeListRecord, ProbePermissionRecord, ProbeRowRecord,
    ProbeSheetRecord, ProbesDesk, SettingsAgentRow, SettingsChangedRow, SettingsError, SettingsFlagRow,
    SettingsHandle, SettingsHealth, SettingsHit, SettingsHookState, SettingsImportRow, SettingsListener,
    SettingsOptionRow, SettingsPaths, SettingsProfileRow, SettingsSkillRow, SettingsSkillState,
    SettingsTextSpan, ShellChoiceRecord, ShortcutVerdict, ThemeTrashPlan, check_shortcut,
    settings_agent_rows, settings_create_profile, settings_delete_profile, settings_import_rows,
    settings_mcp_command, settings_profiles, settings_schema_json, settings_set_agent_hooks,
    settings_set_skill, settings_skill_rows, settings_switch_profile,
};
pub use terminal::{FrameTiming, KeyPress, Terminal};
pub use studio_files::{
    StudioKeyRename, StudioRead, StudioThemeChoice, StudioThemeFamily, StudioThemeList, StudioUpdate,
    StudioUpdateChange, studio_add_appearance, studio_canonical, studio_convert, studio_copy_theme,
    studio_export, studio_export_dir, studio_key_table, studio_new_id, studio_new_theme, studio_read,
    studio_remember_export_dir, studio_theme_list, studio_token_in_both, studio_update_apply,
    studio_update_check,
};
pub use themes::{list_user_themes, resolve_theme, resolve_theme_json, theme_files_stamp};
pub use title_fit::{TitlePathShown, TitleRowFit, TitleRowWants, title_fit_row};
pub use syntax::{SyntaxRun, syntax_keys, syntax_lines};
pub use visible::{VisibleLine, VisibleMark, visible_line};
pub use window_ops::{
    ExtentRecord, FullscreenRecord, MaximizeRecord, PaneSizeRecord, ScreenRect, WindowFrameGoal,
    WindowGeometryRecord, WindowOpRecord, WindowPlace, window_geometry, window_op_frame,
};
pub use windows::{
    Departure, DisplayRecord, FrameKind, FrameRecord, PlannedWindow, SavedWindow, TakenOut, TearGrow,
    TearLanding, TearPlaced, TearRect, TearWindow, WindowAnimationKind, WindowMinSize, Windows, cascade_frame,
    fit_frame, tear_off_grow, tear_off_grow_ms, tear_off_landing, tear_off_place, window_animation,
    window_min_size,
};
pub use workspace::{TerminalFactsRecord, Workspace, WorkspaceConfig, WorkspaceEvent, WorkspaceListener};
pub use workspace_notices::NoticeFactsRecord;
pub use workspace_texts::LaunchFacts;
pub use workspace_waited::Waited;

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
mod launch_tests;
#[cfg(test)]
mod workspace_feed_tests;
#[cfg(test)]
mod workspace_tests;
