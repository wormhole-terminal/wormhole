//! What Settings → Changes & diffs shows of the Changes panel: a few changed files in the panel's own
//! cards, bands, folder rows and file rows, laid out by the same `build_rows` as the list, from the
//! settings as they are now (a flat list or a folder tree, the untracked files in a section of their
//! own or not). The rows are the panel's; only what a click or a window would add is left out.

use std::collections::{HashMap, HashSet};

use gpui_kit::assets::IconName;
use gpui_kit::*;
use wormhole_core::git::{ChangeKind, FileChange};

use super::rows::{ChangeRow, Folded, build_rows, file_name};
use super::view::band_card;
use crate::features::git::file_line::preview_row;
use crate::settings::{SettingsStore, UiColors};
use crate::style::{self, Cards};

/// A changed file as `git status` would hand it over.
fn file(path: &str, kind: ChangeKind, added: Option<u32>, removed: Option<u32>) -> FileChange {
    FileChange { path: path.into(), original_path: None, kind, added, removed, lfs: false, submodule: None }
}

/// The sample's files: as few as show every setting of the list (a folder, a file at the top, a binary
/// one and an untracked one), since the preview is a quarter of the window at most.
fn files() -> Vec<FileChange> {
    use ChangeKind::{Modified, Untracked};
    // text: sample (the preview's files)
    vec![
        file("src/orders.rs", Modified, Some(12), Some(3)),
        file("logo.png", Modified, None, None),
        file("todo.md", Untracked, Some(8), None),
    ]
}

/// The panel's list of the sample files: `tree` as folders, else flat; `group_untracked` with a section
/// for the untracked files. No band folds and no folder is collapsed, as in a fresh panel.
pub(crate) fn list(tree: bool, group_untracked: bool, ui: UiColors, cx: &App) -> Div {
    let files = files();
    let none = HashSet::new();
    let rows = build_rows(&files, tree, group_untracked, Folded { set: &none, session: 0 }, &HashMap::new());
    let icons = cx.global::<SettingsStore>().icons.as_deref();
    let mut cards = Cards::new(ui);
    for row in rows {
        match row {
            ChangeRow::Band { group, count, folded, .. } => {
                cards = cards.head(band_card(group, count, folded, None, Vec::new(), ui));
            }
            ChangeRow::Dir { group, path, label, depth, collapsed } => {
                let icon = crate::icons::folder(icons, file_name(&label), !collapsed, ui.muted)
                    .unwrap_or_else(|| style::icon(IconName::Folder).into_any_element());
                cards = cards.child(style::folder_row(
                    SharedString::from(format!("preview-dir-{}-{path}", group.tag())),
                    ui,
                    depth,
                    icon,
                    label,
                    collapsed,
                ));
            }
            ChangeRow::File { ix, depth, in_tree } => {
                if let Some(file) = files.get(ix) {
                    cards = cards.child(preview_row(file, in_tree, depth, ui, cx));
                }
            }
            // Nothing is folded for its size here, and the spacer is for scrolling a long list clear.
            ChangeRow::FoldNote(_) | ChangeRow::Spacer => {}
        }
    }
    div().pt(px(style::PANEL_LIST_TOP)).child(cards)
}

#[cfg(test)]
mod tests {
    use std::collections::{HashMap, HashSet};

    use super::super::rows::Group;
    use super::{ChangeRow, Folded, build_rows, files};

    fn rows(tree: bool, group_untracked: bool) -> Vec<ChangeRow> {
        let none = HashSet::new();
        build_rows(&files(), tree, group_untracked, Folded { set: &none, session: 0 }, &HashMap::new())
    }

    fn bands(rows: &[ChangeRow]) -> Vec<Group> {
        rows.iter()
            .filter_map(|row| match row {
                ChangeRow::Band { group, .. } => Some(*group),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn untracked_files_get_a_section_of_their_own_only_when_asked() {
        assert_eq!(bands(&rows(false, true)), [Group::Changes, Group::Untracked]);
        assert_eq!(bands(&rows(false, false)), [Group::All]);
    }

    #[test]
    fn the_tree_has_folders_and_the_list_has_none() {
        let folders = |rows: &[ChangeRow]| rows.iter().filter(|r| matches!(r, ChangeRow::Dir { .. })).count();
        assert_eq!(folders(&rows(false, false)), 0);
        assert_eq!(folders(&rows(true, false)), 1, "src");
    }
}
