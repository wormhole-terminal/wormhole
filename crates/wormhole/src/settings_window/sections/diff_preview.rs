//! Changes & diffs' preview: the Changes panel and a diff beside it, drawn with the panel's own file
//! rows and the diff view's own rows (`features::git::diff::sample`), under the options of the page:
//! list or tree, untracked files apart or not, split or unified, wrapped or cut, the tab's width.

use gpui_kit::assets::IconName;
use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::git::ChangeKind;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::text;

use super::card::{Words, card};
use crate::features::git::diff::sample::{self, Look};
use crate::features::git::diff_render::DiffColors;
use crate::settings::{SettingsStore, UiColors, settings};
use crate::settings_window::{SettingsWindow, ui};
use crate::style::{self, Edges, FileLines, FileRow, TextRole};

/// The file list's width: the panel's, as the right column keeps it.
const LIST_W: f32 = 200.0;

/// One file of the sample panel.
struct File {
    folder: &'static str,
    name: &'static str,
    kind: ChangeKind,
    lines: FileLines,
}

// text: sample (made-up files of the preview)
const FILES: [File; 4] = [
    File { folder: "src", name: "limits.rs", kind: ChangeKind::Modified, lines: FileLines::Counts(2, 1) },
    File { folder: "src", name: "lib.rs", kind: ChangeKind::Modified, lines: FileLines::Counts(3, 1) },
    File { folder: "tests", name: "limits.rs", kind: ChangeKind::Added, lines: FileLines::Counts(30, 0) },
    File { folder: "", name: "notes.txt", kind: ChangeKind::Untracked, lines: FileLines::Counts(5, 0) },
];

/// Changes & diffs → the panel and a diff as the options make them.
pub(in crate::settings_window) fn draw_diff_preview(
    _: &SettingsWindow,
    cx: &mut Context<SettingsWindow>,
) -> AnyElement {
    let ui = ui(cx);
    let s = settings(cx).clone();
    let store = cx.global::<SettingsStore>();
    let colors = DiffColors::new(&store.tokens);
    let (family, size) = crate::fonts::code_font(cx);
    let look = Look { split: s.diff_split, wrap: s.diff_wrap, tab_width: s.tab_width };
    let list = changes_panel(ui, s.changes_tree, s.group_untracked);
    let diff = div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .bg(colors.surface)
        .child(
            div()
                .flex_none()
                .flex()
                .items_center()
                .h(px(style::ROW_H))
                .px_3()
                .rule_bottom(ui)
                .font_family(family.clone())
                .text_caption()
                .text_color(ui.muted)
                .child("src/limits.rs"), // text: sample (a made-up file of the preview)
        )
        .child(sample::preview_rows(ui, colors, look, family, f32::from(size)));
    // Edge to edge: the panel's and the diff's own surfaces fill the frame.
    let frame = style::preview_frame(ui).p_0().flex().child(list).child(diff);
    card(
        ui,
        Words {
            id: "diff-preview-card",
            what: SETTINGS_WINDOW_DIFF,
            spoken: SETTINGS_WINDOW_PREVIEW_DIFF_LABEL,
            caption: SETTINGS_GIT_PREVIEW_CAPTION,
        },
        frame,
    )
}

/// The Changes panel: the files as a list or a tree, the untracked one under a head of its own or among
/// the others.
fn changes_panel(ui: UiColors, tree: bool, untracked_apart: bool) -> Div {
    let (apart, among): (Vec<_>, Vec<_>) =
        FILES.iter().partition(|file| untracked_apart && file.kind == ChangeKind::Untracked);
    let mut panel = div()
        .flex_none()
        .w(px(LIST_W))
        .flex()
        .flex_col()
        .gap_1()
        .pb_2()
        .bg(ui.panel)
        .rule_right(ui)
        .child(head(ui, text(SETTINGS_GIT_PREVIEW_CHANGES), among.len()));
    panel = panel.children(files(ui, &among, tree, true));
    if !apart.is_empty() {
        panel = panel
            .child(head(ui, text(SETTINGS_GIT_PREVIEW_UNTRACKED), apart.len()))
            .children(files(ui, &apart, tree, false));
    }
    panel
}

/// A block's head: its name and how many files it holds.
fn head(ui: UiColors, title: &str, count: usize) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap_2()
        .h(px(style::ROW_H))
        .px_3()
        .text_caption()
        .font_weight(FontWeight::MEDIUM)
        .text_color(ui.muted)
        .child(title.to_owned())
        .child(style::count_badge(ui, count.to_string()))
}

/// `files` as rows: flat, each with its folder beside it, or under a row for each folder. The first
/// block's first file is the one open in the diff.
fn files(ui: UiColors, files: &[&File], tree: bool, first_block: bool) -> Vec<AnyElement> {
    let id_base = if first_block { 0 } else { 100 };
    let mut rows = Vec::new();
    let mut folder = "";
    for (ix, file) in files.iter().enumerate() {
        if tree && file.folder != folder && !file.folder.is_empty() {
            folder = file.folder;
            rows.push(
                style::folder_row(
                    ("diff-preview-folder", id_base + ix),
                    ui,
                    0,
                    style::icon(IconName::Folder).into_any_element(),
                    folder,
                    false,
                )
                .into_any_element(),
            );
        }
        rows.push(
            style::file_row(
                ("diff-preview-file", id_base + ix),
                ui,
                FileRow {
                    icon: style::icon(IconName::File).into_any_element(),
                    name: file.name.into(),
                    folder: if tree { "".into() } else { file.folder.into() },
                    renamed_from: None,
                    moved: false,
                    kind: file.kind,
                    lines: file.lines,
                    depth: usize::from(tree && !file.folder.is_empty()),
                    selected: first_block && ix == 0,
                },
            )
            .into_any_element(),
        );
    }
    rows
}
