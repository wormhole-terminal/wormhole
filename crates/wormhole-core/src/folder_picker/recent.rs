//! The folders the person's terminals were in lately, ranked by frecency: how often and how recently.
//!
//! Kept per profile in the profile's store (the app saves [`RecentFolders::to_json`] under a key of its
//! own), on this Mac only. Never recorded: anything inside the app's own data folder or a temporary folder
//! (they are not places a person works in, and some are named after a secret), a path that isn't absolute,
//! and one with a control character in it. At most [`CAP`] are kept.
//!
//! A visit adds one to a score that halves every [`HALF_LIFE`], so a folder visited ten times last month ranks
//! with one visited twice today, and a folder not visited for a year fades out: nothing is ever "ranked by
//! count" alone, and there is no table of ages to tune.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use super::typed::tidy;
use crate::storage::files::hostile_char;

/// The most folders kept.
pub const CAP: usize = 200;
/// How long it takes a score to halve.
const HALF_LIFE: f64 = 14.0 * 24.0 * 3600.0;
/// A second visit within this many seconds of the last isn't counted: a prompt that redraws or a `cd .` is
/// not a visit.
const SAME_VISIT: u64 = 30;
/// The longest path kept.
const PATH_CAP: usize = 1024;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
struct Entry {
    path: String,
    /// The score as of `last`.
    score: f64,
    /// Seconds since the epoch.
    last: u64,
}

impl Entry {
    fn now(&self, at: u64) -> f64 {
        self.score * 0.5_f64.powf(at.saturating_sub(self.last) as f64 / HALF_LIFE)
    }
}

/// The folders, with what is needed to rank them.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct RecentFolders {
    entries: Vec<Entry>,
}

/// The places that are never recorded: the app's data folder and the temporary folders (the system's own, and
/// the usual spellings of them, since a shell reports the real path and the system names a link).
pub fn keep_out(data: Option<&Path>, temp: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = ["/tmp", "/private/tmp", "/var/folders", "/private/var/folders"]
        .into_iter()
        .map(PathBuf::from)
        .collect();
    out.push(temp.to_path_buf());
    out.extend(data.map(Path::to_path_buf));
    out
}

fn allowed(path: &str, keep_out: &[PathBuf]) -> bool {
    let p = Path::new(path);
    p.is_absolute()
        && path.len() <= PATH_CAP
        && !path.chars().any(hostile_char)
        && !keep_out.iter().any(|out| p.starts_with(out))
}

impl RecentFolders {
    /// What `json` holds (what [`to_json`](Self::to_json) wrote); nothing for a missing or broken value, and
    /// an entry that is not a place worth keeping is dropped.
    pub fn from_json(json: Option<&str>) -> Self {
        let mut found: Self = json.and_then(|j| serde_json::from_str(j).ok()).unwrap_or_default();
        found.entries.retain(|e| e.score.is_finite() && e.score > 0.0 && allowed(&e.path, &[]));
        found.entries.truncate(CAP);
        found
    }

    pub fn to_json(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }

    /// A visit to `path` at `now` (seconds since the epoch). Whether the list changed: a place that is not
    /// recorded (see the module's words) and a repeat within seconds change nothing.
    pub fn record(&mut self, path: &Path, now: u64, keep_out: &[PathBuf]) -> bool {
        // A shell reports the path it was told, `..` and all: the place is what the steps come to, so a path
        // can't be spelled to get past the places that are never kept.
        let path = tidy(path);
        let Some(path) = path.to_str().map(|p| p.trim_end_matches('/')) else { return false };
        let path = if path.is_empty() { "/" } else { path };
        if !allowed(path, keep_out) {
            return false;
        }
        match self.entries.iter_mut().find(|e| e.path == path) {
            Some(entry) if now.saturating_sub(entry.last) < SAME_VISIT => return false,
            Some(entry) => {
                entry.score = entry.now(now) + 1.0;
                entry.last = now;
            }
            None => self.entries.push(Entry { path: path.to_string(), score: 1.0, last: now }),
        }
        if self.entries.len() > CAP {
            let weakest = self
                .entries
                .iter()
                .enumerate()
                .min_by(|a, b| a.1.now(now).total_cmp(&b.1.now(now)))
                .map(|(at, _)| at);
            if let Some(at) = weakest {
                self.entries.remove(at);
            }
        }
        true
    }

    /// Forget `path`. Whether it was there.
    pub fn forget(&mut self, path: &str) -> bool {
        let before = self.entries.len();
        self.entries.retain(|e| e.path != path);
        self.entries.len() != before
    }

