//! The copies of the database the store makes, and how long they are kept.
//!
//! Each copy holds every terminal's saved text as it was then, so none is kept forever:
//!
//! - `wormhole.db.v<version>-<unix time>` (before an upgrade): the newest is kept for
//!   [`KEEP_FOR`], to go back to the build before; older ones go at once.
//! - `wormhole.db.old-<unix time>` and `wormhole.db.corrupt-<unix time>` (a database set aside, with
//!   its `-wal`/`-shm`): kept for [`KEEP_FOR`], then deleted.
//! - `andromeda-before-wormhole.db` (with `-wal`/`-shm`, kept by the rename to Wormhole in 0.3) and
//!   `andromeda.db.<label>-<unix time>` (copies the app made under its codename): the same rules;
//!   the rename's copy has no time in its name, so its modification time counts.
//!
//! Only these exact names, made by Wormhole, in the database's own folder, are ever deleted.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// How long a copy is kept: long enough to notice a bad upgrade and go back.
pub(super) const KEEP_FOR: Duration = Duration::from_secs(14 * 24 * 60 * 60);

/// The database names copies are made of: today's, and the codename's from before 0.3.
const BASES: [&str; 2] = ["wormhole.db", "andromeda.db"];
const RENAME_COPY: &str = "andromeda-before-wormhole.db";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    /// `VACUUM INTO` before an upgrade.
    BeforeUpgrade,
    /// Moved aside: too old to upgrade, or damaged.
    SetAside,
    /// Kept by the rename to Wormhole.
    Rename,
}

/// What a file name says, when it is one of the copies: its kind, and for the stamped ones the
/// time in the name and the number that makes it unique (`-2`, …), which order copies of a second.
fn parse(name: &str) -> Option<(Kind, Option<(u64, u64)>)> {
    let base = ["-wal", "-shm"].iter().find_map(|s| name.strip_suffix(s)).unwrap_or(name);
    if base == RENAME_COPY {
        return Some((Kind::Rename, None));
    }
    let rest = BASES.iter().find_map(|db| base.strip_prefix(db)?.strip_prefix('.'))?;
    let mut parts = rest.split('-');
    let label = parts.next()?;
    let stamp = number(parts.next()?)?;
    let n = match parts.next() {
        Some(n) => number(n)?,
        None => 0,
    };
    if parts.next().is_some() {
        return None;
    }
    let kind = match label {
        "old" | "corrupt" => Kind::SetAside,
        _ if label.strip_prefix('v').and_then(number).is_some() => Kind::BeforeUpgrade,
        _ => return None,
    };
    Some((kind, Some((stamp, n))))
}

fn number(digits: &str) -> Option<u64> {
    if digits.is_empty() || digits.len() > 20 || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse().ok()
}

/// Delete the copies in `dir` that are past keeping, as of `now`. Returns what was deleted. A file
/// that can't be read or deleted is left for the next launch; a link or folder by one of these
/// names is never touched.
pub(super) fn prune(dir: &Path, now: SystemTime) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else { return Vec::new() };
    let mut copies = Vec::new();
    for entry in entries.flatten() {
        // `DirEntry::file_type` does not follow links.
        if !entry.file_type().is_ok_and(|t| t.is_file()) {
            continue;
        }
        let Some(name) = entry.file_name().to_str().map(str::to_owned) else { continue };
        let Some((kind, stamped)) = parse(&name) else { continue };
        let made = match stamped {
            Some((secs, _)) => UNIX_EPOCH + Duration::from_secs(secs),
            None => match entry.metadata().and_then(|m| m.modified()) {
                Ok(time) => time,
                Err(_) => continue,
            },
        };
        copies.push((entry.path(), kind, stamped, made));
    }
    // The newest copy before an upgrade, by the time and number in its name.
    let newest_upgrade = copies.iter().filter(|c| c.1 == Kind::BeforeUpgrade).filter_map(|c| c.2).max();
    let mut removed = Vec::new();
    for (path, kind, stamped, made) in copies {
        // A time in the future (a clock set back) counts as now.
        let expired = now.duration_since(made).is_ok_and(|age| age >= KEEP_FOR);
        let superseded = kind == Kind::BeforeUpgrade && stamped != newest_upgrade;
        if !(expired || superseded) {
            continue;
        }
        match std::fs::remove_file(&path) {
            Ok(()) => removed.push(path),
            Err(err) => tracing::warn!("couldn't delete the old database copy {}: {err}", path.display()),
        }
    }
    if !removed.is_empty() {
        tracing::info!("deleted {} old database copies in {}", removed.len(), dir.display());
    }
    removed
}

#[cfg(test)]
mod tests {
    use super::{Kind, parse};

    #[test]
    fn only_the_names_wormhole_makes_are_copies() {
        assert_eq!(parse("wormhole.db.v7-1760000000"), Some((Kind::BeforeUpgrade, Some((1760000000, 0)))));
        assert_eq!(parse("wormhole.db.v12-1760000000-2"), Some((Kind::BeforeUpgrade, Some((1760000000, 2)))));
        assert_eq!(parse("wormhole.db.old-5-wal"), Some((Kind::SetAside, Some((5, 0)))));
        assert_eq!(parse("wormhole.db.corrupt-5-3-shm"), Some((Kind::SetAside, Some((5, 3)))));
        assert_eq!(parse("andromeda.db.old-5"), Some((Kind::SetAside, Some((5, 0)))));
        assert_eq!(parse("andromeda-before-wormhole.db-wal"), Some((Kind::Rename, None)));
        for other in [
            "wormhole.db",
            "wormhole.db-wal",
            "andromeda.db",
            "wormhole.db.v-5",
            "wormhole.db.vx-5",
            "wormhole.db.old-",
            "wormhole.db.old-5-",
            "wormhole.db.old-5-2-3",
            "wormhole.db.old-5.bak",
            "wormhole.db.backup-5",
            "my-wormhole.db.old-5",
            "wormhole.db.old-5-journal",
            "andromeda-before-wormhole.db.txt",
            "config.json",
        ] {
            assert_eq!(parse(other), None, "{other}");
        }
    }
}
