//! Owner-only files and folders for Wormhole's own data.
//!
//! The data folder holds the terminals' saved text, profiles' environments and the logs, so nobody
//! but its owner may read it: folders `0700`, files `0600` (`0700` for a program, `bin/wh`). Not
//! left to the umask or to `~/Library` being private: the folder can be elsewhere
//! (`WORMHOLE_DATA_DIR`), copied, or loosened. The process umask itself is never changed, since the
//! shells Wormhole starts inherit it.

use std::fs::{self, OpenOptions};
use std::io;
use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt, PermissionsExt};
use std::path::Path;

/// A folder's mode.
pub const PRIVATE_DIR: u32 = 0o700;
/// A new file's mode.
pub const PRIVATE_FILE: u32 = 0o600;

/// How deep, and how many entries, [`make_tree_private`] goes: the data folder is a few levels of
/// a few hundred files; this only bounds a pile someone dropped in it.
const MAX_DEPTH: usize = 8;
const MAX_ENTRIES: usize = 50_000;

/// Create `dir` (and missing parents) owner-only. Folders that already exist are left as they are:
/// this is for paths that may lead through the user's own folders (`~/.codex/…`).
pub fn create_dir_all_private(dir: &Path) -> io::Result<()> {
    fs::DirBuilder::new().recursive(true).mode(PRIVATE_DIR).create(dir)
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
    if meta.uid() != current_uid() {
        return Err(io::Error::new(
            io::ErrorKind::PermissionDenied,
            format!("{} belongs to another user", dir.display()),
        ));
    }
    if meta.mode() & 0o777 != PRIVATE_DIR {
        fs::set_permissions(dir, fs::Permissions::from_mode(PRIVATE_DIR))?;
    }
    Ok(())
}

/// Create `path` empty and owner-only if it is missing (its contents are never touched), or take
/// group and other access off it if it exists: for a file another library then opens and writes
/// (SQLite gives a database's `-wal` and `-shm` the database file's mode).
pub fn create_private_file(path: &Path) -> io::Result<()> {
    match OpenOptions::new().write(true).create_new(true).mode(PRIVATE_FILE).open(path) {
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
    if meta.is_file() && meta.uid() == current_uid() {
        tighten(path, meta.mode())?;
    }
    Ok(())
}

/// Every folder and regular file under `root` (and `root` itself) owner-only: what launch does
/// for a data folder an earlier version made with the umask's modes. Links are never followed or
/// changed, sockets and other special files are left as they are, and so is anything another user
/// owns. Returns how many were changed. `root` must be a real folder, not a link.
pub fn make_tree_private(root: &Path) -> io::Result<usize> {
    ensure_private_dir(root)?;
    let mut changed = 0;
    let mut budget = MAX_ENTRIES;
    walk(root, 0, &mut budget, &mut changed)?;
    Ok(changed)
}

fn walk(dir: &Path, depth: usize, budget: &mut usize, changed: &mut usize) -> io::Result<()> {
    if depth > MAX_DEPTH {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        if *budget == 0 {
            return Ok(());
        }
        *budget -= 1;
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        // `symlink_metadata`: a link is seen as a link, so it is never followed.
        let Ok(meta) = fs::symlink_metadata(&path) else { continue };
        if meta.uid() != current_uid() {
            continue;
        }
        if meta.is_dir() {
            if meta.mode() & 0o777 != PRIVATE_DIR {
                fs::set_permissions(&path, fs::Permissions::from_mode(PRIVATE_DIR))?;
                *changed += 1;
            }
            walk(&path, depth + 1, budget, changed)?;
        } else if meta.is_file() && tighten(&path, meta.mode())? {
            *changed += 1;
        }
    }
    Ok(())
}

/// Whether the mode had to change.
fn tighten(path: &Path, mode: u32) -> io::Result<bool> {
    if mode & 0o077 == 0 {
        return Ok(false);
    }
    fs::set_permissions(path, fs::Permissions::from_mode(mode & 0o700))?;
    Ok(true)
}

fn current_uid() -> u32 {
    // SAFETY: getuid has no preconditions and cannot fail.
    #[allow(unsafe_code)]
    unsafe {
        libc::getuid()
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::Path;

    use super::{create_private_file, ensure_private_dir, make_tree_private};

    fn mode(path: &Path) -> u32 {
        fs::symlink_metadata(path).expect("exists").permissions().mode() & 0o777
    }

    #[test]
    fn a_loose_tree_is_made_owner_only_without_following_links() {
        let root = tempfile::tempdir().expect("temp dir");
        let elsewhere = tempfile::tempdir().expect("temp dir");
        let data = root.path().join("data");
        fs::create_dir_all(data.join("logs")).expect("dirs");
        fs::set_permissions(&data, fs::Permissions::from_mode(0o755)).expect("chmod");
        fs::set_permissions(data.join("logs"), fs::Permissions::from_mode(0o775)).expect("chmod");
        fs::write(data.join("config.json"), "{}").expect("writes");
        fs::set_permissions(data.join("config.json"), fs::Permissions::from_mode(0o644)).expect("chmod");
        fs::write(data.join("tool"), "#!/bin/sh").expect("writes");
        fs::set_permissions(data.join("tool"), fs::Permissions::from_mode(0o755)).expect("chmod");
        let outside = elsewhere.path().join("theirs.txt");
        fs::write(&outside, "x").expect("writes");
        fs::set_permissions(&outside, fs::Permissions::from_mode(0o644)).expect("chmod");
        std::os::unix::fs::symlink(&outside, data.join("link")).expect("links");

        assert_eq!(
            make_tree_private(&data).expect("walks"),
            3,
            "logs, config.json, tool (the root on its own)"
        );
        assert_eq!(mode(&data), 0o700);
        assert_eq!(mode(&data.join("logs")), 0o700);
        assert_eq!(mode(&data.join("config.json")), 0o600);
        assert_eq!(mode(&data.join("tool")), 0o700);
        assert_eq!(mode(&outside), 0o644, "a link's target is not ours to change");
        assert_eq!(make_tree_private(&data).expect("walks"), 0, "nothing left to do");
    }

    #[test]
    fn a_link_in_place_of_a_private_folder_is_refused() {
        let root = tempfile::tempdir().expect("temp dir");
        let elsewhere = tempfile::tempdir().expect("temp dir");
        fs::set_permissions(elsewhere.path(), fs::Permissions::from_mode(0o755)).expect("chmod");
        let link = root.path().join("data");
        std::os::unix::fs::symlink(elsewhere.path(), &link).expect("links");
        assert!(ensure_private_dir(&link).is_err());
        assert!(make_tree_private(&link).is_err());
        assert_eq!(mode(elsewhere.path()), 0o755);
    }

    #[test]
    fn a_private_file_is_made_once_and_its_content_kept() {
        let dir = tempfile::tempdir().expect("temp dir");
        let file = dir.path().join("wormhole.db");
        create_private_file(&file).expect("creates");
        assert_eq!(mode(&file), 0o600);
        fs::write(&file, "data").expect("writes");
        fs::set_permissions(&file, fs::Permissions::from_mode(0o644)).expect("chmod");
        create_private_file(&file).expect("tightens");
        assert_eq!(mode(&file), 0o600);
        assert_eq!(fs::read_to_string(&file).expect("reads"), "data");
    }
}
