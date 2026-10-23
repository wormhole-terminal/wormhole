//! Git LFS: which changed files `.gitattributes` stores as LFS pointers, and what a pointer
//! says. Git's line counts for them count the pointer's lines (the object's id and size), which
//! says nothing about the file, so the panel shows "LFS" in their place, and the diff shows what
//! the pointers stand for instead of their lines.

use std::collections::HashSet;
use std::path::Path;

use super::{FileChange, FileDiff, GitCmd, LineKind};
use crate::storage::files::{self, LinkPolicy};

/// What a Git LFS pointer file says: the object stored in the file's place.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LfsPointer {
    /// The hex of the object's SHA-256.
    pub oid: String,
    /// The object's size in bytes: the file's real size.
    pub size: u64,
}

/// A changed file each of whose sides is an LFS pointer (a new file has no old side, a
/// deleted one no new side).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct LfsChange {
    pub old: Option<LfsPointer>,
    pub new: Option<LfsPointer>,
}

/// The pointer versions git-lfs writes (the second is what it wrote before it was named LFS).
const POINTER_VERSIONS: [&str; 2] =
    ["https://git-lfs.github.com/spec/v1", "https://hawser.github.com/spec/v1"];
/// Pointer files are small (the spec caps them at 1024 bytes); anything bigger is a file.
const POINTER_MAX: usize = 1024;

/// Paths per `git check-attr`: a refresh with thousands of changed files stays far below the
/// argument size limit.
const BATCH: usize = 512;
/// The top-level `.gitattributes` is read up to this much to see whether LFS is used at all.
const ATTRIBUTES_CAP: u64 = 256 << 10;

/// Set [`FileChange::lfs`] on the files the repository sends through the LFS filter. Git is
/// asked only when the top-level `.gitattributes` names that filter, so a repository without
/// LFS (nearly every one) pays a small file read and no process. When git can't say, the files
/// keep their line counts: that is all this costs.
pub(super) fn mark_lfs(root: &Path, files: &mut [FileChange]) {
    if files.is_empty() || !uses_lfs(root) {
        return;
    }
    for batch in files.chunks_mut(BATCH) {
        let asked = GitCmd::new(root)
            .args(["check-attr", "-z", "filter"])
            .paths(batch.iter().map(|f| f.path.as_str()))
            .run();
        let lfs = match asked {
            Ok(raw) => lfs_paths(&raw),
            Err(err) => {
                tracing::debug!(?root, "git check-attr failed: {err:#}");
                return;
            }
        };
        for file in batch {
            file.lfs = lfs.contains(&file.path);
        }
    }
}

/// Whether the top-level `.gitattributes` names the LFS filter. (One further down that does
/// while the top one doesn't is missed; LFS setups put it at the top.)
fn uses_lfs(root: &Path) -> bool {
    files::read_capped(&root.join(".gitattributes"), ATTRIBUTES_CAP, LinkPolicy::Within(root))
        .is_ok_and(|text| String::from_utf8_lossy(&text).contains("filter=lfs"))
}

/// The paths `git check-attr -z filter` reports with the filter set to `lfs`. Its output is
/// `path NUL attribute NUL value NUL` per path.
fn lfs_paths(raw: &[u8]) -> HashSet<String> {
    // Lossy, as the paths it is matched against were read.
    let text = String::from_utf8_lossy(raw);
    let fields: Vec<&str> = text.split('\0').collect();
    let (entries, _) = fields.as_chunks::<3>();
    entries.iter().filter(|[_, _, value]| *value == "lfs").map(|[path, ..]| path.to_string()).collect()
}

/// `text` as an LFS pointer, or `None` when it is anything else. A pointer is a `version` line,
/// then `key value` lines with at least `oid sha256:<64 hex digits>` and `size <bytes>`; other
/// keys (`ext-0-…`) are allowed and skipped.
pub fn parse_pointer(text: &str) -> Option<LfsPointer> {
    if text.len() > POINTER_MAX {
        return None;
    }
    let mut lines = text.lines();
    let version = lines.next()?.strip_prefix("version ")?;
    if !POINTER_VERSIONS.contains(&version) {
        return None;
    }
    let (mut oid, mut size) = (None, None);
    for line in lines {
        match line.split_once(' ')? {
            ("oid", value) => oid = Some(value.strip_prefix("sha256:")?),
            ("size", value) => size = Some(value.parse().ok()?),
            _ => {}
        }
    }
    let oid = oid.filter(|oid| oid.len() == 64 && oid.bytes().all(|b| b.is_ascii_hexdigit()))?;
    Some(LfsPointer { oid: oid.to_string(), size: size? })
}

/// The pointers on each side of a patch, when each side it has is a whole pointer file: git
/// diffs those few lines in place of a file stored with LFS (and does so whether or not git-lfs
/// is installed).
pub(super) fn pointer_change(diff: &FileDiff) -> Option<LfsChange> {
    if diff.hunks.is_empty() || diff.binary {
        return None;
    }
    // One side of the file as the patch has it, if the patch has all of it: from its line 1.
    let side = |changed: LineKind| -> Option<String> {
        let mut lines = diff
            .hunks
            .iter()
            .flat_map(|h| &h.lines)
            .filter(|l| l.kind == LineKind::Context || l.kind == changed);
        let first = lines.next()?;
        let number = if changed == LineKind::Removed { first.old } else { first.new };
        if number != Some(1) {
            return None;
        }
        let mut text = String::new();
        for line in std::iter::once(first).chain(lines) {
            text.push_str(&line.text);
            text.push('\n');
            if text.len() > POINTER_MAX {
                return None;
            }
        }
        Some(text)
    };
    let pointer = |changed| side(changed).as_deref().and_then(parse_pointer);
    let old = if diff.new_file { None } else { Some(pointer(LineKind::Removed)?) };
    let new = if diff.deleted_file { None } else { Some(pointer(LineKind::Added)?) };
    (old.is_some() || new.is_some()).then_some(LfsChange { old, new })
}

