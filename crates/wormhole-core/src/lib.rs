//! Wormhole's UI-independent core.
//!
//! Everything here builds and tests without GPUI: the terminal engine
//! (PTY + screen + key/mouse encoding), git access, settings and
//! themes, file icon themes, persistence, coding-agent detection and status hooks, and
//! the self-updater.
//!
//! Its modules are grouped by domain, lowest first; a domain uses only the ones
//! before it (`scripts/structure.json`'s `layers` checks it):
//! `platform`, `storage`, `keys`; `theme`, `agents`, `update`; `view`, `project`; `config`; `git`, `notify`;
//! `ipc`; `terminal`; `onboarding`.

pub mod agents;
pub mod config;
pub mod git;
pub mod ipc;
pub mod keys;
pub mod notify;
pub mod onboarding;
pub mod platform;
pub mod project;
pub mod storage;
pub mod terminal;
pub mod theme;
pub mod update;
pub mod view;

// Old paths, for one commit only: the next commit names every module by its domain and removes them.
#[doc(hidden)]
pub use agents::usage;
#[doc(hidden)]
pub use config::clipboard;
#[doc(hidden)]
pub use config::features;
#[doc(hidden)]
pub use config::profile;
#[doc(hidden)]
pub use config::settings_index;
#[doc(hidden)]
pub use git::review;
#[doc(hidden)]
pub use ipc::host_binary;
#[doc(hidden)]
pub use ipc::hosts;
#[doc(hidden)]
pub use keys::keymap;
#[doc(hidden)]
pub use keys::keyspec;
#[doc(hidden)]
pub use keys::shortcut_record;
#[doc(hidden)]
pub use onboarding::intro;
#[doc(hidden)]
pub use onboarding::welcome;
#[doc(hidden)]
pub use platform::open_file;
#[doc(hidden)]
pub use platform::open_in;
#[doc(hidden)]
pub use platform::quit_signal;
#[doc(hidden)]
pub use storage::fsutil;
#[doc(hidden)]
pub use storage::jsonc;
use storage::jsonc::suggest as text;
#[doc(hidden)]
pub use storage::paths;
#[doc(hidden)]
pub use storage::store;
#[doc(hidden)]
pub use terminal::cli_tool;
#[doc(hidden)]
pub use terminal::default_terminal;
#[doc(hidden)]
pub use terminal::shells;
#[doc(hidden)]
pub use theme::catalog as theme_catalog;
#[doc(hidden)]
pub use theme::folder_color;
#[doc(hidden)]
pub use theme::fonts;
#[doc(hidden)]
pub use theme::icon_theme;
#[doc(hidden)]
pub use theme::registry as theme_registry;
#[doc(hidden)]
pub use theme::tokens;
use theme::tokens::color;
#[doc(hidden)]
pub use update::licenses;
#[doc(hidden)]
pub use view::elide;
#[doc(hidden)]
pub use view::focus;
#[doc(hidden)]
pub use view::layout;
#[doc(hidden)]
pub use view::limits;
#[doc(hidden)]
pub use view::panel;
#[doc(hidden)]
pub use view::panes;
#[doc(hidden)]
pub use view::reorder;
#[doc(hidden)]
pub use view::search;
#[doc(hidden)]
pub use view::sidebar;
#[doc(hidden)]
pub use view::spoken;
#[doc(hidden)]
pub use view::title_extras;
