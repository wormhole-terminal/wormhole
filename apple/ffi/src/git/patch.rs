//! A parsed diff written back as unified text. Core parses once (`git::diff`, with its caps, binary, LFS and
//! too-large rules) and the Swift diff view parses unified text (`UnifiedDiffParser`), so the seam carries the
//! text: the hunks are core's own lines, the header is rebuilt from what core kept of it. Modes are written
//! as the usual ones: the view reads only whether a file is new, deleted or renamed.

use std::fmt::Write as _;

use wormhole_core::git::{FileChange, FileDiff, LineKind};

/// The text of `diff` for `file`, in the form `git diff` prints it.
pub(crate) fn unified(file: &FileChange, diff: &FileDiff) -> String {
    let old = diff.renamed_from.as_deref().or(file.original_path.as_deref()).unwrap_or(&file.path);
    let new = &file.path;
    let mut out = format!("diff --git a/{old} b/{new}\n");
    if let Some((from, to)) = &diff.mode_change {
        let _ = writeln!(out, "old mode {from}\nnew mode {to}");
    }
    if diff.new_file {
        out.push_str("new file mode 100644\n");
    } else if diff.deleted_file {
        out.push_str("deleted file mode 100644\n");
    } else if old != new {
        let _ = writeln!(out, "rename from {old}\nrename to {new}");
    }
    if diff.binary {
        let _ = writeln!(
            out,
            "Binary files {} and {} differ",
            side("a", old, diff.new_file),
            side("b", new, diff.deleted_file)
        );
        return out;
    }
    if !diff.hunks.is_empty() {
        let _ = writeln!(out, "--- {}", if diff.new_file { "/dev/null".into() } else { format!("a/{old}") });
        let _ =
            writeln!(out, "+++ {}", if diff.deleted_file { "/dev/null".into() } else { format!("b/{new}") });
    }
    for hunk in &diff.hunks {
        out.push_str(&hunk.header);
        out.push('\n');
        for line in &hunk.lines {
            out.push(match line.kind {
                LineKind::Context => ' ',
                LineKind::Added => '+',
                LineKind::Removed => '-',
            });
            out.push_str(&line.text);
            out.push('\n');
            if line.no_newline {
                out.push_str("\\ No newline at end of file\n");
            }
        }
    }
    out
}

/// `a/x`, or `/dev/null` for the side a file doesn't have.
fn side(prefix: &str, path: &str, missing: bool) -> String {
    if missing { "/dev/null".into() } else { format!("{prefix}/{path}") }
}

#[cfg(test)]
mod tests {
    use wormhole_core::git::{ChangeKind, DiffLine, Hunk};

    use super::*;

    fn line(kind: LineKind, text: &str, no_newline: bool) -> DiffLine {
        DiffLine { kind, old: None, new: None, text: text.into(), no_newline }
    }

    #[test]
    fn hunks_come_back_line_for_line_with_the_no_newline_marker() {
        let file = FileChange::new("src/a.rs", ChangeKind::Modified);
        let diff = FileDiff {
            hunks: vec![Hunk {
                header: "@@ -1,2 +1,2 @@ fn main".into(),
                lines: vec![
                    line(LineKind::Context, "keep", false),
                    line(LineKind::Removed, "-- not a header", false),
                    line(LineKind::Added, "new", true),
                ],
            }],
            ..FileDiff::default()
        };
        assert_eq!(
            unified(&file, &diff),
            "diff --git a/src/a.rs b/src/a.rs\n--- a/src/a.rs\n+++ b/src/a.rs\n@@ -1,2 +1,2 @@ fn main\n keep\n--- not a header\n+new\n\\ No newline at end of file\n"
        );
    }

    #[test]
    fn renames_new_files_and_binaries_say_so_in_the_header() {
        let mut renamed = FileChange::new("b.rs", ChangeKind::Renamed);
        renamed.original_path = Some("a.rs".into());
        let text = unified(&renamed, &FileDiff { renamed_from: Some("a.rs".into()), ..FileDiff::default() });
        assert_eq!(text, "diff --git a/a.rs b/b.rs\nrename from a.rs\nrename to b.rs\n");

        let added = FileChange::new("logo.png", ChangeKind::Added);
        let binary = FileDiff { binary: true, new_file: true, ..FileDiff::default() };
        assert_eq!(
            unified(&added, &binary),
            "diff --git a/logo.png b/logo.png\nnew file mode 100644\nBinary files /dev/null and b/logo.png differ\n"
        );
    }
}
