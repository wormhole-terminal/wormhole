//! How the Notes tab draws: its rows, the bands with their pinned copy, an open note's text
//! under its title (the lines after the one the title is, cut at `NOTE_OPEN_MAX_LINES` while
//! nobody edits it), and the list kept in step with the session.
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::Sizable;
use gpui_kit::component::input::{InputState, Textarea, TextareaState};
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::keys::keymap::{self, ActionId};
use wormhole_core::view::focus::list::Item;
use wormhole_core::view::limits::count_grouped;
use wormhole_core::view::panel::{self as rules, StepRow};
use wormhole_core::view::spoken;

use super::{NOTE_ROWS_MIN, is_blank_note};
use crate::settings::UiColors;
use crate::settings::ui_in;
use crate::style::{self, TextRole};
use crate::workspace::focus::{Activate, Walkable};
use crate::workspace::items::with_item_keys;
use crate::workspace::panel_list::{CardPart, Rows};
use crate::workspace::text::{note_body, note_title};
use crate::workspace::{NOTE_PLACEHOLDER, TaskItem, Workspace};

/// The Notes band's + names its key, from the keymap (the band takes a `&'static str`).
static NEW_NOTE_TOOLTIP: std::sync::LazyLock<String> =
    std::sync::LazyLock::new(|| keymap::with_key("New note", ActionId::NewNote));

/// One line of the Notes tab (tasks and notes share one virtual list).
#[derive(Clone, PartialEq)]
pub(in crate::workspace) enum NotesRow {
    TasksBand,
    /// `text` is only here so that a different task in the same slot re-measures.
    Task {
        ix: usize,
        text: SharedString,
        editing: bool,
    },
    /// A new task being written, in the place it will go.
    TaskDraft,
    /// "N done": the done steps, folded after the open ones; `open` shows them below it.
    DoneFold {
        count: usize,
        open: bool,
    },
    NotesBand,
    /// `id` (the note's editor) tells notes apart when they change places.
    Note {
        ix: usize,
        id: EntityId,
        collapsed: bool,
    },
    /// Bottom padding, so the last row can scroll clear of the panel edge.
    Spacer,
}

impl NotesRow {
    /// Next steps and Notes are a card each; the spacer under the last is outside them.
    fn part(&self) -> CardPart {
        match self {
            Self::TasksBand | Self::NotesBand => CardPart::Head,
            Self::Spacer => CardPart::Outside,
            _ => CardPart::Body,
        }
    }
}

/// The Notes tab of one session: its rows, and whether its folded done steps are shown.
#[derive(Default)]
pub(in crate::workspace) struct NotesList {
    rows: Rows<NotesRow>,
    pub(super) done_open: bool,
}

/// What the Notes list's rows draw from, captured for the render callback.
/// Rows only describe the layout; data is read here, for the rows in view.
pub(in crate::workspace) struct NotesView {
    pub(in crate::workspace) tasks: Vec<TaskItem>,
    pub(in crate::workspace) tasks_collapsed: bool,
    pub(in crate::workspace) notes_collapsed: bool,
    /// The task being edited: its index, whether it's a new one, and its editor.
    pub(in crate::workspace) task_edit: Option<(usize, bool, Entity<InputState>)>,
    pub(in crate::workspace) notes: Vec<Entity<TextareaState>>,
    /// Per note, whether "Show all" lifted its line cap.
    pub(in crate::workspace) show_all: Vec<bool>,
    /// Notes with something in them (the band's count).
    pub(in crate::workspace) filled_notes: usize,
    /// The note whose text has the keyboard.
    pub(in crate::workspace) editing: Option<EntityId>,
    /// Drag-and-drop keys of this window's task and note lists.
    pub(in crate::workspace) task_list: SharedString,
    pub(in crate::workspace) note_list: SharedString,
    /// The panel's width, which an open note's text wraps in.
    pub(in crate::workspace) width: f32,
    pub(in crate::workspace) this: WeakEntity<Workspace>,
}

