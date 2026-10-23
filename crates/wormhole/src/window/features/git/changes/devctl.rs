//! What devctl's Changes commands do (`changes`, `diff`, `split`, `compare`, `picker`, `comment`, ...):
//! the selected terminal's Changes, through the window's `Changes` entity like the keys do.

use gpui_kit::*;
use wormhole_core::config::TabId;

use super::{Changes, PickerHost};
use crate::window::Workspace;

/// Runs `f` on the window's Changes for the selected terminal: its id and its terminal.
fn run<R>(
    ws: &Workspace,
    cx: &mut Context<Workspace>,
    f: impl FnOnce(&mut Changes, i64, &Entity<crate::terminal_view::TerminalView>, &mut Context<Changes>) -> R,
) -> Option<R> {
    let changes = ws.feature::<Changes>()?;
    let (session, terminal) = (ws.active_session().id, ws.active_terminal());
    Some(changes.update(cx, |changes, ccx| f(changes, session, &terminal, ccx)))
}

/// `changes`: the Changes tab.
pub(crate) fn show(ws: &mut Workspace, cx: &mut Context<Workspace>) {
    ws.show_tab(TabId::Changes, cx);
}

/// `diff <n>`: the n-th changed file's diff.
pub(crate) fn open_nth(ws: &Workspace, ix: usize, cx: &mut Context<Workspace>) {
    let file = ws.active_session().git.status.as_ref().and_then(|s| s.files.get(ix).cloned());
    let Some(file) = file else { return };
    let Some(target) = ws.feature::<Changes>().and_then(|c| c.read(cx).target(ws.active_session().id)) else {
        return;
    };
    run(ws, cx, |changes, session, _, cx| {
        changes.open_change(session, file, &target, cx);
    });
}

/// `split`: Split or Unified.
pub(crate) fn toggle_split(ws: &Workspace, cx: &mut Context<Workspace>) {
    run(ws, cx, |changes, _, _, cx| changes.toggle_split(cx));
}

/// `diff-scroll-x <n>`: the code scrolled sideways.
pub(crate) fn scroll_x(ws: &Workspace, delta: f32, cx: &mut Context<Workspace>) {
    run(ws, cx, |changes, _, _, cx| changes.scroll_diff_by(delta, cx));
}

/// `compare <branch>`: compare with a branch (`None` stops).
pub(crate) fn compare(ws: &Workspace, branch: Option<String>, cx: &mut Context<Workspace>) {
    run(ws, cx, |changes, session, _, cx| {
        changes.picker.close();
        changes.pick(session, branch, cx);
    });
}

/// `picker`: the compare picker open, with `query` typed into its search box.
pub(crate) fn picker(ws: &mut Workspace, query: &str, window: &mut Window, cx: &mut Context<Workspace>) {
    ws.show_tab(TabId::Changes, cx);
    let query = query.to_string();
    run(ws, cx, |changes, session, terminal, cx| {
        if !changes.picker.is_open() {
            changes.picker.toggle(session, terminal, window, cx);
        }
        changes.picker.search().update(cx, |search, cx| search.set_value(query, window, cx));
    });
}

/// `comment <text>`: the open dialog's field set to `text`, then saved and closed.
pub(crate) fn comment(
    ws: &Workspace,
    text: &str,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) -> Result<String, &'static str> {
    run(ws, cx, |changes, _, _, cx| changes.devctl_comment(text, window, cx)).ok_or("no changes feature")?
}
