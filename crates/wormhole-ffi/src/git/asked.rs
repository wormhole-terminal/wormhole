//! Small questions the History tab asks beside its reads: whether a commit is on the branch shown, which of
//! a commit's files are in the working tree now, and when the ages in the log next read differently. The
//! rules are core's; a path a repository names is hostile input and is resolved inside the root only.

use std::time::{SystemTime, UNIX_EPOCH};

use wormhole_core::git;
use wormhole_core::storage::files::resolve_within;
use wormhole_core::view::panel::next_age_change;

use super::records::{GitRev, GitRevKind};
use super::service::{Git, root_of};

/// How many of a commit's files are looked for in the working tree; a commit that touches more shows no
/// open icon on the rest (GPUI's `PRESENT_CHECKED`).
const PRESENT_CHECKED: usize = 2000;

#[uniffi::export]
impl Git {
    /// Whether commit `sha` is on `rev` (`git merge-base --is-ancestor`): the palette's commit keeps the
    /// History tab's branch when this says so. The stashes are a list of their own, so never; false too when
    /// git cannot say.
    pub fn is_on(&self, folder: String, sha: String, rev: GitRev) -> bool {
        if rev.kind == GitRevKind::Stashes {
            return false;
        }
        let Ok(root) = root_of(&folder) else { return false };
        git::is_ancestor(&root, &sha, &rev.name).unwrap_or(false)
    }

    /// The ones of `paths` (relative to the repository) that are regular files inside it now: a file the
    /// commit deleted, one removed since, or a link that leads out of the repository is not.
    pub fn present_files(&self, folder: String, paths: Vec<String>) -> Vec<String> {
        let Ok(root) = root_of(&folder) else { return Vec::new() };
        paths.into_iter().take(PRESENT_CHECKED).filter(|path| resolve_within(&root, path).is_ok()).collect()
    }
}

/// Milliseconds until the earliest of `times` (Unix seconds) has an age that reads differently ("12m" to
/// "13m"); `None` for no times. The list arms a one-shot timer with it.
#[uniffi::export]
pub fn git_next_age_change(times: Vec<i64>) -> Option<u64> {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    next_age_change(times, now).map(|wait| u64::try_from(wait.as_millis()).unwrap_or(u64::MAX))
}