impl NotesView {
    pub(super) fn render_row(
        &self,
        row: &NotesRow,
        ui: UiColors,
        window: &mut Window,
        cx: &App,
    ) -> AnyElement {
        match row {
            NotesRow::TasksBand => self.render_tasks_band(ui),
            NotesRow::Task { ix, .. } => self.render_task(Some(*ix), ui, cx),
            NotesRow::TaskDraft => self.render_task(None, ui, cx),
            NotesRow::DoneFold { count, open } => self.render_done_fold(*count, *open, ui),
            NotesRow::NotesBand => self.render_notes_band(ui),
            NotesRow::Note { ix, collapsed, .. } => self.render_note(*ix, *collapsed, ui, window, cx),
            NotesRow::Spacer => div().h(px(16.0)).into_any_element(),
        }
    }

    fn render_notes_band(&self, ui: UiColors) -> AnyElement {
        let (toggle, add) = (self.this.clone(), self.this.clone());
        style::section_card(
            "notes-band",
            ui,
            style::BandParts {
                kind: style::BandKind::Section,
                section: style::Section::of(self.notes.len(), self.notes_collapsed),
                title: "Notes".into(),
                detail: None,
                count: (self.filled_notes > 0).then(|| count_grouped(self.filled_notes).into()),
                waiting: None,
                add: Some((
                    NEW_NOTE_TOOLTIP.as_str(),
                    Box::new(move |_, window, cx| {
                        add.update(cx, |this, cx| this.new_note(None, window, cx)).ok();
                    }),
                )),
                on_toggle: Some(Box::new(move |_, _, cx| {
                    toggle.update(cx, |this, cx| this.toggle_notes_section(cx)).ok();
                })),
                drop_target: false,
                sticky: false,
                joined: false,
            },
            Vec::new(),
        )
        .aria_label(spoken::section_band("Notes", self.filled_notes, spoken::Counted::Notes))
        .into_any_element()
    }

    fn render_note(
        &self,
        ix: usize,
        collapsed: bool,
        ui: UiColors,
        window: &mut Window,
        cx: &App,
    ) -> AnyElement {
        let Some(state) = self.notes.get(ix) else { return div().into_any_element() };
        let text = state.read(cx).value();
        // Only a note being written is empty (and it's thrown away if left that way).
        let title: SharedString = note_title(&text).map_or_else(|| "New note".into(), SharedString::from);
        // Focused counts too: a click on a cut note focuses its editor before the editor is drawn
        // (and so before it can say it has the focus).
        let focused = state.read(cx).focus_handle(cx).is_focused(window);
        let editing = (self.editing == Some(state.entity_id()) || focused) && !collapsed;
        let (toggle, open, send, delete, keys, moved, show) = (
            self.this.clone(),
            self.this.clone(),
            self.this.clone(),
            self.this.clone(),
            self.this.clone(),
            self.this.clone(),
            self.this.clone(),
        );
        let lead = div()
            .id(("note-toggle", ix))
            .flex()
            .items_center()
            .justify_center()
            .size(px(style::ITEM_LEAD_W))
            .cursor_pointer()
            .child(style::disclosure(ui, collapsed))
            .on_click(move |_, _, cx| {
                toggle.update(cx, |this, cx| this.toggle_note(ix, cx)).ok();
            });
        // While the editor is open the whole text is in it, the title's line too: the title
        // quiets down so it doesn't read as duplicated content. Clicking it goes to the text, as
        // a task's does.
        let main = style::item_label(ui, title.clone(), editing)
            .id(("note-title", ix))
            .truncate()
            .cursor_text()
            .on_click(move |_, window, cx| {
                open.update(cx, |this, cx| this.open_note(ix, window, cx)).ok();
            });
        let sent = state.clone();
        let actions = vec![
            style::item_action(("note-send", ix), ui, IconName::ArrowUpRight, "Send to terminal", {
                move |_, window, cx| {
                    // Read at click time: rows don't carry the text.
                    let text = sent.read(cx).value().to_string();
                    send.update(cx, |this, cx| this.send_text(text, false, window, cx)).ok();
                }
            }),
            style::item_action(("note-delete", ix), ui, IconName::X, "Delete", move |_, _, cx| {
                delete.update(cx, |this, cx| this.delete_note(ix, cx)).ok();
            }),
        ];
        let body = if collapsed {
            None
        } else if editing {
            let keyed = state.clone();
            // An empty editor grows to its whole hint, which the editor itself doesn't count.
            let hint_rows = text
                .is_empty()
                .then(|| state.read(cx).presentation().placeholder().split('\n').count().max(NOTE_ROWS_MIN));
            let editor = Textarea::new(state)
                .small()
                .appearance(false)
                .when_some(hint_rows, |editor, rows| editor.h(rems(NOTE_LINE_REMS * rows as f32)));
            Some(
                with_item_keys(editor, move |event, window, cx| {
                    keys.update(cx, |this, cx| this.note_key(&keyed, event, window, cx)).unwrap_or(false)
                })
                .into_any_element(),
            )
        } else {
            // The title is the note's first line, so only the lines after it show under it; a
            // note of one line shows it once. As the editor would wrap them, and
            // cut at the cap until "Show all".
            note_body(&text).map(|body| {
                let show_all = self.show_all.get(ix).copied().unwrap_or(false);
                let lines = wrapped_note(body, self.width, window, cx);
                let cap = rules::note_cap(lines.len(), editing, show_all);
                let opened = self.this.clone();
                note_preview(
                    ix,
                    &lines,
                    cap,
                    ui,
                    window,
                    move |_, window, cx| {
                        opened.update(cx, |this, cx| this.open_note(ix, window, cx)).ok();
                    },
                    move |_, _, cx| {
                        show.update(cx, |this, cx| this.show_all_of_note(ix, cx)).ok();
                    },
                )
            })
        };
        let reorder = style::Reorder {
            dragged: style::Reorder::dragging(&self.note_list, ix, cx),
            list: self.note_list.clone(),
            ix,
            label: title,
            on_move: Rc::new(move |from, to, _, cx| {
                moved.update(cx, |this, cx| this.move_note(from, to, cx)).ok();
            }),
        };
        style::item_row(
            ("note", ix),
            ui,
            style::ItemParts {
                lead: lead.into_any_element(),
                main: main.into_any_element(),
                actions,
                body,
                active: editing,
                reorder: Some(reorder),
            },
        )
        .into_any_element()
    }
}

