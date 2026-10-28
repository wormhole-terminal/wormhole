//! Listing a folder for the Files tab: what is in it and of what kind, without opening any of it.
//!
//! A folder's content is hostile input (a cloned repository chooses its names and plants links), and
//! a folder can be enormous or sit on a network volume that answers slowly, so:
//! - a name stays an [`OsString`] (a name that is not UTF-8 is still a name);
//! - an entry is told apart by its own type (`lstat`): a link is a [`EntryKind::Link`], never the
//!   folder or file it points to, and [`list_below`] refuses to walk through one, so a tree built from
//!   it never leaves its root by a link;
//! - the number of entries read is bounded ([`HARD_LIMIT`]), and the caller says so when it was reached.
//!
//! It blocks on the disk: call it off the main thread.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Component, Path};

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
    /// A symbolic link; `to_folder` says whether it points at a folder (a broken link does not).
    Link { to_folder: bool },
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
    /// Entries past [`HARD_LIMIT`] that were counted and not kept.
    pub unread: usize,
}

/// The entries of the folder `relative` names below `root` (`""` is `root` itself). Every folder on
/// the way below `root` must be a real folder: a link in the path, or a path that is not plain, is an
/// error, so the answer is always inside `root`.
pub fn list_below(root: &Path, relative: &Path) -> io::Result<Listing> {
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
    list(&at)
}

/// The entries of the folder `dir`.
pub fn list(dir: &Path) -> io::Result<Listing> {
    let mut listing = Listing::default();
    for entry in fs::read_dir(dir)? {
        // An entry that vanishes or can't be read while the folder is read is left out.
        let Ok(entry) = entry else { continue };
        if listing.entries.len() >= HARD_LIMIT {
            listing.unread += 1;
            continue;
        }
        let Ok(kind) = entry.file_type() else { continue };
        let kind = if kind.is_dir() {
            EntryKind::Folder
        } else if kind.is_symlink() {
            EntryKind::Link { to_folder: fs::metadata(entry.path()).is_ok_and(|meta| meta.is_dir()) }
        } else {
            EntryKind::File
        };
        listing.entries.push(DirEntry { name: entry.file_name(), kind });
    }
    Ok(listing)
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::{EntryKind, list, list_below};
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
                ("alias".to_string(), EntryKind::Link { to_folder: false }),
                ("broken".to_string(), EntryKind::Link { to_folder: false }),
                ("out".to_string(), EntryKind::Link { to_folder: true }),
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
}
