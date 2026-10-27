//! What devctl's History commands do (`history`, `commit`, `commit-details`, `commit-copy`, `commit-file`,
//! `picker history`): the selected
//! terminal's History, through the window's `History` entity like the keys do.

use gpui_kit::*;
use wormhole_core::config::TabId;

use super::copy::CopyWhat;
use super::state::CommitInfo;
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
    let Some(changes) = ws.feature::<crate::features::git::changes::Changes>() else { return };
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

/// `commit-details`: what the details block of the open commit shows, one fact a line (`loading` and the
/// reason it `failed` too); `none` when no commit is open.
pub(crate) fn details(ws: &Workspace, cx: &mut Context<Workspace>) -> String {
    run(ws, cx, |history, session, _, _| {
        let Some(state) = history.of.get(&session) else { return "none".into() };
        let Some(sha) = state.expanded.clone() else { return "none".into() };
        let mut out = vec![format!("commit {sha}")];
        match state.details.get(&sha) {
            None | Some(CommitInfo::Loading) => out.push("loading".into()),
            Some(CommitInfo::Failed(err)) => out.push(format!("failed: {err}")),
            Some(CommitInfo::Loaded(shown)) => {
                let details = &shown.details;
                out.push(format!("subject: {}", details.subject()));
                let view = if state.body_expanded { "whole" } else { "compact" };
                let (compact, more) = shown.compact(40);
                out.push(format!(
                    "message: {view} (toggle {}; compact at 40 chars: {:?})",
                    if more { "offered" } else { "none" },
                    compact
                ));
                out.push(format!("body: {}", details.body().replace('\n', "\\n")));
                out.push(format!("author: {} | {}", details.author.line(), details.author.when()));
                if details.committer_differs() {
                    out.push(format!(
                        "committer: {} | {}",
                        details.committer.line(),
                        details.committer.when()
                    ));
                }
                for co in &details.co_authors {
                    out.push(format!("co-author: {} <{}>", co.name, co.email));
                }
                out.push(format!("parents: {}", details.parents.join(" ")));
                let refs: Vec<_> = details.refs.iter().map(|r| format!("{:?} {}", r.kind, r.name)).collect();
                out.push(format!("refs: {}", refs.join(", ")));
                out.push(format!("message cut: {}", details.message_cut));
            }
        }
        if let Some(files) = state.loaded(&sha) {
            let stat = wormhole_core::git::DiffStat::of(files);
            out.push(format!("changes: {} files +{} -{}", stat.files, stat.added, stat.removed));
        }
        out.join("\n")
    })
    .unwrap_or_else(|| "none".into())
}

/// `commit-more`: "Show more" / "Show less" under the open commit's message.
pub(crate) fn more(ws: &Workspace, cx: &mut Context<Workspace>) {
    run(ws, cx, |history, session, _, cx| history.toggle_body(session, cx));
}

/// `commit-copy <hash|short|message|subject|author> [n]`: what the commit's menu item does, for the open
/// commit or the n-th one of the log. The message and the author are read first when they have to be:
/// look at `clipboard` a moment later.
pub(crate) fn copy(
    ws: &Workspace,
    what: &str,
    nth: Option<usize>,
    window: &Window,
    cx: &mut Context<Workspace>,
) -> Result<(), String> {
    let what = match what {
        "hash" => CopyWhat::Hash,
        "short" => CopyWhat::ShortHash,
        "message" => CopyWhat::Message,
        "subject" => CopyWhat::Subject,
        "author" => CopyWhat::Author,
        other => return Err(format!("copy what? {other} (hash, short, message, subject or author)")),
    };
    run(ws, cx, |history, session, _, cx| {
        let state = history.of.get(&session).ok_or("no history for this terminal")?;
        let sha = match nth {
            Some(ix) => state.commits.get(ix).map(|c| c.sha.clone()),
            None => state.expanded.clone(),
        };
        let sha = sha.ok_or("no such commit (open one, or name its number)")?;
        history.copy_commit(session, &sha, what, window, cx);
        Ok(())
    })
    .unwrap_or_else(|| Err("History is off".into()))
}

/// `clipboard`: the text a copy put on the clipboard (a background instance's own, never the system's),
/// or `empty`.
pub(crate) fn clipboard(cx: &App) -> String {
    crate::platform::clipboard::read(cx).unwrap_or_else(|| "empty".into())
}
