//! One row of the tree as the Changes list draws its rows: the lead column with the chevron and the
//! icon, the name in body text, a selected row in the primary's tint. A folder's icon and a file's come from
//! the icon theme in force; a link has the link mark in place of its kind's icon. A note (an empty folder,
//! "N more") is a muted line under the folder it is about.

use gpui_kit::assets::IconName;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::theme::icon_theme::IconTheme;
use wormhole_core::view::file_tree::{Failure, Row, RowKind};
use wormhole_core::view::limits::count_grouped;
use wormhole_text::{keys, text};

use crate::settings::UiColors;
use crate::style::{self, CARD_INSET, CARD_ROW_RADIUS, INDENT, TextRole as _};

/// What a row is drawn with besides the row.
pub(super) struct RowLook<'a> {
    pub ui: UiColors,
    pub icons: Option<&'a IconTheme>,
    pub selected: bool,
}

/// Indent of a row's content, in steps of one lead column and its gap. A folder's chevron starts the
/// step; a file's icon sits under the icon of the folders beside it, so their names line up, and a note
/// is under the names.
fn indent(row: &Row) -> f32 {
    let steps = match row.kind {
        RowKind::Folder { .. } => row.depth,
        RowKind::File | RowKind::Link { .. } => row.depth + 1,
        _ => row.depth + 2,
    };
    CARD_INSET + steps as f32 * INDENT
}

/// The row, to add `.on_click(…)` and a menu to.
#[track_caller]
pub(super) fn entry_row(row: &Row, look: &RowLook<'_>) -> Stateful<Div> {
    let RowLook { ui, icons, selected } = look;
    let (ui, selected) = (*ui, *selected);
    let muted = if selected { ui.selected_muted } else { ui.muted };
    let id = SharedString::from(format!("files-{}", row.path.display()));
    let name = row.label.clone();
    let (icon, spoken) = match row.kind {
        RowKind::Folder { open } => {
            let icon = crate::icons::folder(*icons, &name, open, muted)
                .unwrap_or_else(|| style::icon(IconName::Folder).text_color(muted).into_any_element());
            let words = if open {
                keys::files_spoken_folder_open(&name)
            } else {
                keys::files_spoken_folder_closed(&name)
            };
            (icon, words)
        }
        RowKind::Link { to_folder } => {
            let mark = if to_folder { IconName::FolderSymlink } else { IconName::FileSymlink };
            (style::icon(mark).text_color(muted).into_any_element(), keys::files_spoken_link(&name))
        }
        _ => {
            let icon = crate::icons::file(*icons, &name, muted)
                .unwrap_or_else(|| style::icon(IconName::File).text_color(muted).into_any_element());
            (icon, name.clone())
        }
    };
    let tooltip = SharedString::from(row.label.clone());
    let base = style::flush_row(id, ui, selected)
        .debug_selector({
            let path = row.path.display().to_string();
            move || format!("files-row-{path}")
        })
        .pl(px(indent(row)))
        .pr(px(CARD_INSET))
        .rounded(px(CARD_ROW_RADIUS))
        .aria_label(SharedString::from(spoken))
        .tooltip(move |window, cx| Tooltip::new(tooltip.clone()).build(window, cx));
    let base = match row.kind {
        RowKind::Folder { open } => base.aria_expanded(open).child(style::lead(style::disclosure(ui, !open))),
        _ => base,
    };
    base.child(style::lead(icon))
        .child(div().min_w_0().truncate().when(selected, |d| d.font_weight(FontWeight::SEMIBOLD)).child(name))
}

/// A muted line about a folder: it is being read, empty, can't be read, or has more than is listed.
#[track_caller]
pub(super) fn note_row(row: &Row, ui: UiColors) -> Div {
    let words = match &row.kind {
        RowKind::Reading => text(keys::FILES_READING).to_string(),
        RowKind::Empty => text(keys::FILES_EMPTY).to_string(),
        RowKind::More(count) => keys::files_more(&count_grouped(*count)),
        RowKind::Unreadable(Failure::Gone) => text(keys::FILES_FOLDER_GONE).to_string(),
        RowKind::Unreadable(Failure::Denied) => text(keys::FILES_FOLDER_DENIED).to_string(),
        RowKind::Unreadable(Failure::Other) => text(keys::FILES_FOLDER_UNREADABLE).to_string(),
        RowKind::Folder { .. } | RowKind::File | RowKind::Link { .. } => String::new(),
    };
    div()
        .w_full()
        .min_h(px(style::ROW_H))
        .flex()
        .items_center()
        .pl(px(indent(row)))
        .pr(px(CARD_INSET))
        .text_caption()
        .text_color(ui.muted)
        .child(words)
}
