//! The per-session "Next steps" task list. Tasks are written and edited in place, one at a
//! time; a task left or made empty is never kept.
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::input::Position;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::limits::count_grouped;
use wormhole_core::reorder;
use wormhole_core::spoken;

use super::items::{ItemKey, item_key, with_item_keys};
use super::notes::NotesView;
use super::{RightTab, TaskItem, Workspace};
use crate::settings::UiColors;
use crate::style;

/// The task being written or edited in place.
pub(super) struct TaskEdit {
    /// The task being edited; for a `new` one, where it goes (before the task now there).
    pub ix: usize,
    pub new: bool,
    pub input: Entity<InputState>,
    _subscription: Subscription,
}

/// What finishing an edit did to the list, so an index taken before it can be corrected.
#[derive(Clone, Copy)]
enum Finished {
    /// The task kept its text (or took the new one) at this index.
    Kept(usize),
    /// A new task went in here.
    Inserted(usize),
    /// The task here was emptied, and deleted.
    Removed(usize),
    /// A new task was left empty: nothing changed.
    Dropped,
}

impl Workspace {
    /// Add a finished task at the end (devctl). Blank text adds nothing.
    #[cfg(any(test, feature = "devctl"))]
    pub(crate) fn add_task(&mut self, text: String, cx: &mut Context<Self>) {
        let text = text.trim();
        if text.is_empty() {
            return;
        }
        let session = &mut self.sessions[self.active];
        session.tasks.push(TaskItem { text: text.to_string().into(), done: false });
        session.tasks_collapsed = false;
        self.mark_dirty(cx);
    }

    /// Fill the active session with `open` next steps and `done` done ones (interleaved, one
    /// done after every few open ones), for checking how the list copes with many.
    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_seed_tasks(&mut self, open: usize, done: usize, cx: &mut Context<Self>) {
        let session = &mut self.sessions[self.active];
        let total = open + done;
        for ix in 0..total {
            // Spread evenly: the count of done ones so far grows by one every total / done steps.
            let finish = (ix + 1) * done / total > ix * done / total;
            let n = session.tasks.len() + 1;
            let text = if finish { format!("Finished step {n}") } else { format!("Next step {n}") };
            session.tasks.push(TaskItem { text: text.into(), done: finish });
        }
        session.tasks_collapsed = false;
        self.mark_dirty(cx);
    }

    /// The header's "+": a new, empty task at the end, with the cursor in it.
    pub(super) fn new_task(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let end = self.active_session().tasks.len();
        self.edit_task(end, true, window, cx);
    }

    /// Edit task `ix` in place, or with `new`, start writing a task that goes in at `ix`.
    /// Whatever was being edited is finished first.
    pub(super) fn edit_task(&mut self, ix: usize, new: bool, window: &mut Window, cx: &mut Context<Self>) {
        let mut ix = ix;
        let current = self.active_session().task_edit.as_ref().map(|edit| edit.input.clone());
        if let Some(current) = current {
            // Finishing can insert or delete a task: keep `ix` on the one that was meant.
            match self.finish_task_edit(&current, cx) {
                Some(Finished::Inserted(at)) if at <= ix => ix += 1,
                Some(Finished::Removed(at)) if at < ix => ix -= 1,
                // The task asked for was the one just emptied, and is gone.
                Some(Finished::Removed(at)) if at == ix && !new => return,
                _ => {}
            }
        }
        let session = &mut self.sessions[self.active];
        if !new && ix >= session.tasks.len() {
            return;
        }
        let ix = ix.min(session.tasks.len());
        let text = if new { SharedString::default() } else { session.tasks[ix].text.clone() };
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Next step").default_value(text));
        let subscription =
            cx.subscribe_in(&input, window, |this, input, event: &InputEvent, window, cx| match event {
                InputEvent::PressEnter { secondary: false, .. } => this.task_enter(input, window, cx),
                // Like ⌘↵ in a note. The terminal takes the focus, which finishes the edit.
                InputEvent::PressEnter { secondary: true, shift } => {
                    let text = input.read(cx).value().to_string();
                    this.send_text(text, *shift, window, cx);
                }
                InputEvent::Blur => {
                    this.finish_task_edit(input, cx);
                }
                _ => {}
            });
        session.task_edit = Some(TaskEdit { ix, new, input: input.clone(), _subscription: subscription });
        session.tasks_collapsed = false;
        self.panes.panel_wanted = true;
        self.remember_panes(cx);
        self.right_tab = RightTab::Notes;
        cx.notify();
        // The editor's row only exists once the list has been rebuilt.
        cx.defer_in(window, move |_, window, cx| {
            input.update(cx, |input, cx| {
                // At the end of the text, where clicking to add to it expects it.
                let end = input.value().chars().count() as u32;
                input.set_cursor_position(Position::new(0, end), window, cx);
            });
        });
    }

