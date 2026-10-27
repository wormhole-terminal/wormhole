//! Keeping what each terminal changed across restarts (Settings → Changed files → Remember after
//! restart): the model's claims go to the profile's database as they change, and come back with the
//! terminals at launch.
//!
//! A window writes its own terminals' claims, debounced: not per tick but once its claims have been
//! quiet for a moment (or have waited too long), from the tick and once more on the way out. What is
//! written is the files the terminal changed in its repository, by path: nothing a terminal printed,
//! so "Don't keep this terminal's text" does not apply to it. A terminal closed for good takes its rows
//! with its session (`Store::save_window`); turning the setting off deletes them all.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;
use std::time::{Duration, Instant};

use gpui_kit::*;
use wormhole_core::git::attribution::Attribution;
use wormhole_core::storage::store::ClaimRecord;

use super::attribution::shared;
use crate::app_state::{app, app_mut};
use crate::settings::settings_in;
use crate::window::Workspace;

/// How long the claims stay as they are before they are written.
const QUIET: Duration = Duration::from_secs(2);
/// The longest they wait while they keep changing (an agent editing all the time).
const LONGEST: Duration = Duration::from_secs(30);

/// What has been written down, per window (the terminals of a window are written by it).
#[derive(Default)]
pub(super) struct Kept {
    windows: HashMap<i64, Written>,
    /// The model's revision last seen, and when it changed to it.
    seen: u64,
    changed_at: Option<Instant>,
}

struct Written {
    /// The model's revision that is in the database; `None` for nothing (never, or deleted).
    revision: Option<u64>,
    at: Instant,
}

impl Kept {
    /// Whether the window's claims, as of the model's `revision`, are due to be written at `now`.
    fn due(&mut self, window: i64, revision: u64, now: Instant, force: bool) -> bool {
        let written = self.windows.entry(window).or_insert(Written { revision: None, at: now });
        if written.revision == Some(revision) {
            return false;
        }
        if revision != self.seen || self.changed_at.is_none() {
            (self.seen, self.changed_at) = (revision, Some(now));
        }
        let quiet = self.changed_at.is_some_and(|at| now.duration_since(at) >= QUIET);
        force || quiet || now.duration_since(written.at) >= LONGEST
    }

    fn written(&mut self, window: i64, revision: Option<u64>, now: Instant) {
        self.windows.insert(window, Written { revision, at: now });
    }

    /// The window's rows must be deleted: true once, until something is written again.
    fn must_clear(&mut self, window: i64, now: Instant) -> bool {
        let cleared = self.windows.get(&window).is_some_and(|written| written.revision.is_none());
        if !cleared {
            self.written(window, None, now);
        }
        !cleared
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
        let (window, now) = (self.window_id, Instant::now());
        shared(cx).update(cx, |changes, _| {
            let mut grouped: BTreeMap<(i64, String), Vec<String>> = BTreeMap::new();
            for claim in claims {
                grouped.entry((claim.session_id, claim.root)).or_default().push(claim.path);
            }
            for ((id, root), paths) in grouped {
                changes.model.restore(id, Path::new(&root), paths);
            }
            // Just read: it is what is in the database.
            changes.kept.written(window, Some(changes.model.revision()), now);
        });
    }

    /// From the tick: write this window's terminals' claims when they are due, or delete every row
    /// while the setting is off.
    pub(in crate::window) fn keep_changes(&mut self, cx: &mut Context<Self>) {
        self.write_changes(false, cx);
    }

    /// On the way out: write what is waiting.
    pub(in crate::window) fn keep_changes_now(&mut self, cx: &mut Context<Self>) {
        self.write_changes(true, cx);
    }

    fn write_changes(&mut self, force: bool, cx: &mut Context<Self>) {
        let (window, now) = (self.window_id, Instant::now());
        let remember = settings_in(&self.profile, cx).sidebar_changes_remember;
        let ids = self.session_ids();
        let entity = shared(cx);
        if !remember {
            if entity.update(cx, |changes, _| changes.kept.must_clear(window, now))
                && let Some(store) = app(cx).store_of(&self.profile)
                && let Err(err) = store.clear_claims()
            {
                tracing::warn!("forgetting what the terminals had changed failed: {err:#}");
            }
            return;
        }
        let due = entity.update(cx, |changes, _| {
            let revision = changes.model.revision();
            changes.kept.due(window, revision, now, force).then(|| (revision, rows(&changes.model, &ids)))
        });
        let Some((revision, rows)) = due else { return };
        let Some(store) = app_mut(cx).store_of_mut(&self.profile) else { return };
        match store.save_claims(&ids, &rows) {
            Ok(()) => entity.update(cx, |changes, _| changes.kept.written(window, Some(revision), now)),
            Err(err) => tracing::warn!("keeping what the terminals changed failed: {err:#}"),
        }
    }
}
