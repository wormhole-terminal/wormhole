//! Keeping what each terminal changed across restarts (Settings → Changed files → Remember after
//! restart): the model's claims go to the profile's database as they change, and come back with the
//! terminals at launch.
//!
//! What has been written is tracked per terminal, against that terminal's own revision in the model
//! (`Attribution::revision_of`), and by terminal rather than by window: a terminal that moves to
//! another window (a tear-off, a group sent to a window) takes its state with it, so a change not yet
//! written is written by the window it is in now, and a window rewrites only the terminals that
//! changed. A window that moves to another profile (`rehome`) starts again, for the database it keeps
//! its rows in is another.
//!
//! A terminal's claims are written once they have been quiet for a moment (or have waited too long
//! while they keep changing), and so is the first change after a long while without one. The tick asks
//! for it, and the way out writes what is waiting. Only the files a terminal changed in its repository
//! are written, by path: nothing a terminal printed, so "Don't keep this terminal's text" does not
//! apply to them. A terminal closed for good takes its rows with its session (`Store::save_window`);
//! turning the setting off deletes them all.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;
use std::time::{Duration, Instant};

use gpui_kit::*;
use wormhole_core::git::attribution::Attribution;
use wormhole_core::storage::store::ClaimRecord;

use super::attribution::shared;
use crate::app_state::{app, app_mut};
use crate::settings::settings_in;
use crate::window::Workspace;

/// How long a terminal's claims stay as they are before they are written.
const QUIET: Duration = Duration::from_secs(2);
/// The longest they wait while they keep changing (an agent editing all the time), and the longest
/// between two tries after one failed.
const LONGEST: Duration = Duration::from_secs(30);

/// What has been written down, per terminal, and for which profiles every row was deleted.
#[derive(Default)]
pub(super) struct Kept {
    terminals: HashMap<i64, Written>,
    /// Profiles whose rows were deleted because the setting is off (until something is written there).
    cleared: HashSet<String>,
}

struct Written {
    /// The terminal's revision that is in the database; `None` for nothing (never, or deleted).
    revision: Option<u64>,
    /// The revision last seen, and when it was first seen: the claims are quiet once it is old.
    seen: u64,
    changed_at: Instant,
    /// When it was last written, or first seen.
    at: Instant,
    /// When the last write failed (and nothing since has been written).
    failed_at: Option<Instant>,
}

impl Written {
    fn new(revision: Option<u64>, seen: u64, now: Instant) -> Self {
        Self { revision, seen, changed_at: now, at: now, failed_at: None }
    }
}

impl Kept {
    /// Which of `terminals` (each with its revision in the model) are due to be written at `now`.
    fn due(&mut self, terminals: &[(i64, u64)], now: Instant, force: bool) -> Vec<i64> {
        let mut due = Vec::new();
        for &(id, revision) in terminals {
            // Not seen before: nothing to write if it has no files, else it was never written.
            let written = self
                .terminals
                .entry(id)
                .or_insert_with(|| Written::new((revision == 0).then_some(0), revision, now));
            if written.revision == Some(revision) {
                continue;
            }
            if written.seen != revision {
                (written.seen, written.changed_at) = (revision, now);
            }
            let retry = written.failed_at.is_none_or(|at| now.duration_since(at) >= LONGEST);
            let ready = force
                || (retry
                    && (now.duration_since(written.changed_at) >= QUIET
                        || now.duration_since(written.at) >= LONGEST));
            if ready {
                due.push(id);
            }
        }
        due
    }

    /// `terminals` were written as of their revisions.
    fn written(&mut self, terminals: &[(i64, u64)], profile: &str, now: Instant) {
        for &(id, revision) in terminals {
            self.terminals.insert(id, Written::new(Some(revision), revision, now));
        }
        self.cleared.remove(profile);
    }

    /// Writing `terminals` failed: not tried again for a while.
    fn failed(&mut self, terminals: &[i64], now: Instant) {
        for id in terminals {
            if let Some(written) = self.terminals.get_mut(id) {
                written.failed_at = Some(now);
            }
        }
    }

    /// The database of `profile` no longer has `terminals`' rows (they moved to another profile; or
    /// the setting went off and every row was deleted): they are written again when they are due.
    pub(super) fn invalidate(&mut self, terminals: &[i64]) {
        for id in terminals {
            self.terminals.remove(id);
        }
    }

    /// A terminal is gone: nothing more is known of it.
    pub(super) fn remove(&mut self, id: i64) {
        self.terminals.remove(&id);
    }

    /// The setting is off for `profile` while `terminals` are in it: they have nothing written any
    /// more. True once, when the rows are to be deleted.
    fn must_clear(&mut self, terminals: &[i64], profile: &str) -> bool {
        self.invalidate(terminals);
        self.cleared.insert(profile.to_string())
    }
}

