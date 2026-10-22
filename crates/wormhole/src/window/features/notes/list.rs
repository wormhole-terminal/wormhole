//! The Notes tab's list kept in step with a terminal's notes and tasks, and the keys over its rows:
//! which rows they can be on, what ↩ and Space do on one, what → and ← do. The rows are core's
//! (`view::notes::rows`); this ties them to the editors.
use gpui_kit::*;
use wormhole_core::view::notes::rows::{self, NoteFact, NotesRow};

use super::{Notes, NotesOf};
use crate::window::feature::WalkFns;
use crate::window::focus::{Activate, Walkable};
use crate::window::panel_list::{CardPart, Rows};

/// The Notes tab of one terminal: its rows, and whether its folded done steps are shown.
#[derive(Default)]
pub(super) struct NotesList {
    pub(super) rows: Rows<NotesRow>,
    pub(super) done_open: bool,
}

/// How a row sits in its card: Next steps and Notes are a card each; the spacer under the last is
/// outside them.
pub(super) fn part(row: &NotesRow) -> CardPart {
    match row {
        NotesRow::TasksBand | NotesRow::NotesBand => CardPart::Head,
        NotesRow::Spacer => CardPart::Outside,
        _ => CardPart::Body,
    }
}

impl Notes {
    /// Refresh `session`'s list rows from its tasks and notes. Cheap when nothing changed, so it runs
    /// on every render (every scroll tick). It doesn't notify: the render that calls it is the redraw.
    pub(super) fn sync_list(&mut self, session: i64, cx: &App) {
        let Some(of) = self.of.get_mut(&session) else { return };
        // The Next steps band right under the tab bar, sharing its rule.
        let notes = of
            .notes
            .iter()
            .map(|note| NoteFact { id: note.state.entity_id().as_u64(), collapsed: note.collapsed });
        let rows = rows::rows(
            &of.tasks,
            of.edit.as_ref().map(|e| e.edit),
            of.list.done_open,
            notes,
            of.notes_collapsed,
        );
        let notes = &of.notes;
        let editor = of.edit.as_ref().map(|edit| edit.input.read(cx).focus_handle(cx));
        of.list.rows.set_with(rows, |row| match row {
            NotesRow::Note { ix, .. } => notes.get(*ix).map(|n| n.state.read(cx).focus_handle(cx)),
            NotesRow::TaskDraft | NotesRow::Task { editing: true, .. } => editor.clone(),
            _ => None,
        });
    }

    /// The Notes rows the keys can be on: the two bands (⇥ stops), each next step, the done steps'
    /// fold and each note. A step being written is its editor's.
    fn walkable(of: &NotesOf) -> Vec<Walkable> {
        let tasks_fold = (!of.tasks.is_empty()).then_some(!of.tasks.collapsed);
        let notes_fold = (!of.notes.is_empty()).then_some(!of.notes_collapsed);
        rows::walkable(of.list.rows.items(), tasks_fold, notes_fold)
            .map(|stop| Walkable { row: stop.row, item: stop.item, band: stop.band })
            .collect()
    }

    /// → / ← on a band folds or opens its section, on "N done" the done steps.
    pub(super) fn fold_row(&mut self, session: i64, row: usize, cx: &mut Context<Self>) {
        match self.of.get(&session).and_then(|of| of.list.rows.items().get(row)) {
            Some(NotesRow::TasksBand) => self.toggle_tasks_section(session, cx),
            Some(NotesRow::NotesBand) => self.toggle_notes_section(session, cx),
            Some(NotesRow::DoneFold { .. }) => self.toggle_done_steps(session, cx),
            _ => {}
        }
    }

    /// Space ticks a next step or folds a note; ↩ writes in it. A band or "N done" folds either way.
    /// Whether the list keeps the keyboard (not when an editor takes it).
    fn press_row(
        &mut self,
        session: i64,
        row: usize,
        press: Activate,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let line = self.of.get(&session).and_then(|of| of.list.rows.items().get(row)).cloned();
        match (line, press) {
            (Some(NotesRow::Task { ix, .. }), Activate::Space) => self.toggle_task(session, ix, cx),
            (Some(NotesRow::Task { ix, .. }), Activate::Return) => {
                self.edit_task(session, ix, false, window, cx);
                return false;
            }
            (Some(NotesRow::Note { ix, .. }), Activate::Space) => self.toggle_note(session, ix, cx),
            (Some(NotesRow::Note { ix, .. }), Activate::Return) => {
                self.show_note(session, ix, true, window, cx);
                return false;
            }
            (Some(_), _) => self.fold_row(session, row, cx),
            (None, _) => {}
        }
        true
    }
}

/// The Notes tab's keys, over the entity.
pub(super) const WALK: WalkFns = WalkFns {
    walkable: |entity, facts, cx| {
        let notes = super::def::typed(entity);
        let notes = notes.read(cx);
        facts.active().and_then(|active| notes.of.get(&active.id)).map(Notes::walkable).unwrap_or_default()
    },
    press: |entity, session, row, press, following, window, cx| {
        // With a diff open the keys only follow it from file to file: nothing here to press.
        if following {
            return true;
        }
        super::def::typed(entity).update(cx, |notes, cx| notes.press_row(session, row, press, window, cx))
    },
    fold: |entity, session, row, cx| {
        super::def::typed(entity).update(cx, |notes, cx| notes.fold_row(session, row, cx));
    },
    reveal: |entity, facts, row, cx| {
        let notes = super::def::typed(entity);
        if let Some(of) = facts.active().and_then(|active| notes.read(cx).of.get(&active.id)) {
            of.list.rows.reveal(row);
        }
    },
};
