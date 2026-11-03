//! Where the `wh` command stands for terminals that aren't Wormhole's own: the running binary the script runs, the
//! installed script's state, and a path as a person says it. All of it is `wormhole_core::terminal::cli_tool`.

use std::path::{Path, PathBuf};

use wormhole_core::terminal::cli_tool::{self, Status, Targets};

/// The running binary, which the script runs.
pub(in crate::settings_window) fn binary() -> PathBuf {
    std::env::current_exe().unwrap_or_default()
}

pub(in crate::settings_window) fn current_status() -> Status {
    cli_tool::status(&Targets::from_env(), &binary())
}

/// `path` as a person says it (`~/.local/bin/wh`).
pub(in crate::settings_window) fn shown(path: &Path) -> String {
    wormhole_core::view::sidebar::display_path(path, super::paths::home().as_deref())
}
