//! The Changes list's rows: the sections (all changes, or tracked and untracked apart), the
//! folder tree with its compacted chains, and the order the diff steps through.

use std::collections::{BTreeMap, HashMap, HashSet};

use wormhole_core::git::{ChangeKind, FileChange, RepoStatus};
use wormhole_core::panel as rules;

use crate::style;
use crate::workspace::panel_list::CardPart;

/// The block of the list a folder row sits in. Not an index: with only
/// untracked files "Untracked" is the first block, and its folders keep their
/// identity.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) enum Group {
    /// Grouping is off: one block.
    All,
    Changes,
    Untracked,
}

impl Group {
    pub(super) fn tag(self) -> &'static str {
        match self {
            Group::All => "all",
            Group::Changes => "changes",
            Group::Untracked => "untracked",
        }
    }

    /// The band's title. Without grouping every file is a change.
    pub(super) fn title(self) -> &'static str {
        match self {
            Group::All | Group::Changes => "Changes",
            Group::Untracked => "Untracked",
        }
    }
}

/// The key of a folder row in the window's `collapsed_dirs`. That set is shared
/// by every session, and a folder can sit under both "Changes" and "Untracked", so
/// the key names the session and the block as well as the folder.
pub(super) fn collapse_key(session: i64, group: Group, path: &str) -> String {
    format!("{session}\u{1f}{}\u{1f}{path}", group.tag())
}

/// The folded folders, as one session sees them.
#[derive(Clone, Copy)]
pub(super) struct Folded<'a> {
    pub(super) set: &'a HashSet<String>,
    pub(super) session: i64,
}

impl Folded<'_> {
    fn contains(&self, group: Group, path: &str) -> bool {
        self.set.contains(&collapse_key(self.session, group, path))
    }
}

/// One line of the Changes list. Describes what to draw; the data comes from
/// the file snapshot the rows were built from.
#[derive(Clone, PartialEq)]
pub(super) enum ChangeRow {
    /// A section's card head. `first`: the list's top card, whose head carries the controls.
    Band {
        group: Group,
        count: usize,
        folded: bool,
        first: bool,
    },
    /// Under a section folded for its size: where its files are and why it is folded.
    FoldNote(String),
    Dir {
        group: Group,
        path: String,
        label: String,
        depth: usize,
        collapsed: bool,
    },
    /// `ix` indexes the snapshot; `in_tree` shows just the file name, indented.
    File {
        ix: usize,
        depth: usize,
        in_tree: bool,
    },
    /// Bottom padding, so the last row can scroll clear of the panel edge.
    Spacer,
}

impl ChangeRow {
    /// Each section is a card; the spacer under the last is outside them.
    pub(super) fn part(&self) -> CardPart {
        match self {
            Self::Band { .. } => CardPart::Head,
            Self::Spacer => CardPart::Outside,
            _ => CardPart::Body,
        }
    }
}

/// Whether `status`'s list has a band for the untracked files, given the "group untracked"
/// setting: not when compared with a branch, where the list is one group.
pub(in crate::workspace) fn untracked_apart(status: &RepoStatus, setting: bool) -> bool {
    rules::untracked_apart(setting, status.compare.is_some())
}

