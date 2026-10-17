//! Text and boxes that label or set apart other content: section labels, panel titles,
//! badges, chips, cards, empty notes and the git branch marks.

use gpui_kit::assets::IconName;
use gpui_kit::component::{Icon, Sizable};
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::scale::{BADGE_H, BRANCH_CHIP_MAX, BRANCH_CHIP_MIN, Edges, MARK_DOT, TextRole};
use crate::settings::UiColors;

/// The small uppercase heading above a group of rows or settings (12 / 600, muted).
#[track_caller]
pub fn section_label(ui: UiColors, text: &str) -> Div {
    div().text_caption().font_weight(FontWeight::SEMIBOLD).text_color(ui.muted).child(text.to_uppercase())
}

/// The bold title of a panel: "Terminals" over the sidebar list, "Settings" over its nav.
/// Unlike [`section_label`] it keeps its case and full text color, so the panels read as
/// headings rather than as a quiet group caption.
#[track_caller]
pub fn panel_title(ui: UiColors, text: impl Into<SharedString>) -> Div {
    div().text_heading().font_weight(FontWeight::BOLD).text_color(ui.text).child(text.into())
}

/// A count next to a band's title, a tab or a nav item: a chip filled with the hairline
/// color, at least as wide as it is tall, the digits in `text.primary` (12 / 600).
/// Spell large numbers with `style::count_label` (999+).
#[track_caller]
pub fn count_badge(ui: UiColors, text: impl Into<SharedString>) -> Div {
    badge(ui).px_1().bg(ui.border).text_color(ui.text).font_weight(FontWeight::SEMIBOLD).child(text.into())
}

/// A [`count_badge`] on a selected tint (the active icon tab, the current session): a
/// stronger tint of the primary (`state.selected.inset`) with the tint's text color.
#[track_caller]
pub fn selected_count_badge(ui: UiColors, text: impl Into<SharedString>) -> Div {
    badge(ui)
        .px_1()
        .bg(ui.selected_inset)
        .text_color(ui.selected_text)
        .font_weight(FontWeight::SEMIBOLD)
        .child(text.into())
}

/// A short status word in a color (trusted, changed): the outlined sibling of
/// [`count_badge`], with the same size and corners.
#[track_caller]
pub fn status_badge(ui: UiColors, color: Hsla, text: impl Into<SharedString>) -> Div {
    badge(ui).px_2().outlined(ui, color).text_color(color).child(text.into())
}

/// The box both badges share: caption text centered in [`BADGE_H`] by at least
/// [`BADGE_H`], with the chip corners.
#[track_caller]
fn badge(ui: UiColors) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .h(px(BADGE_H))
        .min_w(px(BADGE_H))
        .rounded(ui.radius_chip)
        .text_caption()
        .tabular_nums()
}

/// A git branch on a folder band, after the folder's name: ⎇ and the name in
/// `git.branch` on its own tint, 20 tall, truncating at [`BRANCH_CHIP_MAX`] so the folder
/// name never gives way first. `None` is a detached HEAD, which isn't on any branch: it
/// reads "detached" in the warning colors. `changes` adds the folder's "±N" after the
/// chip, in muted digits.
#[track_caller]
pub fn branch_chip(ui: UiColors, branch: Option<&str>, changes: Option<usize>) -> Div {
    div().flex().items_center().gap_2().min_w_0().child(branch_chip_box(ui, branch, CHIP_H)).children(
        changes.map(|n| {
            div().flex_none().text_caption().tabular_nums().text_color(ui.muted).child(format!("±{n}"))
        }),
    )
}

/// The chip both branch marks share: ⎇ and the branch (or "detached") `height` tall.
#[track_caller]
fn branch_chip_box(ui: UiColors, branch: Option<&str>, height: f32) -> Div {
    let (color, fill, name) = match branch {
        Some(branch) => (ui.branch, ui.branch_bg, branch),
        None => (ui.warning_text, ui.banner_warning, "detached"),
    };
    div()
        .flex()
        .items_center()
        .gap_1()
        .h(px(height))
        .min_w(px(BRANCH_CHIP_MIN))
        .max_w(px(BRANCH_CHIP_MAX))
        .pl_1()
        .pr_2()
        .rounded(ui.radius_chip)
        .bg(fill)
        .text_color(color)
        .text_caption()
        .font_weight(FontWeight::MEDIUM)
        .child(Icon::new(IconName::GitBranch).xsmall())
        .child(div().min_w_0().truncate().child(name.to_string()))
}

