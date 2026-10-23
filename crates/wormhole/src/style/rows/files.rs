//! The Changes tab's rows: a file with its lines and kind letter, and a folder of the tree.

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable as _};
use gpui_kit::prelude::*;
use gpui_kit::*;

use crate::settings::UiColors;
use crate::style::bands::CARD_INSET;
use crate::style::labels::line_counts;
use crate::style::scale::{CARD_ROW_RADIUS, FILE_NAME_MAX, INDENT, TextRole};
use wormhole_core::git::ChangeKind;

use super::{flush_row, lead};

/// What a [`file_row`] says about a file's lines, on the right before its kind letter.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileLines {
    /// Added and removed lines; a new file has nothing removed, so it says only what it adds.
    Counts(usize, usize),
    /// A binary file: "bin".
    Binary,
    /// Stored with Git LFS, whose counts would be the pointer's: "LFS".
    Lfs,
}

/// One changed file in the Changes panel or under a commit: its icon in the lead
/// column, the name at x 40 (at most [`FILE_NAME_MAX`] of the row), its folder in muted caption
/// text, then on the right the line counts and the kind letter in its status color. A renamed
/// file shows where it came from before the new name, and one moved to another folder a mark.
/// Each tree level steps in by [`INDENT`].
/// The caller shortens a long name in the middle and a long folder at the front
/// (`wormhole_core::view::elide`); what still doesn't fit is cut at the end.
pub struct FileRow {
    /// From `crate::icons::file`, or `style::icon(IconName::File)`.
    pub icon: AnyElement,
    pub name: SharedString,
    /// Empty in a tree, where the folder rows above say it.
    pub folder: SharedString,
    /// The old name of a renamed file, when it differs from the new one.
    pub renamed_from: Option<SharedString>,
    /// The file came from another folder (a `git mv` across folders): a mark before the name
    /// says so, where "a.rs → a.rs" would say nothing.
    pub moved: bool,
    pub kind: ChangeKind,
    pub lines: FileLines,
    pub depth: usize,
    pub selected: bool,
}

/// See [`FileRow`]. Add `.on_click(…)`.
#[track_caller]
pub fn file_row(id: impl Into<ElementId>, ui: UiColors, row: FileRow) -> Stateful<Div> {
    let FileRow { icon, name, folder, renamed_from, moved, kind, lines, depth, selected } = row;
    let muted = if selected { ui.selected_muted } else { ui.muted };
    let letter_color = kind_color(ui, kind);
    flush_row(id, ui, selected)
        .pl(px(CARD_INSET + depth as f32 * INDENT))
        .pr(px(CARD_INSET))
        .rounded(px(CARD_ROW_RADIUS))
        .child(lead(icon))
        .children(renamed_from.map(|from| {
            div()
                .flex()
                .items_center()
                .gap_1()
                .min_w_0()
                .text_caption()
                .text_color(muted)
                .child(div().min_w_0().truncate().child(from))
                .child(Icon::new(IconName::ArrowRight).with_size(px(RENAME_ARROW)))
        }))
        .when(moved, |d| {
            d.child(
                div()
                    .flex_none()
                    .text_color(muted)
                    .child(Icon::new(IconName::FolderInput).with_size(px(MOVED_MARK))),
            )
        })
        .child(
            div()
                .flex_none()
                .max_w(relative(FILE_NAME_MAX))
                .truncate()
                .when(selected, |d| d.font_weight(FontWeight::SEMIBOLD))
                .child(name),
        )
        .when(!folder.is_empty(), |d| {
            d.child(div().min_w_0().truncate().text_caption().text_color(muted).child(folder))
        })
        .child(div().flex_1())
        .child(match lines {
            // A new file has nothing removed, so it says only what it adds.
            FileLines::Counts(added, removed) => {
                line_counts(ui, Some(added), (removed > 0).then_some(removed)).into_any_element()
            }
            FileLines::Binary => line_word(muted, "bin"),
            FileLines::Lfs => line_word(muted, "LFS"),
        })
        .child(
            div()
                .flex_none()
                .w(px(KIND_LETTER_W))
                .flex()
                .justify_end()
                .text_caption()
                .font_weight(FontWeight::BOLD)
                .text_color(letter_color)
                .child(kind.letter()),
        )
}

/// The color of a change's kind letter.
pub(in crate::style) fn kind_color(ui: UiColors, kind: ChangeKind) -> Hsla {
    match kind {
        ChangeKind::Modified => ui.warning_text,
        ChangeKind::Added | ChangeKind::Untracked => ui.success_text,
        ChangeKind::Deleted | ChangeKind::Conflicted => ui.danger_text,
        // The primary as text, judged on the selected tint as well as on the panel.
        ChangeKind::Renamed => ui.selected_text,
    }
}

/// "bin" or "LFS" where a file row's line counts go.
#[track_caller]
fn line_word(color: Hsla, word: &'static str) -> AnyElement {
    div().flex_none().text_caption().text_color(color).child(word).into_any_element()
}

/// The arrow between a renamed file's old and new name, and the mark of a moved file.
const RENAME_ARROW: f32 = 12.0;
pub const MOVED_MARK: f32 = 12.0;
/// The kind letter's column, so the line counts line up above one another.
pub const KIND_LETTER_W: f32 = 12.0;

/// A folder of the Changes tree: the chevron in the lead column, then the folder's
/// icon and its name (a chain of single folders is one row, "crates/wormhole/src") in muted
/// text. Each level steps in by [`INDENT`]. Add `.on_click(…)` to fold it.
#[track_caller]
pub fn folder_row(
    id: impl Into<ElementId>,
    ui: UiColors,
    depth: usize,
    icon: AnyElement,
    label: impl Into<SharedString>,
    collapsed: bool,
) -> Stateful<Div> {
    flush_row(id, ui, false)
        .pl(px(CARD_INSET + depth as f32 * INDENT))
        .pr(px(CARD_INSET))
        .rounded(px(CARD_ROW_RADIUS))
        .text_color(ui.muted)
        .child(lead(crate::style::controls::disclosure(ui, collapsed)))
        // The icon takes a lead column of its own, so the name lands one level in (x 64).
        .child(lead(icon))
        .child(div().min_w_0().truncate().child(label.into()))
}
