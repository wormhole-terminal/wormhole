//! Getting a diff and its rows, and keeping them current.
//!
//! Everything slow happens on the background executor: `git diff` and `git
//! show`, then tab expansion and syntax colors. The UI thread only starts the
//! work and applies the answer, and only if the view is still waiting for
//! exactly that request (see `job`): stepping through files quickly, switching
//! sessions or an edit landing mid-load can no longer put one file's lines under
//! another's name.

use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::sync::Arc;
use std::time::SystemTime;

use gpui_kit::component::highlighter::HighlightTheme;
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::highlight::Highlights;
use super::rows::{BuildKey, Row, RowStats, build_rows, syntax_fingerprint};
use super::{DiffView, Request};
use crate::settings::{settings_in, store_in};
use crate::window::Workspace;
use crate::window::features::git::changes::display_order;
use crate::window::features::git::job::{Ticket, Wanted};
use wormhole_core::git::{self, DiffLimits, FileChange, FileDiff, LineKind, RepoStatus, SubmoduleLog};
use wormhole_core::theme::tokens::ThemeTokens;

/// What a request fetches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kind {
    /// Read the diff and build its rows: opening, and re-reading after an edit.
    Load,
    /// Only build the rows again, for other settings, from the diff on hand.
    Rebuild,
}

/// A request to the job: what for, which diff and which settings. The kind keeps
/// a rebuild in flight from being mistaken for (and swallowing) an identical reload.
pub(super) type Ask = (Kind, Request, BuildKey);

/// Rows with what they were built with.
pub(super) struct Built {
    rows: Vec<Row>,
    stats: RowStats,
    key: BuildKey,
}

/// What the background half of a request found.
pub(super) enum Outcome {
    Loaded {
        diff: Arc<FileDiff>,
        stamp: Option<Stamp>,
        /// The file is in the working tree now (whatever revisions the diff compares), so
        /// Reveal in Finder has something to show.
        on_disk: bool,
        built: Built,
    },
    Rebuilt(Built),
    /// A submodule: the commits between its two sides.
    Submodule(Arc<SubmoduleLog>),
    /// With the stamp taken before reading, so a failure is not retried until the file changes.
    Failed {
        message: String,
        stamp: Option<Stamp>,
    },
}

/// The background half of a request.
pub(super) enum Work {
    Load(Request),
    Rebuild { diff: Arc<FileDiff>, path: String },
}

/// A working-tree file's modification time and size. When they differ from what
/// the diff was read from, the file was edited since.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Stamp {
    modified: Option<SystemTime>,
    len: u64,
}

impl Stamp {
    fn of(path: &Path) -> Option<Self> {
        Some(Self::from(&std::fs::metadata(path).ok()?))
    }
}

impl From<&std::fs::Metadata> for Stamp {
    fn from(meta: &std::fs::Metadata) -> Self {
        Self { modified: meta.modified().ok(), len: meta.len() }
    }
}

/// What a status refresh saw of the open diff's file, read in the same
/// background hop as the status.
pub(crate) struct Observed {
    path: PathBuf,
    stamp: Option<Stamp>,
}

impl Observed {
    pub(crate) fn of(path: PathBuf) -> Self {
        let stamp = Stamp::of(&path);
        Self { path, stamp }
    }
}

/// What a fresh status says about the open working-tree diff.
#[derive(Debug, PartialEq)]
pub(super) enum Check {
    Same,
    /// The file is no longer among the changes.
    Gone,
    /// The file changed (its entry in the status, the base it is compared with,
    /// or the file itself): read the diff again.
    Reload(FileChange),
}

