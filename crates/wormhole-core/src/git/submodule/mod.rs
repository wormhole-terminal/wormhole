//! Submodules in the Changes and History lists.
//!
//! A submodule is a folder that is a repository of its own, which the parent records as one commit
//! id (mode 160000). Git's diff of it is two `Subproject commit` lines, which says nothing a person
//! wants to know; what they want is "2 new commits" and, on a click, which. `status`/`diff --raw` say
//! what the parent sees ([`Submodule`], parsed here); `log` asks the submodule's own repository
//! what lies between the two commits ([`SubmoduleLog`]), once per pair; `superproject` finds the
//! parent from inside a submodule without running git.

mod log;
mod superproject;

use std::sync::Arc;

use super::ChangeKind;
pub use log::{COMMITS_SHOWN, Checkout, Range, SubmoduleLog, annotate, annotate_files, submodule_log};
pub use superproject::{Superproject, superproject};

/// What the parent repository says about one submodule: which commits it records on the two sides
/// of the comparison, and what `git status` found inside the submodule's folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Submodule {
    /// The commit the base side records (HEAD, the merge-base, a commit's parent); `None` for a
    /// submodule that is new.
    pub old: Option<String>,
    /// The commit the other side records, when it is a revision, or the one a staged new
    /// submodule is at. `None`: the commit checked out in the folder now.
    pub new: Option<String>,
    /// The folder is at another commit than the base records.
    pub commit_changed: bool,
    /// The other side has no submodule here: it was removed (`git rm`), not just its folder.
    pub removed: bool,
    /// Tracked files in the submodule are modified (`git status`'s `M`).
    pub tracked: bool,
    /// The submodule has files git does not track (`U`).
    pub untracked: bool,
    /// The commits between `old` and `new`, once asked for (see [`annotate`]): the row says how
    /// many, and a click lists them.
    pub log: Option<Arc<SubmoduleLog>>,
}

/// The mode git gives a submodule's entry.
const GITLINK: &str = "160000";

impl Submodule {
    /// From a porcelain v2 entry: `sub` is its `S<c><m><u>` field (`N...` for anything that is not
    /// a submodule), `modes` the file modes it lists (a submodule that was removed has no flags
    /// but still has the mode), `head` and `index` the commit ids HEAD and the index record.
    pub(crate) fn from_status(
        sub: &str,
        modes: &[&str],
        head: &str,
        index: &str,
        kind: ChangeKind,
    ) -> Option<Self> {
        let Some(flags) = sub.strip_prefix('S') else {
            // Removed, or replaced by a file: the old mode is the only sign.
            return modes.contains(&GITLINK).then(|| Self {
                old: commit_id(head),
                new: None,
                commit_changed: false,
                removed: modes.get(1) == Some(&"000000"),
                tracked: false,
                untracked: false,
                log: None,
            });
        };
        let mut flags = flags.chars();
        let commit_changed = flags.next() == Some('C');
        let tracked = flags.next() == Some('M');
        let untracked = flags.next() == Some('U');
        let new = if kind == ChangeKind::Added { commit_id(index) } else { None };
        // The index's mode (second of a plain entry's three) says whether `git rm` was run.
        let removed = modes.len() == 3 && modes[1] == "000000";
        Some(Self { old: commit_id(head), new, commit_changed, removed, tracked, untracked, log: None })
    }

    /// From one `git diff --raw` entry's (mode, id) of each side. An id of zeros on the new side is
    /// what git prints when it did not look: the working tree, whose commit is the folder's.
    pub(crate) fn from_raw(old: (&str, &str), new: (&str, &str)) -> Option<Self> {
        if old.0 != GITLINK && new.0 != GITLINK {
            return None;
        }
        let new_mode = new.0;
        let side = |(mode, id): (&str, &str)| (mode == GITLINK).then(|| commit_id(id)).flatten();
        let (old, new) = (side(old), side(new));
        // Same commit on both sides and still listed: the folder has changes of its own.
        let tracked = old.is_some() && old == new;
        let removed = new_mode == "000000";
        Some(Self { commit_changed: old != new, removed, old, new, tracked, untracked: false, log: None })
    }

