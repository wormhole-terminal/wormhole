//! The branch a folder is on, read from the repository's `HEAD` file without a git process: a list of folders
//! (the folder picker's) asks about many, and each answer is two small reads.
//!
//! A folder is a repository's top when it has a `.git` of its own: a folder (an ordinary repository), or a
//! file that names the git dir (a linked worktree or a submodule). Nothing is followed but that one named
//! path, and `HEAD` is read under a small cap and only as a regular file. A repository is hostile input: its
//! branch name is shown as text, cut short and cleaned of control characters.

use std::path::{Path, PathBuf};

use super::head::{POINTER_CAP, normalize, pointed_dir};
use crate::storage::files::{LinkPolicy, read_capped};

/// `HEAD` is one line.
const HEAD_CAP: u64 = 1024;
/// The longest branch name shown.
const NAME_CAP: usize = 40;

/// What `HEAD` points at.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Head {
    /// On a branch.
    Branch(String),
    /// On a commit: its short hash.
    Detached(String),
}

impl Head {
    /// The words for a small badge.
    pub fn label(&self) -> &str {
        match self {
            Head::Branch(name) | Head::Detached(name) => name,
        }
    }
}

/// Where `HEAD` is for the repository whose top is `dir`: its branch, or the commit it is detached at. `None`
/// for a folder that is not a repository's top, and for one whose `HEAD` can't be read or understood (a repository
/// is still there; it just has no branch to show).
pub fn head_of(dir: &Path) -> Option<Head> {
    let git_dir = git_dir_of(dir)?;
    let bytes = read_capped(&git_dir.join("HEAD"), HEAD_CAP, LinkPolicy::Within(&git_dir)).ok()?;
    parse_head(&String::from_utf8_lossy(&bytes))
}

/// Whether `dir` has a `.git` of its own, by one look at its metadata (a link doesn't count: it is never
/// followed).
pub fn is_repository_top(dir: &Path) -> bool {
    std::fs::symlink_metadata(dir.join(".git")).is_ok_and(|m| m.is_dir() || m.is_file())
}

fn git_dir_of(dir: &Path) -> Option<PathBuf> {
    let dot_git = dir.join(".git");
    let meta = std::fs::symlink_metadata(&dot_git).ok()?;
    if meta.is_dir() {
        return Some(dot_git);
    }
    if !meta.is_file() {
        return None;
    }
    let pointer = read_capped(&dot_git, POINTER_CAP, LinkPolicy::Within(dir)).ok()?;
    let named = pointed_dir(&String::from_utf8_lossy(&pointer))?.to_string();
    Some(normalize(&dir.join(named)))
}

/// What the text of a `HEAD` file says.
pub fn parse_head(text: &str) -> Option<Head> {
    let line = text.lines().next()?.trim();
    if let Some(target) = line.strip_prefix("ref:") {
        let name = target.trim().strip_prefix("refs/heads/")?;
        let name: String = name.chars().filter(|c| !c.is_control()).collect();
        if name.is_empty() {
            return None;
        }
        return Some(Head::Branch(shorten(&name)));
    }
    let hex = matches!(line.len(), 40 | 64) && line.bytes().all(|b| b.is_ascii_hexdigit());
    hex.then(|| Head::Detached(line[..7].to_string()))
}

fn shorten(name: &str) -> String {
    if name.chars().count() <= NAME_CAP {
        return name.to_string();
    }
    let head: String = name.chars().take(NAME_CAP - 1).collect();
    format!("{head}…")
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::{Head, head_of, is_repository_top, parse_head};

    #[test]
    fn head_says_a_branch_or_a_commit() {
        assert_eq!(parse_head("ref: refs/heads/main\n"), Some(Head::Branch("main".into())));
        assert_eq!(parse_head("ref: refs/heads/feat/x-y\n"), Some(Head::Branch("feat/x-y".into())));
        let hash = "4104e74a5b6c7d8e9f001122334455667788aabb";
        assert_eq!(parse_head(&format!("{hash}\n")), Some(Head::Detached("4104e74".into())));
    }

    #[test]
    fn anything_else_is_no_branch() {
        for text in ["", "nonsense", "ref: refs/tags/v1\n", "ref: refs/heads/\n", "abc123\n", "ref: HEAD"] {
            assert_eq!(parse_head(text), None, "{text:?}");
        }
    }

    #[test]
    fn a_branch_name_is_cleaned_and_cut() {
        assert_eq!(parse_head("ref: refs/heads/a\u{1b}[2Jb\n"), Some(Head::Branch("a[2Jb".into())));
        let long = format!("ref: refs/heads/{}\n", "x".repeat(100));
        let Some(Head::Branch(name)) = parse_head(&long) else { panic!("a branch") };
        assert_eq!(name.chars().count(), 40);
        assert!(name.ends_with('…'));
    }

    #[test]
    fn an_ordinary_repository_and_a_linked_worktree_are_read_and_a_plain_folder_is_not() {
        let root = tempfile::tempdir().unwrap();
        let repo = root.path().join("repo");
        fs::create_dir_all(repo.join(".git")).unwrap();
        fs::write(repo.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        assert!(is_repository_top(&repo));
        assert_eq!(head_of(&repo), Some(Head::Branch("main".into())));

        // A linked worktree: `.git` is a file, and HEAD is in the git dir it names (here relative).
        let tree = root.path().join("tree");
        fs::create_dir_all(repo.join(".git/worktrees/tree")).unwrap();
        fs::write(repo.join(".git/worktrees/tree/HEAD"), "ref: refs/heads/topic\n").unwrap();
        fs::create_dir_all(&tree).unwrap();
        fs::write(tree.join(".git"), "gitdir: ../repo/.git/worktrees/tree\n").unwrap();
        assert!(is_repository_top(&tree));
        assert_eq!(head_of(&tree), Some(Head::Branch("topic".into())));

        let plain = root.path().join("plain");
        fs::create_dir_all(&plain).unwrap();
        assert!(!is_repository_top(&plain));
        assert_eq!(head_of(&plain), None);
    }

    #[test]
    fn a_repository_without_a_readable_head_has_no_branch_and_a_linked_dot_git_is_not_followed() {
        let root = tempfile::tempdir().unwrap();
        let broken = root.path().join("broken");
        fs::create_dir_all(broken.join(".git")).unwrap();
        assert!(is_repository_top(&broken));
        assert_eq!(head_of(&broken), None);

        let linked = root.path().join("linked");
        fs::create_dir_all(&linked).unwrap();
        crate::platform::fs::symlink(root.path().join("broken/.git"), linked.join(".git")).unwrap();
        assert!(!is_repository_top(&linked), "a link is never followed");
        assert_eq!(head_of(&linked), None);
    }
}
