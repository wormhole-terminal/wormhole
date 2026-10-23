//! How the Notes tab draws: its rows, the bands with their pinned copy, an open note's text
//! under its title (the lines after the one the title is, cut at `NOTE_OPEN_MAX_LINES` while
//! nobody edits it), and the panel's body and stash button for the table.
use std::rc::Rc;

use gpui_kit::assets::IconName;
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::component::Sizable;
use gpui_kit::component::input::{InputState, Textarea, TextareaState};
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::keys::keymap::{self, ActionId};
use wormhole_core::view::limits::count_grouped;
use wormhole_core::view::notes::paragraphs::{note_body, note_title};
use wormhole_core::view::notes::rows::NotesRow;
use wormhole_core::view::notes::tasks::{Edit, Tasks};
use wormhole_core::view::panel::{self as rules};
use wormhole_core::view::spoken;

use super::def::typed;
use super::items::with_item_keys;
use super::list::part;
use super::{NOTE_ROWS_MIN, Notes};
use crate::settings::UiColors;
use crate::settings::ui_in;
use crate::style::{self, TextRole};
use crate::window::facts::WindowFacts;

const NOTE_PLACEHOLDER: &str = "Write a note or a prompt draft…\n⌘↵ sends the paragraph under the cursor (or the selection); ⌘⇧↵ also submits.";

/// The Notes band's + names its key, from the keymap (the band takes a `&'static str`).
static NEW_NOTE_TOOLTIP: std::sync::LazyLock<String> =
    std::sync::LazyLock::new(|| keymap::with_key("New note", ActionId::NewNote));

/// What the Notes list's rows draw from, captured for the render callback.
/// Rows only describe the layout; data is read here, for the rows in view.
pub(super) struct NotesView {
    pub(super) session: i64,
    pub(super) tasks: Tasks,
    pub(super) notes_collapsed: bool,
    /// The task being edited: where, whether it's a new one, and its editor.
    pub(super) task_edit: Option<(Edit, Entity<InputState>)>,
    pub(super) notes: Vec<Entity<TextareaState>>,
    /// Per note, whether "Show all" lifted its line cap.
    pub(super) show_all: Vec<bool>,
    /// Notes with something in them (the band's count).
    pub(super) filled_notes: usize,
    /// The note whose text has the keyboard.
    pub(super) editing: Option<EntityId>,
    /// Drag-and-drop keys of this window's task and note lists.
    pub(super) task_list: SharedString,
    pub(super) note_list: SharedString,
    /// The panel's width, which an open note's text wraps in.
    pub(super) width: f32,
    pub(super) this: WeakEntity<Notes>,
}

