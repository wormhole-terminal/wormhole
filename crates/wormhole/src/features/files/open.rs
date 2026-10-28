//! What a row hands to other apps: the file or folder in an app, Finder showing it, its path on the
//! clipboard. Nothing is written and nothing from the folder runs, and a folder's content is hostile, so:
//! - what a path names is checked first: a file or folder that leads out of the tree by a link is refused
//!   (`files::resolve_path_within`), and a link is never handed on;
//! - Finder is asked to *show* a file or folder, never to open it: a folder called `x.app` is a program
//!   to the system (`open_file::launch` makes Finder mean Reveal);
//! - a file is opened only in an editor Wormhole knows (or shown in Finder): a terminal the person chose
//!   for folders would run `evil.sh`, `x.command` and the like;
//! - the path is checked and launched off the main thread, where a slow volume can't stall the window;
//! - a path with control characters in it is not copied: pasted into a terminal, a newline would run
//!   what follows it.

use std::path::{Path, PathBuf};

use gpui_kit::*;
use wormhole_core::platform::open_file::{self, Launch};
use wormhole_core::platform::open_in::{self, FINDER};
use wormhole_core::storage::files;
use wormhole_core::view::file_tree::{Row, RowKind};
use wormhole_core::view::notice::Failed;
use wormhole_text::{keys, text};

#[cfg(not(test))]
use crate::platform::background_mode;
use crate::platform::{a11y, clipboard};

/// The real path of the file or folder `row` stands for, if it is one inside `root` and not a link. Reads
/// the disk: not for the main thread.
fn real_path(root: &Path, row: &Row) -> Option<PathBuf> {
    let real = match files::resolve_path_within(root, &row.path) {
        Ok(real) => real,
        Err(err) => {
            tracing::warn!("not opening {} from {}: {err}", row.path.display(), root.display());
            return None;
        }
    };
    let meta = std::fs::metadata(&real).ok()?;
    match row.kind {
        RowKind::File if meta.is_file() => Some(real),
        RowKind::Folder { .. } if meta.is_dir() => Some(real),
        _ => None,
    }
}

/// Whether Open in… has anything to open for `row`: a file or a folder, not a link or a note.
pub(super) fn can_open(row: &Row) -> bool {
    matches!(row.kind, RowKind::File | RowKind::Folder { .. })
}

/// Whether `bundle` may be offered to open `row`: any app for a folder, only the editors Wormhole knows
/// (and Finder, which shows it) for a file.
pub(super) fn may_open_in(bundle: &str, row: &Row) -> bool {
    row.is_folder() || open_in::known(bundle).is_some()
}

/// The app a double-click or ↩ on a file opens it in: the one Open in… resolves to when it is a known
/// editor, else Finder (which shows the file). Until the viewer replaces it, nothing else opens a file.
pub(super) fn default_bundle(resolved: &str) -> &str {
    match open_in::known(resolved) {
        Some(editor) => editor.bundle,
        None => FINDER.bundle,
    }
}

/// Opens the file or folder of `row` in the app with this bundle identifier, off the main thread.
pub(super) fn open_in(bundle: &str, root: &Path, row: &Row, cx: &mut App) {
    if !may_open_in(bundle, row) {
        return;
    }
    let (bundle, root, row) = (bundle.to_string(), root.to_path_buf(), row.clone());
    cx.background_spawn(async move {
        let Some(real) = real_path(&root, &row) else { return };
        // `launch` shows in Finder what Finder is asked for, and opens the rest.
        if let Some(launch) = open_file::launch(&bundle, &real, None) {
            start(&bundle, launch);
        }
    })
    .detach();
}

/// Asks the app to do it. A test, and a test instance of the app, record it instead.
fn start(bundle: &str, launch: Launch) {
    #[cfg(test)]
    {
        record::push(bundle, &launch);
    }
    #[cfg(not(test))]
    {
        if background_mode::on() {
            tracing::info!(bundle, ?launch, "test instance: not launching");
        } else if let Err(err) = crate::platform::apps::launch(bundle, launch) {
            tracing::warn!("opening in {bundle}: {err}");
        }
    }
}

/// Shows the row in Finder (`open -R`, a process: nothing here touches the disk).
pub(super) fn reveal(root: &Path, row: &Row) {
    let path = root.join(&row.path);
    start(FINDER.bundle, Launch::Reveal { path });
}

/// The path as text that is safe to paste: UTF-8 and without a control character.
pub(super) fn pasteable(path: &Path) -> Option<&str> {
    path.to_str().filter(|text| !text.chars().any(char::is_control))
}

/// The row's path on the clipboard, plain and absolute; VoiceOver says so. A path that is not safe to paste
/// is not copied, and the chip says so.
pub(super) fn copy_path(root: &Path, row: &Row, window: &Window, cx: &mut App) {
    let path = root.join(&row.path);
    match pasteable(&path) {
        Some(pasted) => {
            clipboard::write(pasted.to_string(), cx);
            a11y::announce(text(keys::FILES_SPOKEN_PATH_COPIED), false);
        }
        None => {
            crate::notice::post(window, Failed::CopyPath, None, cx);
            a11y::announce(text(keys::NOTICE_FAILED_COPY_PATH), false);
        }
    }
}

/// What a test asks of the launches it made.
#[cfg(test)]
pub(super) mod record {
    use std::sync::Mutex;

    use wormhole_core::platform::open_file::Launch;

    // A static, not a thread-local: a launch is made off the main thread.
    static LAUNCHED: Mutex<Vec<(String, Launch)>> = Mutex::new(Vec::new());

    pub(super) fn push(bundle: &str, launch: &Launch) {
        if let Ok(mut all) = LAUNCHED.lock() {
            all.push((bundle.to_string(), launch.clone()));
        }
    }

    /// What was launched since the last call: the app's bundle and what it was asked.
    pub(in crate::features::files) fn take() -> Vec<(String, Launch)> {
        LAUNCHED.lock().map(|mut all| std::mem::take(&mut *all)).unwrap_or_default()
    }
}