/// An open note's text as the panel wraps it (14 pt, between x 40 and the right inset): one
/// string per line on screen.
fn wrapped_note(text: &str, panel_w: f32, window: &mut Window, cx: &App) -> Vec<SharedString> {
    let width = px(panel_w - NOTE_TEXT_X - style::PANEL_ROW_X);
    let font_size = rems(NOTE_TEXT_REMS).to_pixels(window.rem_size());
    let run = TextRun { len: text.len(), font: font(cx.theme().font_family.clone()), ..Default::default() };
    let shaped = window.text_system().shape_text(
        SharedString::from(text.to_string()),
        font_size,
        &[run],
        Some(width),
        None,
    );
    let Ok(paragraphs) = shaped else {
        return text.lines().map(|line| SharedString::from(line.to_string())).collect();
    };
    let mut lines = Vec::new();
    for paragraph in &paragraphs {
        let source = paragraph.text.as_ref();
        let mut start = 0;
        for boundary in paragraph.wrap_boundaries() {
            let glyph = paragraph
                .unwrapped_layout
                .runs
                .get(boundary.run_ix)
                .and_then(|run| run.glyphs.get(boundary.glyph_ix));
            let Some(end) = glyph.map(|glyph| glyph.index) else { continue };
            lines.push(SharedString::from(source.get(start..end).unwrap_or_default().to_string()));
            start = end;
        }
        lines.push(SharedString::from(source.get(start..).unwrap_or_default().to_string()));
    }
    lines
}

/// The hint of an empty note, broken into lines that fit the panel at `panel_w`. The editor draws
/// a placeholder one line per newline and never wraps it, so a line longer than the panel's text
/// column was cut at its edge. It breaks where the editor would wrap typed text.
pub(super) fn wrapped_placeholder(panel_w: f32, window: &mut Window, cx: &App) -> SharedString {
    let panel_w = panel_w.max(wormhole_core::view::layout::PANEL_MIN);
    let lines = wrapped_note(NOTE_PLACEHOLDER, panel_w - EDITOR_RIGHT_MARGIN, window, cx);
    lines.iter().map(|line| line.trim_end()).collect::<Vec<_>>().join("\n").into()
}

/// Where a note's text starts (from the panel's edge), and its size and line height (GPUI Kit's `small` textarea: 14 pt
/// on 20).
const NOTE_TEXT_X: f32 = style::PANEL_ROW_X + style::ITEM_LEAD_W + 8.0;
const NOTE_TEXT_REMS: f32 = 0.875;
const NOTE_LINE_REMS: f32 = 1.25;
/// How far the editor keeps its text from its right edge (GPUI Kit's `RIGHT_MARGIN`): where it
/// wraps.
const EDITOR_RIGHT_MARGIN: f32 = 10.0;
/// How opaque the last lines of a cut note are, the last one first: they fade out.
const NOTE_FADE: [f32; 2] = [0.3, 0.65];