    /// The folders with their scores at `now`, the best first.
    pub fn ranked(&self, now: u64) -> Vec<(&str, f64)> {
        let mut ranked: Vec<(&str, f64)> =
            self.entries.iter().map(|e| (e.path.as_str(), e.now(now))).collect();
        ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
        ranked
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{CAP, HALF_LIFE, RecentFolders, keep_out};

    const DAY: u64 = 24 * 3600;

    fn paths(r: &RecentFolders, now: u64) -> Vec<String> {
        r.ranked(now).into_iter().map(|(p, _)| p.to_string()).collect()
    }

    fn out() -> Vec<PathBuf> {
        keep_out(
            Some(Path::new("/Users/me/Library/Application Support/Wormhole")),
            Path::new("/var/folders/x/T"),
        )
    }

    #[test]
    fn a_visit_is_recorded_and_the_most_visited_ranks_first() {
        let mut r = RecentFolders::default();
        let now = 1_000_000;
        for (i, p) in ["/a", "/b", "/a", "/c", "/a", "/b"].iter().enumerate() {
            assert!(r.record(Path::new(p), now + 100 * i as u64, &out()));
        }
        assert_eq!(paths(&r, now + 1000), ["/a", "/b", "/c"]);
    }

    #[test]
    fn recency_beats_a_long_ago_pile_of_visits() {
        let mut r = RecentFolders::default();
        let t0 = 10_000_000;
        for i in 0..6 {
            r.record(Path::new("/old"), t0 + i * 60, &out());
        }
        let later = t0 + 120 * DAY;
        r.record(Path::new("/new"), later, &out());
        assert_eq!(
            paths(&r, later + 60),
            ["/new", "/old"],
            "6 visits four months ago are less than one today"
        );
        let still = r.ranked(t0 + 360)[0];
        assert_eq!(still.0, "/old", "…but ranked alone a moment after they happened");
        assert!(r.ranked(t0 + 360 + HALF_LIFE as u64)[0].1 < still.1 * 0.51);
    }

    #[test]
    fn a_repeat_within_seconds_is_not_a_second_visit() {
        let mut r = RecentFolders::default();
        assert!(r.record(Path::new("/a"), 1000, &out()));
        assert!(!r.record(Path::new("/a"), 1010, &out()));
        assert!(r.record(Path::new("/a"), 1100, &out()));
    }

    #[test]
    fn the_apps_data_folder_and_temporary_folders_are_never_recorded() {
        let mut r = RecentFolders::default();
        for p in [
            "/Users/me/Library/Application Support/Wormhole/hosts",
            "/Users/me/Library/Application Support/Wormhole",
            "/tmp/x",
            "/private/tmp/scratch",
            "/var/folders/ab/cd/T/thing",
            "/private/var/folders/ab",
            "relative/path",
            "/has\nnewline",
            "/Users/me/Documents/../Library/Application Support/Wormhole/hosts",
            "/work/\u{202e}x",
        ] {
            assert!(!r.record(Path::new(p), 1000, &out()), "{p}");
        }
        assert!(r.is_empty());
        assert!(
            r.record(Path::new("/Users/me/Library/Application Support/Wormholes"), 1000, &out()),
            "a sibling"
        );
    }

    #[test]
    fn a_trailing_slash_is_the_same_folder_and_the_root_is_kept() {
        let mut r = RecentFolders::default();
        r.record(Path::new("/a/b/"), 1000, &out());
        r.record(Path::new("/a/b"), 5000, &out());
        r.record(Path::new("/"), 1000, &out());
        assert_eq!(r.len(), 2);
    }

    #[test]
    fn only_the_best_two_hundred_are_kept() {
        let mut r = RecentFolders::default();
        for i in 0..CAP + 25 {
            r.record(Path::new(&format!("/p{i}")), 1000 + i as u64 * 100, &out());
        }
        assert_eq!(r.len(), CAP);
        let now = 1000 + (CAP as u64 + 25) * 100;
        assert!(paths(&r, now).contains(&format!("/p{}", CAP + 24)), "the newest stays");
        assert!(!paths(&r, now).contains(&"/p0".to_string()), "the oldest went");
    }

    #[test]
    fn a_folder_can_be_forgotten() {
        let mut r = RecentFolders::default();
        r.record(Path::new("/a"), 1000, &out());
        assert!(r.forget("/a"));
        assert!(!r.forget("/a"));
        assert!(r.is_empty());
    }

    #[test]
    fn it_round_trips_through_json_and_a_broken_value_is_nothing() {
        let mut r = RecentFolders::default();
        r.record(Path::new("/a"), 1000, &out());
        let back = RecentFolders::from_json(Some(&r.to_json()));
        assert_eq!(back, r);
        assert!(RecentFolders::from_json(Some("{nope")).is_empty());
        assert!(RecentFolders::from_json(None).is_empty());
        let hostile = r#"{"entries":[{"path":"rel","score":1.0,"last":1},{"path":"/ok","score":1.0,"last":1},{"path":"/nan","score":-1.0,"last":1}]}"#;
        assert_eq!(paths(&RecentFolders::from_json(Some(hostile)), 1), ["/ok"]);
    }
}
