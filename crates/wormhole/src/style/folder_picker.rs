//! The folder picker's card (⌘J at the prompt): a short list of folders, each a folder icon and a name with
//! the selected one in the list's selection colour, and a search field with a magnifier along the bottom.
//! The card is the palette's surface, which the menus and popovers share, so it takes the theme as they do.

use std::ops::Range;

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::buttons::icon;
use super::palette::{marked_text, palette_surface};
use super::scale::{CONTROL_H, Edges, ROW_H, TextRole};
use crate::settings::UiColors;

/// The card's width.
pub const FOLDER_PICKER_W: f32 = 320.0;
/// The most rows it shows before its list scrolls.
pub const FOLDER_PICKER_MAX_ROWS: usize = 8;
/// The room round the rows inside the card (`_1`).
pub const FOLDER_PICKER_LIST_PAD: f32 = 4.0;
/// The search band along the bottom: the field and the room round it (`_2`).
pub const FOLDER_PICKER_SEARCH_H: f32 = CONTROL_H + 16.0;
/// A row's height: the app's one line of a list.
pub const FOLDER_PICKER_ROW_H: f32 = ROW_H;

/// What the card is made of.
pub struct FolderPickerParts {
    /// The rows, in a box [`FolderPickerParts::list_h`] tall (a list, or the rows themselves).
    pub list: AnyElement,
    /// The list's height in rows' worth: `rows` rows of [`FOLDER_PICKER_ROW_H`] and the padding round them.
    pub rows: usize,
    /// The search field (a text field; the gallery shows plain text).
    pub field: AnyElement,
    /// The field has the keyboard: its outline is the focus colour.
    pub focused: bool,
    pub width: f32,
}

/// The card: the list above, the search field under it. `id` keeps one card's parts from another's. The card
/// is as tall as its rows ask (`2 × border + 2 × padding + the search band + rows × row height`), which the
/// window needs to place it, so none of those is left to the content.
#[track_caller]
pub fn folder_picker(id: impl Into<ElementId>, ui: UiColors, parts: FolderPickerParts) -> Stateful<Div> {
    let FolderPickerParts { list, rows, field, focused, width } = parts;
    let list_h = rows as f32 * FOLDER_PICKER_ROW_H + 2.0 * FOLDER_PICKER_LIST_PAD;
    palette_surface(ui, true)
        .id(id)
        .w(px(width))
        // A click on the card must not start a selection in the terminal under it.
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .child(div().flex_none().h(px(list_h)).p_1().child(list))
        .child(
            div().flex().flex_none().items_center().h(px(FOLDER_PICKER_SEARCH_H)).px_2().rule_top(ui).child(
                div()
                    .flex()
                    .flex_1()
                    .min_w_0()
                    .items_center()
                    .gap_2()
                    .h(px(CONTROL_H))
                    .pl_2()
                    .rounded(ui.radius)
                    .outlined(ui, if focused { ui.focus } else { ui.border })
                    .bg(ui.input)
                    .child(div().flex_none().text_color(ui.muted).child(icon(IconName::Search)))
                    .child(div().flex_1().min_w_0().child(field)),
            ),
        )
}

/// One folder: its icon and name at [`FOLDER_PICKER_ROW_H`], the characters the typed text matched marked.
/// The selected row takes the list's selection colour. Add `.on_click(…)`.
#[track_caller]
pub fn folder_picker_row(
    id: impl Into<ElementId>,
    ui: UiColors,
    selected: bool,
    name: SharedString,
    mark: Option<Range<usize>>,
) -> Stateful<Div> {
    let marks: Vec<Range<usize>> = mark.into_iter().collect();
    div()
        .id(id)
        .role(Role::ListBoxOption)
        .aria_label(name.clone())
        .aria_selected(selected)
        .flex()
        .items_center()
        .gap_2()
        .w_full()
        .h(px(FOLDER_PICKER_ROW_H))
        .px_2()
        .rounded(ui.radius)
        .cursor_pointer()
        .text_label()
        .text_color(if selected { ui.list_selected_text } else { ui.text })
        .when(selected, |d| d.bg(ui.list_selected))
        .hover(move |d| if selected { d } else { d.bg(ui.hover) })
        .child(
            div()
                .flex_none()
                .text_color(if selected { ui.list_selected_text } else { ui.primary })
                .child(icon(IconName::Folder)),
        )
        .child(div().flex_1().min_w_0().truncate().child(marked_text(ui, name, &marks, selected)))
}

/// The one line of a list with nothing to pick ("No folders"), in the place of a row.
#[track_caller]
pub fn folder_picker_note(ui: UiColors, text: impl Into<SharedString>) -> Div {
    div()
        .flex()
        .items_center()
        .h(px(FOLDER_PICKER_ROW_H))
        .px_2()
        .text_label()
        .text_color(ui.muted)
        .child(text.into())
}
