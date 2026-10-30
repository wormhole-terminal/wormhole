//! Renaming a file or folder without replacing anything, inside a folder that was opened once: the one
//! thing the Files tab's Rename needs from the system, because `std::fs::rename` silently overwrites a
//! target that exists and walks both paths again on every call.
//!
//! The folder is opened with a descriptor (`O_DIRECTORY | O_NOFOLLOW`: a link at its last step is
//! refused) and everything after is by name inside that descriptor, so a folder swapped for a link
//! after it was checked is not followed. Nothing is ever replaced: the system call fails when the
//! target exists, and a rename that only changes case goes through a temporary name with two such calls.

use std::ffi::{OsStr, OsString};
use std::io;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[cfg(target_os = "linux")]
use super::linux::rename as os;
#[cfg(target_os = "macos")]
use super::macos::rename as os;

/// A folder, held open.
pub struct Folder(os::Folder);

impl Folder {
    /// Opens the folder at `path`; a link at its last step, or a path that is no folder, is an error.
    pub fn open(path: &Path) -> io::Result<Self> {
        os::Folder::open(path).map(Self)
    }

    /// Where the open folder really is, as the system says: what a check of "inside the root" must be made
    /// of, since the path it was opened by may have been swapped.
    pub fn real_path(&self) -> io::Result<PathBuf> {
        self.0.real_path()
    }

    /// Whether `name` is in the folder (a link counts as itself, whatever it points at).
    pub fn has(&self, name: &OsStr) -> io::Result<bool> {
        self.0.has(name)
    }

    /// Moves `from` to `to`, both names in this folder, unless something is called `to`, which is an
    /// [`io::ErrorKind::AlreadyExists`] error. One step for the system: a file that appears at `to` after
    /// a look is not overwritten either. A link is renamed as the link it is.
    ///
    /// It also fails with `AlreadyExists` for a rename that only changes case on a volume that ignores
    /// it, where `to` is `from` itself: [`Self::rename_case`] is for that.
    pub fn rename_no_replace(&self, from: &OsStr, to: &OsStr) -> io::Result<()> {
        self.0.rename_no_replace(from, to)
    }

    /// Like [`Self::rename_no_replace`], but `to` is a name in `dest`, another folder: how something made
    /// elsewhere (on the same volume; otherwise the error is `EXDEV`) is put in place without ever
    /// replacing what is there.
    pub fn move_no_replace_into(&self, from: &OsStr, dest: &Folder, to: &OsStr) -> io::Result<()> {
        self.0.move_no_replace_into(from, &dest.0, to)
    }

    /// [`Self::rename_no_replace`] for names that differ only in case, which may be one entry on a volume that
    /// ignores case: `from` goes to a name nothing has, then to `to`, each step refusing to replace. If
    /// something else is called `to` (a volume that tells case apart) `from` is put back and the error is
    /// `AlreadyExists`. Nothing is ever replaced.
    pub fn rename_case(&self, from: &OsStr, to: &OsStr) -> io::Result<()> {
        let temporary = self.unused_name()?;
        self.rename_no_replace(from, &temporary)?;
        match self.rename_no_replace(&temporary, to) {
            Ok(()) => Ok(()),
            Err(err) => {
                // Put it back; if even that fails the file is left under its temporary name, and the
                // error says so.
                match self.rename_no_replace(&temporary, from) {
                    Ok(()) => Err(err),
                    Err(back) => Err(io::Error::other(format!(
                        "could not rename, and {} is left as {}: {back}",
                        from.to_string_lossy(),
                        temporary.to_string_lossy()
                    ))),
                }
            }
        }
    }

    /// A name nothing in the folder has, for the middle step of [`Self::rename_case`].
    fn unused_name(&self) -> io::Result<OsString> {
        let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_nanos());
        for attempt in 0..8u32 {
            let name = OsString::from(format!(".wormhole-rename-{}-{nanos}-{attempt}", std::process::id()));
            if !self.has(&name)? {
                return Ok(name);
            }
        }
        Err(io::Error::new(io::ErrorKind::AlreadyExists, "no unused temporary name"))
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::fs;

    use super::Folder;

    fn names(dir: &std::path::Path) -> Vec<String> {
        let mut found: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        found.sort();
        found
    }

    #[test]
    fn a_case_only_rename_goes_by_a_temporary_name_and_leaves_none_behind() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("readme.md"), "r").unwrap();
        let folder = Folder::open(dir.path()).unwrap();
        folder.rename_case(OsStr::new("readme.md"), OsStr::new("README.md")).unwrap();
        assert_eq!(names(dir.path()), ["README.md"]);
        assert_eq!(fs::read_to_string(dir.path().join("README.md")).unwrap(), "r");
    }

    #[test]
    fn a_case_rename_onto_another_entry_puts_the_file_back_and_replaces_nothing() {
        // On a volume that tells case apart `a` and `A` are two files; on one that does not, `a` and `b`
        // stand in for them: either way the target is another entry.
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a"), "a").unwrap();
        fs::write(dir.path().join("b"), "b").unwrap();
        let folder = Folder::open(dir.path()).unwrap();
        let err = folder.rename_case(OsStr::new("a"), OsStr::new("b")).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(names(dir.path()), ["a", "b"], "put back, no temporary name left");
        assert_eq!(fs::read_to_string(dir.path().join("a")).unwrap(), "a");
        assert_eq!(fs::read_to_string(dir.path().join("b")).unwrap(), "b");
    }

    #[test]
    fn the_exclusive_rename_refuses_a_target_and_a_link_at_the_folder_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a"), "a").unwrap();
        fs::write(dir.path().join("b"), "b").unwrap();
        let folder = Folder::open(dir.path()).unwrap();
        assert_eq!(
            folder.rename_no_replace(OsStr::new("a"), OsStr::new("b")).unwrap_err().kind(),
            std::io::ErrorKind::AlreadyExists
        );
        assert!(folder.has(OsStr::new("a")).unwrap() && !folder.has(OsStr::new("c")).unwrap());
        assert_eq!(folder.real_path().unwrap(), fs::canonicalize(dir.path()).unwrap());
        // Opened by a link: refused, not followed.
        let link = dir.path().join("link");
        crate::platform::fs::symlink(dir.path(), &link).unwrap();
        assert!(Folder::open(&link).is_err());
        assert!(Folder::open(&dir.path().join("a")).is_err(), "a file is no folder");
    }

    #[test]
    fn a_move_into_another_folder_refuses_a_target_and_replaces_nothing() {
        let (one, two) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
        fs::create_dir(one.path().join("made")).unwrap();
        fs::write(one.path().join("made/file"), "new").unwrap();
        fs::create_dir(two.path().join("there")).unwrap();
        fs::write(two.path().join("there/keep"), "mine").unwrap();
        let (from, to) = (Folder::open(one.path()).unwrap(), Folder::open(two.path()).unwrap());
        let err = from.move_no_replace_into(OsStr::new("made"), &to, OsStr::new("there")).unwrap_err();
        assert_eq!(err.kind(), std::io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read_to_string(two.path().join("there/keep")).unwrap(), "mine");
        assert!(!two.path().join("there/made").exists(), "not nested into the existing folder");
        from.move_no_replace_into(OsStr::new("made"), &to, OsStr::new("here")).unwrap();
        assert_eq!(fs::read_to_string(two.path().join("here/file")).unwrap(), "new");
        assert!(!one.path().join("made").exists());
    }
}
