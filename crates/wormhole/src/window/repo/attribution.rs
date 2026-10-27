//! Whose changes a terminal's row counts. Git names the files a repository has changed, the same for
//! every terminal in it; what the row shows is the files this terminal changed, which
//! `wormhole_core::git::attribution` works out from when each terminal was working.
//!
//! The model is the app's one, in an entity of its own: two windows can hold terminals of one
//! repository, and each learns of a count that moves in the other. The windows feed it (the tick says
//! which terminals are working, a status read hands in its snapshot) and the sidebar reads it
//! ([`changes_of`]); a count that moves notifies the entity, which every window observes to tell its
//! sidebar, never while it draws. What a terminal changed is also kept across restarts, by
//! `remembered`.

use std::time::Instant;

use gpui_kit::*;
use wormhole_core::config::{PlainChanges, SharedChanges};
use wormhole_core::git::attribution::{Attribution, Kind, Snapshot};
use wormhole_core::view::sidebar::RowState;
use wormhole_core::view::sidebar::command::ends_by_itself;

use super::remembered::Kept;
use crate::settings::settings_in;
use crate::window::Workspace;
use crate::window::session::Session;

/// The model, as an entity that notifies when a terminal's count changes, and what of it has been
/// written down (`remembered`).
#[derive(Default)]
pub(super) struct Changes {
    pub(super) model: Attribution,
    pub(super) kept: Kept,
}

/// The app's one [`Changes`], made when the first window asks.
struct Shared(Entity<Changes>);

impl Global for Shared {}

pub(super) fn shared(cx: &mut App) -> Entity<Changes> {
    if let Some(shared) = cx.try_global::<Shared>() {
        return shared.0.clone();
    }
    let changes = cx.new(|_| Changes::default());
    cx.set_global(Shared(changes.clone()));
    changes
}

/// The uncommitted files terminal `id` changed, if any: what its sidebar row counts.
pub(crate) fn changes_of(id: i64, shared: SharedChanges, cx: &App) -> Option<usize> {
    cx.try_global::<Shared>()?.0.read(cx).model.changes_of(id, shared)
}

impl Workspace {
    /// Hear of every terminal's count moving, in any window, and let go of this window's terminals
    /// when it closes.
    pub(in crate::window) fn watch_changes(&mut self, cx: &mut Context<Self>) {
        let changes = shared(cx);
        cx.observe(&changes, |this, _, cx| this.sync_sidebar_soon(cx)).detach();
        cx.on_release(|this, cx| {
            let ids = this.session_ids();
            shared(cx).update(cx, |changes, cx| {
                let moved = ids.into_iter().fold(false, |moved, id| changes.model.forget(id) | moved);
                if moved {
                    cx.notify();
                }
            });
        })
        .detach();
    }

    /// Tell the model, from the tick, where each terminal is and whether it is working. After the
    /// summaries are read, so it is the status the row shows.
    pub(in crate::window) fn track_changes(&mut self, cx: &mut Context<Self>) {
        let (sessions, now) = (&self.sessions, Instant::now());
        let plain = settings_in(&self.profile, cx).sidebar_changes_plain;
        shared(cx).update(cx, |changes, cx| {
            let moved =
                sessions.iter().fold(false, |moved, session| track(changes, session, plain, now, cx) | moved);
            if moved {
                cx.notify();
            }
        });
    }

    /// [`track_changes`](Self::track_changes) for the terminal at `ix` alone: when git has just said
    /// which repository it is in, so the model knows before the picture that comes with the answer.
    fn track_changes_of(&mut self, ix: usize, cx: &mut Context<Self>) {
        let (session, now) = (&self.sessions[ix], Instant::now());
        let plain = settings_in(&self.profile, cx).sidebar_changes_plain;
        shared(cx).update(cx, |changes, cx| {
            if track(changes, session, plain, now, cx) {
                cx.notify();
            }
        });
    }

    /// Terminal `id` closed: its files are not counted any more.
    pub(in crate::window) fn forget_changes(&mut self, id: i64, cx: &mut Context<Self>) {
        shared(cx).update(cx, |changes, cx| {
            if changes.model.forget(id) {
                cx.notify();
            }
        });
    }

    /// A status read came back with its picture of the disk, for the terminal at `ix`.
    pub(super) fn observe_changes(&mut self, ix: usize, snapshot: Snapshot, cx: &mut Context<Self>) {
        self.track_changes_of(ix, cx);
        shared(cx).update(cx, |changes, cx| {
            if changes.model.observe(snapshot) {
                cx.notify();
            }
        });
    }
}

/// Tell the model about `session`. Returns whether a count changed.
///
/// A terminal that tells its status is working while the row says so. A plain one is working by
/// `plain` (Settings → Changed files → Plain terminals), read as the terminal is tracked, so a change
/// of the setting does not reach back to what was counted before. By default it is while a command
/// that ends by itself runs in front: a pager, an editor, `ssh` or a dev server stays open and would
/// take every edit made outside the app. Whether something is in front is asked of the system now
/// (one call), because the command's line is read only every fifth tick and a command that ended in
/// between would otherwise go on working for two seconds; which command it is comes from the summary,
/// up to that old.
fn track(
    changes: &mut Changes,
    session: &Session,
    plain: PlainChanges,
    now: Instant,
    cx: &Context<Changes>,
) -> bool {
    let summary = &session.pane.summary;
    let (kind, working) = if summary.source.told() {
        (Kind::Told, summary.row_state() == RowState::Working)
    } else {
        let in_front = || {
            session.terminal().read(cx).program_in_foreground_now().unwrap_or(summary.program_in_foreground)
        };
        let ends = summary.foreground.as_ref().is_some_and(|front| ends_by_itself(&front.command));
        let working = plain_is_working(plain, ends, in_front);
        (Kind::Plain, working)
    };
    let place = session.git.place(summary.cwd.as_deref());
    changes.model.track(session.id, place, kind, working, now)
}

/// Whether a plain terminal is working, by `plain`: `ends` is whether the command in front ends by
/// itself, `in_front` asks the system whether any program is (asked only when the answer needs it).
fn plain_is_working(plain: PlainChanges, ends: bool, in_front: impl FnOnce() -> bool) -> bool {
    match plain {
        PlainChanges::None => false,
        PlainChanges::Any => in_front(),
        PlainChanges::Finishing => ends && in_front(),
    }
}

#[cfg(test)]
mod tests {
    use wormhole_core::config::PlainChanges;

    use super::plain_is_working;

    #[test]
    fn a_plain_terminal_works_by_the_setting() {
        let front = || true;
        let empty = || false;
        // A build runs: the default and "any" count it, "none" never does.
        assert!(plain_is_working(PlainChanges::Finishing, true, front));
        assert!(plain_is_working(PlainChanges::Any, true, front));
        assert!(!plain_is_working(PlainChanges::None, true, front));
        // An editor stays open: only "any" counts it.
        assert!(!plain_is_working(PlainChanges::Finishing, false, front));
        assert!(plain_is_working(PlainChanges::Any, false, front));
        assert!(!plain_is_working(PlainChanges::None, false, front));
        // Nothing in front (the shell at its prompt), or a command that ended since it was read.
        assert!(!plain_is_working(PlainChanges::Finishing, true, empty));
        assert!(!plain_is_working(PlainChanges::Any, false, empty));
        // The system is not asked when the answer doesn't depend on it.
        assert!(!plain_is_working(PlainChanges::None, true, || unreachable!("never asked")));
        assert!(!plain_is_working(PlainChanges::Finishing, false, || unreachable!("never asked")));
    }
}
