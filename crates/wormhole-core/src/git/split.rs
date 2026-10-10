//! The side-by-side layout of a file diff: old lines on the left, new lines on
//! the right, one row per screen line, so both columns always line up.

use super::{ChangeKind, DiffLine, FileDiff, Hunk, LineKind};

/// One side of a side-by-side row.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Cell<'a> {
    Line(&'a DiffLine),
    /// Nothing on this side: the other side removed or added more lines here.
    /// Drawn hatched, so it is clear the gap is intentional.
    Filler,
}

impl<'a> Cell<'a> {
    pub fn line(self) -> Option<&'a DiffLine> {
        match self {
            Cell::Line(line) => Some(line),
            Cell::Filler => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SplitRow<'a> {
    /// A hunk's `@@ -a,b +c,d @@ context` line, above its rows.
    Hunk(&'a Hunk),
    /// `(old, new)`. A context line sits on both sides; a changed line is
    /// paired with the change across from it, or with a [`Cell::Filler`].
    Pair(Cell<'a>, Cell<'a>),
}

/// Rows for the whole file: each hunk's header, then its lines. A run of
/// removed lines and the added lines that follow it are paired line by line
/// (a replacement reads across), and the shorter run is padded with fillers.
pub fn split_rows(diff: &FileDiff) -> Vec<SplitRow<'_>> {
    let mut rows = Vec::new();
    for hunk in &diff.hunks {
        rows.push(SplitRow::Hunk(hunk));
        let mut removed: Vec<&DiffLine> = Vec::new();
        let mut added: Vec<&DiffLine> = Vec::new();
        for line in &hunk.lines {
            match line.kind {
                LineKind::Removed => {
                    // A removal after additions starts a new change block.
                    if !added.is_empty() {
                        flush(&mut rows, &mut removed, &mut added);
                    }
                    removed.push(line);
                }
                LineKind::Added => added.push(line),
                LineKind::Context => {
                    flush(&mut rows, &mut removed, &mut added);
                    rows.push(SplitRow::Pair(Cell::Line(line), Cell::Line(line)));
                }
            }
        }
        flush(&mut rows, &mut removed, &mut added);
    }
    rows
}

fn flush<'a>(rows: &mut Vec<SplitRow<'a>>, removed: &mut Vec<&'a DiffLine>, added: &mut Vec<&'a DiffLine>) {
    let cell = |line: Option<&&'a DiffLine>| line.map_or(Cell::Filler, |l| Cell::Line(l));
    for i in 0..removed.len().max(added.len()) {
        rows.push(SplitRow::Pair(cell(removed.get(i)), cell(added.get(i))));
    }
    removed.clear();
    added.clear();
}

/// What one side of the comparison holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Text,
    /// The file doesn't exist on this side: it is new (old side) or deleted (new side).
    Missing,
    Binary,
    /// A Git LFS pointer: the file is stored elsewhere ([`FileDiff::lfs`]).
    Lfs,
}

