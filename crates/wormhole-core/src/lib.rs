//! Wormhole's UI-independent core.
//!
//! Everything here builds and tests without GPUI: the terminal engine
//! (PTY + alacritty grid + key/mouse encoding), git access, settings and
//! themes, file icon themes, persistence, coding-agent detection and status hooks, and
//! the self-updater.

pub mod agents;
mod color;
pub mod config;
pub mod default_terminal;
pub mod elide;
pub mod features;
pub mod focus;
pub mod folder_color;
pub mod fonts;
pub mod fsutil;
pub mod git;
pub mod icon_theme;
pub mod intro;
pub mod ipc;
pub mod jsonc;
pub mod keymap;
pub mod keyspec;
pub mod layout;
pub mod licenses;
pub mod limits;
pub mod notify;
pub mod onboarding;
pub mod open_in;
pub mod panel;
pub mod panes;
pub mod project;
pub mod reorder;
pub mod search;
pub mod settings_index;
pub mod shells;
pub mod shortcut_record;
pub mod sidebar;
pub mod spoken;
pub mod store;
pub mod terminal;
mod text;
pub mod theme;
pub mod theme_catalog;
pub mod theme_registry;
pub mod title_extras;
pub mod tokens;
pub mod update;
pub mod usage;

/// The terminal state engine, re-exported so the app uses exactly this
/// version (and so a future swap to libghostty stays a core-only change).
pub use alacritty_terminal;