/// An open note's text while nobody edits it: its lines as the editor would wrap them. Longer
/// than `cap` ("A very long note") only that many show, the last two fading out, then
/// "Show all · N lines". Clicking the text edits the note, which shows all of it.
fn note_preview(
    ix: usize,
    lines: &[SharedString],
    cap: Option<usize>,
    ui: UiColors,
    window: &Window,
    on_open: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    on_show_all: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
) -> AnyElement {
    let line_h = rems(NOTE_LINE_REMS).to_pixels(window.rem_size());
    let shown = cap.unwrap_or(lines.len()).min(lines.len());
    let cut = shown < lines.len();
    // The editor's own inset, which the body's indent leaves for it.
    div()
        .flex()
        .flex_col()
        .pl_2()
        .child(
            div()
                .id(("note-preview", ix))
                .flex()
                .flex_col()
                .cursor_text()
                .text_body()
                .on_click(on_open)
                .children(lines.iter().take(shown).enumerate().map(|(n, line)| {
                    let opacity =
                        if cut { NOTE_FADE.get(shown - 1 - n).copied().unwrap_or(1.0) } else { 1.0 };
                    div()
                        .h(line_h)
                        .flex()
                        .items_center()
                        .whitespace_nowrap()
                        .overflow_hidden()
                        .text_color(ui.text.opacity(opacity))
                        .child(line.clone())
                })),
        )
        .when(cut, |d| {
            d.child(
                div()
                    .id(("note-show-all", ix))
                    .pt_1()
                    .text_body()
                    .text_color(ui.primary)
                    .cursor_pointer()
                    .on_click(on_show_all)
                    .child(format!("Show all · {} lines", count_grouped(lines.len()))),
            )
        })
        .into_any_element()
}

impl Workspace {
    /// Refresh the list's rows from the session's tasks and notes. Cheap when
    /// nothing changed, so it runs on every render (every scroll tick).
    pub(in crate::workspace) fn sync_notes_list(&mut self, cx: &App) {
        let session = &mut self.sessions[self.active];
        // The Next steps band right under the tab bar, sharing its rule.
        let mut rows = vec![NotesRow::TasksBand];
        let edit = session.task_edit.as_ref().map(|edit| (edit.ix, edit.new));
        let draft = edit.filter(|(_, new)| *new).map(|(ix, _)| ix.min(session.tasks.len()));
        let tasks = session.tasks.len() + usize::from(draft.is_some());
        if !style::Section::of(tasks, session.tasks_collapsed).is_collapsed() {
            let done: Vec<bool> = session.tasks.iter().map(|task| task.done).collect();
            rows.extend(rules::steps(&done, draft, session.notes_list.done_open).into_iter().map(|row| {
                match row {
                    StepRow::Step(ix) => NotesRow::Task {
                        ix,
                        text: session.tasks[ix].text.clone(),
                        editing: edit == Some((ix, false)),
                    },
                    StepRow::Draft => NotesRow::TaskDraft,
                    StepRow::Done(count) => NotesRow::DoneFold { count, open: session.notes_list.done_open },
                }
            }));
        }
        rows.push(NotesRow::NotesBand);
        if !style::Section::of(session.notes.len(), session.notes_collapsed).is_collapsed() {
            rows.extend(session.notes.iter().enumerate().map(|(ix, n)| NotesRow::Note {
                ix,
                id: n.state.entity_id(),
                collapsed: n.collapsed,
            }));
        }
        rows.push(NotesRow::Spacer);
        let notes = &session.notes;
        let editor = session.task_edit.as_ref().map(|edit| edit.input.read(cx).focus_handle(cx));
        session.notes_list.rows.set_with(rows, |row| match row {
            NotesRow::Note { ix, .. } => notes.get(*ix).map(|n| n.state.read(cx).focus_handle(cx)),
            NotesRow::TaskDraft | NotesRow::Task { editing: true, .. } => editor.clone(),
            _ => None,
        });
    }

    // ------------------------------------------------------------ keys

