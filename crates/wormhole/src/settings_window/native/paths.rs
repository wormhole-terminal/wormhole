//! Places and files the window's buttons reach: the home folder (paths are shown as `~/…` under it), a folder shown in
//! the Finder, a file opened in the text editor.

use std::path::{Path, PathBuf};

use gpui_kit::App;
use wormhole_core::platform::paths;

/// The user's home folder, asked each time: the pages that show a path as `~/…` or look under it are cold code, so
/// nothing is cached.
pub(in crate::settings_window) fn home() -> Option<PathBuf> {
    paths::home()
}

/// `dir` as `~/…` when it is under the home folder, as the default location is shown.
pub(in crate::settings_window) fn with_tilde(dir: &Path) -> String {
    match home().as_deref().and_then(|home| dir.strip_prefix(home).ok()) {
        Some(rest) => format!("~/{}", rest.display()),
        None => dir.display().to_string(),
    }
}

/// Show one of our folders in the Finder, creating it first so there's somewhere to drop files.
// GPUI's test platform panics on `open_with_system` (gpui-pre 0.3.9, `platform/test/platform.rs`), so a test that
// reaches this needs a stand-in first.
pub(in crate::settings_window) fn open_folder(dir: &Path, cx: &App) {
    match wormhole_core::storage::files::create_dir_all_private(dir) {
        Ok(()) => cx.open_with_system(dir),
        Err(err) => tracing::warn!("opening {}: {err}", dir.display()),
    }
}

/// Open a file in the text editor (config.json).
pub(in crate::settings_window) fn open_in_editor(path: &Path) {
    if let Err(err) = crate::platform::apps::open_in_text_editor(path) {
        tracing::warn!("opening {}: {err}", path.display());
    }
}
