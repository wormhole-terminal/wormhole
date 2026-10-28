//! Renaming a file or folder of a project in place: the Files tab's one write.
//!
//! The name comes from a person typing and the folder from a repository, so nothing is taken on trust:
//! - the new name is one plain name ([`check_name`]): no separator, control or bidirectional-control
//!   character (which can make a name read as another), nothing the system would read as `.` or `..`;
//! - the folder it is in is a real path inside the root with no link on the way ([`resolve_path_within`]
//!   resolves links, so the resolved folder must also be the one the plain path spells);
//! - nothing is replaced: the system call refuses a target that exists ([`rename_no_replace`]). The one
//!   exception is a rename that only changes case on a volume that ignores it, where the target is the
//!   entry itself.
//!
//! A link is renamed as the link it is; what it points at is never looked at.

use std::ffi::OsStr;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Component, Path, PathBuf};

use super::resolve_path_within;
use crate::platform::fs::same_file;
use crate::platform::rename::{rename_no_replace, rename_same_entry};

/// The longest name a file system keeps (APFS and HFS+ both): in bytes.
pub const MAX_NAME_BYTES: usize = 255;

/// What is wrong with a name.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum NameProblem {
    /// Nothing, or only spaces.
    Empty,
    /// `.` or `..`.
    Dots,
    /// A `/`: a name is one step of a path.
    Separator,
    /// A NUL or another control character.
    Control,
    /// A character that changes the direction text is read in, which can make a name look like another.
    Bidi,
    /// Not text.
    NotText,
    /// More than [`MAX_NAME_BYTES`] bytes.
    TooLong,
}

/// Why a rename was refused or failed.
#[derive(Debug)]
pub enum RenameError {
    /// The new name can't be one.
    Name(NameProblem),
    /// The path is not a plain one inside the root, or a link is on the way to it: nothing is renamed
    /// through a link.
    Outside,
    /// What was to be renamed is not there any more.
    Missing,
    /// Something is called that already.
    Exists,
    /// The system does not allow it (permissions, a read-only volume).
    Denied,
    /// Anything else the system said.
    Failed(io::Error),
}