/// Height of a chip that sits in a row's text line: a branch chip, a project's tint chip.
const CHIP_H: f32 = 20.0;

/// A chip in the primary's tint (`chip.tint`) with primary text: what overrides a setting
/// (a project, with its folder icon), a font's "Bundled" tag, the sidebar's "Filtered" chip.
#[track_caller]
pub fn tint_chip(ui: UiColors, icon: Option<IconName>, text: impl Into<SharedString>) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap_1()
        .h(px(CHIP_H))
        .px_2()
        .rounded(ui.radius_chip)
        .bg(ui.chip_tint)
        .text_color(ui.selected_text)
        .text_caption()
        .font_weight(FontWeight::MEDIUM)
        .whitespace_nowrap()
        .children(icon.map(|icon| Icon::new(icon).xsmall()))
        .child(text.into())
}

/// A chip in a neutral fill (`band.hover`, the text color at 8 %) with `text.primary` text: a
/// short name in a list that should not pull the eye, as the Theme Studio's "Used by".
/// The [`tint_chip`] is the primary's, for what overrides something.
#[track_caller]
pub fn neutral_chip(ui: UiColors, text: impl Into<SharedString>) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .h(px(CHIP_H))
        .px_2()
        .rounded(ui.radius_chip)
        .bg(ui.band_hover)
        .text_color(ui.text)
        .text_caption()
        .font_weight(FontWeight::MEDIUM)
        .whitespace_nowrap()
        .child(text.into())
}

/// The primary dot that marks something changed from its default (a setting) or wanted
/// (a hidden panel, an active filter): [`MARK_DOT`] across.
#[track_caller]
pub fn changed_dot(ui: UiColors) -> Div {
    div().flex_none().size(px(MARK_DOT)).rounded_full().bg(ui.primary)
}

/// A file's added and removed line counts, "+N −M" in the diff colors (caption digits that
/// keep their width). A binary file, which has no lines, passes `None` for both.
#[track_caller]
pub fn line_counts(ui: UiColors, added: Option<usize>, removed: Option<usize>) -> Div {
    div()
        .flex_none()
        .flex()
        .gap_1()
        .text_caption()
        .tabular_nums()
        .children(added.map(|n| div().text_color(ui.added).child(format!("+{n}"))))
        .children(removed.map(|n| div().text_color(ui.removed).child(format!("−{n}"))))
}

/// A bordered block that sets a few lines apart from the page around it: release
/// notes, a trust prompt, the commands a project wants to run.
#[track_caller]
pub fn card(ui: UiColors, border: Hsla) -> Div {
    div().p_3().rounded(ui.radius_lg).outlined(ui, border)
}

/// A line of quiet text where a list would be: loading, empty, not a repository.
#[track_caller]
pub fn empty_note(ui: UiColors, text: impl Into<SharedString>) -> Div {
    div().px_3().py_2().text_body().text_color(ui.muted).child(text.into())
}

/// A git branch name after a ⎇, in the `git.branch` color, truncating; the caller
/// sets the text size. `None` is a detached HEAD, which isn't on any branch, so it
/// reads "detached" in the warning color instead.
#[track_caller]
pub fn branch_label(ui: UiColors, branch: Option<&str>) -> Div {
    let (color, name) = match branch {
        Some(branch) => (ui.branch, branch),
        None => (ui.warning_text, "detached"),
    };
    div().min_w_0().truncate().text_color(color).child(format!("⎇ {name}"))
}

/// The ⎇ alone, in the branch color: marks a branch name inside a control that
/// keeps its own text color (the branch selects in the git panel).
#[track_caller]
pub fn branch_mark(ui: UiColors) -> Div {
    div().flex_none().text_color(ui.branch).child("⎇")
}
