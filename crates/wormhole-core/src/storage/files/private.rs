//! Owner-only files and folders for Wormhole's own data.
//!
//! The data folder holds the terminals' saved text, profiles' environments and the logs, so nobody
//! but its owner may read it: folders `0700`, files `0600` (`0700` for a program, `bin/wh`). Not
//! left to the umask or to `~/Library` being private: the folder can be elsewhere
//! (`WORMHOLE_DATA_DIR`), copied, or loosened. The process umask itself is never changed, since the
//! shells Wormhole starts inherit it.

use std::ffi::OsStr;
use std::fs::{self, OpenOptions};
use std::io;
use std::path::Path;

use crate::platform::fs::{
    self as platform_fs, mode, no_block_no_follow, permissions, private_dir, private_file, same_file,
};
use crate::platform::user::UserId;

/// A folder's mode.
pub(super) const PRIVATE_DIR: u32 = 0o700;
/// A new file's mode.
pub const PRIVATE_FILE: u32 = 0o600;

/// How deep, and how many entries, [`make_tree_private`] goes: the data folder is a few levels of
/// a few hundred files; this only bounds a pile someone dropped in it.
const MAX_DEPTH: usize = 8;
const MAX_ENTRIES: usize = 50_000;

/// Create `dir` (and missing parents) owner-only. Folders that already exist are left as they are:
/// this is for paths that may lead through the user's own folders (`~/.codex/…`).
pub fn create_dir_all_private(dir: &Path) -> io::Result<()> {
    private_dir(fs::DirBuilder::new().recursive(true)).create(dir)
}

/// `dir`, created owner-only when missing; one that exists must be a real folder (not a link) of
/// this user and has its mode set back to `0700` when something loosened it. For Wormhole's own
/// folders only (the data folder, a profile's, the logs).
pub fn ensure_private_dir(dir: &Path) -> io::Result<()> {
    create_dir_all_private(dir)?;
    // Not `metadata`: a link planted at the folder's name must not be followed.
    let meta = fs::symlink_metadata(dir)?;
    if !meta.is_dir() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{} is not a folder", dir.display()),
        ));
    }
    if UserId::owner_of(&meta) != UserId::current() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("{} belongs to another user", dir.display()),
        ));
    }
    if mode(&meta) & 0o777 != PRIVATE_DIR {
        platform_fs::set_mode(dir, PRIVATE_DIR)?;
    }
    Ok(())
}

/// Create `path` empty and owner-only if it is missing (its contents are never touched), or take
/// group and other access off it if it exists: for a file another library then opens and writes
/// (SQLite gives a database's `-wal` and `-shm` the database file's mode).
pub fn create_private_file(path: &Path) -> io::Result<()> {
    match private_file(OpenOptions::new().write(true).create_new(true)).open(path) {
        Ok(_) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::AlreadyExists => make_private(path),
        Err(err) => Err(err),
    }
}

/// Take group and other access off an existing regular file of this user (`0644` → `0600`, `0755`
/// → `0700`). A missing file, a link, or anything else is left alone.
pub fn make_private(path: &Path) -> io::Result<()> {
    let meta = match fs::symlink_metadata(path) {
        Ok(meta) => meta,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(err) => return Err(err),
    };
    if meta.is_file() && UserId::owner_of(&meta) == UserId::current() && mode(&meta) & 0o077 != 0 {
        tighten_nofollow(path, PRIVATE_DIR)?;
    }
    Ok(())
}

/// Every folder and regular file under `root` (and `root` itself) owner-only: what launch does
/// for a data folder an earlier version made with the umask's modes. At the top only the entries
/// `ours` accepts are looked at (below them, everything): the data folder may be one a person
/// pointed somewhere with other files in it. Links are never followed or changed, sockets and
/// other special files are left as they are, and so is anything another user owns; an entry that
/// can't be read or changed is skipped. Returns how many were changed. `root` must be a real
/// folder, not a link.
pub fn make_tree_private(root: &Path, ours: &dyn Fn(&OsStr) -> bool) -> io::Result<usize> {
    ensure_private_dir(root)?;
    let mut walk = Walk { changed: 0, budget: MAX_ENTRIES };
    walk.dir(root, 0, Some(ours));
    Ok(walk.changed)
}

struct Walk {
    changed: usize,
    budget: usize,
}

impl Walk {
    fn dir(&mut self, dir: &Path, depth: usize, ours: Option<&dyn Fn(&OsStr) -> bool>) {
        if depth > MAX_DEPTH {
            return;
        }
        let Ok(entries) = fs::read_dir(dir) else { return };
        for entry in entries {
            if self.budget == 0 {
                return;
            }
            self.budget -= 1;
            let Ok(entry) = entry else { continue };
            if ours.is_some_and(|ours| !ours(&entry.file_name())) {
                continue;
            }
            let path = entry.path();
            match tighten_nofollow(&path, PRIVATE_DIR) {
                Ok(Tightened { changed, dir: is_dir }) => {
                    self.changed += usize::from(changed);
                    if is_dir {
                        self.dir(&path, depth + 1, None);
                    }
                }
                Err(err) => tracing::debug!("not made owner-only: {}: {err}", path.display()),
            }
        }
    }
}

struct Tightened {
    changed: bool,
    dir: bool,
}