impl NotesView {
    fn render_row(&self, row: &NotesRow, ui: UiColors, window: &mut Window, cx: &App) -> AnyElement {
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
        let width = self.width;
        let (toggle, add, session) = (self.this.clone(), self.this.clone(), self.session);
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
                        add.update(cx, |this, cx| this.new_note(session, None, width, window, cx)).ok();
                    }),
                )),
                on_toggle: Some(Box::new(move |_, _, cx| {
                    toggle.update(cx, |this, cx| this.toggle_notes_section(session, cx)).ok();
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
        let session = self.session;
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
                toggle.update(cx, |this, cx| this.toggle_note(session, ix, cx)).ok();
            });
        // While the editor is open the whole text is in it, the title's line too: the title
        // quiets down so it doesn't read as duplicated content. Clicking it goes to the text, as
        // a task's does.
        let main = style::item_label(ui, title.clone(), editing)
            .id(("note-title", ix))
            .truncate()
            .cursor_text()
            .on_click(move |_, window, cx| {
                open.update(cx, |this, cx| this.open_note(session, ix, window, cx)).ok();
            });
        let sent = state.clone();
        let actions = vec![
            style::item_action(("note-send", ix), ui, IconName::ArrowUpRight, "Send to terminal", {
                move |_, _, cx| {
                    // Read at click time: rows don't carry the text.
                    let text = sent.read(cx).value().to_string();
                    send.update(cx, |this, cx| this.send_text(session, text, false, cx)).ok();
                }
            }),
            style::item_action(("note-delete", ix), ui, IconName::X, "Delete", move |_, _, cx| {
                delete.update(cx, |this, cx| this.delete_note(session, ix, cx)).ok();
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
                    keys.update(cx, |this, cx| this.note_key(session, &keyed, event, window, cx))
                        .unwrap_or(false)
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
                        opened.update(cx, |this, cx| this.open_note(session, ix, window, cx)).ok();
                    },
                    move |_, _, cx| {
                        show.update(cx, |this, cx| this.show_all_of_note(session, ix, cx)).ok();
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
                moved.update(cx, |this, cx| this.move_note(session, from, to, cx)).ok();
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

// ---------------------------------------------------------------- the table's hooks

/// The tab's words for VoiceOver: how many steps are open and notes written.
pub(super) fn spoken_tab(entity: &AnyEntity, facts: &WindowFacts<'_>, cx: &App) -> String {
    let notes = typed(entity);
    let (open_steps, notes) = facts
        .active()
        .and_then(|active| notes.read(cx).of.get(&active.id))
        .map_or((0, 0), |of| (of.tasks.open(), of.filled));
    spoken::panel_tab(spoken::PanelTab::Notes { open_steps, notes })
}

/// The tab's body: the virtual list of the terminal on screen, in a column that fills the panel so
/// the list gets a bounded height. The window gives it the area that holds the keyboard for it.
pub(super) fn tab(entity: &AnyEntity, facts: &WindowFacts<'_>, _: &mut Window, cx: &mut App) -> AnyElement {
    let notes = typed(entity);
    let Some(active) = facts.active() else { return div().into_any_element() };
    let session = active.id;
    let width = f32::from(facts.panel_width);
    let ui = ui_in(facts.profile, cx);
    // Rows are rebuilt for this frame's data; the entity is not notified, this draw is the redraw.
    notes.update(cx, |notes, cx| {
        notes.panel = width;
        notes.sync_list(session, cx);
    });
    let this = notes.read(cx);
    let Some(of) = this.of.get(&session) else { return div().into_any_element() };
    let view = Rc::new(NotesView {
        session,
        tasks: of.tasks.clone(),
        notes_collapsed: of.notes_collapsed,
        task_edit: of.edit.as_ref().map(|edit| (edit.edit, edit.input.clone())),
        notes: of.notes.iter().map(|n| n.state.clone()).collect(),
        show_all: of.notes.iter().map(|n| n.show_all).collect(),
        filled_notes: of.filled,
        editing: this.editing,
        task_list: format!("tasks-{}", notes.entity_id()).into(),
        note_list: format!("notes-{}", notes.entity_id()).into(),
        width,
        this: notes.downgrade(),
    });
    let rows = &of.list.rows;
    let ring = facts.panel_cursor.map(|(row, shown)| (row, shown.then_some(ui)));
    let profile = facts.profile.clone();
    div()
        .relative()
        .flex()
        .flex_col()
        .flex_1()
        .min_h_0()
        .child(rows.with_edges(
            rows.element_carded(ring, ui, part, move |row, window, cx| {
                view.render_row(row, ui_in(&profile, cx), window, cx)
            }),
            ui,
            "notes-scrollbar",
            cx,
        ))
        .into_any_element()
}

/// The Notes tab's one action, beside the panel toggle rather than on a row of its own above the list.
pub(super) fn stash_button(
    entity: &AnyEntity,
    facts: &WindowFacts<'_>,
    _: &mut Window,
    cx: &mut App,
) -> Option<AnyElement> {
    if !super::on(cx) {
        return None;
    }
    let active = facts.active()?;
    let ui = ui_in(facts.profile, cx);
    let (notes, session, terminal, width) =
        (typed(entity), active.id, active.terminal.clone(), f32::from(facts.panel_width));
    Some(
        div()
            .id("panel-actions")
            .occlude()
            .flex_none()
            .child(
                style::icon_tab(
                    "stash-prompt",
                    ui,
                    IconName::Inbox,
                    keymap::with_key("Stash prompt into a new note", ActionId::StashPrompt),
                    false,
                    None,
                )
                .on_click(move |_, window, cx| {
                    notes.update(cx, |notes, cx| notes.stash_prompt(session, &terminal, width, window, cx));
                }),
            )
            .into_any_element(),
    )
}
