//! A submodule in the Changes and History lists: a file row that says what changed in words
//! ("2 new commits") where a file has line counts, with a button on the right while the pointer is
//! over it that opens a terminal in the submodule's own folder.

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::buttons::row_actions;
use super::rows::{KIND_LETTER_W, flush_row, kind_color, lead};
use super::scale::{INDENT, PANEL_INSET, TextRole};
use super::section::item_action;
use crate::settings::UiColors;
use wormhole_core::git::ChangeKind;

/// What a [`submodule_row`] shows.
pub struct SubmoduleRow {
    /// The submodule's folder name, and the folders above it (empty in a tree).
    pub name: SharedString,
    pub folder: SharedString,
    /// What changed, in words: the commits that moved, what is uncommitted and untracked.
    pub what: SharedString,
    pub kind: ChangeKind,
    pub depth: usize,
    pub selected: bool,
    /// The folder can be entered: the button's tooltip and what a click on it does. Without it
    /// (the submodule isn't checked out) the row has no button.
    pub enter: Option<Enter>,
}

/// The button that opens a terminal in the submodule.
pub struct Enter {
    pub tooltip: SharedString,
    pub on_click: OnEnter,
}

/// What a click on the button does.
pub type OnEnter = Box<dyn Fn(&ClickEvent, &mut Window, &mut App)>;

/// See [`SubmoduleRow`]. Add `.on_click(…)` to open its commits.
#[track_caller]
pub fn submodule_row(id: impl Into<ElementId>, ui: UiColors, row: SubmoduleRow) -> Stateful<Div> {
    let SubmoduleRow { name, folder, what, kind, depth, selected, enter } = row;
    let id: ElementId = id.into();
    let group = SharedString::from(format!("submodule-row-{id}"));
    let muted = if selected { ui.selected_muted } else { ui.muted };
    flush_row(id.clone(), ui, selected)
        .group(group.clone())
        .pl(px(PANEL_INSET + depth as f32 * INDENT))
        .child(lead(super::icon(IconName::FolderGit2).text_color(muted)))
        .child(div().flex_none().max_w(relative(0.45)).truncate().child(name))
        .when(!folder.is_empty(), |d| {
            d.child(div().min_w_0().truncate().text_caption().text_color(muted).child(folder))
        })
        .child(div().flex_1())
        .child(
            div().min_w_0().flex_shrink(WHAT_SHRINK).truncate().text_caption().text_color(muted).child(what),
        )
        .children(enter.map(|enter| {
            row_actions(
                (id.clone(), "actions"),
                group.clone(),
                false,
                vec![item_action(
                    (id.clone(), "enter"),
                    ui,
                    IconName::SquareTerminal,
                    enter.tooltip,
                    enter.on_click,
                )],
            )
        }))
        .child(
            div()
                .flex_none()
                .w(px(KIND_LETTER_W))
                .flex()
                .justify_end()
                .text_caption()
                .font_weight(FontWeight::BOLD)
                .text_color(kind_color(ui, kind))
                .child(kind.letter()),
        )
}

/// The words give way after the folder does, but before the name.
const WHAT_SHRINK: f32 = 0.5;
