//! The picker's card: placed against the terminal's cursor (core's `place`), the rows in a virtual list, the
//! search field along the bottom. The parts are `style::folder_picker*`.

use std::ops::Range;

use gpui_kit::component::input::Input;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::folder_picker::{Card, place};
use wormhole_core::view::panes::Rect;
use wormhole_text::{keys, text};

use super::FolderPicker;
use crate::settings::{UiColors, ui};
use crate::style;

/// What the cursor is taken to be where the terminal has none to show (a full-screen program hid it): a line
/// at the pane's bottom left.
const FALLBACK_CELL: (f32, f32) = (8.0, 18.0);

impl FolderPicker {
    /// The pane and the cursor's cell, in the window's space: the terminal's own last frame, else a guess.
    fn anchor(&self, window: &Window, cx: &App) -> (Rect, Rect) {
        let viewport = window.viewport_size();
        let window_rect = Rect::new(0.0, 0.0, f32::from(viewport.width), f32::from(viewport.height));
        let Some(terminal) = self.terminal.upgrade() else { return (window_rect, Rect::default()) };
        let terminal = terminal.read(cx);
        let rect = |b: Bounds<Pixels>| {
            Rect::new(
                f32::from(b.origin.x),
                f32::from(b.origin.y),
                f32::from(b.size.width),
                f32::from(b.size.height),
            )
        };
        // A pane not drawn yet is zero wide: the window stands in.
        let pane =
            Some(rect(terminal.pane_bounds())).filter(|r| r.w > 0.0 && r.h > 0.0).unwrap_or(window_rect);
        let cursor = terminal.cursor_cell().map(rect).unwrap_or_else(|| {
            Rect::new(
                pane.x + style::EDGE_INSET,
                pane.bottom() - 2.0 * FALLBACK_CELL.1,
                FALLBACK_CELL.0,
                FALLBACK_CELL.1,
            )
        });
        (pane, cursor)
    }

    /// A row of the list: a folder, or the note after the last one that says the folder has more than are read.
    fn row(&self, ix: usize, ui: UiColors, cx: &mut Context<Self>) -> AnyElement {
        let Some(row) = self.state.rows().get(ix) else {
            let count =
                wormhole_core::view::limits::count_grouped(wormhole_core::storage::files::FOLDERS_CAP);
            return style::folder_picker_note(ui, keys::folder_picker_cut(&count)).into_any_element();
        };
        let selected = self.state.selected() == Some(ix);
        style::folder_picker_row(
            ("folder-picker-row", ix),
            ui,
            selected,
            row.name.clone().into(),
            row.mark.clone(),
        )
        .on_click(cx.listener(move |this, _, window, cx| {
            this.state.select(ix);
            this.pick(window, cx);
        }))
        .into_any_element()
    }
}

impl Render for FolderPicker {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = ui(cx);
        let (pane, cursor) = self.anchor(window, cx);
        let rows = self.state.rows().len();
        // The note after the rows, the empty list's one line, or a list still being read (one blank line).
        let notes = usize::from(self.state.cut() && rows > 0);
        let shown = rows + notes;
        let card = Card {
            width: style::FOLDER_PICKER_W,
            chrome: 2.0 * style::FOLDER_PICKER_LIST_PAD
                + style::FOLDER_PICKER_SEARCH_H
                + 2.0 * f32::from(ui.border_width),
            row: style::FOLDER_PICKER_ROW_H,
            max_rows: style::FOLDER_PICKER_MAX_ROWS,
        };
        let placed = place(pane, cursor, card, shown.max(1));
        self.placed.set(Some(placed));

        let list = if shown == 0 && self.state.loaded() {
            let words = if self.state.unreadable() {
                text(keys::FOLDER_PICKER_UNREADABLE)
            } else {
                text(keys::FOLDER_PICKER_EMPTY)
            };
            style::folder_picker_note(ui, words).into_any_element()
        } else {
            uniform_list(
                "folder-picker-list",
                shown,
                cx.processor(move |this, range: Range<usize>, _, cx| {
                    range.map(|ix| this.row(ix, ui, cx)).collect::<Vec<_>>()
                }),
            )
            .track_scroll(&self.scroll)
            .size_full()
            .into_any_element()
        };
        let field = Input::new(&self.field).appearance(false).into_any_element();
        let focused = self.field.focus_handle(cx).is_focused(window);
        let r = placed.rect;
        let surface = style::folder_picker(
            "folder-picker",
            ui,
            style::FolderPickerParts { list, rows: placed.rows, field, focused, width: r.w },
        )
        .debug_selector(|| "folder-picker".into())
        .on_mouse_down_out(cx.listener(|this, _, window, cx| this.close(window, cx)));
        // The overlay is the window's size and takes nothing itself: only the card does.
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .child(div().absolute().left(px(r.x)).top(px(r.y)).child(surface))
    }
}
