//! Cloning a folder from one project checkout into another (copy-on-write, so it shares disk space
//! until files change). The paths come from a repository, so nothing here follows a link out of
//! either checkout: the source must itself be a real folder (not a link), and the folder that is to
//! hold the copy must already be inside the destination checkout, with links resolved.
//!
//! The clone is made in a folder of its own in a scratch place and only then moved into the checkout
//! with a rename that refuses an existing target, so the checkout never holds a half-made folder and
//! nothing in it is ever replaced or merged into.

use std::ffi::OsString;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::platform::clone as platform_clone;
use crate::platform::rename::Folder;

fn invalid(message: impl Into<String>) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidInput, message.into())
}

/// A new, empty folder in `scratch` that nothing else has (created exclusively), private to the user.
fn unique_folder(scratch: &Path) -> io::Result<PathBuf> {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_nanos());
    for attempt in 0..8u32 {
        let dir = scratch.join(format!("wormhole-clone-{}-{nanos}-{attempt}", std::process::id()));
        match fs::create_dir(&dir) {
            Ok(()) => {
                crate::platform::fs::set_dir_mode_no_follow(&dir, 0o700)?;
                return Ok(dir);
            }
            Err(err) if err.kind() == io::ErrorKind::AlreadyExists => {}
            Err(err) => return Err(err),
        }
    }
    Err(io::Error::new(io::ErrorKind::AlreadyExists, "no unused scratch folder name"))
}

/// Clone the folder `rel` of the checkout `from_root` to the same place in `to_root`. `rel` must be
/// relative and free of `..`; the folder's parent must exist in `to_root` and `rel` must not
/// ([`io::ErrorKind::AlreadyExists`]). Links inside the folder are cloned as links.
///
/// The clone is made in a new folder inside `scratch` and moved into place by a rename that fails when
/// the target exists (also when something appeared there while it was being made), so nothing already in
/// `to_root` is replaced or nested into. `scratch` must be on the volume of both checkouts, or this fails
/// with [`io::ErrorKind::Unsupported`] before copying anything. Whatever the outcome, only the folder made
/// in `scratch` is ever removed, and none is left behind.
pub fn clone_dir(from_root: &Path, to_root: &Path, rel: &Path, scratch: &Path) -> io::Result<()> {
    if rel.as_os_str().is_empty() || !rel.components().all(|c| matches!(c, Component::Normal(_))) {
        return Err(invalid(format!("{} is not a path inside the checkout", rel.display())));
    }
    let (from, to) = (from_root.join(rel), to_root.join(rel));
    if !fs::symlink_metadata(&from)?.file_type().is_dir() {
        return Err(invalid(format!("{} is not a folder (or is a link)", from.display())));
    }
    let (Some(from_parent), Some(to_parent), Some(name)) = (from.parent(), to.parent(), to.file_name())
    else {
        return Err(invalid("no parent folder"));
    };
    if !fs::canonicalize(from_parent)?.starts_with(fs::canonicalize(from_root)?) {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "the source leaves its checkout"));
    }
    let to_parent = fs::canonicalize(to_parent)?;
    if !to_parent.starts_with(fs::canonicalize(to_root)?) {
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "the destination leaves its checkout"));
    }
    if fs::symlink_metadata(&to).is_ok() {
        return Err(io::Error::new(io::ErrorKind::AlreadyExists, format!("{} exists", to.display())));
    }
    let holder = unique_folder(scratch)?;
    let made = clone_and_place(&from, &holder, &to_parent, name);
    // Only the folder this call made; a link inside it is removed, never followed, by `remove_dir_all`.
    let cleaned = fs::remove_dir_all(&holder);
    made?;
    cleaned
}

/// Clone `from` into `holder` and move the result to `name` in `to_parent`, refusing to replace.
fn clone_and_place(from: &Path, holder: &Path, to_parent: &Path, name: &std::ffi::OsStr) -> io::Result<()> {
    let staged: OsString = "clone".into();
    platform_clone::clone_dir(from, &holder.join(&staged), to_parent)?;
    Folder::open(holder)?.move_no_replace_into(&staged, &Folder::open(to_parent)?, name)
}

#[cfg(test)]
mod tests {
    use crate::platform::fs::symlink;
    use std::fs;

    use super::clone_dir;
    use crate::platform::backup;

