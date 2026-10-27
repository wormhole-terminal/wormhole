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
use wormhole_core::git::attribution::{Attribution, Kind, Snapshot, TerminalId};
use wormhole_core::view::sidebar::RowState;

use crate::window::Workspace;

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
pub(crate) fn changes_of(id: TerminalId, cx: &App) -> Option<usize> {
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
            let mut moved = false;
            for session in sessions {
                let summary = &session.pane.summary;
                let kind = if summary.source.told() { Kind::Told } else { Kind::Plain };
                let working = summary.row_state() == RowState::Working;
                moved |= changes.0.track(session.id, session.git.root(), kind, working, now);
            }
            if moved {
                cx.notify();
            }
        });
    }

    /// Terminal `id` closed: its files are not counted any more.
    pub(in crate::window) fn forget_changes(&mut self, id: TerminalId, cx: &mut Context<Self>) {
        shared(cx).update(cx, |changes, cx| {
            if changes.0.forget(id) {
                cx.notify();
            }
        });
    }

    /// A status read came back with its snapshot, asked for at `asked`.
    pub(super) fn observe_changes(&mut self, snapshot: Snapshot, asked: Instant, cx: &mut Context<Self>) {
        shared(cx).update(cx, |changes, cx| {
            if changes.0.observe(snapshot, asked) {
                cx.notify();
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::time::{Duration, Instant};

    use gpui_kit::TestAppContext;
    use wormhole_core::git::attribution::Snapshot;
    use wormhole_core::git::{ChangeKind, FileChange, RepoStatus};

    use super::changes_of;
    use crate::window::test_harness::{draw, drive, hook, isolate_in, read_cx, sidebar_of, start};

    fn status(paths: &[&str]) -> RepoStatus {
        RepoStatus {
            root: "/repo".into(),
            branch: Some("main".into()),
            files: paths.iter().map(|path| FileChange::new(*path, ChangeKind::Modified)).collect(),
            base_rev: "HEAD".into(),
            compare: None,
            compare_missing: None,
            dirty: !paths.is_empty(),
            upstream: None,
            superproject: None,
        }
    }

    /// Two terminals in one repository: while an agent works in the first, files appear; its row
    /// counts them and the other terminal's does not, in the list the sidebar was told without a draw.
    #[gpui_kit::test]
    fn a_row_counts_the_files_its_own_terminal_changed(cx: &mut TestAppContext) {
        if isolate_in(module_path!(), "a_row_counts_the_files_its_own_terminal_changed") {
            return;
        }
        let window = start(cx, 2);
        draw(cx, &window);
        let rows = |cx: &mut TestAppContext| {
            let told = sidebar_of(cx, &window, |sidebar| {
                sidebar.input().rows.iter().map(|row| row.changes).collect::<Vec<_>>()
            });
            let read = read_cx(cx, &window, |ws, cx| {
                ws.row_infos(cx).iter().map(|row| row.changes).collect::<Vec<_>>()
            });
            assert_eq!(told, read, "the sidebar was told what the window reads");
            read
        };
        let started = Instant::now();
        drive(cx, &window, |ws, _, cx| {
            // Git has answered for both: the repository they share.
            for session in &mut ws.sessions {
                let key = (PathBuf::from("/repo/src"), None);
                let ticket = session.git.begin_refresh(key.clone()).expect("starts").ticket();
                session.git.finish_refresh(ticket, Some(&key), Ok(Some(status(&[]))));
            }
            ws.track_changes(cx);
            ws.observe_changes(Snapshot::take(&status(&[])).expect("against HEAD"), started, cx);
        });
        assert_eq!(rows(cx), [None, None]);

        hook(cx, &window, 1, "UserPromptSubmit", None);
        drive(cx, &window, |ws, _, cx| {
            ws.track_changes(cx);
            let edited = Snapshot::take(&status(&["src/a.rs", "src/b.rs"])).expect("against HEAD");
            ws.observe_changes(edited, started + Duration::from_secs(1), cx);
        });
        // The same repository, so git reports the same two files for both: only the agent's has them.
        assert_eq!(rows(cx), [Some(2), None]);
        draw(cx, &window);

        // Closing the terminal lets go of its count.
        let id = read_cx(cx, &window, |ws, _| ws.sessions[0].id);
        drive(cx, &window, |ws, window, cx| ws.close_session(0, window, cx));
        assert_eq!(cx.update(|cx| changes_of(id, cx)), None);
    }
}
