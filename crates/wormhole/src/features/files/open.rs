//! What a row hands to other apps: the file or folder in the app Open in… uses, Finder showing it, its
//! path on the clipboard. Nothing is written and nothing from the folder runs: the person asks an app
//! they chose to open something, as when they drag it onto the app, so project trust doesn't come
//! into it. What the path names is checked first, though (a folder's content is hostile): a file or
//! folder that leads out of the tree by a link is refused (`files::resolve_path_within`).

use std::path::{Path, PathBuf};

use gpui_kit::*;
use wormhole_core::platform::open_file::{self, Launch};
use wormhole_core::storage::files;
use wormhole_core::view::file_tree::{Row, RowKind};
use wormhole_text::{keys, text};

use crate::platform::{a11y, background_mode, clipboard};

/// The real path of the file or folder `row` stands for, if it is one inside `root` and not a link.
pub(super) fn real_path(root: &Path, row: &Row) -> Option<PathBuf> {
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

/// Opens the file or folder of `row` in the app with this bundle identifier.
pub(super) fn open_in(bundle: &str, root: &Path, row: &Row) {
    let Some(real) = real_path(root, row) else { return };
    let launch = if row.is_folder() {
        Some(Launch::Open { path: real })
    } else {
        open_file::launch(bundle, &real, None)
    };
    if let Some(launch) = launch {
        start(bundle, launch);
    }
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

/// Shows the row in Finder.
pub(super) fn reveal(root: &Path, row: &Row, cx: &App) {
    let path = root.join(&row.path);
    // GPUI's test platform panics on `reveal_path` (gpui-pre 0.3.9, `platform/test/platform.rs`).
    if cfg!(test) || background_mode::on() {
        tracing::info!(path = %path.display(), "not revealing in a test instance");
        #[cfg(test)]
        record::push("reveal", &Launch::Reveal { path });
        return;
    }
    cx.reveal_path(&path);
}

/// The row's path on the clipboard, plain and absolute; VoiceOver says so.
pub(super) fn copy_path(root: &Path, row: &Row, cx: &mut App) {
    clipboard::write(root.join(&row.path).to_string_lossy().into_owned(), cx);
    a11y::announce(text(keys::FILES_SPOKEN_PATH_COPIED), false);
}

/// What a test asks of the launches it made.
#[cfg(test)]
pub(super) mod record {
    use std::cell::RefCell;

    use wormhole_core::platform::open_file::Launch;

    thread_local! {
        static LAUNCHED: RefCell<Vec<(String, Launch)>> = const { RefCell::new(Vec::new()) };
    }

    pub(super) fn push(bundle: &str, launch: &Launch) {
        LAUNCHED.with(|all| all.borrow_mut().push((bundle.to_string(), launch.clone())));
    }

    /// What was launched since the last call: the app's bundle and what it was asked.
    pub(in crate::features::files) fn take() -> Vec<(String, Launch)> {
        LAUNCHED.with(|all| std::mem::take(&mut *all.borrow_mut()))
    }
}