/// Fetch (when asked to) and build. `None` when nobody waits for the answer any
/// more, which it finds out between its steps.
pub(super) fn run(
    work: Work,
    key: BuildKey,
    theme: &Arc<HighlightTheme>,
    wanted: &Wanted,
) -> Option<Outcome> {
    if !wanted.still_wanted() {
        return None;
    }
    match work {
        Work::Load(request) if request.file.submodule.is_some() => {
            let sub = request.file.submodule.as_ref()?;
            Some(match git::submodule_log(&request.root, &request.file.path, sub) {
                Ok(log) => Outcome::Submodule(log),
                Err(err) => Outcome::Failed { message: format!("{err:#}"), stamp: None },
            })
        }
        Work::Load(request) => {
            // One look at the file: whether it is there, and for a working-tree diff its stamp,
            // taken before reading so an edit that lands while git runs shows up as a newer
            // stamp at the next refresh.
            let meta = std::fs::metadata(request.root.join(&request.file.path)).ok();
            let on_disk = meta.as_ref().is_some_and(|meta| meta.is_file());
            let stamp = meta.filter(|_| request.spec.new.is_none()).map(|meta| Stamp::from(&meta));
            let diff =
                match git::diff_with_limits(&request.root, &request.file, &request.spec, &request.limits) {
                    Ok(diff) => diff,
                    Err(err) => return Some(Outcome::Failed { message: format!("{err:#}"), stamp }),
                };
            if !wanted.still_wanted() {
                return None;
            }
            let built = build(&diff, &request.file.path, key, theme);
            Some(Outcome::Loaded { diff: Arc::new(diff), stamp, on_disk, built })
        }
        Work::Rebuild { diff, path } => Some(Outcome::Rebuilt(build(&diff, &path, key, theme))),
    }
}

fn build(diff: &FileDiff, path: &str, key: BuildKey, theme: &Arc<HighlightTheme>) -> Built {
    let highlights = Highlights::for_diff(diff, path, theme);
    let (rows, stats) = build_rows(diff, key, highlights.as_ref());
    Built { rows, stats, key }
}

pub(super) fn file_counts(file: &FileChange) -> (usize, usize) {
    (file.added.unwrap_or(0) as usize, file.removed.unwrap_or(0) as usize)
}

fn count_lines(diff: &FileDiff) -> (usize, usize) {
    let lines = diff.hunks.iter().flat_map(|h| &h.lines);
    let count = |kind| lines.clone().filter(|l| l.kind == kind).count();
    (count(LineKind::Added), count(LineKind::Removed))
}

impl DiffView {
    /// Ask for the diff to be read (again) and built with `build`, superseding
    /// whatever was awaited. `None` when exactly this is already running.
    pub(super) fn begin_load(&mut self, build: BuildKey) -> Option<(Wanted, Work)> {
        let wanted = self.job.begin((Kind::Load, self.request.clone(), build))?;
        Some((wanted, Work::Load(self.request.clone())))
    }

    /// Ask for a diff that was too large to load unasked, without that limit. `None` when the
    /// diff on view is not one.
    pub(super) fn begin_load_anyway(&mut self, build: BuildKey) -> Option<(Wanted, Work)> {
        if !self.too_large() {
            return None;
        }
        self.request.limits = DiffLimits::LOAD_ANYWAY;
        // Nothing to keep on screen meanwhile; a failure then shows as the view's error.
        self.diff = None;
        self.loading = true;
        self.begin_load(build)
    }

    /// Ask for the rows to be built again from the diff on hand. `None` when
    /// there is none yet, or exactly this rebuild is already running.
    pub(super) fn begin_rebuild(&mut self, build: BuildKey) -> Option<(Wanted, Work)> {
        let diff = self.diff.clone()?;
        let wanted = self.job.begin((Kind::Rebuild, self.request.clone(), build))?;
        Some((wanted, Work::Rebuild { diff, path: self.request.file.path.clone() }))
    }

