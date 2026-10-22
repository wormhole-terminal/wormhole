//! Unified diffs of a single file, parsed into hunks and lines.

use std::path::Path;

use anyhow::Result;

use super::lfs;
use super::{
    ChangeKind, DiffLine, DiffSpec, EMPTY_TREE, FileChange, FileDiff, GitCmd, GitError, Hunk, LineKind,
    MAX_TEXT_BYTES, Side, sides,
};
use crate::storage::fsutil::{self, LinkPolicy};

/// For each line of a hunk, the line of the file as it is now that an editor should open for it:
/// its own number on the new side; for a removed line, which has none there, the line that
/// follows it (where its text used to be), or the last one before it when the removal ends the
/// hunk. `None` for a hunk that leaves no line at all.
pub fn lines_in_new_file(lines: &[DiffLine]) -> Vec<Option<u32>> {
    let mut found = vec![None; lines.len()];
    let mut next = None;
    for (ix, line) in lines.iter().enumerate().rev() {
        next = line.new.or(next);
        found[ix] = next;
    }
    let mut before = None;
    for (ix, line) in lines.iter().enumerate() {
        before = line.new.or(before);
        found[ix] = found[ix].or(before);
    }
    found
}

/// A patch bigger than this is not loaded until the user asks for it.
pub const DIFF_ASK_BYTES: u64 = 8 << 20;
/// A patch with more lines than this loads without syntax highlighting.
pub const DIFF_HIGHLIGHT_MAX_LINES: usize = 20_000;
/// A patch too large to load unasked is still read to its end (and dropped) to say how big it
/// is, up to this size; git is stopped past it.
pub const DIFF_MEASURE_BYTES: u64 = 100_000_000;

/// How much of one file's diff is worth building.
///
/// Everything below is bounded by what the Changes panel can show and highlight on its UI
/// thread. Without it, a 60 MB generated file became 2 000 000 `DiffLine`s and two 60 MB strings
/// in 1.4 s, and the view then laid all of it out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DiffLimits {
    /// Bytes of patch text git may print. More and git is stopped (so even a huge change costs
    /// about this much, not its size) and the diff comes back [`FileDiff::too_large`].
    pub max_patch_bytes: u64,
    /// Lines in the patch beyond which the whole files are not read for highlighting: parsing
    /// them and coloring every row is what makes a long diff slow, not its lines.
    pub max_highlight_lines: usize,
    /// Largest whole file kept as [`FileDiff::old_text`]/[`FileDiff::new_text`], the source of
    /// syntax highlighting. A larger file only loses its highlighting, not its diff.
    pub max_text_bytes: u64,
}

impl DiffLimits {
    /// No limit at all.
    pub const UNLIMITED: Self =
        Self { max_patch_bytes: u64::MAX, max_highlight_lines: usize::MAX, max_text_bytes: u64::MAX };

    /// After the user asked to load a diff [`FileDiff::too_large`] anyway: any size, but a long
    /// one still without highlighting.
    pub const LOAD_ANYWAY: Self = Self { max_patch_bytes: u64::MAX, ..Self::DEFAULT };

    const DEFAULT: Self = Self {
        max_patch_bytes: DIFF_ASK_BYTES,
        max_highlight_lines: DIFF_HIGHLIGHT_MAX_LINES,
        max_text_bytes: MAX_TEXT_BYTES,
    };
}

