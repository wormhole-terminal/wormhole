//! The repository a submodule belongs to, found from the submodule's own files.
//!
//! A submodule's `.git` is a file: `gitdir: ../.git/modules/<name>`, the parent's copy of its
//! repository (absorbed there by `git submodule add`). So the title of a terminal in a submodule can
//! say "in terminal" with a file read and no git process, which the title asks for on every
//! refresh. Nested submodules work the same way, one level up: `.git/modules/a/modules/b`.

use std::path::{Path, PathBuf};

use crate::fsutil::{LinkPolicy, read_capped};
use crate::git::find_root;
use crate::git::head::{POINTER_CAP, normalize, pointed_dir};

/// The repository that has the submodule whose top is `root`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Superproject {
    /// The parent's working tree.
    pub root: PathBuf,
    /// Where the submodule sits in it (`vendor/lib`).
    pub path: String,
}

impl Superproject {
    /// The parent's name, as its folder says it.
    pub fn name(&self) -> String {
        self.root.file_name().map_or_else(|| self.root.display().to_string(), |n| n.to_string_lossy().into())
    }

    /// What the title says after the submodule's own name: "in terminal".
    pub fn phrase(&self) -> String {
        format!("in {}", self.name())
    }
}

/// The superproject of the submodule whose working tree is `root`, or `None` for anything else:
/// an ordinary repository (its `.git` is a folder), a worktree (`.git/worktrees/<name>`), or a
/// `.git` file that points nowhere a submodule's would. Nothing is followed outside `root`.
pub fn superproject(root: &Path) -> Option<Superproject> {
    let dot_git = root.join(".git");
    if !dot_git.is_file() {
        return None;
    }
    let pointer = read_capped(&dot_git, POINTER_CAP, LinkPolicy::Within(root)).ok()?;
    let gitdir = normalize(&root.join(pointed_dir(&String::from_utf8_lossy(&pointer))?));
    // `<parent git dir>/modules/<name>`, with more `modules/<name>` pairs for the nested ones.
    if !in_modules(gitdir.parent()?) {
        return None;
    }
    let parent = find_root(root.parent()?)?;
    if !gitdir.starts_with(git_dir_of(&parent)?) {
        return None;
    }
    let path = root.strip_prefix(&parent).ok()?.to_string_lossy().replace('\\', "/");
    Some(Superproject { root: parent, path })
}

/// Whether `dir` is a `modules` folder or below one (the name of a submodule that has a slash in
/// it, `git submodule add --name vendor/lib`, is a folder deeper), and not in a worktree's.
fn in_modules(dir: &Path) -> bool {
    let named = |a: &Path, name: &str| a.file_name().is_some_and(|n| n == name);
    dir.ancestors().any(|a| named(a, "modules")) && !dir.ancestors().any(|a| named(a, "worktrees"))
}

/// A working tree's git dir: its `.git` folder, or the one its `.git` file points to.
fn git_dir_of(tree: &Path) -> Option<PathBuf> {
    let dot_git = tree.join(".git");
    if dot_git.is_dir() {
        return Some(normalize(&dot_git));
    }
    let pointer = read_capped(&dot_git, POINTER_CAP, LinkPolicy::Within(tree)).ok()?;
    Some(normalize(&tree.join(pointed_dir(&String::from_utf8_lossy(&pointer))?)))
}
