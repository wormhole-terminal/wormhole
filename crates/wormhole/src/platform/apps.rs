//! Other apps: where one is by its bundle identifier, its icon as the system shows it, and asking
//! one to open a file, a folder or a URL (Open in…, a git panel file in the editor, config.json in
//! the text editor). Which apps are offered and how each takes a line are core's (`open_in`,
//! `open_file`). Revealing a file, opening a folder or a URL with the system's own choice go
//! through GPUI (`reveal_path`, `open_with_system`, `open_url`), not here.

use std::io;
use std::path::{Path, PathBuf};

use wormhole_core::open_file::Launch;

use super::os::apps as os;
use super::spawn::spawn_detached;

/// Where the app with this bundle identifier is (Finder's is `/System/Library/CoreServices/Finder.app`),
/// as the system knows it; `None` when it isn't installed.
pub fn app_path(bundle: &str) -> Option<PathBuf> {
    os::app_path(bundle)
}

/// The app's name as Finder shows it: its folder's name without `.app`.
pub fn name_of(bundle: &str) -> Option<String> {
    let path = app_path(bundle)?;
    Some(path.file_stem()?.to_string_lossy().into_owned())
}

/// The bundle identifier of the app at `path` (an `.app` the person picked), if it is one.
pub fn bundle_of(path: &Path) -> Option<String> {
    os::bundle_of(path)
}

/// The icon of the app with this bundle identifier as a PNG, `size` pixels square.
pub fn icon_png(bundle: &str, size: usize) -> Option<Vec<u8>> {
    os::icon_png(bundle, size)
}

/// Ask the app with this bundle identifier to do what `launch` says; nobody waits for it.
pub fn launch(bundle: &str, launch: Launch) -> io::Result<()> {
    match os::command(bundle, launch) {
        Some(command) => spawn_detached(command),
        None => Err(io::Error::from(io::ErrorKind::Unsupported)),
    }
}

/// Open `path` in the user's text editor, whatever opens the file otherwise.
pub fn open_in_text_editor(path: &Path) -> io::Result<()> {
    match os::text_editor_command(path) {
        Some(command) => spawn_detached(command),
        None => Err(io::Error::from(io::ErrorKind::Unsupported)),
    }
}