/// Take group and other access off the file or folder at `path` through a descriptor opened
/// without following a link (`O_NOFOLLOW`), so a link swapped in after the folder was listed is
/// refused rather than its target changed. A folder gets `dir_mode`; a file keeps its owner's
/// bits. Anything else, or another user's, is left alone.
fn tighten_nofollow(path: &Path, dir_mode: u32) -> io::Result<Tightened> {
    // (The close-on-exec flag is the standard library's own.)
    let file = no_block_no_follow(OpenOptions::new().read(true)).open(path)?;
    let meta = file.metadata()?;
    let (is_dir, bits) = (meta.is_dir(), mode(&meta) & 0o7777);
    if !(is_dir || meta.is_file()) || UserId::owner_of(&meta) != UserId::current() {
        return Ok(Tightened { changed: false, dir: false });
    }
    let wanted = if is_dir { dir_mode } else { bits & 0o700 };
    let changed = bits != wanted;
    if changed {
        // `fchmod` on the descriptor: the file that was checked is the one changed.
        file.set_permissions(permissions(wanted))?;
    }
    // Recursing goes by path again: only into the folder that was opened, not one swapped in since.
    let still_dir =
        is_dir && fs::symlink_metadata(path).is_ok_and(|now| now.is_dir() && same_file(&now, &meta));
    Ok(Tightened { changed, dir: still_dir })
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::fs;
    use std::path::Path;

    use super::{create_private_file, ensure_private_dir, make_tree_private};
    use crate::platform::fs::{mode as mode_of, permissions, symlink};

    fn mode(path: &Path) -> u32 {
        mode_of(&fs::symlink_metadata(path).expect("exists")) & 0o777
    }

    #[test]
    fn a_loose_tree_is_made_owner_only_without_following_links() {
        let root = tempfile::tempdir().expect("temp dir");
        let elsewhere = tempfile::tempdir().expect("temp dir");
        let data = root.path().join("data");
        fs::create_dir_all(data.join("logs")).expect("dirs");
        fs::set_permissions(&data, permissions(0o755)).expect("chmod");
        fs::set_permissions(data.join("logs"), permissions(0o775)).expect("chmod");
        fs::write(data.join("config.json"), "{}").expect("writes");
        fs::set_permissions(data.join("config.json"), permissions(0o644)).expect("chmod");
        fs::write(data.join("tool"), "#!/bin/sh").expect("writes");
        fs::set_permissions(data.join("tool"), permissions(0o755)).expect("chmod");
        let outside = elsewhere.path().join("theirs.txt");
        fs::write(&outside, "x").expect("writes");
        fs::set_permissions(&outside, permissions(0o644)).expect("chmod");
        symlink(&outside, data.join("link")).expect("links");

        assert_eq!(
            make_tree_private(&data, &|_| true).expect("walks"),
            3,
            "logs, config.json, tool (the root on its own)"
        );
        assert_eq!(mode(&data), 0o700);
        assert_eq!(mode(&data.join("logs")), 0o700);
        assert_eq!(mode(&data.join("config.json")), 0o600);
        assert_eq!(mode(&data.join("tool")), 0o700);
        assert_eq!(mode(&outside), 0o644, "a link's target is not ours to change");
        assert_eq!(make_tree_private(&data, &|_| true).expect("walks"), 0, "nothing left to do");
    }

    #[test]
    fn only_the_names_it_is_given_are_walked_and_a_failing_one_is_skipped() {
        let root = tempfile::tempdir().expect("temp dir");
        let data = root.path().join("data");
        fs::create_dir_all(data.join("logs")).expect("dirs");
        fs::create_dir_all(data.join("someone-elses")).expect("dirs");
        for (name, mode) in [("config.json", 0o644), ("notes.txt", 0o644), ("logs/locked.log", 0o244)] {
            fs::write(data.join(name), "x").expect("writes");
            fs::set_permissions(data.join(name), permissions(mode)).expect("chmod");
        }
        fs::write(data.join("logs/today.log"), "x").expect("writes");
        fs::set_permissions(data.join("logs/today.log"), permissions(0o644)).expect("chmod");
        fs::set_permissions(data.join("someone-elses"), permissions(0o755)).expect("chmod");

        let ours = |name: &OsStr| name == "logs" || name == "config.json";
        make_tree_private(&data, &ours).expect("walks");
        assert_eq!(mode(&data.join("config.json")), 0o600);
        assert_eq!(mode(&data.join("logs")), 0o700);
        assert_eq!(
            mode(&data.join("logs/today.log")),
            0o600,
            "the unreadable file before it didn't stop the walk"
        );
        assert_eq!(mode(&data.join("notes.txt")), 0o644, "not a name of ours");
        assert_eq!(mode(&data.join("someone-elses")), 0o755);
    }

    #[test]
    fn a_link_in_place_of_a_private_folder_is_refused() {
        let root = tempfile::tempdir().expect("temp dir");
        let elsewhere = tempfile::tempdir().expect("temp dir");
        fs::set_permissions(elsewhere.path(), permissions(0o755)).expect("chmod");
        let link = root.path().join("data");
        symlink(elsewhere.path(), &link).expect("links");
        assert!(ensure_private_dir(&link).is_err());
        assert!(make_tree_private(&link, &|_| true).is_err());
        assert_eq!(mode(elsewhere.path()), 0o755);
    }

    #[test]
    fn a_private_file_is_made_once_and_its_content_kept() {
        let dir = tempfile::tempdir().expect("temp dir");
        let file = dir.path().join("wormhole.db");
        create_private_file(&file).expect("creates");
        assert_eq!(mode(&file), 0o600);
        fs::write(&file, "data").expect("writes");
        fs::set_permissions(&file, permissions(0o644)).expect("chmod");
        create_private_file(&file).expect("tightens");
        assert_eq!(mode(&file), 0o600);
        assert_eq!(fs::read_to_string(&file).expect("reads"), "data");
    }
}
