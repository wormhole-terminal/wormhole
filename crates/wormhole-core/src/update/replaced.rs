//! Noticing that the app bundle was replaced on disk while this process runs: an update made outside
//! the app (Homebrew, a copy from a disk image over the old app), or the in-app install itself.
//!
//! Such an app is half out of date: what it launches next (the session host, the CLI) comes from the new
//! bundle. The remedy is the one after an in-app install: restart. It is reported only when the new
//! bundle is another version: the same version copied again (`brew reinstall`, a touch) changes nothing
//! the running app depends on, since the session host admits it by the kernel's record of its signature
//! while its own executable is gone, so a restart would only interrupt.
//!
//! What identifies "the same executable" is the file itself (device, inode) with its size and
//! modification time: a rename or a delete-and-copy gives it another inode, an overwrite in place the
//! other two. Not the change time, which a harmless `xattr` changes too. Only a bundle that can update
//! itself is watched: a translocated copy (a read-only mount of a disk image, never replaced), and
//! anything that doesn't run from a `.app` (`cargo run`, a dev build's rebuilds) are not.

use std::fs::{self, Metadata};
use std::io::Cursor;
use std::path::{Path, PathBuf};

use crate::platform::fs::{mtime, same_file};
use crate::storage::files::{LinkPolicy, read_capped};

use super::install::{bundle_from_exe, is_translocated};

/// The most of `Info.plist` read (they are a few KiB).
const PLIST_CAP: u64 = 256 * 1024;

/// The executable this process started from, as it was then.
#[derive(Clone, Debug)]
pub struct Launched {
    exe: PathBuf,
    bundle: PathBuf,
    meta: Metadata,
    /// The version this process is, to tell a new one from the same one copied again.
    running: String,
}

/// What the bundle on disk is now, once it is not what this process started from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Replaced {
    /// The version in the new bundle's `Info.plist`; never the running one.
    pub version: String,
}

impl Launched {
    /// The running executable, when it is in a bundle that could be replaced: `None` for a dev build,
    /// a translocated copy, or one that can't be looked at.
    pub fn capture(running: &str) -> Option<Self> {
        Self::of(&std::env::current_exe().ok()?, running)
    }

    /// [`capture`](Self::capture) for the executable at `exe`.
    pub(crate) fn of(exe: &Path, running: &str) -> Option<Self> {
        let bundle = bundle_from_exe(exe).filter(|bundle| !is_translocated(bundle))?;
        let meta = fs::metadata(exe).ok()?;
        Some(Self { exe: exe.to_path_buf(), bundle, meta, running: running.to_owned() })
    }

    /// Whether the executable at the same path is another file now, or the same file changed, and the
    /// bundle's `Info.plist` names another version than the running one. A path that can't be looked
    /// at (it is mid-swap, or the app was trashed) and a plist that can't be read are not replaced:
    /// the next look settles it, and no replacement has been seen.
    ///
    /// Blocking file system calls: not for the UI thread.
    pub fn replaced(&self) -> Option<Replaced> {
        let now = fs::metadata(&self.exe).ok()?;
        let same =
            same_file(&self.meta, &now) && self.meta.len() == now.len() && mtime(&self.meta) == mtime(&now);
        if same {
            return None;
        }
        let version = bundle_version(&self.bundle)?;
        (version != self.running).then_some(Replaced { version })
    }
}

