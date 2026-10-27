//! Install, Update and Uninstall of the skill in an agent's skills folder.
//!
//! The skill lives in a folder of its own (`<skills>/wormhole/`) and the folder is ours only when its
//! `SKILL.md` carries [`MARKER`], or when everything in it is a file an earlier build shipped
//! ([`RETIRED`]: what is left after a build dropped a file, or after an Uninstall that was cut short).
//! A folder that is neither is somebody else's skill (or a link, or a file): it is reported as foreign
//! and never written to, replaced or removed. Writes are atomic, through `storage::files` with every
//! link inside the skill's folder refused (a link planted there cannot send a write, or a removal,
//! elsewhere); the skills folder above it is the user's own and may be a link (dotfiles).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{FILES, File, MARKER, Target, targets::TARGETS};
use crate::storage::files::{self, LinkPolicy};

/// More than any file of the skill: a larger `SKILL.md` is not ours.
const READ_CAP: u64 = 256 << 10;

/// Every path an earlier build shipped that the current one no longer does: Update and Uninstall remove
/// them, and a folder holding only these is ours. A file dropped from the skill is added here, and
/// never taken out again.
pub const RETIRED: &[&str] = &[];

/// What a build ships and what it used to: the thing an install is done with, so tests can stand in
/// another build's.
#[derive(Clone, Copy)]
pub(super) struct Set<'a> {
    pub files: &'a [File],
    pub retired: &'a [&'a str],
}

impl Set<'static> {
    pub(super) const CURRENT: Set<'static> = Set { files: FILES, retired: RETIRED };
}

/// Where the skill stands in an agent's folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    NotInstalled,
    /// Ours, and the same as the skill in this build.
    Current,
    /// Ours, from another build (or edited): Update puts this build's in its place.
    Outdated,
    /// Something else is at that path: another skill of the same name, a link, a file. Not touched.
    Foreign,
}

/// What an Install did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Done {
    Installed,
    Updated,
    /// It was current already.
    Unchanged,
}

/// The state of the skill for `target` under `home`.
pub fn state(target: &Target, home: &Path) -> Result<State, String> {
    state_at(&target.path(home), Set::CURRENT).map_err(|e| format!("{}: {e}", target.display_path(home)))
}

/// The folder of ours that holds the skill, among those `target`'s agent also reads, when our current
/// copy is in one: the agent has the skill already, whatever its own folder has.
pub fn covered_by(target: &Target, home: &Path) -> Option<String> {
    target
        .other_paths(home)
        .find(|dir| state_at(dir, Set::CURRENT) == Ok(State::Current))
        .and_then(|dir| dir.parent().map(Path::to_path_buf))
        .map(|skills| display(&skills, home))
}

fn display(path: &Path, home: &Path) -> String {
    match path.strip_prefix(home) {
        Ok(rest) if !home.as_os_str().is_empty() => format!("~/{}", rest.display()),
        _ => path.display().to_string(),
    }
}

fn state_at(dir: &Path, set: Set<'_>) -> Result<State, String> {
    let meta = match fs::symlink_metadata(dir) {
        Ok(meta) => meta,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(State::NotInstalled),
        Err(err) => return Err(err.to_string()),
    };
    // A link is never followed: where it leads is not ours to write.
    if !meta.is_dir() {
        return Ok(State::Foreign);
    }
    let skill_md =
        match files::read_to_string_capped(&dir.join("SKILL.md"), READ_CAP, LinkPolicy::Within(dir)) {
            Ok(text) => text,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                return Ok(if holds_only(dir, set.retired, "") {
                    State::NotInstalled
                } else {
                    State::Foreign
                });
            }
            // Unreadable, a link, not text: not something we wrote.
            Err(_) => return Ok(State::Foreign),
        };
    if !skill_md.contains(MARKER) {
        return Ok(State::Foreign);
    }
    for file in set.files {
        let same = files::read_to_string_capped(&dir.join(file.path), READ_CAP, LinkPolicy::Within(dir))
            .is_ok_and(|text| text == file.text);
        if !same {
            return Ok(State::Outdated);
        }
    }
    // A file an earlier build shipped and this one does not: Update takes it away.
    if set.retired.iter().any(|path| fs::symlink_metadata(dir.join(path)).is_ok()) {
        return Ok(State::Outdated);
    }
    Ok(State::Current)
}

/// Whether `dir` (at `rel` inside the skill's folder) holds nothing but retired files and the folders
/// they are in; an empty folder qualifies. A link is something else.
fn holds_only(dir: &Path, retired: &[&str], rel: &str) -> bool {
    let Ok(entries) = fs::read_dir(dir) else { return false };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let here = if rel.is_empty() {
            name.to_string_lossy().into_owned()
        } else {
            format!("{rel}/{}", name.to_string_lossy())
        };
        let Ok(meta) = fs::symlink_metadata(entry.path()) else { return false };
        let fine = if meta.is_dir() {
            retired.iter().any(|path| path.starts_with(&format!("{here}/")))
                && holds_only(&entry.path(), retired, &here)
        } else {
            meta.is_file() && retired.contains(&here.as_str())
        };
        if !fine {
            return false;
        }
    }
    true
}

