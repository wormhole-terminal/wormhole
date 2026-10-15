//! Reading what `git status --porcelain=v2` and `git diff --raw/--numstat` print.

use super::submodule::Submodule;
use super::{ChangeKind, FileChange, Upstream};

/// Parse `git diff --raw -z -M --no-abbrev`: `:100644 100644 <sha> <sha> M\0path\0`, and for a
/// rename `…R100\0old\0new\0`. The modes and ids are what tell a submodule (mode 160000) from a
/// file, and say which commit each side records.
pub(crate) fn parse_raw(raw: &[u8]) -> Vec<FileChange> {
    let text = String::from_utf8_lossy(raw);
    let mut fields = text.split('\0').filter(|f| !f.is_empty());
    let mut files = Vec::new();
    while let Some(header) = fields.next() {
        let mut cols = header.trim_start_matches(':').split(' ');
        let (Some(old_mode), Some(new_mode), Some(old_id), Some(new_id), Some(status)) =
            (cols.next(), cols.next(), cols.next(), cols.next(), cols.next())
        else {
            continue;
        };
        let (kind, original) = match status.chars().next() {
            Some('A') => (ChangeKind::Added, None),
            Some('D') => (ChangeKind::Deleted, None),
            Some('R') | Some('C') => (ChangeKind::Renamed, fields.next().map(str::to_string)),
            Some('U') => (ChangeKind::Conflicted, None),
            _ => (ChangeKind::Modified, None),
        };
        let Some(path) = fields.next() else { break };
        files.push(FileChange {
            path: path.to_string(),
            original_path: original,
            kind,
            added: None,
            removed: None,
            lfs: false,
            submodule: Submodule::from_raw((old_mode, old_id), (new_mode, new_id)),
        });
    }
    files
}

/// What `git status --porcelain=v2 --branch -z` says.
pub(super) struct Porcelain {
    pub(super) branch: Option<String>,
    /// From `# branch.oid`: `(initial)` means no commit yet. `None` if the header is missing.
    pub(super) has_head: Option<bool>,
    /// From `# branch.upstream` and `# branch.ab`, which git prints only for a branch whose
    /// upstream exists.
    pub(super) upstream: Option<Upstream>,
    pub(super) files: Vec<FileChange>,
}

/// Parse `git status --porcelain=v2 --branch -z`.
pub(super) fn parse_porcelain(raw: &[u8]) -> Porcelain {
    let text = String::from_utf8_lossy(raw);
    let mut fields = text.split('\0').peekable();
    let mut branch = None;
    let mut has_head = None;
    let mut upstream_name = None;
    let mut counts = None;
    let mut files = Vec::new();

    let kind_from_xy = |xy: &str| -> ChangeKind {
        let mut chars = xy.chars();
        let (x, y) = (chars.next().unwrap_or('.'), chars.next().unwrap_or('.'));
        if x == 'A' {
            ChangeKind::Added
        } else if x == 'D' || y == 'D' {
            ChangeKind::Deleted
        } else {
            ChangeKind::Modified
        }
    };

    while let Some(entry) = fields.next() {
        if entry.is_empty() {
            continue;
        }
        let mut parts = entry.splitn(2, ' ');
        let tag = parts.next().unwrap_or_default();
        let rest = parts.next().unwrap_or_default();
        match tag {
            "#" => {
                if let Some(head) = rest.strip_prefix("branch.head ")
                    && head != "(detached)"
                {
                    branch = Some(head.to_string());
                } else if let Some(oid) = rest.strip_prefix("branch.oid ") {
                    has_head = Some(oid != "(initial)");
                } else if let Some(name) = rest.strip_prefix("branch.upstream ") {
                    upstream_name = Some(name.to_string());
                } else if let Some(ab) = rest.strip_prefix("branch.ab ") {
                    counts = parse_ahead_behind(ab);
                }
            }
            // 1 XY sub mH mI mW hH hI path
            "1" => {
                let cols: Vec<&str> = rest.splitn(8, ' ').collect();
                if cols.len() == 8 {
                    let mut file = FileChange::new(cols[7], kind_from_xy(cols[0]));
                    file.submodule =
                        Submodule::from_status(cols[1], &cols[2..5], cols[5], cols[6], file.kind);
                    files.push(file);
                }
            }
            // 2 XY sub mH mI mW hH hI Xscore path \0 origPath
            "2" => {
                let cols: Vec<&str> = rest.splitn(9, ' ').collect();
                let original = fields.next().map(str::to_string);
                if cols.len() == 9 {
                    let mut file = FileChange::new(cols[8], ChangeKind::Renamed);
                    file.original_path = original;
                    file.submodule =
                        Submodule::from_status(cols[1], &cols[2..5], cols[5], cols[6], file.kind);
                    files.push(file);
                }
            }
            // u XY sub m1 m2 m3 mW h1 h2 h3 path: the commit HEAD has is stage 2's
            "u" => {
                let cols: Vec<&str> = rest.splitn(10, ' ').collect();
                if cols.len() == 10 {
                    let mut file = FileChange::new(cols[9], ChangeKind::Conflicted);
                    file.submodule =
                        Submodule::from_status(cols[1], &cols[2..6], cols[7], cols[7], file.kind);
                    files.push(file);
                }
            }
            "?" => files.push(FileChange::new(rest, ChangeKind::Untracked)),
            _ => {}
        }
    }
    // Both lines or nothing: an upstream that is gone has a name but no counts.
    let upstream = upstream_name.zip(counts).map(|(name, (ahead, behind))| Upstream { name, ahead, behind });
    Porcelain { branch, has_head, upstream, files }
}