/// `CFBundleShortVersionString` of the bundle at `bundle`.
fn bundle_version(bundle: &Path) -> Option<String> {
    let bytes = read_capped(&bundle.join("Contents/Info.plist"), PLIST_CAP, LinkPolicy::Follow).ok()?;
    let info = plist::Value::from_reader(Cursor::new(bytes)).ok()?;
    let version = info.as_dictionary()?.get("CFBundleShortVersionString")?.as_string()?;
    Some(version.to_owned())
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use super::{Launched, Replaced};
    use crate::update::test_support::info_plist;

    /// `<dir>/Wormhole.app` with an executable holding `body` and an Info.plist of `version`.
    fn bundle(dir: &Path, body: &str, version: &str) -> PathBuf {
        let app = dir.join("Wormhole.app");
        fs::create_dir_all(app.join("Contents/MacOS")).expect("dirs");
        fs::write(app.join("Contents/MacOS/Wormhole"), body).expect("exe");
        fs::write(app.join("Contents/Info.plist"), info_plist("app.getwormhole.terminal", version))
            .expect("plist");
        app
    }

    fn launched(app: &Path, running: &str) -> Launched {
        Launched::of(&exe_of(app), running).expect("a bundle")
    }

    fn exe_of(app: &Path) -> PathBuf {
        app.join("Contents/MacOS/Wormhole")
    }

    #[test]
    fn an_untouched_bundle_is_not_replaced() {
        let dir = tempfile::tempdir().expect("dir");
        let app = bundle(dir.path(), "one", "1.0.0");
        let launched = launched(&app, "1.0.0");
        assert_eq!(launched.replaced(), None);
    }

    #[test]
    fn a_bundle_swapped_for_another_is_replaced_and_names_its_version() {
        let dir = tempfile::tempdir().expect("dir");
        let app = bundle(dir.path(), "one", "1.0.0");
        let launched = launched(&app, "1.0.0");
        // What Finder, Homebrew and the in-app install do: the old bundle goes, a new one is there.
        fs::rename(&app, dir.path().join("old.app")).expect("moved aside");
        bundle(dir.path(), "two", "1.1.0");
        fs::remove_dir_all(dir.path().join("old.app")).expect("removed");
        assert_eq!(launched.replaced(), Some(Replaced { version: "1.1.0".into() }));
    }

    #[test]
    fn an_executable_overwritten_in_place_is_replaced_too() {
        let dir = tempfile::tempdir().expect("dir");
        let app = bundle(dir.path(), "one", "1.0.0");
        let launched = launched(&app, "1.0.0");
        // Same inode, other size.
        fs::write(exe_of(&app), "a longer executable").expect("overwritten");
        fs::write(app.join("Contents/Info.plist"), info_plist("app.getwormhole.terminal", "1.0.1"))
            .expect("plist");
        assert!(launched.replaced().is_some());
    }

    #[test]
    fn an_overwrite_of_the_same_size_a_moment_later_is_seen_by_its_time() {
        let dir = tempfile::tempdir().expect("dir");
        let app = bundle(dir.path(), "one", "1.0.0");
        let launched = launched(&app, "1.0.0");
        std::thread::sleep(Duration::from_millis(20));
        fs::write(exe_of(&app), "two").expect("overwritten");
        fs::write(app.join("Contents/Info.plist"), info_plist("app.getwormhole.terminal", "1.0.1"))
            .expect("plist");
        assert!(launched.replaced().is_some());
    }

    #[test]
    fn a_bundle_that_is_gone_for_the_moment_is_not_replaced_yet() {
        let dir = tempfile::tempdir().expect("dir");
        let app = bundle(dir.path(), "one", "1.0.0");
        let launched = launched(&app, "1.0.0");
        fs::remove_dir_all(&app).expect("trashed");
        assert_eq!(launched.replaced(), None);
    }

    #[test]
    fn the_same_version_copied_again_is_not_reported() {
        let dir = tempfile::tempdir().expect("dir");
        let app = bundle(dir.path(), "one", "1.0.0");
        let launched = launched(&app, "1.0.0");
        // `brew reinstall`: another file, the same version.
        fs::rename(&app, dir.path().join("old.app")).expect("moved aside");
        bundle(dir.path(), "one again", "1.0.0");
        fs::remove_dir_all(dir.path().join("old.app")).expect("removed");
        assert_eq!(launched.replaced(), None);
    }

    #[test]
    fn a_new_bundle_whose_version_cannot_be_read_is_not_reported() {
        let dir = tempfile::tempdir().expect("dir");
        let app = bundle(dir.path(), "one", "1.0.0");
        let launched = launched(&app, "1.0.0");
        fs::write(exe_of(&app), "longer").expect("overwritten");
        fs::write(app.join("Contents/Info.plist"), "not a plist").expect("broken");
        assert_eq!(launched.replaced(), None, "no guess at a version");
    }

    #[test]
    fn what_is_not_a_bundle_or_is_translocated_is_not_watched() {
        let dir = tempfile::tempdir().expect("dir");
        let loose = dir.path().join("target/debug/Wormhole");
        fs::create_dir_all(loose.parent().expect("parent")).expect("dirs");
        fs::write(&loose, "dev build").expect("exe");
        assert!(Launched::of(&loose, "1.0.0").is_none(), "cargo run");

        let moved = dir.path().join("AppTranslocation/uuid/d");
        fs::create_dir_all(&moved).expect("dirs");
        let app = bundle(&moved, "one", "1.0.0");
        assert!(Launched::of(&exe_of(&app), "1.0.0").is_none(), "a mounted disk image");
    }
}