    /// End the edit `input` belongs to, if it is still going: save the text, delete a task
    /// edited down to nothing, drop a new one left empty.
    fn finish_task_edit(&mut self, input: &Entity<InputState>, cx: &mut Context<Self>) -> Option<Finished> {
        let session = self
            .sessions
            .iter_mut()
            .find(|s| s.task_edit.as_ref().is_some_and(|edit| &edit.input == input))?;
        let edit = session.task_edit.take()?;
        // Tasks are one line (and one line in notes.md).
        let text = input.read(cx).value().split_whitespace().collect::<Vec<_>>().join(" ");
        let finished = if edit.new {
            if text.is_empty() {
                Finished::Dropped
            } else {
                let at = edit.ix.min(session.tasks.len());
                session.tasks.insert(at, TaskItem { text: text.into(), done: false });
                Finished::Inserted(at)
            }
        } else if edit.ix >= session.tasks.len() {
            Finished::Dropped
        } else if text.is_empty() {
            session.tasks.remove(edit.ix);
            Finished::Removed(edit.ix)
        } else {
            if session.tasks[edit.ix].text != text {
                session.tasks[edit.ix].text = text.into();
            }
            Finished::Kept(edit.ix)
        };
        // An empty section has no chevron to open it with.
        session.tasks_collapsed &= !session.tasks.is_empty();
        self.mark_dirty(cx);
        Some(finished)
    }

    /// Enter in a task: keep it and write the next one right below, like a checklist.
    /// Enter in an empty one is done writing.
    fn task_enter(&mut self, input: &Entity<InputState>, window: &mut Window, cx: &mut Context<Self>) {
        match self.finish_task_edit(input, cx) {
            Some(Finished::Kept(ix) | Finished::Inserted(ix)) => self.edit_task(ix + 1, true, window, cx),
            Some(Finished::Removed(_) | Finished::Dropped) => self.focus_terminal(window, cx),
            None => {}
        }
    }

    /// The keys a task's editor shares with a note's ([`ItemKey`]).
    fn task_key(
        &mut self,
        input: &Entity<InputState>,
        event: &KeyDownEvent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let session = self.active_session();
        let Some(edit) = session.task_edit.as_ref().filter(|edit| &edit.input == input) else {
            return false;
        };
        let (ix, new, len) = (edit.ix, edit.new, session.tasks.len());
        match item_key(&event.keystroke, input.read(cx).value().is_empty()) {
            // The terminal takes the focus, which finishes the edit.
            Some(ItemKey::Leave) => self.focus_terminal(window, cx),
            Some(ItemKey::DeleteEmpty) => {
                self.finish_task_edit(input, cx);
                match ix.checked_sub(1) {
                    Some(above) => self.edit_task(above, false, window, cx),
                    None => self.focus_terminal(window, cx),
                }
            }
            Some(ItemKey::Move(delta)) if !new => {
                if let Some(to) = reorder::step(ix, delta, len) {
                    self.move_task(ix, to, cx);
                }
            }
            _ => return false,
        }
        true
    }