/// `+2 -1`, what `# branch.ab` says: commits ahead of and behind the upstream.
fn parse_ahead_behind(text: &str) -> Option<(u32, u32)> {
    let (ahead, behind) = text.split_once(' ')?;
    Some((ahead.strip_prefix('+')?.parse().ok()?, behind.strip_prefix('-')?.parse().ok()?))
}

/// Parse `git diff --numstat -z [-M]`: `added\tremoved\tpath\0`, or for
/// renames `added\tremoved\t\0old\0new\0` (keyed by the new path).
pub(crate) fn parse_numstat(raw: &[u8]) -> Vec<(String, Option<u32>, Option<u32>)> {
    let text = String::from_utf8_lossy(raw);
    let mut fields = text.split('\0');
    let mut out = Vec::new();
    while let Some(entry) = fields.next() {
        let mut cols = entry.splitn(3, '\t');
        let (Some(added), Some(removed), Some(path)) = (cols.next(), cols.next(), cols.next()) else {
            continue;
        };
        let path = if path.is_empty() {
            let _old = fields.next();
            fields.next().unwrap_or_default()
        } else {
            path
        };
        out.push((path.to_string(), added.parse().ok(), removed.parse().ok()));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_porcelain_v2() {
        let raw = b"# branch.oid abc\0# branch.head main\0\
1 .M N... 100644 100644 100644 aaa bbb src/main.rs\0\
1 A. N... 000000 100644 100644 000 ccc new file.txt\0\
1 .D N... 100644 100644 000000 ddd ddd gone.rs\0\
2 R. N... 100644 100644 100644 eee eee R100 renamed.rs\0old.rs\0\
u UU N... 100644 100644 100644 100644 a b c conflict.rs\0\
? notes/todo.md\0";
        let Porcelain { branch, files, .. } = parse_porcelain(raw);
        assert_eq!(branch.as_deref(), Some("main"));
        let kinds: Vec<_> = files.iter().map(|f| (f.path.as_str(), f.kind)).collect();
        assert_eq!(
            kinds,
            vec![
                ("src/main.rs", ChangeKind::Modified),
                ("new file.txt", ChangeKind::Added),
                ("gone.rs", ChangeKind::Deleted),
                ("renamed.rs", ChangeKind::Renamed),
                ("conflict.rs", ChangeKind::Conflicted),
                ("notes/todo.md", ChangeKind::Untracked),
            ]
        );
        assert_eq!(files[3].original_path.as_deref(), Some("old.rs"));
    }

    #[test]
    fn detached_head_has_no_branch() {
        let branch = parse_porcelain(b"# branch.head (detached)\0").branch;
        assert_eq!(branch, None);
    }

    #[test]
    fn parses_numstat() {
        let counts = parse_numstat(b"3\t1\tsrc/main.rs\0-\t-\timage.png\0");
        assert_eq!(counts[0], ("src/main.rs".into(), Some(3), Some(1)));
        assert_eq!(counts[1], ("image.png".into(), None, None));
    }

    #[test]
    fn parses_numstat_with_renames() {
        let counts = parse_numstat(b"2\t0\t\0old.rs\0new.rs\0");
        assert_eq!(counts, vec![("new.rs".into(), Some(2), Some(0))]);
    }

    /// Just under 40 hex digits would be no id; these are made up but long enough.
    const A: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
    const B: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
    const ZERO: &str = "0000000000000000000000000000000000000000";

    #[test]
    fn parses_raw() {
        let raw = format!(
            ":100644 100644 {A} {B} M\0a.rs\0:100644 100644 {A} {B} R087\0old.rs\0new.rs\0\
:000000 100644 {ZERO} {B} A\0b.rs\0:100644 000000 {A} {ZERO} D\0c.rs\0"
        );
        let files = parse_raw(raw.as_bytes());
        let kinds: Vec<_> = files.iter().map(|f| (f.path.as_str(), f.kind)).collect();
        assert_eq!(
            kinds,
            vec![
                ("a.rs", ChangeKind::Modified),
                ("new.rs", ChangeKind::Renamed),
                ("b.rs", ChangeKind::Added),
                ("c.rs", ChangeKind::Deleted),
            ]
        );
        assert_eq!(files[1].original_path.as_deref(), Some("old.rs"));
        assert!(files.iter().all(|f| f.submodule.is_none()), "files are not submodules");
    }

    #[test]
    fn a_raw_entry_with_mode_160000_is_a_submodule() {
        let raw = format!(
            ":160000 160000 {A} {B} M\0libs/moved\0\
:160000 160000 {A} {ZERO} M\0libs/worktree\0\
:160000 160000 {A} {A} M\0libs/dirty\0\
:000000 160000 {ZERO} {B} A\0libs/new\0\
:160000 000000 {A} {ZERO} D\0libs/gone\0"
        );
        let files = parse_raw(raw.as_bytes());
        let sub = |ix: usize| files[ix].submodule.clone().expect("a submodule");
        let moved = sub(0);
        assert_eq!((moved.old.as_deref(), moved.new.as_deref()), (Some(A), Some(B)));
        assert!(moved.commit_changed && !moved.tracked);
        // The working tree side prints no id when the folder is at another commit: its HEAD is read later.
        let worktree = sub(1);
        assert_eq!((worktree.old.as_deref(), worktree.new), (Some(A), None));
        assert!(worktree.commit_changed);
        // The same id on both sides and still listed: changes inside the submodule.
        let dirty = sub(2);
        assert!(!dirty.commit_changed && dirty.tracked);
        let added = sub(3);
        assert_eq!((added.old, added.new.as_deref()), (None, Some(B)));
        let removed = sub(4);
        assert_eq!((removed.old.as_deref(), removed.new), (Some(A), None));
        assert_eq!(files[4].kind, ChangeKind::Deleted);
    }

    #[test]
    fn porcelain_entries_say_what_changed_in_a_submodule() {
        let raw = format!(
            "# branch.oid abc\0# branch.head main\0\
1 .M SC.. 160000 160000 160000 {A} {A} libs/moved\0\
1 .M S.MU 160000 160000 160000 {A} {A} libs/dirty\0\
1 .M S..U 160000 160000 160000 {A} {A} libs/untracked\0\
1 A. S... 000000 160000 160000 {ZERO} {B} libs/new\0\
1 .M N... 100644 100644 100644 {A} {A} libs/file.rs\0\
2 R. SC.. 160000 160000 160000 {A} {B} R100 libs/renamed\0libs/old\0\
u UU SC.. 160000 160000 160000 160000 {A} {B} {A} libs/conflict\0"
        );
        let files = parse_porcelain(raw.as_bytes()).files;
        let flags = |ix: usize| {
            let sub = files[ix].submodule.as_ref().expect("a submodule");
            (sub.commit_changed, sub.tracked, sub.untracked)
        };
        assert_eq!(flags(0), (true, false, false));
        assert_eq!(flags(1), (false, true, true));
        assert_eq!(flags(2), (false, false, true));
        assert_eq!(files[0].submodule.as_ref().map(|s| s.old.as_deref()), Some(Some(A)));
        let added = files[3].submodule.as_ref().expect("a submodule");
        assert_eq!((added.old.as_deref(), added.new.as_deref()), (None, Some(B)));
        assert!(files[4].submodule.is_none(), "N... is a file");
        assert!(files[5].submodule.is_some() && files[5].original_path.as_deref() == Some("libs/old"));
        // A conflict keeps what the panel does for it (its kind) and gains the submodule's state; the
        // commit HEAD has is the middle stage's.
        assert_eq!(files[6].kind, ChangeKind::Conflicted);
        assert_eq!(files[6].submodule.as_ref().and_then(|s| s.old.as_deref()), Some(B));
    }

    #[test]
    fn the_branch_header_says_whether_there_is_a_commit() {
        let initial = parse_porcelain(b"# branch.oid (initial)\0# branch.head main\0");
        assert_eq!((initial.has_head, initial.branch.as_deref()), (Some(false), Some("main")));
        let born =
            parse_porcelain(b"# branch.oid 4b825dc642cb6eb9a060e54bf8d69288fbee4904\0# branch.head main\0");
        assert_eq!(born.has_head, Some(true));
        assert_eq!(parse_porcelain(b"? a\0").has_head, None, "no header: ask git");
    }

    #[test]
    fn the_branch_header_carries_the_upstream_and_how_far_apart_they_are() {
        let tracked =
            parse_porcelain(b"# branch.head main\0# branch.upstream origin/main\0# branch.ab +2 -1\0");
        assert_eq!(tracked.upstream, Some(Upstream { name: "origin/main".into(), ahead: 2, behind: 1 }));
        // No upstream: git prints neither line. An upstream that is gone: only the name.
        assert_eq!(parse_porcelain(b"# branch.head main\0").upstream, None);
        let gone = parse_porcelain(b"# branch.head main\0# branch.upstream origin/old\0");
        assert_eq!(gone.upstream, None, "no counts to show");
        // Not numbers: nothing rather than a wrong count.
        let odd = parse_porcelain(b"# branch.upstream origin/main\0# branch.ab 2 1\0");
        assert_eq!(odd.upstream, None);
    }
}
