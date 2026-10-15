//! What the diff card says where it shows no lines: the card in place of a diff
//! too large to load unasked, and the note over a column that has nothing to show (a new or
//! deleted file's missing side, a binary file with its sizes, a file stored with Git LFS, an
//! empty file, a change of mode alone).

use gpui_kit::assets::IconName;
use gpui_kit::component::Sizable;
use gpui_kit::component::button::{Button, ButtonVariants};
use gpui_kit::prelude::*;
use gpui_kit::*;

use super::DiffView;
use super::render::DiffColors;
use crate::settings::UiColors;
use crate::settings::settings_in;
use crate::style::{self, Edges, TextRole};
use crate::workspace::Workspace;
use wormhole_core::config::SizeUnits;
use wormhole_core::git::{self, Side};

/// The text buttons of the too-large card, as tall as the panel's.
const BUTTON_H: f32 = 24.0;

/// The note over a column with no lines: why it is empty, and what more there is to say.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Note {
    pub text: &'static str,
    /// A binary file's size, what a Git LFS pointer stands for.
    pub detail: Option<String>,
    /// The LFS object the detail names by its first digits, whole, for copying.
    pub oid: Option<String>,
}

/// The notes over the columns: one per column when split (`None` over one with lines), a
/// single one when unified, and none over a unified diff that has lines. Sizes are written in
/// `units`.
pub(super) fn notes(view: &DiffView, split: bool, empty: bool, units: SizeUnits) -> Vec<Option<Note>> {
    let (old, new) = view.sides;
    let diff = view.diff.as_deref();
    let sizes = diff.map(|d| d.sizes).unwrap_or_default();
    let lfs = diff.and_then(|d| d.lfs.as_ref());
    let pointer_of =
        |old_side: bool| lfs.and_then(|c| if old_side { c.old.as_ref() } else { c.new.as_ref() });
    let text = |side: Side, old_side: bool| match side {
        Side::Missing if old_side => Some("New file"),
        Side::Missing => Some("Deleted"),
        Side::Binary => Some("Binary file"),
        Side::Lfs => Some("Stored in Git LFS"),
        Side::Text if !empty => None,
        Side::Text if old == Side::Missing || new == Side::Missing => Some("Empty file"),
        Side::Text => Some("No content changes"),
    };
    if split {
        // Each column says its own side: its size, or its object.
        let note = |side: Side, old_side: bool| {
            let (detail, oid) = match side {
                Side::Binary => {
                    ((if old_side { sizes.0 } else { sizes.1 }).map(|b| git::byte_size(b, units)), None)
                }
                Side::Lfs => {
                    let pointer = pointer_of(old_side);
                    (pointer.map(|p| git::lfs_side(p, units)), pointer.map(|p| p.oid.clone()))
                }
                _ => (None, None),
            };
            text(side, old_side).map(|text| Note { text, detail, oid })
        };
        return vec![note(old, true), note(new, false)];
    }
    if !empty {
        return Vec::new();
    }
    // One column: the side the file has now (the old one when it was deleted), with both sides'
    // sizes, and the object the summary names (the new one, or the last one it was).
    let side = if new == Side::Missing { old } else { new };
    let (detail, oid) = match side {
        Side::Binary => (git::sizes_label(sizes.0, sizes.1, units), None),
        Side::Lfs => {
            let named = lfs.and_then(|c| c.new.as_ref().or(c.old.as_ref()));
            (lfs.and_then(|c| git::lfs_summary(c, units)), named.map(|p| p.oid.clone()))
        }
        _ => (None, None),
    };
    vec![text(side, false).map(|text| Note { text, detail, oid })]
}

/// The hover group of a note's detail line, which shows the copy button of an LFS object.
const DETAIL_GROUP: &str = "diff-note-detail";

/// A column's note, set on the card's surface so it reads over the hatch, with its detail
/// below in quiet figures. An LFS object's id is cut to six digits there; hovering the line
/// shows a button that copies the whole id, beside the text so the note stays where it is.
pub(crate) fn side_note(ui: UiColors, colors: DiffColors, note: Note) -> Div {
    let oid = note.oid;
    div()
        .flex()
        .flex_col()
        .items_center()
        .gap_2()
        .child(
            div()
                .px_2()
                .py_1()
                .rounded(ui.radius_chip)
                .outlined(ui, ui.border)
                .bg(colors.surface)
                .text_caption()
                .text_color(ui.muted)
                .child(note.text),
        )
        .children(note.detail.map(|detail| {
            div()
                .group(DETAIL_GROUP)
                .relative()
                .text_caption()
                .tabular_nums()
                .text_color(ui.muted)
                .whitespace_nowrap()
                .child(detail)
                .children(oid.map(|oid| {
                    div()
                        .absolute()
                        .left_full()
                        .top_0()
                        .bottom_0()
                        .ml_1()
                        .flex()
                        .items_center()
                        .invisible()
                        .group_hover(DETAIL_GROUP, |d| d.visible())
                        .child(
                            style::icon_button("diff-copy-oid", ui, IconName::Copy, "Copy object id")
                                .in_row()
                                .on_click(move |_, _, cx| {
                                    cx.write_to_clipboard(ClipboardItem::new_string(oid.clone()))
                                }),
                        )
                }))
        }))
}