    /// Apply the answer to request `ticket`. `false` if it was stale (the view
    /// moved on to another request) or changes nothing on screen.
    pub(super) fn finish(&mut self, ticket: Ticket, outcome: Outcome) -> bool {
        if self.job.finish(ticket).is_none() {
            return false;
        }
        match outcome {
            Outcome::Loaded { diff, stamp, on_disk, built } => {
                // Rows on screen already: this is a refresh, so keep the place in them.
                let refresh = self.diff.is_some();
                // A diff too large to load has no lines to count: keep the status's counts.
                self.counts =
                    if diff.too_large { file_counts(&self.request.file) } else { count_lines(&diff) };
                self.sides = git::sides(&diff, self.request.file.kind);
                self.stamp = stamp;
                self.on_disk = on_disk;
                self.diff = Some(diff);
                self.submodule = None;
                self.loading = false;
                self.error = None;
                self.refresh_error = None;
                self.gone = false;
                self.install(built, refresh);
                true
            }
            Outcome::Rebuilt(built) => {
                self.install(built, false);
                true
            }
            Outcome::Submodule(log) => {
                let changed = self.submodule.as_ref() != Some(&log) || self.loading || self.error.is_some();
                self.submodule = Some(log);
                self.counts = (0, 0);
                (self.loading, self.error, self.refresh_error, self.gone) = (false, None, None, false);
                changed
            }
            Outcome::Failed { message, stamp } => {
                self.loading = false;
                // Without the stamp, every status poll would see the file as edited since and
                // run the failing load again. A real edit (or a new status entry) still retries.
                self.stamp = stamp;
                if self.diff.is_some() || self.submodule.is_some() {
                    // A refresh failed. What is on screen is still a true diff of an earlier
                    // state; the header says it is out of date.
                    tracing::warn!("diff refresh failed: {message}");
                    let message = SharedString::from(message);
                    let changed = self.refresh_error.as_ref() != Some(&message);
                    self.refresh_error = Some(message);
                    return changed;
                }
                self.error = Some(message.into());
                true
            }
        }
    }

    fn install(&mut self, built: Built, keep_place: bool) {
        let top = (keep_place && !self.rows.is_empty()).then(|| self.wrap_list.logical_scroll_top());
        self.stats = built.stats;
        self.wrap_list = ListState::new(built.rows.len(), ListAlignment::Top, px(400.0));
        if let Some(top) = top {
            // The list clamps it to the new row count.
            self.wrap_list.scroll_to(top);
        }
        self.rows = Rc::new(built.rows);
        self.built = Some(built.key);
    }

    /// The syntax fingerprint for the settings as of `revision`, worked out once per revision.
    pub(super) fn syntax(&mut self, revision: u64, tokens: &ThemeTokens) -> u64 {
        match self.syntax_seen {
            Some((seen, fingerprint)) if seen == revision => fingerprint,
            _ => {
                let fingerprint = syntax_fingerprint(tokens);
                self.syntax_seen = Some((revision, fingerprint));
                fingerprint
            }
        }
    }

    /// The file on disk, for a diff of the working tree: what `refresh_git`
    /// looks at to notice an edit. `None` for a commit's file.
    pub(crate) fn watched_path(&self) -> Option<PathBuf> {
        // A submodule has no file to edit: its folder's state is in the status.
        (self.is_working_tree() && self.request.file.submodule.is_none())
            .then(|| self.request.root.join(&self.request.file.path))
    }

    pub(super) fn is_working_tree(&self) -> bool {
        self.request.spec.new.is_none()
    }

    #[cfg(test)]
    fn split_for_test(&self) -> bool {
        self.built.is_some_and(|key| key.split)
    }

    /// What `status` (and what the refresh saw of the file) says about this diff.
    pub(super) fn check(&self, status: &RepoStatus, observed: Option<&Observed>) -> Check {
        // A commit's diff never changes. One still being read is compared at the
        // next refresh, against the stamp it ends with.
        if !self.is_working_tree() || self.request.root != status.root || self.job.is_pending() {
            return Check::Same;
        }
        let Some(file) = status.files.iter().find(|f| f.path == self.request.file.path) else {
            return Check::Gone;
        };
        let edited = observed
            .filter(|o| self.watched_path().as_ref() == Some(&o.path))
            .is_some_and(|o| o.stamp != self.stamp);
        let moved = *file != self.request.file || status.base_rev != self.request.spec.old;
        if self.gone || moved || edited { Check::Reload(file.clone()) } else { Check::Same }
    }
}