#[cfg(test)]
mod tests {
    use super::{LfsChange, LfsPointer, lfs_paths, parse_pointer, pointer_change};
    use crate::git::parse_diff;

    const OLD_OID: &str = "4d7a214614ab2935c943f9e0ff69d22eadbb8f32b1258daaa5e2ca24d17e2393";
    const NEW_OID: &str = "1a2b3c4d5e6f708192a3b4c5d6e7f8091a2b3c4d5e6f708192a3b4c5d6e7f809";

    fn pointer(oid: &str, size: u64) -> String {
        format!("version https://git-lfs.github.com/spec/v1\noid sha256:{oid}\nsize {size}\n")
    }

    #[test]
    fn reads_a_pointer_and_nothing_else() {
        let expected = LfsPointer { oid: OLD_OID.into(), size: 12_345 };
        assert_eq!(parse_pointer(&pointer(OLD_OID, 12_345)), Some(expected.clone()));
        // Extension keys sit between them; the old version name still reads.
        let with_ext = format!(
            "version https://hawser.github.com/spec/v1\next-0-foo sha256:{NEW_OID}\noid sha256:{OLD_OID}\nsize 12345\n"
        );
        assert_eq!(parse_pointer(&with_ext), Some(expected));
        // Not a pointer: another version, a missing or malformed key, a short id, a file.
        for text in [
            pointer(OLD_OID, 1).replace("spec/v1", "spec/v2"),
            format!("version https://git-lfs.github.com/spec/v1\noid sha256:{OLD_OID}\n"),
            "version https://git-lfs.github.com/spec/v1\nsize 12\n".into(),
            pointer(OLD_OID, 1).replace("size 1", "size twelve"),
            pointer(&OLD_OID[..40], 1),
            pointer(OLD_OID, 1).replace("oid sha256:", "oid md5:"),
            format!("{}\n\n", pointer(OLD_OID, 1)),
            "fn main() {}\n".into(),
            format!("{}{}", pointer(OLD_OID, 1), "x".repeat(1024)),
        ] {
            assert_eq!(parse_pointer(&text), None, "{text}");
        }
    }

    /// A patch from one file's text to another's, the way git prints it (context, then the
    /// removed lines, then the added ones).
    fn patch(header: &str, old: &str, new: &str) -> String {
        let (old, new): (Vec<_>, Vec<_>) = (old.lines().collect(), new.lines().collect());
        let mut body = String::new();
        let common = old.iter().zip(&new).take_while(|(a, b)| a == b).count();
        for line in &old[..common] {
            body.push_str(&format!(" {line}\n"));
        }
        for line in &old[common..] {
            body.push_str(&format!("-{line}\n"));
        }
        for line in &new[common..] {
            body.push_str(&format!("+{line}\n"));
        }
        let start = |lines: &[&str]| if lines.is_empty() { 0 } else { 1 };
        format!("{header}@@ -{},{} +{},{} @@\n{body}", start(&old), old.len(), start(&new), new.len())
    }

    #[test]
    fn a_patch_of_pointers_says_what_they_stand_for() {
        let (old, new) = (pointer(OLD_OID, 12_300_000), pointer(NEW_OID, 12_800_000));
        let change = pointer_change(&parse_diff(&patch("", &old, &new))).expect("both sides are pointers");
        assert_eq!(change.old, Some(LfsPointer { oid: OLD_OID.into(), size: 12_300_000 }));
        assert_eq!(change.new, Some(LfsPointer { oid: NEW_OID.into(), size: 12_800_000 }));

        // A new file has only its new side, a deleted one only its old side.
        let added = parse_diff(&patch("new file mode 100644\n--- /dev/null\n", "", &new));
        assert_eq!(pointer_change(&added).map(|c| (c.old.is_none(), c.new.is_some())), Some((true, true)));
        let deleted = parse_diff(&patch("deleted file mode 100644\n+++ /dev/null\n", &old, ""));
        assert!(matches!(pointer_change(&deleted), Some(LfsChange { old: Some(_), new: None })));
    }

    #[test]
    fn a_side_that_is_not_a_pointer_is_shown_as_it_is() {
        let old = pointer(OLD_OID, 10);
        // The file was replaced by its content (git-lfs not set up): its lines are the diff.
        assert_eq!(pointer_change(&parse_diff(&patch("", &old, "real\ncontent\n"))), None);
        // Pointer-like lines in the middle of a file are not the whole file.
        let mut mid = parse_diff(&patch("", &old, &pointer(NEW_OID, 11)));
        for line in mid.hunks.iter_mut().flat_map(|h| &mut h.lines) {
            line.old = line.old.map(|n| n + 40);
            line.new = line.new.map(|n| n + 40);
        }
        assert_eq!(pointer_change(&mid), None);
        // Only a mode change: no lines, nothing to read.
        assert_eq!(pointer_change(&parse_diff("old mode 100644\nnew mode 100755\n")), None);
    }

    #[test]
    fn reads_the_paths_whose_filter_is_lfs() {
        let raw = b"assets/demo.mov\0filter\0lfs\0src/main.rs\0filter\0unspecified\0a b.psd\0filter\0lfs\0";
        let mut found: Vec<String> = lfs_paths(raw).into_iter().collect();
        found.sort();
        assert_eq!(found, ["a b.psd", "assets/demo.mov"]);
        assert!(lfs_paths(b"").is_empty());
    }
}