/// The rows of `ids`' claims, one per file.
fn rows(model: &Attribution, ids: &[i64]) -> Vec<ClaimRecord> {
    ids.iter()
        .filter_map(|&id| model.claims_of(id).map(|(root, paths)| (id, root, paths)))
        .flat_map(|(session_id, root, paths)| {
            let root = root.to_string_lossy().into_owned();
            paths.iter().map(move |path| ClaimRecord { session_id, root: root.clone(), path: path.clone() })
        })
        .collect()
}

impl Workspace {
    /// The terminals come back with what they had changed (a window opening at launch with its saved
    /// terminals). The first picture of each repository then prunes what has been committed meanwhile.
    pub(in crate::window) fn restore_changes(&mut self, cx: &mut Context<Self>) {
        if !settings_in(&self.profile, cx).sidebar_changes_remember {
            return;
        }
        let ids = self.session_ids();
        let claims = match app(cx).store_of(&self.profile).map(|store| store.claims(&ids)) {
            Some(Ok(claims)) => claims,
            Some(Err(err)) => {
                tracing::warn!("reading what the terminals had changed failed: {err:#}");
                return;
            }
            None => return,
        };
        let now = Instant::now();
        shared(cx).update(cx, |changes, cx| {
            let mut grouped: BTreeMap<(i64, String), Vec<String>> = BTreeMap::new();
            for claim in claims {
                grouped.entry((claim.session_id, claim.root)).or_default().push(claim.path);
            }
            let mut restored = false;
            for ((id, root), paths) in grouped {
                if changes.model.restore(id, Path::new(&root), paths) {
                    // Just read: it is what is in the database.
                    let revision = changes.model.revision_of(id);
                    changes.kept.terminals.insert(id, Written::new(Some(revision), revision, now));
                    restored = true;
                }
            }
            // The rows of the sidebar show them.
            if restored {
                cx.notify();
            }
        });
    }

    /// From the tick: write the claims of this window's terminals that are due, or delete every row
    /// while the setting is off.
    pub(in crate::window) fn keep_changes(&mut self, cx: &mut Context<Self>) {
        self.write_changes(false, cx);
    }

    /// On the way out: write what is waiting.
    pub(in crate::window) fn keep_changes_now(&mut self, cx: &mut Context<Self>) {
        self.write_changes(true, cx);
    }

    /// This window moved to another profile: its terminals' claims are not in the database it keeps
    /// its rows in now, so they are written there.
    pub(in crate::window) fn rehomed_changes(&mut self, cx: &mut Context<Self>) {
        let ids = self.session_ids();
        shared(cx).update(cx, |changes, _| changes.kept.invalidate(&ids));
    }

