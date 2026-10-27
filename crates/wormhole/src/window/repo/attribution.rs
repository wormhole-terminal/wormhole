//! Whose changes a terminal's row counts. Git names the files a repository has changed, the same for
//! every terminal in it; what the row shows is the files this terminal changed, which
//! `wormhole_core::git::attribution` works out from when each terminal was working.
//!
//! The model is the app's one, in an entity of its own: two windows can hold terminals of one
//! repository, and each learns of a count that moves in the other. The windows feed it (the tick says
//! which terminals are working, a status read hands in its snapshot) and the sidebar reads it
//! ([`changes_of`]); a count that moves notifies the entity, which every window observes to tell its
//! sidebar, never while it draws.

use std::time::Instant;

use gpui_kit::*;
use wormhole_core::git::attribution::{Attribution, Kind, Snapshot};
use wormhole_core::view::sidebar::RowState;
use wormhole_core::view::sidebar::command::ends_by_itself;

use crate::window::Workspace;
use crate::window::session::Session;

/// The model, as an entity that notifies when a terminal's count changes.
#[derive(Default)]
struct Changes(Attribution);

/// The app's one [`Changes`], made when the first window asks.
struct Shared(Entity<Changes>);

impl Global for Shared {}

fn shared(cx: &mut App) -> Entity<Changes> {
    if let Some(shared) = cx.try_global::<Shared>() {
        return shared.0.clone();
    }
    let changes = cx.new(|_| Changes::default());
    cx.set_global(Shared(changes.clone()));
    changes
}

/// The uncommitted files terminal `id` changed, if any: what its sidebar row counts.
pub(crate) fn changes_of(id: i64, cx: &App) -> Option<usize> {
    cx.try_global::<Shared>()?.0.read(cx).0.changes_of(id)
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
                let moved = ids.into_iter().fold(false, |moved, id| changes.0.forget(id) | moved);
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
        shared(cx).update(cx, |changes, cx| {
            let moved =
                sessions.iter().fold(false, |moved, session| track(changes, session, now, cx) | moved);
            if moved {
                cx.notify();
            }
        });
    }

    /// [`track_changes`](Self::track_changes) for the terminal at `ix` alone: when git has just said
    /// which repository it is in, so the model knows before the picture that comes with the answer.
    fn track_changes_of(&mut self, ix: usize, cx: &mut Context<Self>) {
        let (session, now) = (&self.sessions[ix], Instant::now());
        shared(cx).update(cx, |changes, cx| {
            if track(changes, session, now, cx) {
                cx.notify();
            }
        });
    }

    /// Terminal `id` closed: its files are not counted any more.
    pub(in crate::window) fn forget_changes(&mut self, id: i64, cx: &mut Context<Self>) {
        shared(cx).update(cx, |changes, cx| {
            if changes.0.forget(id) {
                cx.notify();
            }
        });
    }

    /// A status read came back with its picture of the disk, for the terminal at `ix`.
    pub(super) fn observe_changes(&mut self, ix: usize, snapshot: Snapshot, cx: &mut Context<Self>) {
        self.track_changes_of(ix, cx);
        shared(cx).update(cx, |changes, cx| {
            if changes.0.observe(snapshot) {
                cx.notify();
            }
        });
    }
}

/// Tell the model about `session`. Returns whether a count changed.
///
/// A terminal that tells its status is working while the row says so. A plain one is working while a
/// command that ends by itself runs in front: whether something is in front is asked of the system
/// now (one call), because the command's line is read only every fifth tick and a command that
/// ended in between would otherwise go on working for two seconds; which command it is comes from the
/// summary, up to that old. A pager, an editor, `ssh` or a dev server stays open and would take every
/// edit made outside the app, so they are not work.
fn track(changes: &mut Changes, session: &Session, now: Instant, cx: &Context<Changes>) -> bool {
    let summary = &session.pane.summary;
    let (kind, working) = if summary.source.told() {
        (Kind::Told, summary.row_state() == RowState::Working)
    } else {
        let in_front =
            session.terminal().read(cx).program_in_foreground_now().unwrap_or(summary.program_in_foreground);
        let ends = summary.foreground.as_ref().is_some_and(|front| ends_by_itself(&front.command));
        (Kind::Plain, in_front && ends)
    };
    let place = session.git.place(summary.cwd.as_deref());
    changes.0.track(session.id, place, kind, working, now)
}