    #[test]
    fn a_tree_is_cloned_with_its_links_kept_as_links() {
        let tmp = tempfile::tempdir().unwrap();
        let (from, to) = (tmp.path().join("main"), tmp.path().join("wt"));
        let scratch = tmp.path().join("scratch");
        fs::create_dir_all(&scratch).unwrap();
        fs::create_dir_all(from.join("pkg/node_modules/a/b")).unwrap();
        fs::create_dir_all(&to).unwrap();
        fs::create_dir_all(to.join("pkg")).unwrap();
        let tree = from.join("pkg/node_modules");
        fs::write(tree.join("a/file.txt"), "hello").unwrap();
        fs::write(tree.join("a/b/deep.txt"), "deep").unwrap();
        symlink("a/file.txt", tree.join("link")).unwrap();
        symlink(tmp.path(), tree.join("outside")).unwrap();

        let rel = std::path::Path::new("pkg/node_modules");
        match clone_dir(&from, &to, rel, &scratch) {
            Ok(()) => {}
            // A temp dir on a volume that can't clone (not APFS) has nothing to check.
            Err(err) if err.kind() == std::io::ErrorKind::Unsupported => return,
            Err(err) => panic!("{err}"),
        }
        assert_eq!(fs::read_dir(&scratch).unwrap().count(), 0, "no scratch folder left behind");
        let copy = to.join(rel);
        assert_eq!(fs::read_to_string(copy.join("a/file.txt")).unwrap(), "hello");
        assert_eq!(fs::read_to_string(copy.join("a/b/deep.txt")).unwrap(), "deep");
        assert_eq!(fs::read_link(copy.join("link")).unwrap(), std::path::Path::new("a/file.txt"));
        assert_eq!(fs::read_link(copy.join("outside")).unwrap(), tmp.path(), "a link is not followed");
        // The two are independent after the clone.
        fs::write(copy.join("a/file.txt"), "changed").unwrap();
        assert_eq!(fs::read_to_string(tree.join("a/file.txt")).unwrap(), "hello");
        // Exclusion is the caller's to set; the helper alone doesn't.
        assert!(!backup::is_excluded(&copy));
        backup::exclude(&copy).unwrap();
        assert!(backup::is_excluded(&copy));
    }

    #[test]
    fn nothing_is_replaced_and_odd_paths_are_refused() {
        let tmp = tempfile::tempdir().unwrap();
        let (from, to) = (tmp.path().join("main"), tmp.path().join("wt"));
        let scratch = tmp.path().join("scratch");
        fs::create_dir_all(&scratch).unwrap();
        fs::create_dir_all(from.join("target")).unwrap();
        fs::create_dir_all(to.join("target")).unwrap();
        fs::write(to.join("target/keep"), "mine").unwrap();
        let kind = |rel: &str| clone_dir(&from, &to, std::path::Path::new(rel), &scratch).unwrap_err().kind();
        assert_eq!(kind("target"), std::io::ErrorKind::AlreadyExists);
        assert_eq!(fs::read_to_string(to.join("target/keep")).unwrap(), "mine");
        assert_eq!(kind("../x"), std::io::ErrorKind::InvalidInput);
        assert_eq!(kind("/etc"), std::io::ErrorKind::InvalidInput);
        // A link to a folder is not a folder to clone.
        symlink(tmp.path(), from.join("node_modules")).unwrap();
        assert_eq!(kind("node_modules"), std::io::ErrorKind::InvalidInput);
        // A missing parent in the destination is refused, not created.
        fs::create_dir_all(from.join("deep/target")).unwrap();
        assert_eq!(kind("deep/target"), std::io::ErrorKind::NotFound);
    }

    #[test]
    fn a_failed_clone_leaves_neither_a_scratch_folder_nor_anything_in_the_checkout() {
        let tmp = tempfile::tempdir().unwrap();
        let (from, to) = (tmp.path().join("main"), tmp.path().join("wt"));
        let scratch = tmp.path().join("scratch");
        fs::create_dir_all(from.join("target")).unwrap();
        fs::create_dir_all(&to).unwrap();
        // A scratch place that is not there: nothing can be made, and nothing appears in `to`.
        assert!(clone_dir(&from, &to, std::path::Path::new("target"), &scratch).is_err());
        assert_eq!(fs::read_dir(&to).unwrap().count(), 0);
        assert!(!scratch.exists());
    }
}