/// Put the skill of this build in `target`'s folder: new, or in place of an older copy of ours.
pub fn install(target: &Target, home: &Path) -> Result<Done, String> {
    install_at(&target.path(home), Set::CURRENT, &target.display_path(home))
}

fn install_at(dir: &Path, set: Set<'_>, shown: &str) -> Result<Done, String> {
    let err = |e: io::Error| format!("{shown}: {e}");
    let before = state_at(dir, set).map_err(|e| format!("{shown}: {e}"))?;
    match before {
        State::Foreign => return Err(format!("Another skill is at {shown}; it was left alone")),
        State::Current => return Ok(Done::Unchanged),
        State::NotInstalled | State::Outdated => {}
    }
    // The skills folder is the user's (maybe a link to a dotfiles repository): made if missing, followed.
    files::create_dir_all_private(dir).map_err(err)?;
    // `SKILL.md` first: it carries the marker, so a stop half way leaves a folder that is ours.
    for file in set.files {
        files::write_atomic(&dir.join(file.path), file.text.as_bytes(), LinkPolicy::Within(dir))
            .map_err(err)?;
    }
    remove_ours(dir, set.retired.iter().copied(), shown)?;
    prune(dir, set);
    Ok(if before == State::Outdated { Done::Updated } else { Done::Installed })
}

/// Remove the skill's files from `target`'s folder (those of this build and of earlier ones), and the
/// folders if that empties them. A file the person added to the folder stays (and so does the folder).
/// `Ok(false)` when there was nothing.
pub fn uninstall(target: &Target, home: &Path) -> Result<bool, String> {
    uninstall_at(&target.path(home), Set::CURRENT, &target.display_path(home))
}

fn uninstall_at(dir: &Path, set: Set<'_>, shown: &str) -> Result<bool, String> {
    match state_at(dir, set).map_err(|e| format!("{shown}: {e}"))? {
        State::Foreign => return Err(format!("Another skill is at {shown}; it was left alone")),
        State::Current | State::Outdated => {}
        State::NotInstalled => {
            // Nothing of the skill, unless an earlier build's files are all that is left.
            if !dir.is_dir() || fs::read_dir(dir).is_ok_and(|mut entries| entries.next().is_none()) {
                return Ok(false);
            }
        }
    }
    let ours = set.files.iter().map(|f| f.path).chain(set.retired.iter().copied());
    remove_ours(dir, ours, shown)?;
    prune(dir, set);
    Ok(true)
}

/// Remove each of `paths` (relative to `dir`) that is there. A folder on the way that is a link is where
/// a removal would leave the skill's folder: it is refused, and nothing is removed through it.
fn remove_ours<'a>(dir: &Path, paths: impl Iterator<Item = &'a str>, shown: &str) -> Result<(), String> {
    for rel in paths {
        let path = dir.join(rel);
        if let Some(link) = link_on_the_way(dir, &path) {
            return Err(format!(
                "{shown}: {} is a link; nothing was removed through it",
                display_rel(&link, dir)
            ));
        }
        match fs::remove_file(&path) {
            Ok(()) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(format!("{shown}: {err}")),
        }
    }
    Ok(())
}

/// A folder between `dir` (exclusive) and `path` (exclusive) that is a link.
fn link_on_the_way(dir: &Path, path: &Path) -> Option<PathBuf> {
    let mut at = path.parent()?;
    while at != dir && at.starts_with(dir) {
        if fs::symlink_metadata(at).is_ok_and(|meta| meta.file_type().is_symlink()) {
            return Some(at.to_path_buf());
        }
        at = at.parent()?;
    }
    None
}

fn display_rel(path: &Path, dir: &Path) -> String {
    path.strip_prefix(dir).map_or_else(|_| path.display().to_string(), |rest| rest.display().to_string())
}

/// Remove the folders of the skill that have nothing left in them, the skill's own last; `remove_dir`
/// refuses one with anything in it.
fn prune(dir: &Path, set: Set<'_>) {
    let mut folders: Vec<PathBuf> = set
        .files
        .iter()
        .map(|f| f.path)
        .chain(set.retired.iter().copied())
        .filter_map(|rel| Path::new(rel).parent().map(|p| dir.join(p)))
        .filter(|p| p != dir)
        .collect();
    folders.sort();
    folders.dedup();
    // The deepest first.
    folders.sort_by_key(|p| std::cmp::Reverse(p.components().count()));
    for folder in folders {
        if fs::symlink_metadata(&folder).is_ok_and(|meta| meta.is_dir()) {
            let _ = fs::remove_dir(&folder);
        }
    }
    let _ = fs::remove_dir(dir);
}