impl fmt::Display for RenameError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Name(problem) => write!(f, "not a usable name ({problem:?})"),
            Self::Outside => f.write_str("the path leaves the folder or goes through a link"),
            Self::Missing => f.write_str("it is not there any more"),
            Self::Exists => f.write_str("something with that name exists already"),
            Self::Denied => f.write_str("not allowed"),
            Self::Failed(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for RenameError {}

impl From<io::Error> for RenameError {
    fn from(err: io::Error) -> Self {
        match err.kind() {
            io::ErrorKind::NotFound => Self::Missing,
            io::ErrorKind::AlreadyExists => Self::Exists,
            io::ErrorKind::PermissionDenied => Self::Denied,
            _ => Self::Failed(err),
        }
    }
}

/// Whether `ch` changes the direction text is shown in (the embeddings, overrides and isolates, and the
/// marks).
fn is_bidi(ch: char) -> bool {
    matches!(ch, '\u{061C}' | '\u{200E}' | '\u{200F}' | '\u{202A}'..='\u{202E}' | '\u{2066}'..='\u{2069}')
}

/// Whether `name` can be the name of one file or folder.
pub fn check_name(name: &OsStr) -> Result<&str, NameProblem> {
    let name = name.to_str().ok_or(NameProblem::NotText)?;
    if name.trim().is_empty() {
        return Err(NameProblem::Empty);
    }
    if name == "." || name == ".." {
        return Err(NameProblem::Dots);
    }
    if name.len() > MAX_NAME_BYTES {
        return Err(NameProblem::TooLong);
    }
    for ch in name.chars() {
        if ch == '/' {
            return Err(NameProblem::Separator);
        }
        if ch.is_control() {
            return Err(NameProblem::Control);
        }
        if is_bidi(ch) {
            return Err(NameProblem::Bidi);
        }
    }
    Ok(name)
}

/// The folder `parent` (below `root`, `""` for `root`), as a real path that has no link in it.
fn real_parent(root: &Path, parent: &Path) -> Result<PathBuf, RenameError> {
    let real_root = fs::canonicalize(root)?;
    if parent.as_os_str().is_empty() {
        return Ok(real_root);
    }
    let real = resolve_path_within(root, parent).map_err(|err| match err.kind() {
        io::ErrorKind::NotFound => RenameError::Missing,
        _ => RenameError::Outside,
    })?;
    // Resolving followed any link on the way; the path as spelled must be where it ended.
    if real != real_root.join(parent) {
        return Err(RenameError::Outside);
    }
    Ok(real)
}

/// Renames the file, folder or link `from` (a plain path below `root`) to `new_name`, in the same folder.
/// Returns its new path below `root`. Blocks on the disk: not for the main thread.
///
/// Nothing is replaced, nothing is renamed through a link, and the root itself can't be renamed. The
/// same name again is not an error and changes nothing.
pub fn rename_within(root: &Path, from: &Path, new_name: &OsStr) -> Result<PathBuf, RenameError> {
    let name = check_name(new_name).map_err(RenameError::Name)?;
    let mut parts = from.components();
    let (Some(Component::Normal(old_name)), parent) = (parts.next_back(), parts.as_path()) else {
        return Err(RenameError::Outside);
    };
    if !parent.components().all(|part| matches!(part, Component::Normal(_))) {
        return Err(RenameError::Outside);
    }
    let folder = real_parent(root, parent)?;
    let (source, target) = (folder.join(old_name), folder.join(name));
    let new_relative = parent.join(name);
    if old_name == OsStr::new(name) {
        // Nothing to do, but a name that is not there is still an error.
        fs::symlink_metadata(&source)?;
        return Ok(new_relative);
    }
    match rename_no_replace(&source, &target) {
        Ok(()) => Ok(new_relative),
        Err(err)
            if err.kind() == io::ErrorKind::AlreadyExists
                && only_case_differs(&source, &target, name, old_name) =>
        {
            rename_same_entry(&source, &target)?;
            Ok(new_relative)
        }
        Err(err) => Err(err.into()),
    }
}

/// Whether `target` is `source` itself under another case: the same entry on disk, and names that differ
/// only in case. What a volume that ignores case answers when `readme.md` is renamed to `README.md`.
fn only_case_differs(source: &Path, target: &Path, name: &str, old_name: &OsStr) -> bool {
    let Some(old) = old_name.to_str() else { return false };
    if name.to_lowercase() != old.to_lowercase() {
        return false;
    }
    match (fs::symlink_metadata(source), fs::symlink_metadata(target)) {
        (Ok(a), Ok(b)) => same_file(&a, &b),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsStr;
    use std::fs;
    use std::path::{Path, PathBuf};

    use super::{NameProblem, RenameError, check_name, rename_within};
    use crate::platform::fs::symlink;

    fn names(dir: &Path) -> Vec<String> {
        let mut found: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        found.sort();
        found
    }

    fn rename(root: &Path, from: &str, to: &str) -> Result<PathBuf, RenameError> {
        rename_within(root, Path::new(from), OsStr::new(to))
    }

    #[test]
    fn a_file_is_renamed_in_its_folder_and_the_new_path_is_told() {
        let root = tempfile::tempdir().unwrap();
        fs::create_dir(root.path().join("src")).unwrap();
        fs::write(root.path().join("src/a.rs"), "x").unwrap();
        assert_eq!(rename(root.path(), "src/a.rs", "b.rs").unwrap(), Path::new("src/b.rs"));
        assert_eq!(names(&root.path().join("src")), ["b.rs"]);
        assert_eq!(rename(root.path(), "src", "lib").unwrap(), Path::new("lib"));
        assert_eq!(fs::read_to_string(root.path().join("lib/b.rs")).unwrap(), "x");
    }

    #[test]
    fn nothing_is_overwritten() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("a"), "a").unwrap();
        fs::write(root.path().join("b"), "b").unwrap();
        fs::create_dir(root.path().join("d")).unwrap();
        for (from, to) in [("a", "b"), ("a", "d"), ("d", "a")] {
            assert!(matches!(rename(root.path(), from, to), Err(RenameError::Exists)), "{from} to {to}");
        }
        assert_eq!(fs::read_to_string(root.path().join("a")).unwrap(), "a");
        assert_eq!(fs::read_to_string(root.path().join("b")).unwrap(), "b");
    }

    #[test]
    fn a_name_that_cannot_be_one_is_refused() {
        let cases = [
            ("", NameProblem::Empty),
            ("   ", NameProblem::Empty),
            (".", NameProblem::Dots),
            ("..", NameProblem::Dots),
            ("a/b", NameProblem::Separator),
            ("/", NameProblem::Separator),
            ("a\0b", NameProblem::Control),
            ("a\nb", NameProblem::Control),
            ("a\u{7f}b", NameProblem::Control),
            ("a\u{202E}b", NameProblem::Bidi),
            ("a\u{2066}b", NameProblem::Bidi),
            ("a\u{200F}b", NameProblem::Bidi),
        ];
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("a"), "").unwrap();
        for (name, problem) in cases {
            assert!(
                matches!(rename(root.path(), "a", name), Err(RenameError::Name(found)) if found == problem),
                "{name:?}"
            );
        }
        let long = "x".repeat(256);
        assert!(matches!(rename(root.path(), "a", &long), Err(RenameError::Name(NameProblem::TooLong))));
        assert_eq!(check_name(OsStr::new(&"x".repeat(255))), Ok(&*"x".repeat(255)));
        // Counted in bytes, not characters.
        assert_eq!(check_name(OsStr::new(&"é".repeat(128))), Err(NameProblem::TooLong));
        assert_eq!(names(root.path()), ["a"]);
    }

    #[test]
    fn a_name_that_is_not_text_is_refused() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("a"), "").unwrap();
        let bad = crate::platform::fs::path_from_bytes(b"caf\xe9").as_os_str();
        assert!(matches!(
            rename_within(root.path(), Path::new("a"), bad),
            Err(RenameError::Name(NameProblem::NotText))
        ));
    }

    #[test]
    fn a_path_that_is_not_plain_or_is_the_root_is_refused() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("a"), "").unwrap();
        for from in ["", "..", "../a", "/a", "./a", "x/../a"] {
            assert!(matches!(rename(root.path(), from, "z"), Err(RenameError::Outside)), "{from:?}");
        }
        assert_eq!(names(root.path()), ["a"]);
    }

    #[test]
    fn a_folder_that_is_a_link_is_not_renamed_through() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("secret"), "").unwrap();
        symlink(outside.path(), root.path().join("out")).unwrap();
        // A link out of the root.
        assert!(matches!(rename(root.path(), "out/secret", "x"), Err(RenameError::Outside)));
        // A link to a folder inside it is a link all the same.
        fs::create_dir(root.path().join("real")).unwrap();
        fs::write(root.path().join("real/f"), "").unwrap();
        symlink(root.path().join("real"), root.path().join("alias")).unwrap();
        assert!(matches!(rename(root.path(), "alias/f", "g"), Err(RenameError::Outside)));
        assert_eq!(names(outside.path()), ["secret"]);
        assert_eq!(names(&root.path().join("real")), ["f"]);
    }

    #[test]
    fn a_link_is_renamed_and_not_what_it_points_at() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        fs::write(outside.path().join("target"), "t").unwrap();
        symlink(outside.path().join("target"), root.path().join("link")).unwrap();
        assert_eq!(rename(root.path(), "link", "renamed").unwrap(), Path::new("renamed"));
        assert!(fs::symlink_metadata(root.path().join("renamed")).unwrap().file_type().is_symlink());
        assert!(fs::symlink_metadata(root.path().join("link")).is_err());
        assert_eq!(names(outside.path()), ["target"]);
    }

    #[test]
    fn something_that_is_gone_is_missing() {
        let root = tempfile::tempdir().unwrap();
        assert!(matches!(rename(root.path(), "nope", "x"), Err(RenameError::Missing)));
        assert!(matches!(rename(root.path(), "nope", "nope"), Err(RenameError::Missing)));
        assert!(matches!(rename(root.path(), "no/such/file", "x"), Err(RenameError::Missing)));
    }

    #[test]
    fn the_same_name_changes_nothing() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("a"), "").unwrap();
        assert_eq!(rename(root.path(), "a", "a").unwrap(), Path::new("a"));
        assert_eq!(names(root.path()), ["a"]);
    }

    #[test]
    fn a_rename_that_only_changes_case_works_whatever_the_volume_does() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("readme.md"), "r").unwrap();
        assert_eq!(rename(root.path(), "readme.md", "README.md").unwrap(), Path::new("README.md"));
        assert_eq!(names(root.path()), ["README.md"]);
        assert_eq!(fs::read_to_string(root.path().join("README.md")).unwrap(), "r");
        fs::create_dir(root.path().join("docs")).unwrap();
        rename(root.path(), "docs", "Docs").unwrap();
        assert_eq!(names(root.path()), ["Docs", "README.md"]);
    }

    #[test]
    fn a_case_variant_that_is_another_file_is_not_replaced() {
        // On a volume that tells case apart these are two files; on one that does not, the second write
        // is the first file. Either way renaming `a` to `A` must not destroy a file that is not `a`.
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("a"), "a").unwrap();
        fs::write(root.path().join("b"), "b").unwrap();
        assert!(matches!(rename(root.path(), "a", "b"), Err(RenameError::Exists)));
        assert_eq!(fs::read_to_string(root.path().join("b")).unwrap(), "b");
    }

    #[test]
    fn a_file_with_a_hard_link_under_another_case_is_not_taken_for_itself() {
        let root = tempfile::tempdir().unwrap();
        fs::write(root.path().join("a"), "a").unwrap();
        // A hard link is the same inode under a different name; only a case difference counts as the entry itself.
        fs::hard_link(root.path().join("a"), root.path().join("other")).unwrap();
        assert!(matches!(rename(root.path(), "a", "other"), Err(RenameError::Exists)));
    }
}