/// The old and new side of a file, from the patch header and the change kind
/// (the Changes list knows an untracked file is new even before git says so).
pub fn sides(diff: &FileDiff, kind: ChangeKind) -> (Side, Side) {
    let text = if diff.binary {
        Side::Binary
    } else if diff.lfs.is_some() {
        Side::Lfs
    } else {
        Side::Text
    };
    let old_missing = diff.new_file || matches!(kind, ChangeKind::Added | ChangeKind::Untracked);
    let new_missing = diff.deleted_file || kind == ChangeKind::Deleted;
    (if old_missing { Side::Missing } else { text }, if new_missing { Side::Missing } else { text })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::parse_diff;

    /// Rows as text: hunk headers as "@@", fillers as "·".
    fn layout(patch: &str) -> Vec<(String, String)> {
        let diff = parse_diff(patch);
        let side = |cell: Cell| cell.line().map_or("·".to_string(), |l| l.text.clone());
        split_rows(&diff)
            .into_iter()
            .map(|row| match row {
                SplitRow::Hunk(_) => ("@@".into(), "@@".into()),
                SplitRow::Pair(old, new) => (side(old), side(new)),
            })
            .collect()
    }

    fn rows(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
    }

    #[test]
    fn pure_addition_pads_the_old_side() {
        let got = layout("@@ -1,2 +1,4 @@\n a\n+b\n+c\n d\n");
        assert_eq!(got, rows(&[("@@", "@@"), ("a", "a"), ("·", "b"), ("·", "c"), ("d", "d")]));
    }

    #[test]
    fn pure_deletion_pads_the_new_side() {
        let got = layout("@@ -1,3 +1,1 @@\n a\n-b\n-c\n");
        assert_eq!(got, rows(&[("@@", "@@"), ("a", "a"), ("b", "·"), ("c", "·")]));
    }

    #[test]
    fn replacement_pairs_lines_and_pads_the_shorter_side() {
        // The reference case: 2 removed, 4 added, then context.
        let got = layout("@@ -41,4 +41,6 @@ impl OrdersApi\n x\n-r1\n-r2\n+a1\n+a2\n+a3\n+a4\n y\n");
        assert_eq!(
            got,
            rows(&[
                ("@@", "@@"),
                ("x", "x"),
                ("r1", "a1"),
                ("r2", "a2"),
                ("·", "a3"),
                ("·", "a4"),
                ("y", "y"),
            ])
        );
        let longer_old = layout("@@ -1,3 +1,1 @@\n-r1\n-r2\n-r3\n+a1\n");
        assert_eq!(longer_old, rows(&[("@@", "@@"), ("r1", "a1"), ("r2", "·"), ("r3", "·")]));
    }

    #[test]
    fn removal_after_addition_starts_a_new_block() {
        let got = layout("@@ -1,2 +1,2 @@\n+a\n-r\n");
        assert_eq!(got, rows(&[("@@", "@@"), ("·", "a"), ("r", "·")]));
    }

    #[test]
    fn context_keeps_both_line_numbers() {
        let diff = parse_diff("@@ -44,2 +46,2 @@\n p\n q\n");
        let numbers: Vec<_> = split_rows(&diff)
            .into_iter()
            .filter_map(|row| match row {
                SplitRow::Pair(old, new) => Some((old.line()?.old, new.line()?.new)),
                SplitRow::Hunk(_) => None,
            })
            .collect();
        assert_eq!(numbers, vec![(Some(44), Some(46)), (Some(45), Some(47))]);
    }

    #[test]
    fn multiple_hunks_each_get_a_header() {
        let got = layout("@@ -1,2 +1,2 @@\n a\n-b\n+B\n@@ -10,1 +10,2 @@\n-r\n+s\n+t\n");
        assert_eq!(got, rows(&[("@@", "@@"), ("a", "a"), ("b", "B"), ("@@", "@@"), ("r", "s"), ("·", "t")]));
    }

    #[test]
    fn new_file_is_filler_on_the_left_and_missing() {
        let patch = "diff --git a/n.rs b/n.rs\nnew file mode 100644\n--- /dev/null\n+++ b/n.rs\n@@ -0,0 +1,2 @@\n+a\n+b\n";
        assert_eq!(layout(patch), rows(&[("@@", "@@"), ("·", "a"), ("·", "b")]));
        let diff = parse_diff(patch);
        assert_eq!(sides(&diff, ChangeKind::Added), (Side::Missing, Side::Text));
        // An untracked file is new even if the header were missing.
        assert_eq!(sides(&FileDiff::default(), ChangeKind::Untracked), (Side::Missing, Side::Text));
    }

    #[test]
    fn deleted_file_is_filler_on_the_right_and_missing() {
        let patch = "diff --git a/d.rs b/d.rs\ndeleted file mode 100644\n--- a/d.rs\n+++ /dev/null\n@@ -1,2 +0,0 @@\n-a\n-b\n";
        assert_eq!(layout(patch), rows(&[("@@", "@@"), ("a", "·"), ("b", "·")]));
        assert_eq!(sides(&parse_diff(patch), ChangeKind::Deleted), (Side::Text, Side::Missing));
    }

    #[test]
    fn binary_files_have_no_rows() {
        let modified = parse_diff("diff --git a/x.png b/x.png\nBinary files a/x.png and b/x.png differ\n");
        assert!(split_rows(&modified).is_empty());
        assert_eq!(sides(&modified, ChangeKind::Modified), (Side::Binary, Side::Binary));
        let added = parse_diff(
            "diff --git a/x.png b/x.png\nnew file mode 100644\nBinary files /dev/null and b/x.png differ\n",
        );
        assert_eq!(sides(&added, ChangeKind::Added), (Side::Missing, Side::Binary));
    }

    #[test]
    fn a_file_stored_with_lfs_has_pointer_sides_and_no_rows() {
        use crate::git::{LfsChange, LfsPointer};
        let pointer = LfsPointer { oid: "ab".repeat(32), size: 7 };
        let modified = FileDiff {
            lfs: Some(LfsChange { old: Some(pointer.clone()), new: Some(pointer.clone()) }),
            ..FileDiff::default()
        };
        assert!(split_rows(&modified).is_empty());
        assert_eq!(sides(&modified, ChangeKind::Modified), (Side::Lfs, Side::Lfs));
        let added = FileDiff {
            new_file: true,
            lfs: Some(LfsChange { old: None, new: Some(pointer) }),
            ..FileDiff::default()
        };
        assert_eq!(sides(&added, ChangeKind::Added), (Side::Missing, Side::Lfs));
    }

    #[test]
    fn no_newline_at_eof_stays_on_its_line() {
        let diff = parse_diff("@@ -1 +1 @@\n-a\n\\ No newline at end of file\n+a\n");
        let rows = split_rows(&diff);
        let SplitRow::Pair(old, new) = rows[1] else { panic!("expected a pair, got {:?}", rows[1]) };
        assert!(old.line().is_some_and(|l| l.no_newline));
        assert!(new.line().is_some_and(|l| !l.no_newline));
    }
}