    /// Move a task to another place in the list (drag and drop, ⌥⌘↑/↓).
    pub(super) fn move_task(&mut self, from: usize, to: usize, cx: &mut Context<Self>) {
        let session = &mut self.sessions[self.active];
        if !reorder::move_item(&mut session.tasks, from, to) {
            return;
        }
        if let Some(edit) = session.task_edit.as_mut().filter(|edit| !edit.new) {
            edit.ix = reorder::follow(edit.ix, from, to);
        }
        self.mark_dirty(cx);
    }

    pub(super) fn toggle_task(&mut self, ix: usize, cx: &mut Context<Self>) {
        if let Some(task) = self.sessions[self.active].tasks.get_mut(ix) {
            task.done = !task.done;
            self.mark_dirty(cx);
        }
    }

    pub(super) fn delete_task(&mut self, ix: usize, cx: &mut Context<Self>) {
        let session = &mut self.sessions[self.active];
        if ix >= session.tasks.len() {
            return;
        }
        session.tasks.remove(ix);
        // Keep an edit in progress on its task (or drop it with its task).
        match &mut session.task_edit {
            Some(edit) if !edit.new && edit.ix == ix => session.task_edit = None,
            Some(edit) if edit.ix > ix => edit.ix -= 1,
            _ => {}
        }
        // An empty section has no chevron to open it with.
        session.tasks_collapsed &= !session.tasks.is_empty();
        self.mark_dirty(cx);
    }

    /// Throw away the new task being written (its hover ✕).
    fn discard_task_draft(&mut self, cx: &mut Context<Self>) {
        let session = &mut self.sessions[self.active];
        if session.task_edit.as_ref().is_some_and(|edit| edit.new) {
            session.task_edit = None;
            cx.notify();
        }
    }
}

// ---------------------------------------------------------------- rows

impl NotesView {
    /// The Next steps card's head.
    pub(super) fn render_tasks_band(&self, ui: UiColors) -> AnyElement {
        let (toggle, add) = (self.this.clone(), self.this.clone());
        let open = self.tasks.iter().filter(|t| !t.done).count();
        style::section_card(
            "tasks-band",
            ui,
            style::BandParts {
                kind: style::BandKind::Section,
                section: self.tasks_section(),
                title: "Next steps".into(),
                detail: None,
                // The steps still to do; done ones don't count.
                count: (open > 0).then(|| count_grouped(open).into()),
                waiting: None,
                add: Some((
                    "Add next step",
                    Box::new(move |_, window, cx| {
                        add.update(cx, |this, cx| this.new_task(window, cx)).ok();
                    }),
                )),
                on_toggle: Some(Box::new(move |_, _, cx| {
                    toggle
                        .update(cx, |this, cx| {
                            let session = &mut this.sessions[this.active];
                            session.tasks_collapsed = !session.tasks_collapsed;
                            this.mark_dirty(cx);
                        })
                        .ok();
                })),
                drop_target: false,
                sticky: false,
                joined: false,
            },
            Vec::new(),
        )
        .aria_label(spoken::section_band("Next steps", open, spoken::Counted::Open))
        .into_any_element()
    }

    /// The section counts a task being written, so it has its chevron while it's there.
    pub(super) fn tasks_section(&self) -> style::Section {
        let draft = self.task_edit.as_ref().is_some_and(|(_, new, _)| *new);
        style::Section::of(self.tasks.len() + usize::from(draft), self.tasks_collapsed)
    }

    /// "N done": the done steps folded after the open ones, a click shows or folds them.
    pub(super) fn render_done_fold(&self, count: usize, open: bool, ui: UiColors) -> AnyElement {
        let this = self.this.clone();
        style::flush_row("done-steps", ui, false)
            .child(style::lead(style::disclosure(ui, !open)))
            .child(div().text_color(ui.muted).child(format!("{} done", count_grouped(count))))
            .on_click(move |_, _, cx| {
                this.update(cx, |this, cx| this.toggle_done_steps(cx)).ok();
            })
            .into_any_element()
    }

