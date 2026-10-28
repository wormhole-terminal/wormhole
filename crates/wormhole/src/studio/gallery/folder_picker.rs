//! The folder picker's card: folders with one selected and the typed text marked, and the line of an empty list.

use gpui_kit::prelude::*;
use gpui_kit::*;

use super::Env;
use super::view::GalleryView;
use crate::style::{self, TextRole};

pub(super) fn render(
    _studio: &GalleryView,
    env: &Env,
    _part: usize,
    _cx: &mut Context<GalleryView>,
) -> AnyElement {
    let ui = env.ui;
    // The names are what a folder holds; "sr" is typed, so it is marked where it matched.
    let folders: [(&'static str, bool, Option<std::ops::Range<usize>>); 4] = [
        ("src", true, Some(0..2)),
        ("resources", false, None),
        ("My Documents", false, None),
        ("Sources", false, Some(0..2)),
    ];
    let rows = folders
        .into_iter()
        .enumerate()
        .map(|(ix, (name, selected, mark))| {
            style::folder_picker_row(("g-folder-row", ix), ui, selected, name.into(), mark).into_any_element()
        })
        .collect::<Vec<_>>();
    let field = |text: &'static str| div().text_body().text_color(ui.text).child(text).into_any_element();
    let card = style::folder_picker(
        "g-folder-picker",
        ui,
        style::FolderPickerParts {
            list: div().flex().flex_col().children(rows).into_any_element(),
            rows: 4,
            field: field("sr"),
            focused: true,
            width: style::FOLDER_PICKER_W,
        },
    );
    let empty = style::folder_picker(
        "g-folder-picker-empty",
        ui,
        style::FolderPickerParts {
            list: style::folder_picker_note(ui, "No folders").into_any_element(),
            rows: 1,
            field: field("zzz"),
            focused: false,
            width: style::FOLDER_PICKER_W,
        },
    );
    div().flex().items_start().gap_4().child(card).child(empty).into_any_element()
}
