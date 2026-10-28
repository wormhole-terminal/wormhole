//! The subfolders of a folder, read for a list a person picks from (the folder picker at the prompt).
//!
//! A folder somewhere on the disk is not ours: it may hold millions of entries, sit on a share that answers
//! slowly or hold names no shell could be handed. So the read is bounded in what it keeps and in what it
//! looks at, skips what it can't read, and says when it stopped early.

use std::fs;
use std::io;
use std::path::Path;

/// The most subfolders one read keeps.
pub const FOLDERS_CAP: usize = 2_000;
/// The most entries one read looks at, folders or not: a folder of a million files must not hold the
/// reader up to find its three subfolders.
pub const SCAN_CAP: usize = 50_000;

/// What [`list_folders`] found, in the order the disk gave it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Folders {
    /// The names of the subfolders. A link to a folder is one, as in `ls` and `cd`; the link is not followed
    /// further than one `stat`, and nothing is listed below it.
    pub names: Vec<String>,
    /// The read stopped at a cap, so there are more than `names` has.
    pub cut: bool,
}

/// The subfolders of `dir`, at most `cap` of them and out of at most [`SCAN_CAP`] entries. An entry that can't
/// be read, a link that points nowhere and a name that isn't UTF-8 (it could not be typed back as text) are
/// left out. An error is only for the folder itself: it is missing, not a folder, or not readable.
pub fn list_folders(dir: &Path, cap: usize) -> io::Result<Folders> {
    let mut found = Folders::default();
    for (seen, entry) in fs::read_dir(dir)?.enumerate() {
        if seen >= SCAN_CAP {
            found.cut = true;
            break;
        }
        let Ok(entry) = entry else { continue };
        let Ok(kind) = entry.file_type() else { continue };
        let is_folder = if kind.is_symlink() {
            fs::metadata(entry.path()).is_ok_and(|m| m.is_dir())
        } else {
            kind.is_dir()
        };
        if !is_folder {
            continue;
        }
        let Ok(name) = entry.file_name().into_string() else { continue };
        if found.names.len() >= cap {
            found.cut = true;
            break;
        }
        found.names.push(name);
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{Folders, list_folders};

    fn sorted(found: Folders) -> Vec<String> {
        let mut names = found.names;
        names.sort();
        names
    }

    #[test]
    fn it_lists_folders_and_leaves_files_out() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("src")).unwrap();
        fs::create_dir(dir.path().join(".git")).unwrap();
        fs::create_dir(dir.path().join("My Folder")).unwrap();
        fs::write(dir.path().join("README.md"), "x").unwrap();
        let found = list_folders(dir.path(), 100).unwrap();
        assert!(!found.cut);
        assert_eq!(sorted(found), [".git", "My Folder", "src"]);
    }

    #[test]
    fn a_link_to_a_folder_is_a_folder_and_a_link_to_a_file_or_nowhere_is_not() {
        use crate::platform::fs::symlink;
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("real")).unwrap();
        fs::write(dir.path().join("file"), "x").unwrap();
        symlink(dir.path().join("real"), dir.path().join("to-folder")).unwrap();
        symlink(dir.path().join("file"), dir.path().join("to-file")).unwrap();
        symlink(dir.path().join("gone"), dir.path().join("to-nowhere")).unwrap();
        assert_eq!(sorted(list_folders(dir.path(), 100).unwrap()), ["real", "to-folder"]);
    }

    #[test]
    fn it_stops_at_the_cap_and_says_so() {
        let dir = tempfile::tempdir().unwrap();
        for n in 0..5 {
            fs::create_dir(dir.path().join(format!("d{n}"))).unwrap();
        }
        let found = list_folders(dir.path(), 3).unwrap();
        assert_eq!(found.names.len(), 3);
        assert!(found.cut);
        let all = list_folders(dir.path(), 5).unwrap();
        assert_eq!(all.names.len(), 5);
        assert!(!all.cut, "a folder with exactly the cap's worth is not cut");
    }

    #[test]
    fn a_missing_folder_or_a_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(list_folders(&dir.path().join("nope"), 10).is_err());
        let file = dir.path().join("f");
        fs::write(&file, "x").unwrap();
        assert!(list_folders(&file, 10).is_err());
    }

    #[test]
    fn a_folder_that_cant_be_read_is_still_listed_beside_its_neighbours() {
        use crate::platform::fs::permissions;
        let dir = tempfile::tempdir().unwrap();
        let locked = dir.path().join("locked");
        fs::create_dir(&locked).unwrap();
        fs::create_dir(dir.path().join("open")).unwrap();
        fs::set_permissions(&locked, permissions(0o000)).unwrap();
        let beside = sorted(list_folders(dir.path(), 10).unwrap());
        fs::set_permissions(&locked, permissions(0o700)).unwrap();
        assert_eq!(beside, ["locked", "open"], "listing the parent doesn't need to read the child");
    }
}