impl Workspace {
    /// The key rows would be built with now. `None` without a diff on view.
    pub(super) fn build_key(&mut self, cx: &App) -> Option<BuildKey> {
        let split_fits = self.diff_split_fits(cx);
        let store = store_in(&self.profile, cx);
        let view = self.diff_view.as_mut()?;
        Some(BuildKey::of(&store.settings, split_fits, view.syntax(store.revision, &store.tokens)))
    }

    /// Run `work` in the background and apply its answer to the view that asked.
    pub(super) fn spawn_diff_work(
        &mut self,
        wanted: Wanted,
        work: Work,
        build: BuildKey,
        cx: &mut Context<Self>,
    ) {
        let theme = store_in(&self.profile, cx).highlight.clone();
        let ticket = wanted.ticket();
        let task = cx.background_executor().spawn(async move { run(work, build, &theme, &wanted) });
        cx.spawn(async move |this, cx| {
            let Some(outcome) = task.await else { return };
            let _ = this.update(cx, |this, cx| {
                // Whatever is on view now takes the answer only if it is the one that asked.
                if this.diff_view.as_mut().is_some_and(|view| view.finish(ticket, outcome)) {
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// Build the rows again if a setting they depend on (layout, tab width,
    /// syntax colors) or the room for a split view changed. Called every tick and
    /// after a frame that found the room changed; cheap when nothing did.
    pub(crate) fn sync_diff(&mut self, cx: &mut Context<Self>) {
        let Some(want) = self.build_key(cx) else { return };
        let Some(view) = self.diff_view.as_mut() else { return };
        // A rebuild never supersedes a load in flight: that answer lands first
        // and the next tick sees whatever differs.
        if view.built == Some(want) || view.job.is_pending() {
            return;
        }
        let Some((wanted, work)) = view.begin_rebuild(want) else { return };
        self.spawn_diff_work(wanted, work, want, cx);
    }

    /// A status refresh for the active session finished: bring the open
    /// working-tree diff in line. The list ‹ › steps through follows the
    /// Changes list; a file that changed is read again in place, so the scroll
    /// position stays; one that is no longer changed is marked as such.
    pub(crate) fn refresh_open_diff(&mut self, observed: Option<Observed>, cx: &mut Context<Self>) {
        let (tree, group_untracked) = {
            let s = settings_in(&self.profile, cx);
            (s.changes_tree, s.group_untracked)
        };
        let reload = {
            let Some(view) = self.diff_view.as_mut() else { return };
            let Some(status) = self.sessions.get(self.active).and_then(|s| s.git.status.as_ref()) else {
                return;
            };
            if !view.is_working_tree() {
                return;
            }
            let mut notify = false;
            let order = display_order(status, tree, group_untracked);
            if view.siblings[..] != order[..] {
                view.siblings = Rc::new(order);
                notify = true;
            }
            let reload = match view.check(status, observed.as_ref()) {
                Check::Same => None,
                Check::Gone => {
                    notify |= !view.gone;
                    view.gone = true;
                    None
                }
                Check::Reload(file) => Some((file, status.base_rev.clone())),
            };
            if notify {
                cx.notify();
            }
            reload
        };
        if let Some((file, base_rev)) = reload {
            self.reload_diff(file, base_rev, cx);
        }
    }

    /// Read the open diff again for `file`, as the status now describes it.
    fn reload_diff(&mut self, file: FileChange, base_rev: String, cx: &mut Context<Self>) {
        let Some(build) = self.build_key(cx) else { return };
        let Some(view) = self.diff_view.as_mut() else { return };
        view.request.file = file;
        view.request.spec.old = base_rev;
        view.gone = false;
        let Some((wanted, work)) = view.begin_load(build) else { return };
        self.spawn_diff_work(wanted, work, build, cx);
        // The header's counts and the title follow the new file entry.
        cx.notify();
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod bench;