impl Default for DiffLimits {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Diff of one file between two sides (see [`DiffSpec`]), within [`DiffLimits::default`]:
/// a bigger one comes back with [`FileDiff::too_large`] set and nothing else.
pub fn diff(root: &Path, file: &FileChange, spec: &DiffSpec) -> Result<FileDiff> {
    diff_with_limits(root, file, spec, &DiffLimits::default())
}

/// [`diff`] with other limits ([`DiffLimits::LOAD_ANYWAY`] after the user asked to see it anyway).
pub fn diff_with_limits(
    root: &Path,
    file: &FileChange,
    spec: &DiffSpec,
    limits: &DiffLimits,
) -> Result<FileDiff> {
    let patch = if file.kind == ChangeKind::Untracked {
        untracked_patch(root, &file.path, limits)
    } else {
        let mut cmd = GitCmd::diff(root);
        cmd.args(["--no-color", "-M"]);
        cmd.revs(std::iter::once(&spec.old).chain(&spec.new));
        cmd.paths(file.original_path.iter().chain(std::iter::once(&file.path)));
        capped(&mut cmd, limits, |cmd| cmd.run())
    };
    let raw = match patch? {
        Patch::Text(raw) => raw,
        Patch::TooLarge(patch_bytes) => {
            return Ok(FileDiff { too_large: true, patch_bytes, ..FileDiff::default() });
        }
    };
    let long = raw.iter().filter(|&&b| b == b'\n').count() > limits.max_highlight_lines;
    let mut diff = parse_diff(&String::from_utf8_lossy(&raw));
    drop(raw);

    if let Some(change) = lfs::pointer_change(&diff) {
        // Stored with Git LFS: what the pointers stand for is the diff, not their lines.
        diff.lfs = Some(change);
        diff.hunks.clear();
    } else if diff.binary {
        diff.sizes = binary_sizes(root, file, spec, &diff);
    } else if long {
        diff.highlight_skipped = true;
    } else {
        let show = |rev: &str, path: &str| {
            let mut cmd = GitCmd::new(root);
            cmd.arg("show").revs([format!("{rev}:{path}")]);
            // A side that is too big has no text (and no highlighting), same as a worktree file.
            capped_text(&mut cmd, limits.max_text_bytes)
        };
        if file.kind != ChangeKind::Deleted {
            diff.new_text = match &spec.new {
                None => read_text(&root.join(&file.path), limits.max_text_bytes),
                Some(rev) => show(rev, &file.path),
            };
        }
        if !matches!(file.kind, ChangeKind::Added | ChangeKind::Untracked) && spec.old != EMPTY_TREE {
            let old_path = file.original_path.as_deref().unwrap_or(&file.path);
            diff.old_text = show(&spec.old, old_path);
        }
    }
    Ok(diff)
}

/// What git printed for a patch.
enum Patch {
    Text(Vec<u8>),
    /// More than the limit: none of it, only its size (`None` past [`DIFF_MEASURE_BYTES`]).
    TooLarge(Option<u64>),
}

/// Run a command whose output is bounded by `limits.max_patch_bytes`: past that its output is
/// only measured.
fn capped(
    cmd: &mut GitCmd,
    limits: &DiffLimits,
    run: impl FnOnce(&mut GitCmd) -> Result<Vec<u8>>,
) -> Result<Patch> {
    if limits.max_patch_bytes != u64::MAX {
        cmd.max_output(limits.max_patch_bytes).measure_up_to(DIFF_MEASURE_BYTES);
    }
    match run(cmd) {
        Ok(bytes) => Ok(Patch::Text(bytes)),
        Err(err) => match err.downcast_ref::<GitError>() {
            Some(GitError::OutputTooLarge { printed, .. }) => Ok(Patch::TooLarge(*printed)),
            _ => Err(err),
        },
    }
}

/// A binary file's size on each side it has: what git stores (`cat-file -s` reads the object's
/// header, not its content), and the working tree's file as it is on disk.
fn binary_sizes(
    root: &Path,
    file: &FileChange,
    spec: &DiffSpec,
    diff: &FileDiff,
) -> (Option<u64>, Option<u64>) {
    let blob = |rev: &str, path: &str| {
        let mut cmd = GitCmd::new(root);
        cmd.args(["cat-file", "-s"]).revs([format!("{rev}:{path}")]);
        cmd.run().ok().and_then(|out| String::from_utf8_lossy(&out).trim().parse().ok())
    };
    let (old_side, new_side) = sides(diff, file.kind);
    let new = match &spec.new {
        _ if new_side == Side::Missing => None,
        None => {
            std::fs::symlink_metadata(root.join(&file.path)).ok().filter(|m| m.is_file()).map(|m| m.len())
        }
        Some(rev) => blob(rev, &file.path),
    };
    let old_path = file.original_path.as_deref().unwrap_or(&file.path);
    let old =
        (old_side != Side::Missing && spec.old != EMPTY_TREE).then(|| blob(&spec.old, old_path)).flatten();
    (old, new)
}

fn capped_text(cmd: &mut GitCmd, max_bytes: u64) -> Option<String> {
    if max_bytes != u64::MAX {
        cmd.max_output(max_bytes);
    }
    cmd.run().ok().map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
}

/// The patch for a file git does not know yet: what `git diff --no-index` prints for it.
fn untracked_patch(root: &Path, path: &str, limits: &DiffLimits) -> Result<Patch> {
    let full = root.join(path);
    if let Ok(meta) = std::fs::symlink_metadata(&full) {
        let kind = meta.file_type();
        if kind.is_symlink() {
            // git follows a link on its command line to a FIFO or a device and then waits for
            // data that never comes; the patch of a link is its target anyway.
            let special = std::fs::metadata(&full).is_ok_and(|m| !m.is_file() && !m.is_dir());
            if special && let Ok(target) = std::fs::read_link(&full) {
                return Ok(Patch::Text(link_patch(path, &target.to_string_lossy()).into_bytes()));
            }
        } else if !kind.is_file() && !kind.is_dir() {
            anyhow::bail!("{path} is not a regular file");
        }
    }
    let mut cmd = GitCmd::diff(root);
    cmd.args(["--no-color", "--no-index"]).paths(["/dev/null", path]);
    // Exits 1 when files differ, so don't treat that as an error.
    capped(&mut cmd, limits, |cmd| Ok(cmd.output()?.stdout))
}

/// What git prints for a new symbolic link.
fn link_patch(path: &str, target: &str) -> String {
    format!(
        "diff --git a/{path} b/{path}\nnew file mode 120000\n--- /dev/null\n+++ b/{path}\n@@ -0,0 +1 @@\n+{target}\n\\ No newline at end of file\n"
    )
}

/// A worktree file as text for highlighting, or `None` (missing, binary, over `max_bytes`, or
/// not a regular file: a FIFO or a link to one must never be opened). A link reads as its
/// target, the way git stores and diffs it.
fn read_text(path: &Path, max_bytes: u64) -> Option<String> {
    let meta = std::fs::symlink_metadata(path).ok()?;
    if meta.file_type().is_symlink() {
        return std::fs::read_link(path).ok().map(|target| target.to_string_lossy().into_owned());
    }
    if !meta.is_file() || meta.len() > max_bytes {
        return None;
    }
    // `read_capped` adds one to its cap, so `u64::MAX` (unlimited) must not reach it.
    let bytes = fsutil::read_capped(path, max_bytes.min(u64::MAX - 1), LinkPolicy::Follow).ok()?;
    (!bytes.contains(&0)).then(|| String::from_utf8_lossy(&bytes).into_owned())
}

/// Parse unified diff output for a single file.
pub(crate) fn parse_diff(text: &str) -> FileDiff {
    let mut diff = FileDiff::default();
    let mut old = 0u32;
    let mut new = 0u32;
    for line in text.lines() {
        if line.starts_with("Binary files ") || line.starts_with("GIT binary patch") {
            diff.binary = true;
        } else if diff.hunks.is_empty() && parse_header_line(&mut diff, line) {
            // File header: new / deleted / renamed / mode change.
        } else if let Some(rest) = line.strip_prefix("@@ ") {
            // @@ -a,b +c,d @@ context
            let mut ranges = rest.split(' ');
            let parse_start =
                |s: Option<&str>| s.and_then(|r| r[1..].split(',').next()?.parse::<u32>().ok()).unwrap_or(1);
            old = parse_start(ranges.next());
            new = parse_start(ranges.next());
            diff.hunks.push(Hunk { header: line.to_string(), lines: Vec::new() });
        } else if let Some(hunk) = diff.hunks.last_mut() {
            let (kind, body) = match line.as_bytes().first() {
                Some(b'+') => (LineKind::Added, &line[1..]),
                Some(b'-') => (LineKind::Removed, &line[1..]),
                Some(b' ') => (LineKind::Context, &line[1..]),
                // "\ No newline at end of file", about the line before it.
                Some(b'\\') => {
                    if let Some(last) = hunk.lines.last_mut() {
                        last.no_newline = true;
                    }
                    continue;
                }
                None => (LineKind::Context, ""),
                _ => continue,
            };
            let (o, n) = match kind {
                LineKind::Context => (Some(old), Some(new)),
                LineKind::Added => (None, Some(new)),
                LineKind::Removed => (Some(old), None),
            };
            if o.is_some() {
                old += 1;
            }
            if n.is_some() {
                new += 1;
            }
            hunk.lines.push(DiffLine { kind, old: o, new: n, text: body.to_string(), no_newline: false });
        }
    }
    diff
}

/// Record what a line of the file header (before the first hunk) says about
/// the file as a whole. False if it isn't one of those lines.
fn parse_header_line(diff: &mut FileDiff, line: &str) -> bool {
    if line.starts_with("new file mode") || line == "--- /dev/null" {
        diff.new_file = true;
    } else if line.starts_with("deleted file mode") || line == "+++ /dev/null" {
        diff.deleted_file = true;
    } else if let Some(from) = line.strip_prefix("rename from ") {
        diff.renamed_from = Some(from.to_string());
    } else if let Some(mode) = line.strip_prefix("old mode ") {
        diff.mode_change = Some((mode.to_string(), String::new()));
    } else if let Some(mode) = line.strip_prefix("new mode ") {
        let old = diff.mode_change.take().map(|m| m.0).unwrap_or_default();
        diff.mode_change = Some((old, mode.to_string()));
    } else {
        return false;
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_unified_diff_with_line_numbers() {
        let text = "diff --git a/f.rs b/f.rs\n--- a/f.rs\n+++ b/f.rs\n@@ -1,3 +1,4 @@ fn main\n a\n-b\n+B\n+c\n d\n\\ No newline at end of file\n";
        let diff = parse_diff(text);
        assert_eq!(diff.hunks.len(), 1);
        let lines = &diff.hunks[0].lines;
        assert_eq!(lines.len(), 5);
        assert_eq!((lines[0].old, lines[0].new), (Some(1), Some(1)));
        assert_eq!((lines[1].kind, lines[1].old), (LineKind::Removed, Some(2)));
        assert_eq!((lines[2].kind, lines[2].new), (LineKind::Added, Some(2)));
        assert_eq!((lines[3].kind, lines[3].new), (LineKind::Added, Some(3)));
        assert_eq!((lines[4].old, lines[4].new), (Some(3), Some(4)));
    }

    #[test]
    fn detects_binary() {
        assert!(parse_diff("Binary files a/x.png and b/x.png differ\n").binary);
    }

    #[test]
    fn no_newline_marker_flags_the_line_before_it() {
        let text =
            "@@ -1,2 +1,2 @@\n a\n-b\n\\ No newline at end of file\n+b\n\\ No newline at end of file\n";
        let lines = &parse_diff(text).hunks[0].lines;
        let flags: Vec<_> = lines.iter().map(|l| (l.kind, l.no_newline)).collect();
        assert_eq!(
            flags,
            vec![(LineKind::Context, false), (LineKind::Removed, true), (LineKind::Added, true)]
        );
    }

    #[test]
    fn header_says_new_deleted_renamed_and_mode() {
        let new = parse_diff(
            "diff --git a/n.rs b/n.rs\nnew file mode 100644\nindex 0000000..e69de29\n--- /dev/null\n+++ b/n.rs\n@@ -0,0 +1 @@\n+x\n",
        );
        assert!(new.new_file && !new.deleted_file);
        let deleted = parse_diff(
            "diff --git a/d.rs b/d.rs\ndeleted file mode 100644\n--- a/d.rs\n+++ /dev/null\n@@ -1 +0,0 @@\n-x\n",
        );
        assert!(deleted.deleted_file && !deleted.new_file);
        let renamed = parse_diff(
            "diff --git a/a.rs b/b.rs\nold mode 100644\nnew mode 100755\nsimilarity index 100%\nrename from a.rs\nrename to b.rs\n",
        );
        assert_eq!(renamed.renamed_from.as_deref(), Some("a.rs"));
        assert_eq!(renamed.mode_change, Some(("100644".into(), "100755".into())));
        assert!(renamed.hunks.is_empty());
    }

    #[test]
    fn header_lookalikes_inside_a_hunk_are_content() {
        // A removed "-- /dev/null" line reads "--- /dev/null" in the patch.
        let diff = parse_diff("@@ -1 +1 @@\n--- /dev/null\n+x\n");
        assert!(!diff.new_file);
        assert_eq!(diff.hunks[0].lines[0].text, "-- /dev/null");
    }

    #[test]
    fn a_removed_line_opens_where_its_text_used_to_be() {
        use LineKind::{Added, Context, Removed};
        let line = |kind, old, new| DiffLine { kind, old, new, text: String::new(), no_newline: false };
        let hunk = [
            line(Context, Some(1), Some(1)),
            line(Removed, Some(2), None),
            line(Removed, Some(3), None),
            line(Added, None, Some(2)),
            line(Context, Some(4), Some(3)),
            line(Removed, Some(5), None),
        ];
        // The removals sit before new line 2; the one at the end falls back to the line above.
        assert_eq!(lines_in_new_file(&hunk), [Some(1), Some(2), Some(2), Some(2), Some(3), Some(3)]);
        // Nothing left at all: no line to open.
        assert_eq!(lines_in_new_file(&[line(Removed, Some(1), None)]), [None]);
        assert_eq!(lines_in_new_file(&[]), Vec::<Option<u32>>::new());
    }
}
