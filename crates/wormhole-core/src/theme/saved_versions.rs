//! Earlier saves of a user theme: every save keeps the file it replaces under
//! `themes/.versions/<id>/<timestamp>.json`, so a save can be gone back on.
//!
//! The first version 0 original of a theme is pinned (`….original.json`) and never removed:
//! it is what the person wrote by hand, before Wormhole rewrote the file. The rest thin out
//! with age ([`keep_policy`]): all of the last hour, then one an hour for a day, then one a
//! day for 30 days. The folder is hidden, so the theme list skips it, and owner-only like
//! the rest of the data folder; everything is read and written through `storage::files`.

use std::path::{Path, PathBuf};

use chrono::DateTime;

use crate::storage::files::{self, LinkPolicy, listing};
use crate::theme::tokens::{self, MAX_THEME_BYTES};

/// The folder in the themes folder that holds the saved versions.
pub const VERSIONS_DIR: &str = ".versions";

/// Most saves kept from the last hour: a burst of saves cannot fill the disk.
const MAX_RECENT: usize = 120;
const HOUR: i64 = 3600;
const DAY: i64 = 24 * HOUR;
const MAX_AGE: i64 = 30 * DAY;

/// One earlier save.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedVersion {
    pub path: PathBuf,
    /// When it was saved, in seconds since 1970 (UTC).
    pub saved: i64,
    /// The original version 0 file: never thinned out.
    pub pinned: bool,
}

/// Which of `saves` (a time, and whether it is pinned) to keep at time `now`, in the same
/// order. Pinned saves are kept; the others as the module's docs say. The newest save of
/// each hour (or day) is the one kept for it.
pub fn keep_policy(saves: &[(i64, bool)], now: i64) -> Vec<bool> {
    let mut keep = vec![false; saves.len()];
    // Newest first, so the first of a bucket is its newest.
    let mut order: Vec<usize> = (0..saves.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(saves[i].0));
    let mut recent = 0;
    let mut buckets = std::collections::HashSet::new();
    for i in order {
        let (at, pinned) = saves[i];
        let age = now - at;
        keep[i] = if pinned {
            true
        } else if age <= HOUR {
            recent += 1;
            recent <= MAX_RECENT
        } else if age <= DAY {
            buckets.insert((0, at.div_euclid(HOUR)))
        } else if age <= MAX_AGE {
            buckets.insert((1, at.div_euclid(DAY)))
        } else {
            false
        };
    }
    keep
}

fn dir_for(themes: &Path, id: &str) -> Result<PathBuf, String> {
    // The id becomes a folder name: only a slug can.
    if !tokens::slug::is_slug(id) {
        return Err(format!("{id:?} is not a theme id"));
    }
    Ok(themes.join(VERSIONS_DIR).join(id))
}

/// Whether `dir` is a real folder: a link at `.versions/<id>` (or `.versions`) is never followed,
/// so a save and its thinning stay inside the themes folder.
fn is_real_dir(dir: &Path) -> bool {
    std::fs::symlink_metadata(dir).is_ok_and(|m| m.is_dir())
}

fn file_name(saved: i64, pinned: bool, n: usize) -> Option<String> {
    let t = DateTime::from_timestamp(saved, 0)?.format("%Y-%m-%dT%H-%M-%S");
    let n = if n > 0 { format!("-{n}") } else { String::new() };
    Some(format!("{t}{n}{}.json", if pinned { ".original" } else { "" }))
}

fn parse_name(name: &str) -> Option<(i64, bool)> {
    let stem = name.strip_suffix(".json")?;
    let (stem, pinned) = stem.strip_suffix(".original").map_or((stem, false), |s| (s, true));
    let stamp = stem.get(..19)?;
    let t = chrono::NaiveDateTime::parse_from_str(stamp, "%Y-%m-%dT%H-%M-%S").ok()?;
    Some((t.and_utc().timestamp(), pinned))
}

/// The saved versions of theme `id`, newest first.
pub fn list_versions(themes: &Path, id: &str) -> Vec<SavedVersion> {
    let Ok(dir) = dir_for(themes, id) else { return Vec::new() };
    if !is_real_dir(&dir) {
        return Vec::new();
    }
    let Ok(listing) = listing::list(&dir) else { return Vec::new() };
    let mut all: Vec<SavedVersion> = listing
        .entries
        .iter()
        .filter_map(|e| {
            let name = e.name.to_str()?;
            let (saved, pinned) = parse_name(name)?;
            Some(SavedVersion { path: dir.join(name), saved, pinned })
        })
        .collect();
    all.sort_by(|a, b| b.saved.cmp(&a.saved).then_with(|| b.path.cmp(&a.path)));
    all
}

/// Keep `previous` (the text of the file a save is about to replace) as a saved version of
/// theme `id`, taken at `now`, and thin the older ones out. The first file that is still
/// version 0 becomes the pinned original.
pub fn keep_version(themes: &Path, id: &str, previous: &str, now: i64) -> Result<SavedVersion, String> {
    let dir = dir_for(themes, id)?;
    files::ensure_private_dir(&themes.join(VERSIONS_DIR)).map_err(|e| e.to_string())?;
    if std::fs::symlink_metadata(&dir).is_ok() && !is_real_dir(&dir) {
        return Err(format!("{} is a link", dir.display()));
    }
    files::ensure_private_dir(&dir).map_err(|e| e.to_string())?;
    let existing = list_versions(themes, id);
    let is_v0 = tokens::read_theme_file(previous).is_ok_and(|f| f.read.source_version == 0);
    let pinned = is_v0 && !existing.iter().any(|v| v.pinned);
    let path = (0..)
        .filter_map(|n| file_name(now, pinned, n))
        .map(|name| dir.join(name))
        .find(|p| !p.exists())
        .ok_or("no free file name")?;
    files::write_atomic(&path, previous.as_bytes(), LinkPolicy::Within(&dir)).map_err(|e| e.to_string())?;
    let saved = SavedVersion { path, saved: now, pinned };
    thin(themes, id, now);
    Ok(saved)
}