    /// Task `ix`, or with `ix == None` the new task being written.
    pub(super) fn render_task(&self, ix: Option<usize>, ui: UiColors, cx: &App) -> AnyElement {
        let task = match ix {
            Some(ix) => match self.tasks.get(ix) {
                Some(task) => Some(task),
                None => return div().into_any_element(),
            },
            None => None,
        };
        let editor = self
            .task_edit
            .as_ref()
            .filter(|(edit_ix, new, _)| if task.is_some() { !new && Some(*edit_ix) == ix } else { *new })
            .map(|(_, _, input)| input.clone());
        let done = task.is_some_and(|t| t.done);
        let key: SharedString = match ix {
            Some(ix) => format!("task-{ix}").into(),
            None => "task-new".into(),
        };

        // The 14 pt box, clickable across the lead column and beyond (28 pt).
        let toggle = self.this.clone();
        let lead = style::checkbox(SharedString::from(format!("{key}-check")), ui, done)
            .spoken(task.map(|t| t.text.clone()).unwrap_or_else(|| "New next step".into()))
            .when_some(ix, |check, ix| {
                check.on_change(move |_, _, cx| {
                    toggle.update(cx, |this, cx| this.toggle_task(ix, cx)).ok();
                })
            });
        let edit = self.this.clone();
        let main = match (&editor, task) {
            (Some(input), _) => {
                let (this, keyed) = (self.this.clone(), input.clone());
                with_item_keys(
                    style::item_editor(Input::new(input).small().appearance(false)),
                    move |event, window, cx| {
                        this.update(cx, |this, cx| this.task_key(&keyed, event, window, cx)).unwrap_or(false)
                    },
                )
                .into_any_element()
            }
            (None, Some(task)) => style::item_label(ui, task.text.clone(), done)
                .id(SharedString::from(format!("{key}-text")))
                .when(done, |d| d.line_through())
                .cursor_text()
                .when_some(ix, |d, ix| {
                    d.on_click(move |_, window, cx| {
                        edit.update(cx, |this, cx| this.edit_task(ix, false, window, cx)).ok();
                    })
                })
                .into_any_element(),
            (None, None) => div().into_any_element(),
        };

        // Read at click time: the text may be mid-edit.
        let text = task.map(|t| t.text.clone());
        let typed = editor.clone();
        let (send, delete) = (self.this.clone(), self.this.clone());
        let actions = vec![
            style::item_action(
                SharedString::from(format!("{key}-send")),
                ui,
                IconName::ArrowUpRight,
                "Send to terminal",
                move |_, window, cx| {
                    let text = match &typed {
                        Some(input) => input.read(cx).value().to_string(),
                        None => text.clone().unwrap_or_default().to_string(),
                    };
                    send.update(cx, |this, cx| this.send_text(text, false, window, cx)).ok();
                },
            ),
            style::item_action(SharedString::from(format!("{key}-delete")), ui, IconName::X, "Delete", {
                move |_, _, cx| {
                    delete
                        .update(cx, |this, cx| match ix {
                            Some(ix) => this.delete_task(ix, cx),
                            None => this.discard_task_draft(cx),
                        })
                        .ok();
                }
            }),
        ];

        // A task moves by its text; not while that is an editor, where dragging selects.
        let reorder = task.filter(|_| editor.is_none()).zip(ix).map(|(task, ix)| {
            let this = self.this.clone();
            style::Reorder {
                dragged: style::Reorder::dragging(&self.task_list, ix, cx),
                list: self.task_list.clone(),
                ix,
                label: task.text.clone(),
                on_move: Rc::new(move |from, to, _, cx| {
                    this.update(cx, |this, cx| this.move_task(from, to, cx)).ok();
                }),
            }
        });
        style::item_row(
            key,
            ui,
            style::ItemParts {
                lead: lead.into_any_element(),
                main,
                actions,
                body: None,
                active: editor.is_some(),
                reorder,
            },
        )
        .into_any_element()
    }
}