impl Workspace {
    /// In place of a diff too large to load unasked: how big it is and how many lines changed
    /// (git's count, from the status), then Load diff, and Reveal in Finder for a file on disk.
    pub(super) fn render_too_large(&self, view: &DiffView, ui: UiColors, cx: &mut Context<Self>) -> Div {
        let file = view.file();
        let changed = match (file.added, file.removed) {
            (None, None) => None,
            (added, removed) => Some(added.unwrap_or(0) as usize + removed.unwrap_or(0) as usize),
        };
        let patch_bytes = view.diff.as_ref().and_then(|d| d.patch_bytes);
        let units = settings_in(&self.profile, cx).size_units;
        let button = |id: &'static str, label: &'static str| {
            Button::new(id).outline().xsmall().h(px(BUTTON_H)).px_2().label(label)
        };
        div()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap_3()
            .p_4()
            .child(div().text_body().text_color(ui.text).child(git::too_large_title(patch_bytes, units)))
            .child(
                div().text_caption().text_color(ui.muted).text_center().child(git::too_large_detail(changed)),
            )
            .child(
                div()
                    .flex()
                    .gap_2()
                    .child(
                        button("diff-load-anyway", "Load diff")
                            .primary()
                            .on_click(cx.listener(|this, _, _, cx| this.load_diff_anyway(cx))),
                    )
                    .when(view.on_disk().is_some(), |d| {
                        d.child(
                            button("diff-reveal", view.reveal_label())
                                .on_click(cx.listener(|this, _, _, _| this.reveal_diff_file())),
                        )
                    }),
            )
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;
    use std::rc::Rc;
    use std::sync::Arc;

    use super::{Note, notes};
    use crate::workspace::git_panel::diff::{DiffView, Request};
    use wormhole_core::config::SizeUnits::{Binary, Decimal};
    use wormhole_core::git::{
        self, ChangeKind, DiffLimits, DiffSpec, FileChange, FileDiff, LfsChange, LfsPointer,
    };

    fn view(kind: ChangeKind, diff: FileDiff) -> DiffView {
        let file = FileChange {
            path: "assets/icon.png".into(),
            original_path: None,
            kind,
            added: None,
            removed: None,
            lfs: false,
            submodule: None,
        };
        let spec = DiffSpec { old: "HEAD".into(), new: None };
        let request = Request { root: PathBuf::from("/repo"), file, spec, limits: DiffLimits::default() };
        let mut view = DiffView::new(request, Rc::default(), None);
        view.sides = git::sides(&diff, kind);
        view.diff = Some(Arc::new(diff));
        view
    }

    fn note(text: &'static str, detail: Option<&str>) -> Option<Note> {
        Some(Note { text, detail: detail.map(str::to_string), oid: None })
    }

    #[test]
    fn a_binary_file_says_its_sizes() {
        let diff = FileDiff { binary: true, sizes: (Some(48_200), Some(52_900)), ..FileDiff::default() };
        let modified = view(ChangeKind::Modified, diff);
        // Split: each column its own side; unified: before → after.
        assert_eq!(
            notes(&modified, true, true, Decimal),
            vec![note("Binary file", Some("48.2 KB")), note("Binary file", Some("52.9 KB"))]
        );
        assert_eq!(
            notes(&modified, false, true, Decimal),
            vec![note("Binary file", Some("48.2 KB → 52.9 KB"))]
        );
        // In the units the setting picks.
        assert_eq!(
            notes(&modified, false, true, Binary),
            vec![note("Binary file", Some("47.1 KiB → 51.7 KiB"))]
        );

        let added = FileDiff { binary: true, new_file: true, sizes: (None, Some(10)), ..FileDiff::default() };
        let added = view(ChangeKind::Added, added);
        assert_eq!(
            notes(&added, true, true, Decimal),
            vec![note("New file", None), note("Binary file", Some("10 bytes"))]
        );
        assert_eq!(notes(&added, false, true, Decimal), vec![note("Binary file", Some("10 bytes"))]);
    }

    #[test]
    fn a_file_stored_with_lfs_says_what_it_points_at_and_keeps_the_object_for_copying() {
        let pointer = |oid: &str, size| Some(LfsPointer { oid: oid.repeat(32), size });
        let change = LfsChange { old: pointer("1a", 12_300_000), new: pointer("4d", 12_800_000) };
        let lfs = view(ChangeKind::Modified, FileDiff { lfs: Some(change.clone()), ..FileDiff::default() });
        let unified = notes(&lfs, false, true, Decimal);
        assert_eq!(
            unified,
            vec![Some(Note {
                text: "Stored in Git LFS",
                detail: Some("12.3 MB → 12.8 MB · oid 4d4d4d…".into()),
                // The id the detail cuts short: the new object's.
                oid: Some("4d".repeat(32)),
            })]
        );
        let split = notes(&lfs, true, true, Decimal);
        let old = change.old.as_ref().map(|p| git::lfs_side(p, Decimal));
        assert_eq!(split[0].as_ref().map(|n| n.detail.clone()), Some(old));
        assert_eq!(split[0].as_ref().and_then(|n| n.oid.clone()), Some("1a".repeat(32)));
        assert_eq!(split[1].as_ref().and_then(|n| n.oid.clone()), Some("4d".repeat(32)));
        // Deleted: the last object it was.
        let deleted = LfsChange { old: pointer("1a", 12_300_000), new: None };
        let deleted = view(ChangeKind::Deleted, FileDiff { lfs: Some(deleted), ..FileDiff::default() });
        assert_eq!(
            notes(&deleted, false, true, Decimal)[0].as_ref().and_then(|n| n.oid.clone()),
            Some("1a".repeat(32))
        );
    }

    #[test]
    fn a_file_on_disk_can_be_revealed_whatever_the_diff_compares() {
        // Nothing read yet: nothing known to be on disk.
        let modified = view(ChangeKind::Modified, FileDiff::default());
        assert_eq!(modified.on_disk(), None);
        let on_disk = |kind, spec: Option<DiffSpec>| {
            let mut view = view(kind, FileDiff::default());
            view.on_disk = true;
            if let Some(spec) = spec {
                view.request.spec = spec;
            }
            view
        };
        let working = on_disk(ChangeKind::Modified, None);
        assert_eq!(working.on_disk(), Some(PathBuf::from("/repo/assets/icon.png")));
        assert_eq!(working.reveal_label(), "Reveal in Finder");
        assert_eq!(on_disk(ChangeKind::Untracked, None).reveal_label(), "Reveal in Finder");
        // A commit's file that is still in the working tree: Finder shows today's version.
        let commit =
            on_disk(ChangeKind::Modified, Some(DiffSpec { old: "abc^".into(), new: Some("abc".into()) }));
        assert_eq!(commit.on_disk(), Some(PathBuf::from("/repo/assets/icon.png")));
        assert_eq!(commit.reveal_label(), "Reveal Current File in Finder");
        // Compared with a branch: the working tree, but against a branch point.
        let compared = on_disk(ChangeKind::Modified, Some(DiffSpec { old: "0123abcd".into(), new: None }));
        assert_eq!(compared.reveal_label(), "Reveal Current File in Finder");
        // An empty repository compares with git's empty tree: still the plain working tree.
        let first = on_disk(ChangeKind::Added, Some(DiffSpec { old: git::EMPTY_TREE.into(), new: None }));
        assert_eq!(first.reveal_label(), "Reveal in Finder");
    }

    #[test]
    fn a_column_with_lines_has_no_note_and_an_empty_one_says_why() {
        let text = view(ChangeKind::Modified, FileDiff::default());
        assert_eq!(notes(&text, true, false, Decimal), vec![None, None]);
        assert!(notes(&text, false, false, Decimal).is_empty());
        // A change of mode alone, an empty new file.
        assert_eq!(notes(&text, true, true, Decimal), vec![note("No content changes", None); 2]);
        let empty = view(ChangeKind::Added, FileDiff { new_file: true, ..FileDiff::default() });
        assert_eq!(
            notes(&empty, true, true, Decimal),
            vec![note("New file", None), note("Empty file", None)]
        );
        let deleted = view(ChangeKind::Deleted, FileDiff { deleted_file: true, ..FileDiff::default() });
        assert_eq!(notes(&deleted, false, true, Decimal), vec![note("Empty file", None)]);
    }
}