/// Flatten the (optionally grouped, optionally folder-nested) file list into rows: a band per
/// section (one, "Changes", without grouping), folded as the user left it or by its size.
pub(super) fn build_rows(
    files: &[FileChange],
    tree: bool,
    group_untracked: bool,
    folded: Folded,
    folds: &HashMap<Group, bool>,
) -> Vec<ChangeRow> {
    let all: Vec<usize> = (0..files.len()).collect();
    let groups: Vec<(Group, Vec<usize>)> = if group_untracked {
        let (untracked, tracked): (Vec<usize>, Vec<usize>) =
            all.into_iter().partition(|&ix| files[ix].kind == ChangeKind::Untracked);
        [(Group::Changes, tracked), (Group::Untracked, untracked)]
            .into_iter()
            .filter(|(_, ids)| !ids.is_empty())
            .collect()
    } else {
        vec![(Group::All, all)]
    };

    let mut rows = Vec::with_capacity(files.len() + 8);
    for (group, ids) in groups {
        let section_folded = rules::section_folded(ids.len(), folds.get(&group).copied());
        let first = rows.is_empty();
        rows.push(ChangeRow::Band { group, count: ids.len(), folded: section_folded, first });
        if section_folded {
            if ids.len() > style::FOLD_ABOVE {
                let paths = ids.iter().map(|&ix| files[ix].path.as_str());
                rows.push(ChangeRow::FoldNote(rules::folded_note(paths, group == Group::Untracked)));
            }
            continue;
        }
        if tree {
            push_tree(&build_tree(files, &ids), "", 0, group, folded, &mut rows);
        } else {
            rows.extend(ids.into_iter().map(|ix| ChangeRow::File { ix, depth: 0, in_tree: false }));
        }
    }
    rows.push(ChangeRow::Spacer);
    rows
}

/// The files in the order the list shows them (blocks, then folders before
/// files in tree mode, folded folders included): what the diff's ‹ › step
/// through, so the next file is the one below in the list.
pub(in crate::workspace) fn display_order(
    status: &RepoStatus,
    tree: bool,
    group_untracked: bool,
) -> Vec<FileChange> {
    let (files, group_untracked) = (&status.files[..], untracked_apart(status, group_untracked));
    let none = HashSet::new();
    // Folded sections and folders too: stepping goes through every file.
    let open = HashMap::from([(Group::All, false), (Group::Changes, false), (Group::Untracked, false)]);
    build_rows(files, tree, group_untracked, Folded { set: &none, session: 0 }, &open)
        .into_iter()
        .filter_map(|row| match row {
            ChangeRow::File { ix, .. } => files.get(ix).cloned(),
            _ => None,
        })
        .collect()
}

fn push_tree(
    node: &DirNode,
    prefix: &str,
    depth: usize,
    group: Group,
    folded: Folded,
    rows: &mut Vec<ChangeRow>,
) {
    for (name, child) in &node.dirs {
        // Compact chains of single folders into "a/b/c", like most editors.
        let (mut label, mut child) = (name.clone(), child);
        while child.files.is_empty() && child.dirs.len() == 1 {
            let Some((next_name, next)) = child.dirs.iter().next() else { break };
            label = format!("{label}/{next_name}");
            child = next;
        }
        let path = if prefix.is_empty() { label.clone() } else { format!("{prefix}/{label}") };
        let is_collapsed = folded.contains(group, &path);
        rows.push(ChangeRow::Dir { group, path: path.clone(), label, depth, collapsed: is_collapsed });
        if !is_collapsed {
            push_tree(child, &path, depth + 1, group, folded, rows);
        }
    }
    rows.extend(node.files.iter().map(|&ix| ChangeRow::File { ix, depth, in_tree: true }));
}

// ---------------------------------------------------------------- tree

#[derive(Default)]
pub(super) struct DirNode {
    pub(super) dirs: BTreeMap<String, DirNode>,
    /// Indices into the file list.
    pub(super) files: Vec<usize>,
}

/// The folder tree of `files[ix]` for each `ix` in `ids`.
pub(super) fn build_tree(files: &[FileChange], ids: &[usize]) -> DirNode {
    let mut root = DirNode::default();
    for &ix in ids {
        let mut node = &mut root;
        let mut parts: Vec<&str> = files[ix].path.split('/').collect();
        parts.pop();
        for part in parts {
            node = node.dirs.entry(part.to_string()).or_default();
        }
        node.files.push(ix);
    }
    root
}

pub(super) fn file_name(path: &str) -> &str {
    path.rsplit_once('/').map_or(path, |(_, name)| name)
}
