//! The profile's fonts folder as a change the settings handle reports, so the app registers the user's
//! font files with CoreText at launch and again when one is added, replaced or removed, as the GPUI app's
//! `fonts::poll_custom` does. The decisions are core's (`theme::fonts::custom`: which files are fonts, the
//! caps, the fingerprint and what changed between two); Swift only calls CoreText with the answer.
//!
//! The watch is the settings follower's (`watch`): one mark from the OS thread, settle, then `check`,
//! which asks [`FontsFolder::moved`] besides the file. The slow poll there is the fallback.

use std::path::PathBuf;
use std::sync::{Mutex, PoisonError};

use wormhole_core::theme::fonts::custom::{self, Fingerprint};

use super::handle::SettingsHandle;
use crate::fonts::{CustomFontFileRecord, file_record};

/// What the fonts folder holds now that it did not when the app last asked.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct FontFolderChanges {
    /// Files to unregister: gone, or changed (a changed file is also in `added`).
    pub removed: Vec<String>,
    /// Files to register: new, or changed, and only those that parse as fonts.
    pub added: Vec<CustomFontFileRecord>,
}

pub(crate) struct FontsFolder {
    dir: PathBuf,
    /// What the app was last told; empty before the first ask, so that ask lists every file.
    taken: Mutex<Fingerprint>,
}

impl FontsFolder {
    pub(crate) fn new(dir: PathBuf) -> Self {
        Self { dir, taken: Mutex::new(Fingerprint::new()) }
    }

    /// Whether the folder differs from what the app was last told: stat calls, no font is read.
    pub(crate) fn moved(&self) -> bool {
        custom::fingerprint(&self.dir) != *self.taken.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn take(&self) -> FontFolderChanges {
        let now = custom::fingerprint(&self.dir);
        let mut taken = self.taken.lock().unwrap_or_else(PoisonError::into_inner);
        let (removed, added) = custom::changes(&taken, &now);
        *taken = now;
        FontFolderChanges {
            removed: removed.iter().map(|path| path.to_string_lossy().into_owned()).collect(),
            added: added
                .iter()
                .filter_map(|path| custom::read_font_file(path).ok())
                .map(|file| file_record(&file))
                .collect(),
        }
    }
}

#[uniffi::export]
impl SettingsHandle {
    /// The font files to register or unregister since the last call (the first lists every file in the
    /// profile's fonts folder). Reads the files that are new: call it off the main thread when it matters.
    pub fn font_folder_changes(&self) -> FontFolderChanges {
        self.shared.fonts.take()
    }
}

#[cfg(test)]
mod tests {
    use wormhole_core::storage::files::{self, LinkPolicy};

    use wormhole_core::theme::fonts::bundled;

    use super::*;

    #[test]
    fn a_real_font_is_registered_when_it_arrives_and_dropped_when_it_goes() {
        let dir = tempfile::tempdir().expect("temp dir");
        let folder = FontsFolder::new(dir.path().to_path_buf());
        let file = dir.path().join("Mine.ttf");
        files::write_atomic(&file, bundled::FACES[0], LinkPolicy::Follow).expect("write");
        assert!(folder.moved());
        let found = folder.take();
        assert!(found.removed.is_empty());
        let [added] = found.added.as_slice() else { panic!("one font: {found:?}") };
        assert_eq!(added.families[0].name, bundled::FAMILY);
        assert!(!folder.moved());
        std::fs::remove_file(&file).expect("remove");
        assert!(folder.moved());
        let gone = folder.take();
        assert_eq!((gone.removed.len(), gone.added.len()), (1, 0));
    }

    #[test]
    fn a_file_added_changed_and_removed_is_reported_once_each() {
        let dir = tempfile::tempdir().expect("temp dir");
        let folder = FontsFolder::new(dir.path().to_path_buf());
        assert!(!folder.moved(), "an empty folder is what the app already knows");
        let junk = dir.path().join("Broken.ttf");
        files::write_atomic(&junk, b"not a font", LinkPolicy::Follow).expect("write");
        assert!(folder.moved());
        let first = folder.take();
        assert!(first.added.is_empty(), "a file that is not a font is never registered");
        assert!(!folder.moved());
        files::write_atomic(&junk, b"still not a font, longer", LinkPolicy::Follow).expect("write");
        let second = folder.take();
        assert_eq!(second.removed.len(), 1, "a changed file is dropped first");
        std::fs::remove_file(&junk).expect("remove");
        assert_eq!(folder.take().removed.len(), 1);
        assert!(!folder.moved());
    }
}
