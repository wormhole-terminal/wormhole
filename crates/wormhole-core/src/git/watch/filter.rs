//! Which changed paths matter to `git status`: the work tree except what git ignores, and the few
//! files in the git dirs that decide what status says. Not `objects/` (every commit and fetch writes
//! there, and status reads nothing new from it), not `logs/` (the reflog), not lock files (git writes
//! `index.lock` and renames it over `index`, which is the event that counts).

use std::path::{Component, Path};

use super::ignored::Ignored;
use super::layout::Layout;

/// What a changed path means for the repository's status.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Relevance {
    /// Nothing status shows.
    None,
    /// Read again.
    Status,
    /// Read again, and the ignore rules changed (a `.gitignore`, `info/exclude`): list the ignored
    /// paths again as well.
    Rules,
}

/// The rules for one repository. Cheap to call for every event: a few string comparisons, and a hash
/// lookup per folder level for a path in the work tree.
#[derive(Clone, Debug)]
pub struct Filter {
    layout: Layout,
    ignored: Ignored,
}

impl Filter {
    pub fn new(layout: Layout, ignored: Ignored) -> Self {
        Self { layout, ignored }
    }

    pub fn layout(&self) -> &Layout {
        &self.layout
    }

    /// What `path` (absolute, as the file events name it) means.
    pub fn classify(&self, path: &Path) -> Relevance {
        let Some(name) = path.file_name() else { return Relevance::None };
        if name.as_encoded_bytes().ends_with(b".lock") {
            return Relevance::None;
        }
        let Layout { work_tree, git_dir, common_dir } = &self.layout;
        // The git dirs first: the usual one is inside the work tree.
        if let Ok(rel) = path.strip_prefix(git_dir)
            && let found @ (Relevance::Status | Relevance::Rules) = own_file(rel)
        {
            return found;
        }
        if let Ok(rel) = path.strip_prefix(common_dir)
            && let found @ (Relevance::Status | Relevance::Rules) = shared_file(rel)
        {
            return found;
        }
        if path.starts_with(git_dir) || path.starts_with(common_dir) {
            return Relevance::None;
        }
        let Ok(rel) = path.strip_prefix(work_tree) else { return Relevance::None };
        // A nested repository's or a linked worktree's `.git` is its own business: changes in its files
        // show through the files, and what git keeps there is not this status's.
        if rel.components().any(|part| part == Component::Normal(".git".as_ref())) {
            return Relevance::None;
        }
        if self.ignored.covers(rel) {
            return Relevance::None;
        }
        if name == ".gitignore" { Relevance::Rules } else { Relevance::Status }
    }

    /// Whether any of `paths` matters, and the strongest thing it means.
    pub fn classify_all<'a>(&self, paths: impl IntoIterator<Item = &'a Path>) -> Relevance {
        paths.into_iter().map(|path| self.classify(path)).fold(Relevance::None, |a, b| match (a, b) {
            (Relevance::Rules, _) | (_, Relevance::Rules) => Relevance::Rules,
            (Relevance::Status, _) | (_, Relevance::Status) => Relevance::Status,
            _ => Relevance::None,
        })
    }
}

/// In this work tree's own git dir (`index`, `HEAD`, the state of a merge, rebase or cherry-pick).
fn own_file(rel: &Path) -> Relevance {
    let Some(Component::Normal(first)) = rel.components().next() else { return Relevance::None };
    match first.to_str() {
        Some(
            "index" | "HEAD" | "MERGE_HEAD" | "CHERRY_PICK_HEAD" | "REVERT_HEAD" | "BISECT_LOG"
            | "rebase-merge" | "rebase-apply" | "sequencer",
        ) => Relevance::Status,
        _ => Relevance::None,
    }
}

