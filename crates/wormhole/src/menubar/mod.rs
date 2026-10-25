//! The status item ("menu extra"): one for every window, whose icon says whether a terminal
//! needs input and whose menu lists the terminals by status.
//!
//! The snapshot → layout mapping is pure (`model`) and tested; `platform::status_item` draws it
//! (on macOS an `NSStatusItem` + `NSMenu`, whose own glyph `platform/macos/status_item/icon.rs`
//! draws). Everything here must run on the main thread.

pub mod model;

use std::path::PathBuf;

use wormhole_core::config::MenuBarIcon;
use wormhole_core::notify::Terminal;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionItem {
    /// The project's folder name, shown in its header.
    pub project: String,
    /// The project the action belongs to: folder names repeat (clones, worktrees), roots don't.
    pub root: PathBuf,
    pub id: String,
    pub title: String,
}

/// A color the status item and its menu draw a glyph in (sRGB, 0–1): the theme's status colours,
/// not the system's.
pub type Rgba = [f32; 4];

/// The theme's status colours, as the native menu glyphs take them.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct StatusColors {
    /// `agent.attention`: needs input, the bell.
    pub attention: Rgba,
    /// `agent.done`: finished.
    pub done: Rgba,
    /// `agent.working`: an agent at work.
    pub working: Rgba,
    /// `status.danger`: failed.
    pub danger: Rgba,
    /// `text.muted`: a command running without an agent.
    pub running: Rgba,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MenuSnapshot {
    /// Every sidebar terminal of every window.
    pub terminals: Vec<Terminal>,
    pub actions: Vec<ActionItem>,
    /// (display name, path)
    pub recent_projects: Vec<(String, PathBuf)>,
    /// The global hotkey, shown as the key equivalent of "Show Wormhole".
    pub hotkey: Option<model::KeyEquivalent>,
    /// Jump to Next Waiting's key (⌥⌘J unless set otherwise).
    pub jump_key: Option<model::KeyEquivalent>,
    /// What the icon shows (`menu_bar_icon`) and whether it may animate while working.
    pub icon: MenuBarIcon,
    pub animate: bool,
    pub colors: StatusColors,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MenuCommand {
    ShowApp,
    NewSession,
    /// Pick a folder and open a new terminal there.
    OpenFolder,
    /// Bring the tab forward, in whichever window it is.
    OpenTab(i64),
    /// Run action `id` of the project at `root`.
    RunAction {
        root: PathBuf,
        id: String,
    },
    OpenProject(PathBuf),
    JumpToWaiting,
    OpenSettings,
    CheckForUpdates,
    Quit,
    /// From the global hotkey: what `hotkey_action` says (the quick panel, or show / hide).
    Hotkey,
}
