//! The Next steps card of the Notes tab: its band, the "N done" fold and a step's row, which is a
//! label or the editor it is being written in.
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::input::Input;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::limits::count_grouped;
use wormhole_core::view::spoken;

use super::items::with_item_keys;
use super::view::NotesView;
use crate::settings::UiColors;
use crate::style;

// ---------------------------------------------------------------- rows

impl NotesView {
    /// The Next steps card's head.
    pub(super) fn render_tasks_band(&self, ui: UiColors) -> AnyElement {
        let (toggle, add, session) = (self.this.clone(), self.this.clone(), self.session);
        let open = self.tasks.open();
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
                        add.update(cx, |this, cx| this.new_task(session, window, cx)).ok();
                    }),
                )),
                on_toggle: Some(Box::new(move |_, _, cx| {
                    toggle.update(cx, |this, cx| this.toggle_tasks_section(session, cx)).ok();
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
        let draft = self.task_edit.as_ref().is_some_and(|(edit, _)| edit.new);
        style::Section::of(self.tasks.len() + usize::from(draft), self.tasks.collapsed)
    }

    /// "N done": the done steps folded after the open ones, a click shows or folds them.
    pub(super) fn render_done_fold(&self, count: usize, open: bool, ui: UiColors) -> AnyElement {
        let (this, session) = (self.this.clone(), self.session);
        style::flush_row("done-steps", ui, false)
            .child(style::lead(style::disclosure(ui, !open)))
            .child(div().text_color(ui.muted).child(format!("{} done", count_grouped(count))))
            .on_click(move |_, _, cx| {
                this.update(cx, |this, cx| this.toggle_done_steps(session, cx)).ok();
            })
            .into_any_element()
    }

    /// Task `ix`, or with `ix == None` the new task being written.
    pub(super) fn render_task(&self, ix: Option<usize>, ui: UiColors, cx: &App) -> AnyElement {
        let task = match ix {
            Some(ix) => match self.tasks.items().get(ix) {
                Some(task) => Some(task),
                None => return div().into_any_element(),
            },
            None => None,
        };
        let editor = self
            .task_edit
            .as_ref()
            .filter(|(edit, _)| if task.is_some() { !edit.new && Some(edit.ix) == ix } else { edit.new })
            .map(|(_, input)| input.clone());
        let done = task.is_some_and(|t| t.done);
        let key: SharedString = match ix {
            Some(ix) => format!("task-{ix}").into(),
            None => "task-new".into(),
        };

        // The 14 pt box, clickable across the lead column and beyond (28 pt).
        let (toggle, session) = (self.this.clone(), self.session);
        let fallback = || {
            style::checkbox(SharedString::from(format!("{key}-check")), ui, done)
                .spoken(task.map_or_else(
                    || SharedString::from("New next step"),
                    |t| SharedString::from(t.text.clone()),
                ))
                .when_some(ix, |check, ix| {
                    check.on_change(move |_, _, cx| {
                        toggle.update(cx, |this, cx| this.toggle_task(session, ix, cx)).ok();
                    })
                })
                .into_any_element()
        };
        let lead = if let (Some(native), Some(ix), Some(task)) = (&self.native_checks, ix, task) {
            let id = format!("note-task-{}-{}-{ix}", self.checks_epoch, session);
            let event_id = id.clone();
            let host = native.clone();
            let owner = self.this.clone();
            let epoch = self.checks_epoch;
            crate::style::native_switch::NativeCheckbox {
                native: native.clone(),
                id,
                label: task.text.clone().into(),
                on: done,
                enabled: true,
                show_label: false,
                focus: None,
                appearance: self.appearance,
                ui,
                change: Rc::new(move |_, _, cx| {
                    if !host.accepts(&event_id) {
                        return;
                    }
                    owner
                        .update(cx, |notes, cx| {
                            if notes.checks_epoch.get() == epoch {
                                notes.toggle_task(session, ix, cx);
                            }
                        })
                        .ok();
                }),
            }
            .into_any_element()
        } else if let Some(native) = &self.native_checks {
            crate::style::native_switch::NativeCheckbox {
                native: native.clone(),
                id: format!("note-task-draft-{}-{session}", self.checks_epoch),
                label: "New next step".into(),
                on: false,
                enabled: false,
                show_label: false,
                focus: None,
                appearance: self.appearance,
                ui,
                change: Rc::new(|_, _, _| {}),
            }
            .into_any_element()
        } else {
            fallback()
        };
        let edit = self.this.clone();
        let main = match (&editor, task) {
            (Some(input), _) => {
                let (this, keyed) = (self.this.clone(), input.clone());
                with_item_keys(
                    style::item_editor(Input::new(input).small().appearance(false)),
                    move |event, window, cx| {
                        this.update(cx, |this, cx| this.task_key(session, &keyed, event, window, cx))
                            .unwrap_or(false)
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
                        edit.update(cx, |this, cx| this.edit_task(session, ix, false, window, cx)).ok();
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
                move |_, _, cx| {
                    let text = match &typed {
                        Some(input) => input.read(cx).value().to_string(),
                        None => text.clone().unwrap_or_default().to_string(),
                    };
                    send.update(cx, |this, cx| this.send_text(session, text, false, cx)).ok();
                },
            ),
            style::item_action(SharedString::from(format!("{key}-delete")), ui, IconName::X, "Delete", {
                move |_, _, cx| {
                    delete
                        .update(cx, |this, cx| match ix {
                            Some(ix) => this.delete_task(session, ix, cx),
                            None => this.discard_task_draft(session, cx),
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
                label: task.text.clone().into(),
                on_move: Rc::new(move |from, to, _, cx| {
                    this.update(cx, |this, cx| this.move_task(session, from, to, cx)).ok();
                }),
            }
        });
        style::item_row(
            key,
            ui,
            style::ItemParts { lead, main, actions, body: None, active: editor.is_some(), reorder },
        )
        .into_any_element()
    }
}