/// Remove the saved versions of `id` that [`keep_policy`] drops.
fn thin(themes: &Path, id: &str, now: i64) {
    let all = list_versions(themes, id);
    let keep = keep_policy(&all.iter().map(|v| (v.saved, v.pinned)).collect::<Vec<_>>(), now);
    let Ok(dir) = dir_for(themes, id) else { return };
    for (v, keep) in all.iter().zip(keep) {
        if !keep && let Err(e) = files::remove_file_within(&dir, &v.path) {
            tracing::warn!("saved theme version {}: {e}", v.path.display());
        }
    }
}

/// The text of a saved version (capped like a theme).
pub fn read_version(version: &SavedVersion) -> Result<String, String> {
    let dir = version.path.parent().ok_or("not a saved version")?;
    files::read_to_string_capped(&version.path, MAX_THEME_BYTES as u64, LinkPolicy::Within(dir))
        .map_err(|e| e.to_string())
}

/// [`keep_version`] at the current time.
pub fn keep_version_now(themes: &Path, id: &str, previous: &str) -> Result<SavedVersion, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_secs()).unwrap_or(i64::MAX));
    keep_version(themes, id, previous, now)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: i64 = 1_800_000_000;

    #[test]
    fn the_policy_thins_by_age() {
        let m = 60;
        let saves = [
            (NOW - 5 * m, false),             // recent: kept
            (NOW - 20 * m, false),            // recent: kept
            (NOW - 3 * HOUR - 10 * m, false), // same hour as next, older
            (NOW - 3 * HOUR - 5 * m, false),  // newest of its hour: kept
            (NOW - 5 * HOUR, false),          // own hour: kept
            (NOW - 2 * DAY, false),           // own day
            (NOW - 2 * DAY - HOUR, false),    // same day (maybe) as the previous
            (NOW - 40 * DAY, false),          // too old
            (NOW - 400 * DAY, true),          // pinned: always
        ];
        let keep = keep_policy(&saves, NOW);
        assert_eq!(&keep[..5], &[true, true, false, true, true]);
        assert!(keep[5], "a day's newest");
        assert!(!(keep[5] && keep[6]), "one a day");
        assert!(!keep[7] && keep[8]);
    }

    #[test]
    fn a_burst_in_the_last_hour_is_capped() {
        let saves: Vec<(i64, bool)> = (0..300).map(|i| (NOW - i, false)).collect();
        let keep = keep_policy(&saves, NOW);
        assert_eq!(keep.iter().filter(|k| **k).count(), MAX_RECENT);
        assert!(keep[0] && !keep[299], "the newest are the ones kept");
    }

    #[test]
    fn names_round_trip() {
        let n = file_name(NOW, false, 0).unwrap();
        assert_eq!(parse_name(&n), Some((NOW, false)));
        assert_eq!(parse_name(&file_name(NOW, true, 0).unwrap()), Some((NOW, true)));
        assert_eq!(parse_name(&file_name(NOW, false, 2).unwrap()), Some((NOW, false)));
        assert_eq!(parse_name("notes.txt"), None);
    }

    #[test]
    fn saves_are_kept_listed_pinned_and_thinned() {
        let dir = tempfile::tempdir().unwrap();
        let t = dir.path();
        let v0 = r#"{"name": "A", "extends": "Nord"}"#;
        let v1 = "{\"version\": 1, \"name\": \"A\"}";
        let first = keep_version(t, "a", v0, NOW - 100 * DAY).unwrap();
        assert!(first.pinned);
        // Not v0, or a second v0: not pinned.
        assert!(!keep_version(t, "a", v1, NOW - 50 * DAY).unwrap().pinned);
        assert!(!keep_version(t, "a", v0, NOW - 3 * DAY).unwrap().pinned);
        let b = keep_version(t, "a", v1, NOW - 60).unwrap();
        let c = keep_version(t, "a", v1, NOW - 60).unwrap();
        assert_ne!(b.path, c.path, "two saves in one second both stay");
        let all = list_versions(t, "a");
        // The 50-day one was thinned away when the later saves came.
        assert_eq!(all.len(), 4, "{all:?}");
        assert!(all.iter().any(|v| v.pinned) && all.windows(2).all(|w| w[0].saved >= w[1].saved));
        assert_eq!(read_version(&first).unwrap(), v0);
        assert!(t.join(VERSIONS_DIR).is_dir());
    }

    #[cfg(unix)]
    #[test]
    fn a_linked_versions_folder_is_never_followed() {
        let dir = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        let t = dir.path();
        std::fs::create_dir(t.join(VERSIONS_DIR)).unwrap();
        let old = elsewhere.path().join("2026-01-01T00-00-00.json");
        std::fs::write(&old, "{}").unwrap();
        std::os::unix::fs::symlink(elsewhere.path(), t.join(VERSIONS_DIR).join("a")).unwrap();
        assert!(keep_version(t, "a", "{}", NOW).is_err());
        assert!(list_versions(t, "a").is_empty());
        thin(t, "a", NOW);
        assert!(old.exists(), "nothing outside the themes folder is touched");
        assert_eq!(std::fs::read_dir(elsewhere.path()).unwrap().count(), 1);
    }

    #[test]
    fn an_id_that_is_not_a_slug_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        for bad in ["../x", "a/b", "", "A"] {
            assert!(keep_version(dir.path(), bad, "{}", NOW).is_err(), "{bad}");
            assert!(list_versions(dir.path(), bad).is_empty());
        }
    }
}
