//! Install, Update and Uninstall of the skill in an agent's skills folder.
//!
//! The skill lives in a folder of its own (`<skills>/wormhole/`) and the folder is ours only when its
//! `SKILL.md` carries [`MARKER`]. A folder without it is somebody else's skill (or a link, or a file):
//! it is reported as foreign and never written to, replaced or removed. Writes are atomic, through
//! `storage::files` with every link inside the skill's folder refused (a link planted there cannot
//! send a write elsewhere); the skills folder above it is the user's own and may be a link (dotfiles).

use std::fs;
use std::io;
use std::path::Path;

use super::{FILES, MARKER, Target, targets::TARGETS};
use crate::storage::files::{self, LinkPolicy};

/// More than any file of the skill: a larger `SKILL.md` is not ours.
const READ_CAP: u64 = 256 << 10;

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
    let dir = target.path(home);
    let meta = match fs::symlink_metadata(&dir) {
        Ok(meta) => meta,
        Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(State::NotInstalled),
        Err(err) => return Err(format!("{}: {err}", target.display_path(home))),
    };
    // A link is never followed: where it leads is not ours to write.
    if !meta.is_dir() {
        return Ok(State::Foreign);
    }
    let skill_md =
        match files::read_to_string_capped(&dir.join("SKILL.md"), READ_CAP, LinkPolicy::Within(&dir)) {
            Ok(text) => text,
            Err(err) if err.kind() == io::ErrorKind::NotFound => return Ok(empty_or_foreign(&dir)),
            // Unreadable, a link, not text: not something we wrote.
            Err(_) => return Ok(State::Foreign),
        };
    if !skill_md.contains(MARKER) {
        return Ok(State::Foreign);
    }
    for file in FILES {
        let same = files::read_to_string_capped(&dir.join(file.path), READ_CAP, LinkPolicy::Within(&dir))
            .is_ok_and(|text| text == file.text);
        if !same {
            return Ok(State::Outdated);
        }
    }
    Ok(State::Current)
}

/// A folder with no `SKILL.md`: an empty one is nobody's; one with anything in it is somebody's.
fn empty_or_foreign(dir: &Path) -> State {
    match fs::read_dir(dir).map(|mut entries| entries.next().is_none()) {
        Ok(true) => State::NotInstalled,
        _ => State::Foreign,
    }
}

/// Put the skill of this build in `target`'s folder: new, or in place of an older copy of ours.
pub fn install(target: &Target, home: &Path) -> Result<Done, String> {
    let shown = target.display_path(home);
    let before = state(target, home)?;
    match before {
        State::Foreign => return Err(format!("Another skill is at {shown}; it was left alone")),
        State::Current => return Ok(Done::Unchanged),
        State::NotInstalled | State::Outdated => {}
    }
    let dir = target.path(home);
    let err = |e: io::Error| format!("{shown}: {e}");
    // The skills folder is the user's (maybe a link to a dotfiles repository): made if missing, followed.
    files::create_dir_all_private(&dir).map_err(err)?;
    // `SKILL.md` first: it carries the marker, so a stop half way leaves a folder that is ours.
    for file in FILES {
        files::write_atomic(&dir.join(file.path), file.text.as_bytes(), LinkPolicy::Within(&dir))
            .map_err(err)?;
    }
    Ok(if before == State::Outdated { Done::Updated } else { Done::Installed })
}

/// Remove the skill's files from `target`'s folder, and the folders if that empties them. A file the
/// person added to the folder stays (and so does the folder). `Ok(false)` when there was nothing.
pub fn uninstall(target: &Target, home: &Path) -> Result<bool, String> {
    let shown = target.display_path(home);
    match state(target, home)? {
        State::NotInstalled => return Ok(false),
        State::Foreign => return Err(format!("Another skill is at {shown}; it was left alone")),
        State::Current | State::Outdated => {}
    }
    let dir = target.path(home);
    for file in FILES {
        match fs::remove_file(dir.join(file.path)) {
            Ok(()) => {}
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(format!("{shown}: {err}")),
        }
    }
    // Only empty folders go; `remove_dir` refuses one with anything in it.
    let _ = fs::remove_dir(dir.join("references"));
    let _ = fs::remove_dir(&dir);
    Ok(true)
}

/// The state under `home` for every agent that loads skills.
pub fn states(home: &Path) -> Vec<(&'static Target, Result<State, String>)> {
    TARGETS.iter().map(|target| (target, state(target, home))).collect()
}