/// The state under `home` for every agent that loads skills.
pub fn states(home: &Path) -> Vec<(&'static Target, Result<State, String>)> {
    TARGETS.iter().map(|target| (target, state(target, home))).collect()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::{Done, File, MARKER, Set, State, install_at, state_at, uninstall_at};

    const SKILL: &str = "---\nname: wormhole\n---\n<!-- managed-by: wormhole -->\nbody\n";

    fn build<'a>(files: &'a [File], retired: &'a [&'a str]) -> Set<'a> {
        Set { files, retired }
    }

    fn write(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(path, text).expect("write");
    }

    #[test]
    fn a_file_a_newer_build_dropped_goes_on_update_and_on_uninstall() {
        const OLD: &[File] = &[
            File { path: "SKILL.md", text: SKILL },
            File { path: "references/old.md", text: "old" },
            File { path: "references/keep.md", text: "keep" },
        ];
        const NEW: &[File] =
            &[File { path: "SKILL.md", text: SKILL }, File { path: "references/keep.md", text: "keep" }];
        let home = tempfile::tempdir().expect("tempdir");
        let dir = home.path().join("skills/wormhole");
        assert_eq!(install_at(&dir, build(OLD, &[]), "x"), Ok(Done::Installed));
        let newer = build(NEW, &["references/old.md"]);
        assert_eq!(state_at(&dir, newer), Ok(State::Outdated), "the dropped file is out of date");
        assert_eq!(install_at(&dir, newer, "x"), Ok(Done::Updated));
        assert!(!dir.join("references/old.md").exists() && dir.join("references/keep.md").is_file());
        assert_eq!(state_at(&dir, newer), Ok(State::Current));
        assert_eq!(uninstall_at(&dir, newer, "x"), Ok(true));
        assert!(!dir.exists(), "nothing of ours is left: the folder goes");
    }

    #[test]
    fn a_folder_with_only_retired_files_is_ours_and_not_foreign_forever() {
        const NEW: &[File] = &[File { path: "SKILL.md", text: SKILL }];
        let home = tempfile::tempdir().expect("tempdir");
        let dir = home.path().join("skills/wormhole");
        let set = build(NEW, &["references/old.md", "notes/gone.md"]);
        // What an Uninstall of an older build left behind.
        write(&dir.join("references/old.md"), "old");
        assert_eq!(state_at(&dir, set), Ok(State::NotInstalled));
        assert_eq!(uninstall_at(&dir, set, "x"), Ok(true));
        assert!(!dir.exists());
        write(&dir.join("references/old.md"), "old");
        assert_eq!(install_at(&dir, set, "x"), Ok(Done::Installed));
        assert!(!dir.join("references").exists());
        // Anything else in the folder, and it is somebody's.
        let other = home.path().join("other/wormhole");
        write(&other.join("references/old.md"), "old");
        write(&other.join("mine.md"), "mine");
        assert_eq!(state_at(&other, set), Ok(State::Foreign));
        assert!(install_at(&other, set, "x").is_err());
        assert_eq!(state_at(&other.parent().expect("p").join("none"), set), Ok(State::NotInstalled));
        assert!(SKILL.contains(MARKER));
    }

    #[test]
    fn a_link_in_the_way_is_never_removed_through() {
        const FILES: &[File] =
            &[File { path: "SKILL.md", text: SKILL }, File { path: "references/a.md", text: "a" }];
        let home = tempfile::tempdir().expect("tempdir");
        let outside = tempfile::tempdir().expect("tempdir");
        write(&outside.path().join("a.md"), "not ours: it is where the link leads");
        write(&outside.path().join("old.md"), "nor this");
        let dir = home.path().join("skills/wormhole");
        let set = build(FILES, &["references/old.md"]);
        install_at(&dir, set, "x").expect("installs");
        std::fs::remove_dir_all(dir.join("references")).expect("remove");
        crate::platform::fs::symlink(outside.path(), dir.join("references")).expect("link");
        let err = uninstall_at(&dir, set, "x").expect_err("refused");
        assert!(err.contains("is a link"), "{err}");
        assert!(outside.path().join("a.md").is_file() && outside.path().join("old.md").is_file());
        // Updating does not write or remove through it either.
        let _ = install_at(&dir, set, "x");
        assert!(outside.path().join("a.md").is_file() && outside.path().join("old.md").is_file());
        assert_eq!(
            std::fs::read_to_string(outside.path().join("a.md")).expect("read"),
            "not ours: it is where the link leads"
        );
    }
}
