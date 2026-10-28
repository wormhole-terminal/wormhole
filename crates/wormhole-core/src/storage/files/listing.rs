//! Listing a folder for the Files tab: what is in it and of what kind, without opening any of it.
//!
//! A folder's content is hostile input (a cloned repository chooses its names and plants links), and
//! a folder can be enormous or sit on a network volume that answers slowly, so:
//! - a name stays an [`OsString`] (a name that is not UTF-8 is still a name);
//! - an entry is told apart by its own type (`lstat`, which `readdir` gives for free): a link is a
//!   [`EntryKind::Link`] and what it points at is never looked at, so a link to `/net/host/share` can't
//!   make the system mount it just because the tab is open; [`list_below`] refuses to walk through one, so
//!   a tree built from it never leaves its root by a link;
//! - the number of entries read is bounded ([`HARD_LIMIT`]): reading stops there and says so.
//!
//! It blocks on the disk: call it off the main thread.

use std::ffi::{OsStr, OsString};
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use super::FileStamp;

/// The most entries read from one folder, however many it has: the memory a folder of millions of
/// files can cost is bounded, and the tab shows a note that there is more.
pub const HARD_LIMIT: usize = 200_000;

/// What an entry is, by its own type.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind {
    /// A real folder.
    Folder,
    /// A file, and anything else that is not a folder or a link (a socket, a device).
    File,
    /// A symbolic link, whatever it points at (it is never looked at).
    Link,
}

/// One entry of a folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirEntry {
    pub name: OsString,
    pub kind: EntryKind,
}

/// What a folder holds, in the order the disk gave it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Listing {
    pub entries: Vec<DirEntry>,
    /// The folder has more than [`HARD_LIMIT`] entries and the rest were not read.
    pub truncated: bool,
}

/// The real folder `relative` names below `root` (`""` is `root` itself). Every folder on the way below
/// `root` must be a real folder: a link in the path, or a path that is not plain, is an error, so the
/// answer is always inside `root`.
fn walk_below(root: &Path, relative: &Path) -> io::Result<PathBuf> {
    let mut at = root.to_path_buf();
    for part in relative.components() {
        let Component::Normal(name) = part else {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "not a plain relative path"));
        };
        at.push(name);
        if !fs::symlink_metadata(&at)?.is_dir() {
            return Err(io::Error::new(io::ErrorKind::NotADirectory, "not a folder, or a link to one"));
        }
    }
    Ok(at)
}

/// The entries of the folder `relative` names below `root` (`""` is `root` itself), as
/// [`walk_below`] allows.
pub fn list_below(root: &Path, relative: &Path) -> io::Result<Listing> {
    list(&walk_below(root, relative)?)
}

/// What the folder `relative` names below `root` looks like from outside (one `lstat`): the same until an
/// entry is added, removed or renamed in it, so a folder whose stamp is as it was need not be listed
/// again. Take it before listing, so a change during the read shows up as one next time.
pub fn stamp_below(root: &Path, relative: &Path) -> io::Result<FileStamp> {
    let at = walk_below(root, relative)?;
    // The root is the person's own folder, which may be reached through a link (a terminal in a symlinked
    // project): then what it points at is what changes. Below it nothing is a link.
    let stamp = if relative.as_os_str().is_empty() { FileStamp::of(&at) } else { FileStamp::of_link(&at) };
    stamp.ok_or_else(|| io::Error::from(io::ErrorKind::NotFound))
}

/// The entries of the folder `dir`, at most [`HARD_LIMIT`] of them.
pub fn list(dir: &Path) -> io::Result<Listing> {
    let scanned = scan(dir, Limits { keep: HARD_LIMIT, scan: usize::MAX }, |_, _| true)?;
    Ok(Listing { truncated: scanned.cut.is_some(), ..scanned.listing })
}

/// How much one [`scan`] may keep and look at.
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    /// The most entries kept.
    pub keep: usize,
    /// The most entries looked at, kept or not: a folder of a million files must not hold a read up that only
    /// wants its few folders.
    pub scan: usize,
}

/// Why a scan stopped before it had seen everything.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cut {
    /// It kept as many as it was allowed to and there were more.
    Kept,
    /// It looked at as many entries as it was allowed to and there were more, so some it would have kept
    /// may be missing even when few were kept.
    Scanned,
}

/// What [`scan`] found.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Scan {
    pub listing: Listing,
    pub cut: Option<Cut>,
}

