use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

// Not `super::*`: that would pull in GPUI's `test` macro and shadow `#[test]`.
use super::rows::{
    ChangeRow, Folded, Group, build_rows, build_tree, collapse_key, display_order, untracked_apart,
};
use wormhole_core::git::{ChangeKind, FileChange, RepoStatus};

fn change(path: &str, kind: ChangeKind) -> FileChange {
    FileChange {
        path: path.to_string(),
        original_path: None,
        kind,
        added: None,
        removed: None,
        lfs: false,
        submodule: None,
    }
}

fn modified(paths: &[&str]) -> Vec<FileChange> {
    paths.iter().map(|p| change(p, ChangeKind::Modified)).collect()
}

fn status(files: Vec<FileChange>, compare: Option<&str>) -> RepoStatus {
    RepoStatus {
        root: PathBuf::from("/repo"),
        branch: Some("main".into()),
        files,
        base_rev: "HEAD".into(),
        compare: compare.map(String::from),
        compare_missing: None,
        dirty: false,
        upstream: None,
        superproject: None,
    }
}

#[test]
fn tree_groups_by_folder() {
    let files = modified(&["src/a.rs", "src/ui/b.rs", "README.md"]);
    let ids: Vec<usize> = (0..files.len()).collect();
    let tree = build_tree(&files, &ids);
    assert_eq!(tree.files, [2]);
    let src = &tree.dirs["src"];
    assert_eq!(src.files, [0]);
    assert_eq!(src.dirs["ui"].files, [1]);
}
#[test]
fn folders_fold_per_session_and_per_block() {
    // `src` holds a tracked and an untracked file, so it is listed in both blocks.
    let files = vec![change("src/a.rs", ChangeKind::Modified), change("src/b.rs", ChangeKind::Untracked)];
    let mut set = HashSet::new();
    set.insert(collapse_key(1, Group::Changes, "src"));

    let rows = |session| build_rows(&files, true, true, Folded { set: &set, session }, &HashMap::new());
    let dirs = |rows: Vec<ChangeRow>| -> Vec<(Group, bool)> {
        rows.into_iter()
            .filter_map(|row| match row {
                ChangeRow::Dir { group, collapsed, .. } => Some((group, collapsed)),
                _ => None,
            })
            .collect()
    };
    // In session 1 only the "Changes" copy of `src` is folded...
    assert_eq!(dirs(rows(1)), [(Group::Changes, true), (Group::Untracked, false)]);
    // ...and its file is gone from the list, the untracked one is not.
    let listed = |rows: Vec<ChangeRow>| rows.iter().filter(|r| matches!(r, ChangeRow::File { .. })).count();
    assert_eq!(listed(rows(1)), 1);
    // Session 2 never folded anything.
    assert_eq!(dirs(rows(2)), [(Group::Changes, false), (Group::Untracked, false)]);
    assert_eq!(listed(rows(2)), 2);
}

#[test]
fn a_block_keeps_its_folders_when_the_other_block_is_empty() {
    // Only untracked files: "Untracked" is the first block, but not group 0 of a key.
    let files = vec![change("src/b.rs", ChangeKind::Untracked)];
    let mut set = HashSet::new();
    set.insert(collapse_key(1, Group::Untracked, "src"));
    let rows = build_rows(&files, true, true, Folded { set: &set, session: 1 }, &HashMap::new());
    assert!(rows.iter().any(|r| matches!(r, ChangeRow::Dir { collapsed: true, .. })));
}

