//! What devctl's note and task commands do (`note-add`, `task-seed`, ...): the selected terminal's
//! notes and tasks, through the window's `Notes` entity like the keys do.
use gpui_kit::*;

use super::{Notes, selection_or_paragraph};
use crate::window::Workspace;

/// Runs `f` on the window's notes for the selected terminal: its id and the panel's width.
fn run<R>(
    ws: &Workspace,
    cx: &mut Context<Workspace>,
    f: impl FnOnce(&mut Notes, i64, f32, &mut Context<Notes>) -> R,
) -> Option<R> {
    let notes = ws.feature::<Notes>()?;
    let (session, panel) = (ws.active_session().id, ws.columns().panel);
    Some(notes.update(cx, |notes, ncx| f(notes, session, panel, ncx)))
}

/// The selected terminal's Notes list, to scroll.
pub(crate) fn list(ws: &Workspace, cx: &App) -> Option<ListState> {
    let notes = ws.feature::<Notes>()?;
    let of = notes.read(cx).of.get(&ws.active_session().id)?;
    Some(of.list.rows.state())
}

/// `note <text>`: the first note holds `text`, made if there is none.
pub(crate) fn set_first_note(ws: &Workspace, text: String, window: &mut Window, cx: &mut Context<Workspace>) {
    run(ws, cx, |n, session, panel, cx| n.devctl_set_first_note(session, text, panel, window, cx));
}

/// `task <text>`: a finished next step at the end.
pub(crate) fn add_task(ws: &Workspace, text: &str, cx: &mut Context<Workspace>) {
    run(ws, cx, |n, session, _, cx| n.add_task(session, text, cx));
}

/// `stash`: the prompt typed in the terminal moves into a new note.
pub(crate) fn stash_prompt(ws: &Workspace, window: &mut Window, cx: &mut Context<Workspace>) {
    let terminal = ws.active_terminal();
    run(ws, cx, |n, session, panel, cx| n.stash_prompt(session, &terminal, panel, window, cx));
}

/// `send`, `submit`: the focused note goes to the terminal.
pub(crate) fn send_focused_note(ws: &Workspace, submit: bool, cx: &mut Context<Workspace>) {
    run(ws, cx, |n, session, _, cx| n.devctl_send_focused_note(session, submit, cx));
}

/// `task-seed <open> <done>`: that many next steps.
pub(crate) fn seed_tasks(ws: &Workspace, open: usize, done: usize, cx: &mut Context<Workspace>) {
    run(ws, cx, |n, session, _, cx| n.devctl_seed_tasks(session, open, done, cx));
}

/// `task-done <n>`: tick the n-th next step.
pub(crate) fn toggle_task(ws: &Workspace, ix: usize, cx: &mut Context<Workspace>) {
    run(ws, cx, |n, session, _, cx| n.devctl_toggle_task(session, ix, cx));
}

/// `note-add <text>`: a note at the end, open.
pub(crate) fn add_note(ws: &Workspace, text: String, window: &mut Window, cx: &mut Context<Workspace>) {
    run(ws, cx, |n, session, panel, cx| n.devctl_add_note(session, text, panel, window, cx));
}

/// `note-toggle <n>`: fold the n-th note.
pub(crate) fn toggle_note(ws: &Workspace, ix: usize, cx: &mut Context<Workspace>) {
    run(ws, cx, |n, session, _, cx| n.devctl_toggle_note(session, ix, cx));
}

/// `note-seed <count> <lines>`: that many notes of that many lines.
pub(crate) fn seed_notes(
    ws: &Workspace,
    count: usize,
    lines: usize,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    run(ws, cx, |n, session, panel, cx| n.devctl_seed_notes(session, count, lines, panel, window, cx));
}

impl Notes {
    /// Fill `session` with `count` notes of `lines` lines each, for measuring how the Notes tab copes
    /// with a long list.
    fn devctl_seed_notes(
        &mut self,
        session: i64,
        count: usize,
        lines: usize,
        panel: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        for n in 0..count {
            let text = (0..lines)
                .map(|l| match l % 5 {
                    4 => String::new(),
                    _ => {
                        format!("Note {n} line {l}: some text that is long enough to wrap in a narrow panel.")
                    }
                })
                .collect::<Vec<_>>()
                .join("\n");
            self.add_note(session, Some(text), false, panel, window, cx);
        }
    }

    /// Add a note with `text` at the end of the list, open.
    fn devctl_add_note(
        &mut self,
        session: i64,
        text: String,
        panel: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.reveal(session, cx);
        self.add_note(session, Some(text), false, panel, window, cx);
    }

    fn devctl_set_first_note(
        &mut self,
        session: i64,
        text: String,
        panel: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.of.get(&session).is_some_and(|of| of.notes.is_empty()) {
            self.add_note(session, None, false, panel, window, cx);
        }
        let Some(of) = self.of_mut(session) else { return };
        let Some(note) = of.notes.first() else { return };
        let state = note.state.clone();
        of.notes_collapsed = false;
        state.update(cx, |n, cx| n.set_value(SharedString::from(text), window, cx));
        self.focused = Some((session, 0));
    }

    /// ⌘↵ outside a note: send from the last focused note.
    fn devctl_send_focused_note(&mut self, session: i64, submit: bool, cx: &mut Context<Self>) {
        let Some((id, ix)) = self.focused.filter(|&(id, _)| id == session) else { return };
        let Some(note) = self.of.get(&id).and_then(|of| of.notes.get(ix)) else { return };
        let text = selection_or_paragraph(note.state.read(cx));
        self.send_text(session, text, submit, cx);
    }

    fn devctl_toggle_note(&mut self, session: i64, ix: usize, cx: &mut Context<Self>) {
        self.toggle_note(session, ix, cx);
    }

    fn devctl_toggle_task(&mut self, session: i64, ix: usize, cx: &mut Context<Self>) {
        self.toggle_task(session, ix, cx);
    }
}
