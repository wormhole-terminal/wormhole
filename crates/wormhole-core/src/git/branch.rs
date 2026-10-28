//! The branch a folder is on, read from the repository's `HEAD` file without a git process: a list of folders
//! (the folder picker's) asks about many, and each answer is two small reads.
//!
//! A folder is a repository's top when it has a `.git` of its own: a folder (an ordinary repository), or a
//! file that names the git dir (a linked worktree or a submodule). A repository is hostile input, so:
//! - a `.git` file is followed only to a git dir of the shape git makes (`…/.git/worktrees/<name>` or
//!   `…/.git/modules/…`) that is in the same part of the disk as the folder (its first two path components are
//!   the folder's: `/Users/me`, `/private/tmp`). A pointer to `/net/host/share` or to `../../../x` makes the
//!   app touch nothing: browsing a folder must not reach for a network mount or any place the repository
//!   chose. (Git writes an absolute pointer for a worktree, so an absolute one is followed when it passes.)
//! - `HEAD` is read under a small cap and only as a regular file;
//! - the branch name is shown as text, cut short and cleaned of control and invisible characters.

use std::path::{Path, PathBuf};

use super::head::{POINTER_CAP, normalize, pointed_dir};
use crate::storage::files::{LinkPolicy, hostile_char, read_capped};

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
    let git_dir = normalize(&dir.join(named));
    (shaped_like_a_git_dir(&git_dir) && same_area(&git_dir, dir) && !through_a_link(&git_dir))
        .then_some(git_dir)
}

/// Whether the way to `git_dir` goes through a symbolic link below its first two folders (where [`same_area`] has
/// looked): a pointer through `lnk`, which the repository made to point at a network mount, would otherwise
/// pass for a place in the same part of the disk. One `lstat` for each step, all of them local.
fn through_a_link(git_dir: &Path) -> bool {
    let mut at = PathBuf::new();
    let mut named = 0;
    for part in git_dir.components() {
        at.push(part);
        if matches!(part, std::path::Component::Normal(_)) {
            named += 1;
            if named > 2 && std::fs::symlink_metadata(&at).is_ok_and(|m| m.file_type().is_symlink()) {
                return true;
            }
        }
    }
    false
}

/// Whether `git_dir` ends as the git dir of a linked worktree (`.git/worktrees/<name>`) or of a submodule
/// (`.git/modules/…`) does.
fn shaped_like_a_git_dir(git_dir: &Path) -> bool {
    let names: Vec<_> = git_dir.components().filter_map(|c| c.as_os_str().to_str()).collect();
    names.windows(2).enumerate().any(|(at, pair)| {
        let after = names.len() - at - 2;
        pair[0] == ".git" && ((pair[1] == "worktrees" && after == 1) || (pair[1] == "modules" && after >= 1))
    })
}

/// Whether `a` and `b` start with the same two folders: in one part of the disk (a home, a volume's
/// folder), not across mounts the repository could name.
fn same_area(a: &Path, b: &Path) -> bool {
    let head = |p: &Path| -> Vec<std::ffi::OsString> {
        p.components()
            .filter_map(|c| match c {
                std::path::Component::Normal(n) => Some(n.to_os_string()),
                _ => None,
            })
            .take(2)
            .collect()
    };
    let (a, b) = (head(a), head(b));
    a.len() == 2 && a == b
}

/// What the text of a `HEAD` file says.
pub fn parse_head(text: &str) -> Option<Head> {
    let line = text.lines().next()?.trim();
    if let Some(target) = line.strip_prefix("ref:") {
        let name = target.trim().strip_prefix("refs/heads/")?;
        let name: String = name.chars().filter(|&c| !hostile_char(c)).collect();
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
        assert_eq!(parse_head("ref: refs/heads/rel\u{202e}txt\n"), Some(Head::Branch("reltxt".into())));
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
    fn a_git_file_is_followed_only_to_a_git_dir_shaped_like_git_makes_in_the_same_part_of_the_disk() {
        let root = tempfile::tempdir().unwrap();
        let tree = root.path().join("tree");
        fs::create_dir_all(&tree).unwrap();
        let pointing = |to: &str| {
            fs::write(tree.join(".git"), format!("gitdir: {to}\n")).unwrap();
            head_of(&tree)
        };
        // A real worktree's git dir, named relatively and by its absolute path (which git writes).
        let real = root.path().join("repo/.git/worktrees/tree");
        fs::create_dir_all(&real).unwrap();
        fs::write(real.join("HEAD"), "ref: refs/heads/topic\n").unwrap();
        assert_eq!(pointing("../repo/.git/worktrees/tree"), Some(Head::Branch("topic".into())));
        assert_eq!(pointing(&real.to_string_lossy()), Some(Head::Branch("topic".into())));
        // Anything else makes the app touch nothing.
        assert_eq!(pointing("/net/evil.example/.git/worktrees/x"), None, "an absolute pointer elsewhere");
        assert_eq!(pointing("../../../../../../../etc"), None, "one that climbs out");
        assert_eq!(pointing("../repo/.git"), None, "not a worktree's or a submodule's git dir");
        assert_eq!(pointing("../repo/.git/worktrees/tree/extra"), None);
        assert_eq!(pointing("/etc"), None);
    }

    #[test]
    fn a_pointer_through_a_link_inside_the_folder_is_not_followed() {
        let root = tempfile::tempdir().unwrap();
        let elsewhere = tempfile::tempdir().unwrap();
        let real = elsewhere.path().join(".git/worktrees/w");
        fs::create_dir_all(&real).unwrap();
        fs::write(real.join("HEAD"), "ref: refs/heads/far\n").unwrap();
        let tree = root.path().join("tree");
        fs::create_dir_all(&tree).unwrap();
        crate::platform::fs::symlink(elsewhere.path(), tree.join("lnk")).unwrap();
        fs::write(tree.join(".git"), "gitdir: lnk/.git/worktrees/w\n").unwrap();
        assert_eq!(head_of(&tree), None, "the text says it is under the folder, the link says it is not");
        // The same pointer through a real folder is followed.
        fs::remove_file(tree.join("lnk")).unwrap();
        fs::create_dir_all(tree.join("lnk/.git/worktrees/w")).unwrap();
        fs::write(tree.join("lnk/.git/worktrees/w/HEAD"), "ref: refs/heads/near\n").unwrap();
        assert_eq!(head_of(&tree), Some(Head::Branch("near".into())));
    }

    #[test]
    fn shapes_and_areas() {
        use super::{same_area, shaped_like_a_git_dir};
        use std::path::Path;
        assert!(shaped_like_a_git_dir(Path::new("/w/app/.git/worktrees/badge")));
        assert!(shaped_like_a_git_dir(Path::new("/w/app/.git/modules/sub/deeper")));
        assert!(!shaped_like_a_git_dir(Path::new("/w/app/.git/worktrees")));
        assert!(!shaped_like_a_git_dir(Path::new("/w/app/.git")));
        assert!(!shaped_like_a_git_dir(Path::new("/etc")));
        assert!(same_area(Path::new("/Users/me/a/b"), Path::new("/Users/me/c")));
        assert!(!same_area(Path::new("/net/host/x"), Path::new("/Users/me/c")));
        assert!(!same_area(Path::new("/Users"), Path::new("/Users")), "too short to be an area");
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