#[test]
fn the_diff_steps_through_files_in_the_order_the_list_shows_them() {
    let files = vec![
        change("README.md", ChangeKind::Modified),
        change("src/b.rs", ChangeKind::Modified),
        change("scratch.txt", ChangeKind::Untracked),
        change("docs/a.md", ChangeKind::Modified),
    ];
    let against_head = status(files, None);
    let order = |tree, group| -> Vec<String> {
        display_order(&against_head, tree, group).into_iter().map(|f| f.path).collect()
    };
    // A plain list is the status order.
    assert_eq!(order(false, false), ["README.md", "src/b.rs", "scratch.txt", "docs/a.md"]);
    // Grouped: the tracked block, then the untracked one.
    assert_eq!(order(false, true), ["README.md", "src/b.rs", "docs/a.md", "scratch.txt"]);
    // A tree lists folders before the files beside them, each sorted by name.
    assert_eq!(order(true, false), ["docs/a.md", "src/b.rs", "README.md", "scratch.txt"]);
    assert_eq!(order(true, true), ["docs/a.md", "src/b.rs", "README.md", "scratch.txt"]);
}

/// The bands of the list built for `status` with the "group untracked" setting as given.
fn bands(status: &RepoStatus, tree: bool, setting: bool) -> Vec<(Group, usize)> {
    let none = HashSet::new();
    let grouped = untracked_apart(status, setting);
    build_rows(&status.files, tree, grouped, Folded { set: &none, session: 1 }, &HashMap::new())
        .into_iter()
        .filter_map(|row| match row {
            ChangeRow::Band { group, count, .. } => Some((group, count)),
            _ => None,
        })
        .collect()
}

#[test]
fn against_head_untracked_files_get_a_band_when_the_setting_says_so() {
    let files = vec![change("a.rs", ChangeKind::Modified), change("scratch.txt", ChangeKind::Untracked)];
    let head = status(files, None);
    assert_eq!(bands(&head, false, true), [(Group::Changes, 1), (Group::Untracked, 1)]);
    assert_eq!(bands(&head, false, false), [(Group::All, 2)]);
}

/// Compared with a branch the list is one group, untracked files in it, whatever the setting.
#[test]
fn compared_with_a_branch_there_is_one_group_with_the_untracked_files_in_it() {
    let files = vec![
        change("a.rs", ChangeKind::Modified),
        change("b.rs", ChangeKind::Added),
        change("scratch.txt", ChangeKind::Untracked),
    ];
    let compared = status(files, Some("main"));
    for setting in [true, false] {
        for tree in [false, true] {
            assert_eq!(bands(&compared, tree, setting), [(Group::All, 3)], "setting {setting}, tree {tree}");
        }
    }
    // The diff steps through that one list, in its order.
    let order: Vec<String> = display_order(&compared, false, true).into_iter().map(|f| f.path).collect();
    assert_eq!(order, ["a.rs", "b.rs", "scratch.txt"]);
    // A compare branch that is gone falls back to HEAD (`compare` is cleared): bands again.
    let fell_back = RepoStatus { compare: None, compare_missing: Some("main".into()), ..compared };
    assert_eq!(bands(&fell_back, false, true), [(Group::Changes, 2), (Group::Untracked, 1)]);
}

#[test]
fn a_submodule_is_a_row_like_any_file_and_the_diff_steps_through_it() {
    use wormhole_core::git::Submodule;
    let mut lib = change("vendor/lib", ChangeKind::Modified);
    lib.submodule = Some(Submodule {
        old: Some("a".repeat(40)),
        new: None,
        commit_changed: true,
        removed: false,
        tracked: false,
        untracked: false,
        log: None,
    });
    let files = vec![change("src/a.rs", ChangeKind::Modified), lib.clone()];
    let set = HashSet::new();
    let rows = build_rows(&files, false, false, Folded { set: &set, session: 1 }, &HashMap::new());
    let listed: Vec<usize> = rows
        .iter()
        .filter_map(|row| match row {
            ChangeRow::File { ix, .. } => Some(*ix),
            _ => None,
        })
        .collect();
    assert_eq!(listed, [0, 1], "the submodule has a row of its own");
    let order = display_order(&status(files, None), false, false);
    assert_eq!(order.last().and_then(|f| f.submodule.as_ref()), lib.submodule.as_ref());
    assert_eq!(order.last().map(|f| f.path.as_str()), Some("vendor/lib"));
}
