//! What devctl's History commands do (`history`, `commit`, `commit-file`, `picker history`): the selected
//! terminal's History, through the window's `History` entity like the keys do.

use gpui_kit::*;
use wormhole_core::config::TabId;

use super::{History, PickerHost};
use crate::window::Workspace;

/// Runs `f` on the window's History for the selected terminal.
fn run<R>(
    ws: &Workspace,
    cx: &mut Context<Workspace>,
    f: impl FnOnce(&mut History, i64, &Entity<crate::terminal_view::TerminalView>, &mut Context<History>) -> R,
) -> Option<R> {
    let history = ws.feature::<History>()?;
    let (session, terminal) = (ws.active_session().id, ws.active_terminal());
    let facts = ws.facts(cx);
    Some(history.update(cx, |history, hcx| {
        history.follow(&facts);
        f(history, session, &terminal, hcx)
    }))
}

/// `history [branch]`: the History tab; on `branch` when one is named (`None` for HEAD goes back).
pub(crate) fn show(ws: &mut Workspace, branch: Option<Option<String>>, cx: &mut Context<Workspace>) {
    ws.show_tab(TabId::History, cx);
    if let Some(branch) = branch {
        run(ws, cx, |history, session, _, cx| {
            history.picker_mut().close();
            history.pick(session, branch, cx);
        });
    }
}

/// `commit <n>`: show or fold the n-th commit's files.
pub(crate) fn toggle_commit(ws: &Workspace, ix: usize, cx: &mut Context<Workspace>) {
    run(ws, cx, |history, session, _, cx| {
        let sha = history.of.get(&session).and_then(|state| state.commits.get(ix).map(|c| c.sha.clone()));
        if let Some(sha) = sha {
            history.toggle_commit(session, sha, cx);
        }
    });
}

/// `commit-file <n>`: the diff of the n-th file of the open commit.
pub(crate) fn commit_file(ws: &Workspace, ix: usize, cx: &mut Context<Workspace>) {
    let Some(changes) = ws.feature::<crate::window::features::git::changes::Changes>() else { return };
    run(ws, cx, |history, session, _, cx| {
        let Some(state) = history.of.get(&session) else { return };
        let Some(sha) = state.expanded.clone() else { return };
        let commit = state.commits.iter().find(|c| c.sha == sha).cloned();
        let file = state.loaded(&sha).and_then(|f| f.get(ix).cloned());
        if let (Some(commit), Some(file)) = (commit, file) {
            history.open_commit_file(session, commit, file, &changes, cx);
        }
    });
}

/// `picker history [query]`: the branch select open, with `query` typed into its search box.
pub(crate) fn picker(ws: &mut Workspace, query: &str, window: &mut Window, cx: &mut Context<Workspace>) {
    ws.show_tab(TabId::History, cx);
    let query = query.to_string();
    run(ws, cx, |history, session, terminal, cx| {
        if !history.picker().is_open() {
            history.picker_mut().toggle(session, terminal, window, cx);
        }
        history.picker().search().update(cx, |search, cx| search.set_value(query, window, cx));
    });
}

/// Whether a log read is under way for any of the window's terminals.
pub(crate) fn busy(ws: &Workspace, cx: &App) -> bool {
    ws.feature::<History>().is_some_and(|history| history.read(cx).of.values().any(|state| state.loading()))
}
