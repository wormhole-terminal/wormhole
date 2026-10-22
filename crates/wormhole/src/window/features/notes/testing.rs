//! The window's notes as the tests reach them: by the index of a terminal in the window, through
//! the entity like everything else, so a test says what it does to a terminal's notes and not
//! where they are kept.
use gpui_kit::component::input::TextareaState;
use gpui_kit::*;

use super::Notes;
use crate::window::Workspace;

/// Runs `f` on terminal `ix`'s notes (the terminal needn't be the selected one).
fn at<R>(
    ws: &Workspace,
    ix: usize,
    cx: &mut Context<Workspace>,
    f: impl FnOnce(&mut Notes, i64, f32, &mut Context<Notes>) -> R,
) -> R {
    let notes = ws.feature::<Notes>().expect("the window has notes");
    let (session, panel) = (ws.sessions[ix].id, ws.columns().panel);
    notes.update(cx, |notes, ncx| f(notes, session, panel, ncx))
}

/// ⌥⌘N in terminal `ix`.
pub(in crate::window) fn new_note(
    ws: &Workspace,
    ix: usize,
    text: Option<&str>,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    at(ws, ix, cx, |n, session, panel, cx| n.new_note(session, text.map(str::to_string), panel, window, cx));
}

/// A note with `text` at the end of terminal `ix`'s list; its index.
pub(in crate::window) fn add_note(
    ws: &Workspace,
    ix: usize,
    text: &str,
    collapsed: bool,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) -> usize {
    at(ws, ix, cx, |n, session, panel, cx| {
        n.add_note(session, Some(text.to_string()), collapsed, panel, window, cx)
    })
}

/// Note `ix` of terminal `t` on screen with the keyboard (`to_end`: clicking its title).
pub(in crate::window) fn show_note(
    ws: &Workspace,
    t: usize,
    ix: usize,
    to_end: bool,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    at(ws, t, cx, |n, session, _, cx| n.show_note(session, ix, to_end, window, cx));
}

/// Clicking the title of note `ix` of terminal `t`.
pub(in crate::window) fn open_note(
    ws: &Workspace,
    t: usize,
    ix: usize,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) {
    at(ws, t, cx, |n, session, _, cx| n.open_note(session, ix, window, cx));
}

/// A next step added to terminal `ix`.
pub(in crate::window) fn add_task(ws: &Workspace, ix: usize, text: &str, cx: &mut Context<Workspace>) {
    at(ws, ix, cx, |n, session, _, cx| n.add_task(session, text, cx));
}

/// A task of the terminal `ix` as the test looks at it.
pub(in crate::window) fn tasks(ws: &Workspace, ix: usize, cx: &App) -> Vec<(String, bool)> {
    let notes = ws.feature::<Notes>().expect("the window has notes");
    let of = &notes.read(cx).of[&ws.sessions[ix].id];
    of.tasks.items().iter().map(|t| (t.text.to_string(), t.done)).collect()
}

/// Whether terminal `ix`'s Next steps section is folded.
pub(in crate::window) fn tasks_collapsed(ws: &Workspace, ix: usize, cx: &App) -> bool {
    let notes = ws.feature::<Notes>().expect("the window has notes");
    notes.read(cx).of[&ws.sessions[ix].id].tasks.collapsed
}

pub(in crate::window) fn notes_collapsed(ws: &Workspace, ix: usize, cx: &App) -> bool {
    let notes = ws.feature::<Notes>().expect("the window has notes");
    notes.read(cx).of[&ws.sessions[ix].id].notes_collapsed
}

pub(in crate::window) fn set_notes_collapsed(
    ws: &Workspace,
    ix: usize,
    collapsed: bool,
    cx: &mut Context<Workspace>,
) {
    at(ws, ix, cx, |n, session, _, _| n.of_mut(session).expect("a terminal").notes_collapsed = collapsed);
}

/// How many notes terminal `ix` has.
pub(in crate::window) fn note_count(ws: &Workspace, ix: usize, cx: &App) -> usize {
    let notes = ws.feature::<Notes>().expect("the window has notes");
    notes.read(cx).of[&ws.sessions[ix].id].notes.len()
}

/// Note `n` of terminal `ix`: its editor and whether it is folded.
pub(in crate::window) fn note(
    ws: &Workspace,
    ix: usize,
    n: usize,
    cx: &App,
) -> (Entity<TextareaState>, bool) {
    let notes = ws.feature::<Notes>().expect("the window has notes");
    let note = &notes.read(cx).of[&ws.sessions[ix].id].notes[n];
    (note.state.clone(), note.collapsed)
}

pub(in crate::window) fn set_note_collapsed(
    ws: &Workspace,
    ix: usize,
    n: usize,
    collapsed: bool,
    cx: &mut Context<Workspace>,
) {
    at(ws, ix, cx, |notes, session, _, _| {
        notes.of_mut(session).expect("a terminal").notes[n].collapsed = collapsed;
    });
}

/// The note ⌘↵ and the stash act on, in the selected terminal.
pub(in crate::window) fn focused_note(ws: &Workspace, cx: &App) -> Option<usize> {
    let notes = ws.feature::<Notes>()?;
    notes.read(cx).focused.filter(|&(id, _)| id == ws.active_session().id).map(|(_, ix)| ix)
}

pub(in crate::window) fn set_focused_note(ws: &Workspace, ix: Option<usize>, cx: &mut Context<Workspace>) {
    let session = ws.active_session().id;
    let notes = ws.feature::<Notes>().expect("the window has notes");
    notes.update(cx, |notes, _| notes.focused = ix.map(|ix| (session, ix)));
}

/// The ids of the terminals the window's notes keep an entry for, in order.
pub(in crate::window) fn kept_ids(ws: &Workspace, cx: &App) -> Vec<i64> {
    let notes = ws.feature::<Notes>().expect("the window has notes");
    let mut ids: Vec<i64> = notes.read(cx).of.keys().copied().collect();
    ids.sort_unstable();
    ids
}