/// The entries of `dir` that `keep` accepts, within `limits`: the one reader of a folder. `keep` sees each
/// entry's name and its own type (never what a link points at: it may stat that itself, for the few entries
/// it asks about) and is applied while the folder is read, so the cap counts what is kept and a folder far
/// bigger than the caps can still be searched. An entry that can't be read is left out. An error is only for
/// the folder itself: it is missing, not a folder, or not readable.
pub fn scan(dir: &Path, limits: Limits, mut keep: impl FnMut(&OsStr, EntryKind) -> bool) -> io::Result<Scan> {
    let mut found = Scan::default();
    for (seen, entry) in fs::read_dir(dir)?.enumerate() {
        if seen >= limits.scan {
            found.cut = Some(Cut::Scanned);
            break;
        }
        // An entry that vanishes or can't be read while the folder is read is left out.
        let Ok(entry) = entry else { continue };
        let Ok(kind) = entry.file_type() else { continue };
        let kind = if kind.is_dir() {
            EntryKind::Folder
        } else if kind.is_symlink() {
            EntryKind::Link
        } else {
            EntryKind::File
        };
        let name = entry.file_name();
        if !keep(&name, kind) {
            continue;
        }
        if found.listing.entries.len() >= limits.keep {
            found.cut = Some(Cut::Kept);
            break;
        }
        found.listing.entries.push(DirEntry { name, kind });
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::{Cut, EntryKind, Limits, list, list_below, scan, stamp_below};
    use crate::platform::fs::symlink;

    fn kinds(dir: &Path, relative: &str) -> Vec<(String, EntryKind)> {
        let mut found: Vec<_> = list_below(dir, Path::new(relative))
            .unwrap()
            .entries
            .into_iter()
            .map(|entry| (entry.name.to_string_lossy().into_owned(), entry.kind))
            .collect();
        found.sort_by(|a, b| a.0.cmp(&b.0));
        found
    }

    #[test]
    fn folders_files_and_links_are_told_apart_by_their_own_type() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("src")).unwrap();
        fs::write(root.path().join("a.txt"), b"x").unwrap();
        symlink(outside.path(), root.path().join("out")).unwrap();
        symlink(root.path().join("a.txt"), root.path().join("alias")).unwrap();
        symlink(root.path().join("nowhere"), root.path().join("broken")).unwrap();
        assert_eq!(
            kinds(root.path(), ""),
            [
                ("a.txt".to_string(), EntryKind::File),
                ("alias".to_string(), EntryKind::Link),
                ("broken".to_string(), EntryKind::Link),
                ("out".to_string(), EntryKind::Link),
                ("src".to_string(), EntryKind::Folder),
            ]
        );
    }

    #[test]
    fn a_link_in_the_path_is_not_walked() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("secret"), b"x").unwrap();
        symlink(outside.path(), root.path().join("out")).unwrap();
        assert!(list_below(root.path(), Path::new("out")).is_err());
        fs::create_dir(root.path().join("real")).unwrap();
        symlink(outside.path(), root.path().join("real").join("hop")).unwrap();
        assert!(list_below(root.path(), Path::new("real/hop")).is_err());
        assert!(list_below(root.path(), Path::new("real")).is_ok());
    }

    #[test]
    fn a_path_that_is_not_plain_or_not_a_folder_is_refused() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("file"), b"x").unwrap();
        for relative in ["../x", "/etc", "file", "missing"] {
            assert!(list_below(root.path(), Path::new(relative)).is_err(), "{relative}");
        }
        assert!(list(&root.path().join("file")).is_err());
    }

    #[test]
    fn a_stamp_changes_when_an_entry_is_added_and_not_otherwise() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("src")).unwrap();
        let first = stamp_below(root.path(), Path::new("src")).unwrap();
        assert_eq!(stamp_below(root.path(), Path::new("src")).unwrap(), first);
        // Rewriting a file's content does not touch the folder.
        fs::write(root.path().join("src/a"), b"1").unwrap();
        let second = stamp_below(root.path(), Path::new("src")).unwrap();
        assert_ne!(second, first, "an entry was added");
        fs::write(root.path().join("src/a"), b"22").unwrap();
        assert_eq!(stamp_below(root.path(), Path::new("src")).unwrap(), second);
        assert!(stamp_below(root.path(), Path::new("missing")).is_err());
    }

    #[test]
    fn a_scan_keeps_what_it_accepts_and_says_when_it_stopped() {
        let root = tempfile::tempdir().unwrap();
        for n in 0..6 {
            fs::create_dir(root.path().join(format!("d{n}"))).unwrap();
        }
        fs::write(root.path().join("file"), b"x").unwrap();
        let folders = |kind: EntryKind| kind == EntryKind::Folder;
        let all = scan(root.path(), Limits { keep: 10, scan: 100 }, |_, k| folders(k)).unwrap();
        assert_eq!((all.listing.entries.len(), all.cut), (6, None));
        let few = scan(root.path(), Limits { keep: 3, scan: 100 }, |_, k| folders(k)).unwrap();
        assert_eq!((few.listing.entries.len(), few.cut), (3, Some(Cut::Kept)));
        let short = scan(root.path(), Limits { keep: 10, scan: 2 }, |_, _| true).unwrap();
        assert_eq!((short.listing.entries.len(), short.cut), (2, Some(Cut::Scanned)));
        let exact = scan(root.path(), Limits { keep: 6, scan: 100 }, |_, k| folders(k)).unwrap();
        assert_eq!(exact.cut, None, "exactly the cap's worth is not cut");
    }
}