    /// What the row says in place of line counts, in the words of the module's docs: the commits
    /// that moved, then what is uncommitted, then what is untracked, with " · " between.
    pub fn label(&self, kind: ChangeKind) -> String {
        if kind == ChangeKind::Deleted {
            return if self.removed { "removed" } else { "folder missing" }.to_string();
        }
        let log = self.log.as_deref();
        match log.map(|log| log.checkout) {
            Some(Checkout::NotCheckedOut) => return "not checked out".to_string(),
            Some(Checkout::FolderMissing) => return "folder missing".to_string(),
            Some(Checkout::Ready) | None => {}
        }
        let mut parts: Vec<String> = Vec::new();
        if self.old.is_none() {
            let at = self.new.as_deref().or(log.and_then(|log| log.new.as_deref()));
            parts.push(match at {
                Some(id) => format!("added at {}", short(id)),
                None => "added".to_string(),
            });
        } else if self.commit_changed {
            parts.push(log.map_or_else(|| "new commits".to_string(), commits_phrase));
        }
        if self.tracked {
            parts.push("uncommitted changes".to_string());
        }
        if self.untracked {
            parts.push("untracked files".to_string());
        }
        if parts.is_empty() {
            parts.push("changed".to_string());
        }
        parts.join(" · ")
    }

    /// The sentence under the commits (or in their place) about what is in the submodule's folder
    /// and not in a commit: "The submodule also has uncommitted changes."
    pub fn inside_note(&self) -> Option<String> {
        let what = match (self.tracked, self.untracked) {
            (true, true) => "uncommitted changes and untracked files",
            (true, false) => "uncommitted changes",
            (false, true) => "untracked files",
            (false, false) => return None,
        };
        let also = if self.commit_changed { " also" } else { "" };
        Some(format!("The submodule{also} has {what}. Open a terminal in it to see them."))
    }

    /// Whether the folder is a repository that can be asked, as far as is known.
    pub fn checked_out(&self) -> bool {
        self.log.as_deref().is_none_or(|log| log.checkout == Checkout::Ready)
    }
}

impl SubmoduleLog {
    /// What the card says where there are no commits to list; `None` when there are some.
    pub fn nothing_to_list(&self) -> Option<String> {
        Some(match (self.checkout, &self.range) {
            (Checkout::NotCheckedOut, _) => {
                "This submodule isn't checked out, so there are no commits to list. \
                 Check it out to see them."
                    .to_string()
            }
            (Checkout::FolderMissing, _) => "The submodule's folder is missing.".to_string(),
            (Checkout::Ready, Range::NotFetched { .. }) => {
                "These commits aren't in the submodule's clone; fetch it to see them.".to_string()
            }
            (Checkout::Ready, Range::NoBase) => match &self.new {
                Some(id) => format!("A new submodule, at {}.", short(id)),
                None => "A new submodule.".to_string(),
            },
            (Checkout::Ready, Range::Same) => {
                "No new commits: the submodule is still at the commit the parent records.".to_string()
            }
            (Checkout::Ready, Range::Unavailable) => "The submodule has no commit yet.".to_string(),
            (Checkout::Ready, Range::Commits { .. }) => return None,
        })
    }
}

/// "2 new commits", "1 commit back" (the pointer went to an older one), "3 new, 1 dropped" for
/// a switch to another line of history, and "new commits" when they cannot be counted (they are
/// not in the submodule's clone).
fn commits_phrase(log: &SubmoduleLog) -> String {
    let plural = |n: usize| if n == 1 { "commit" } else { "commits" };
    match &log.range {
        Range::Commits { ahead_total: a, behind_total: b, .. } => match (*a, *b) {
            (0, 0) => "changed".to_string(),
            (a, 0) => format!("{a} new {}", plural(a)),
            (0, b) => format!("{b} {} back", plural(b)),
            (a, b) => format!("{a} new {}, {b} dropped", plural(a)),
        },
        Range::Same | Range::NoBase | Range::Unavailable | Range::NotFetched { .. } => {
            "new commits".to_string()
        }
    }
}

/// The first seven characters of a commit id, what git calls its short form.
pub fn short(id: &str) -> &str {
    id.get(..7).unwrap_or(id)
}

/// A commit id git printed, or `None` for the all-zero id it prints for "nothing here" (and for
/// anything that is not an id: it ends up on a command line).
pub(crate) fn commit_id(id: &str) -> Option<String> {
    let hex = matches!(id.len(), 40 | 64) && id.bytes().all(|b| b.is_ascii_hexdigit());
    (hex && id.bytes().any(|b| b != b'0')).then(|| id.to_string())
}

#[cfg(test)]
mod tests;