/// In the git dir all of a repository's work trees share (branches and tags, the upstream's refs, the
/// settings that name an upstream, the ignore rules).
fn shared_file(rel: &Path) -> Relevance {
    let mut parts = rel.components();
    let Some(Component::Normal(first)) = parts.next() else { return Relevance::None };
    match first.to_str() {
        Some("refs" | "packed-refs" | "config") => Relevance::Status,
        Some("info") if parts.next() == Some(Component::Normal("exclude".as_ref())) => Relevance::Rules,
        _ => Relevance::None,
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::*;

    fn ordinary() -> Filter {
        let git = PathBuf::from("/w/app/.git");
        Filter::new(
            Layout::new("/w/app".into(), git.clone(), git),
            Ignored::parse(b"target/\0web/node_modules/\0.env\0"),
        )
    }

    fn linked() -> Filter {
        Filter::new(
            Layout::new("/w/badge".into(), "/w/app/.git/worktrees/badge".into(), "/w/app/.git".into()),
            Ignored::default(),
        )
    }

    fn is(filter: &Filter, path: &str) -> Relevance {
        filter.classify(Path::new(path))
    }

    #[test]
    fn files_in_the_work_tree_matter_unless_ignored() {
        let f = ordinary();
        assert_eq!(is(&f, "/w/app/src/main.rs"), Relevance::Status);
        assert_eq!(is(&f, "/w/app/new-dir"), Relevance::Status);
        assert_eq!(is(&f, "/w/app/target/debug/app"), Relevance::None);
        assert_eq!(is(&f, "/w/app/web/node_modules/a/b.js"), Relevance::None);
        assert_eq!(is(&f, "/w/app/.env"), Relevance::None);
        assert_eq!(is(&f, "/w/app/web/src/a.js"), Relevance::Status);
    }

    #[test]
    fn what_decides_status_in_the_git_dir_matters() {
        let f = ordinary();
        for path in [
            "index",
            "HEAD",
            "MERGE_HEAD",
            "CHERRY_PICK_HEAD",
            "REVERT_HEAD",
            "rebase-merge/done",
            "rebase-apply/next",
            "sequencer/todo",
            "refs/heads/main",
            "refs/remotes/origin/feature/x",
            "refs/tags/v1",
            "packed-refs",
            "config",
        ] {
            assert_eq!(is(&f, &format!("/w/app/.git/{path}")), Relevance::Status, "{path}");
        }
    }

    #[test]
    fn objects_logs_hooks_and_locks_never_do() {
        let f = ordinary();
        for path in [
            "/w/app/.git/objects/ab/cdef",
            "/w/app/.git/objects/pack/pack-1.pack",
            "/w/app/.git/logs/HEAD",
            "/w/app/.git/hooks/pre-commit",
            "/w/app/.git/index.lock",
            "/w/app/.git/refs/heads/main.lock",
            "/w/app/.git/FETCH_HEAD",
            "/w/app/.git/COMMIT_EDITMSG",
            "/w/app/.git",
            "/w/app/src/a.rs.lock",
            "/w/app/Cargo.lock",
        ] {
            assert_eq!(is(&f, path), Relevance::None, "{path}");
        }
    }

    #[test]
    fn the_ignore_rules_changing_is_told_apart() {
        let f = ordinary();
        assert_eq!(is(&f, "/w/app/.gitignore"), Relevance::Rules);
        assert_eq!(is(&f, "/w/app/web/.gitignore"), Relevance::Rules);
        assert_eq!(is(&f, "/w/app/.git/info/exclude"), Relevance::Rules);
        assert_eq!(is(&f, "/w/app/.git/info/refs"), Relevance::None);
    }

    #[test]
    fn nested_git_dirs_and_other_places_are_not_this_repositorys() {
        let f = ordinary();
        assert_eq!(is(&f, "/w/app/vendor/lib/.git/index"), Relevance::None);
        assert_eq!(is(&f, "/w/other/file"), Relevance::None);
        assert_eq!(is(&f, "/w/app-two/file"), Relevance::None, "a name is not a prefix");
        assert_eq!(is(&f, "/"), Relevance::None);
    }

    #[test]
    fn a_linked_worktree_reads_its_own_index_and_the_shared_refs() {
        let f = linked();
        assert_eq!(is(&f, "/w/badge/src/a.rs"), Relevance::Status);
        assert_eq!(is(&f, "/w/app/.git/worktrees/badge/index"), Relevance::Status);
        assert_eq!(is(&f, "/w/app/.git/worktrees/badge/HEAD"), Relevance::Status);
        assert_eq!(is(&f, "/w/app/.git/refs/heads/badge"), Relevance::Status);
        assert_eq!(is(&f, "/w/app/.git/index"), Relevance::None, "the main work tree's index is not ours");
        assert_eq!(is(&f, "/w/app/.git/worktrees/other/index"), Relevance::None);
        assert_eq!(is(&f, "/w/app/.git/worktrees/badge/logs/HEAD"), Relevance::None);
    }

    #[test]
    fn a_batch_is_as_strong_as_its_strongest_path() {
        let f = ordinary();
        let paths = |list: &[&str]| list.iter().map(PathBuf::from).collect::<Vec<_>>();
        let strongest = |list: &[&str]| {
            let list = paths(list);
            f.classify_all(list.iter().map(PathBuf::as_path))
        };
        assert_eq!(strongest(&[]), Relevance::None);
        assert_eq!(strongest(&["/w/app/target/x", "/w/app/.git/objects/a"]), Relevance::None);
        assert_eq!(strongest(&["/w/app/target/x", "/w/app/a.rs"]), Relevance::Status);
        assert_eq!(strongest(&["/w/app/a.rs", "/w/app/.gitignore"]), Relevance::Rules);
    }
}