    /// The Notes rows the keys can be on: the two bands (⇥ stops), each next step, the done steps'
    /// fold and each note. A step being written is its editor's.
    pub(in crate::workspace) fn notes_walkable(&self) -> Vec<Walkable> {
        let session = self.active_session();
        let rows = session.notes_list.rows.items();
        (rows.iter().enumerate())
            .filter_map(|(row, line)| {
                let (level, fold, band) = match line {
                    NotesRow::TasksBand => {
                        (0, (!session.tasks.is_empty()).then_some(!session.tasks_collapsed), true)
                    }
                    NotesRow::NotesBand => {
                        (0, (!session.notes.is_empty()).then_some(!session.notes_collapsed), true)
                    }
                    NotesRow::DoneFold { open, .. } => (1, Some(*open), false),
                    NotesRow::Task { .. } | NotesRow::Note { .. } => (1, None, false),
                    NotesRow::TaskDraft | NotesRow::Spacer => return None,
                };
                Some(Walkable { row, item: Item { level, fold }, band })
            })
            .collect()
    }

    /// → / ← on a band folds or opens its section, on "N done" the done steps.
    pub(in crate::workspace) fn notes_fold(&mut self, row: usize, cx: &mut Context<Self>) {
        match self.active_session().notes_list.rows.items().get(row) {
            Some(NotesRow::TasksBand) => {
                let session = &mut self.sessions[self.active];
                session.tasks_collapsed = !session.tasks_collapsed;
                self.mark_dirty(cx);
            }
            Some(NotesRow::NotesBand) => self.toggle_notes_section(cx),
            Some(NotesRow::DoneFold { .. }) => self.toggle_done_steps(cx),
            _ => {}
        }
    }

    /// Space ticks a next step or folds a note; ↩ writes in it. A band or "N done" folds either
    /// way. Whether the list keeps the keyboard (not when an editor takes it).
    pub(in crate::workspace) fn notes_press(
        &mut self,
        row: usize,
        press: Activate,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        match (self.active_session().notes_list.rows.items().get(row).cloned(), press) {
            (Some(NotesRow::Task { ix, .. }), Activate::Space) => self.toggle_task(ix, cx),
            (Some(NotesRow::Task { ix, .. }), Activate::Return) => {
                self.edit_task(ix, false, window, cx);
                return false;
            }
            (Some(NotesRow::Note { ix, .. }), Activate::Space) => self.toggle_note(ix, cx),
            (Some(NotesRow::Note { ix, .. }), Activate::Return) => {
                self.show_note(ix, true, window, cx);
                return false;
            }
            (Some(_), _) => self.notes_fold(row, cx),
            (None, _) => {}
        }
        true
    }

    pub(in crate::workspace) fn notes_reveal(&self, row: usize) {
        self.active_session().notes_list.rows.reveal(row);
    }

    #[cfg(feature = "devctl")]
    pub(crate) fn devctl_notes_list(&self) -> ListState {
        self.active_session().notes_list.rows.state()
    }

    pub(in crate::workspace) fn render_notes_tab(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let session = self.active_session();
        let view = Rc::new(NotesView {
            tasks: session.tasks.clone(),
            tasks_collapsed: session.tasks_collapsed,
            notes_collapsed: session.notes_collapsed,
            task_edit: session.task_edit.as_ref().map(|edit| (edit.ix, edit.new, edit.input.clone())),
            notes: session.notes.iter().map(|n| n.state.clone()).collect(),
            show_all: session.notes.iter().map(|n| n.show_all).collect(),
            filled_notes: session.notes.iter().filter(|n| !is_blank_note(n.state.read(cx))).count(),
            editing: self.editing_note,
            task_list: self.reorder_list("tasks", cx),
            note_list: self.reorder_list("notes", cx),
            width: self.columns().panel,
            this: cx.weak_entity(),
        });
        let rows = &session.notes_list.rows;
        let in_list = view;
        let ui = ui_in(&self.profile, cx);
        let profile = self.profile.clone();
        // A column that fills the panel, so the list gets a bounded height.
        div()
            .relative()
            .flex()
            .flex_col()
            .flex_1()
            .min_h_0()
            .child(rows.with_edges(
                rows.element_carded(
                    self.panel_ring(window, ui),
                    ui,
                    NotesRow::part,
                    move |row, window, cx| in_list.render_row(row, ui_in(&profile, cx), window, cx),
                ),
                ui,
                "notes-scrollbar",
                cx,
            ))
            .into_any_element()
    }
}