    fn write_changes(&mut self, force: bool, cx: &mut Context<Self>) {
        let settings = settings_in(&self.profile, cx);
        // With the switch off nothing shows the counts, and nothing is written for them.
        if self.discarded || !settings.sidebar_changes {
            return;
        }
        let remember = settings.sidebar_changes_remember;
        let (now, profile) = (Instant::now(), self.profile.to_string());
        let ids = self.session_ids();
        let entity = shared(cx);
        if !remember {
            if entity.update(cx, |changes, _| changes.kept.must_clear(&ids, &profile))
                && let Some(store) = app(cx).store_of(&self.profile)
                && let Err(err) = store.clear_claims()
            {
                tracing::warn!("forgetting what the terminals had changed failed: {err:#}");
            }
            return;
        }
        if app(cx).store_of(&self.profile).is_none() {
            return;
        }
        let due = entity.update(cx, |changes, _| {
            let revisions: Vec<(i64, u64)> =
                ids.iter().map(|&id| (id, changes.model.revision_of(id))).collect();
            let due = changes.kept.due(&revisions, now, force);
            if due.is_empty() {
                return None;
            }
            let rows = rows(&changes.model, &due);
            let written: Vec<(i64, u64)> = revisions.into_iter().filter(|(id, _)| due.contains(id)).collect();
            Some((due, written, rows))
        });
        let Some((due, written, rows)) = due else { return };
        let Some(store) = app_mut(cx).store_of_mut(&self.profile) else { return };
        match store.save_claims(&due, &rows) {
            Ok(()) => entity.update(cx, |changes, _| changes.kept.written(&written, &profile, now)),
            Err(err) => {
                tracing::warn!("keeping what the terminals changed failed: {err:#}");
                entity.update(cx, |changes, _| changes.kept.failed(&due, now));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{Kept, LONGEST, QUIET};

    fn secs(from: Instant, secs: f64) -> Instant {
        from + Duration::from_secs_f64(secs)
    }

    #[test]
    fn claims_are_written_once_they_have_been_quiet_and_not_before() {
        let (mut kept, t0) = (Kept::default(), Instant::now());
        // First seen with nothing: nothing to write.
        assert!(kept.due(&[(1, 0)], t0, false).is_empty());
        // Files appear: not yet, the agent is still editing.
        assert!(kept.due(&[(1, 1)], secs(t0, 1.0), false).is_empty());
        assert!(kept.due(&[(1, 2)], secs(t0, 2.5), false).is_empty(), "another change starts the wait again");
        assert!(kept.due(&[(1, 2)], secs(t0, 4.0), false).is_empty(), "a moment after the last one");
        assert_eq!(kept.due(&[(1, 2)], secs(t0, 2.5 + QUIET.as_secs_f64()), false), [1]);
        kept.written(&[(1, 2)], "p", secs(t0, 5.0));
        assert!(kept.due(&[(1, 2)], secs(t0, 6.0), false).is_empty(), "written: nothing new");
    }

    #[test]
    fn claims_that_never_stop_changing_are_written_at_the_longest_wait() {
        let (mut kept, t0) = (Kept::default(), Instant::now());
        kept.due(&[(1, 0)], t0, false);
        let mut at = 0.0;
        let mut revision = 0;
        let mut written_at = None;
        while at < 40.0 {
            at += 1.0;
            revision += 1;
            if !kept.due(&[(1, revision)], secs(t0, at), false).is_empty() {
                written_at = Some(at);
                break;
            }
        }
        assert_eq!(written_at, Some(LONGEST.as_secs_f64()), "a change a second for as long as it takes");
    }

    #[test]
    fn the_first_change_after_a_long_idle_is_written_at_once_and_force_always_is() {
        let (mut kept, t0) = (Kept::default(), Instant::now());
        kept.written(&[(1, 1)], "p", t0);
        // Idle for a minute, then a change: no wait.
        assert_eq!(kept.due(&[(1, 2)], secs(t0, 60.0), false), [1]);
        // Just written, then a change: waits for quiet.
        kept.written(&[(1, 2)], "p", secs(t0, 60.0));
        assert!(kept.due(&[(1, 3)], secs(t0, 61.0), false).is_empty());
        assert_eq!(kept.due(&[(1, 3)], secs(t0, 61.0), true), [1], "on the way out it does not wait");
    }

    #[test]
    fn two_terminals_are_judged_apart_and_one_changing_does_not_rewrite_the_other() {
        let (mut kept, t0) = (Kept::default(), Instant::now());
        kept.written(&[(1, 3), (2, 5)], "p", t0);
        // Only terminal 2 changed, anywhere in the app.
        assert!(kept.due(&[(1, 3), (2, 6)], secs(t0, 1.0), false).is_empty());
        assert_eq!(kept.due(&[(1, 3), (2, 6)], secs(t0, 3.5), false), [2]);
        // A terminal moved to another window carries its state: still waiting, not up to date.
        assert!(kept.due(&[(2, 6)], secs(t0, 3.6), true).contains(&2), "its pending change is not lost");
    }

    #[test]
    fn a_terminal_that_moved_profile_is_written_again_and_a_closed_one_is_forgotten() {
        let (mut kept, t0) = (Kept::default(), Instant::now());
        kept.written(&[(1, 4)], "a", t0);
        assert!(kept.due(&[(1, 4)], secs(t0, 5.0), true).is_empty(), "up to date in its database");
        kept.invalidate(&[1]);
        assert_eq!(kept.due(&[(1, 4)], secs(t0, 5.0), true), [1], "the other database has nothing");
        kept.remove(1);
        assert!(kept.terminals.is_empty());
    }

    #[test]
    fn a_failed_write_is_not_tried_again_every_tick() {
        let (mut kept, t0) = (Kept::default(), Instant::now());
        kept.due(&[(1, 0)], t0, false);
        assert_eq!(kept.due(&[(1, 1)], secs(t0, 40.0), false), [1]);
        kept.failed(&[1], secs(t0, 40.0));
        assert!(kept.due(&[(1, 1)], secs(t0, 41.0), false).is_empty());
        assert!(kept.due(&[(1, 1)], secs(t0, 60.0), false).is_empty());
        assert_eq!(kept.due(&[(1, 1)], secs(t0, 70.0), false), [1], "after the longest wait");
        assert_eq!(kept.due(&[(1, 1)], secs(t0, 41.0), true), [1], "the way out tries regardless");
    }

    #[test]
    fn turning_the_setting_off_deletes_the_rows_once_per_profile() {
        let (mut kept, t0) = (Kept::default(), Instant::now());
        kept.written(&[(1, 2), (2, 2)], "a", t0);
        assert!(kept.must_clear(&[1], "a"));
        assert!(!kept.must_clear(&[2], "a"), "the rows of the profile are gone already");
        assert!(kept.must_clear(&[3], "b"), "another profile has its own");
        // Both are written again when the setting is on again and something is due.
        assert_eq!(kept.due(&[(1, 2), (2, 2)], secs(t0, 1.0), true), [1, 2]);
        kept.written(&[(1, 2)], "a", secs(t0, 2.0));
        assert!(kept.must_clear(&[1], "a"), "and the next time off deletes again");
    }
}
