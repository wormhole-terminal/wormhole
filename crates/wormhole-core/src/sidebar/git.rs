//! What the terminal list shows of git: a band's branch and changes, a row's own count.

/// What one session's latest `git status` says, as far as the session list shows it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionGit<'a> {
    /// `None` while HEAD is detached.
    pub branch: Option<&'a str>,
    /// Changed files.
    pub changes: usize,
    /// Counted against another branch ("Compare with") rather than HEAD, so
    /// `changes` is this session's own view, not the folder's uncommitted work.
    pub compared: bool,
}

impl SessionGit<'_> {
    /// The count a session's row keeps when its folder's header shows the rest:
    /// only one that means something different from the header's.
    pub fn own_changes(&self) -> Option<usize> {
        (self.compared && self.changes > 0).then_some(self.changes)
    }
}

/// Git facts about a folder, shown once on its header in the grouped list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GroupGit<'a> {
    /// `None` while HEAD is detached.
    pub branch: Option<&'a str>,
    /// Uncommitted changes; `None` when every session compares with another branch.
    pub changes: Option<usize>,
}

/// The branch and change count a folder's sessions share. A folder is one working tree
/// (its repository root, or a worktree's), so every session in it is on the same branch
/// with the same uncommitted changes; the statuses only disagree for a moment while
/// they refresh at different times. The active session's status is the freshest, so it
/// wins, then the sessions in list order. `members` are the group's sessions (a `Group`'s or a
/// band's), indices into `sessions`, which has `None` for sessions without a current status; a
/// group where no session has one (not a repository, or not looked at yet) gets `None`.
pub fn group_git<'a>(
    members: &[usize],
    sessions: &[Option<SessionGit<'a>>],
    active: usize,
) -> Option<GroupGit<'a>> {
    let known = || {
        let active = members.contains(&active).then_some(active);
        active.into_iter().chain(members.iter().copied()).filter_map(|ix| sessions.get(ix).copied().flatten())
    };
    let branch = known().next()?.branch;
    let changes = known().find(|session| !session.compared).map(|session| session.changes);
    Some(GroupGit { branch, changes })
}

#[cfg(test)]
mod tests {
    use super::{GroupGit, SessionGit, group_git};
    use crate::sidebar::group_by_folder;
    use std::path::PathBuf;

    fn dirs(paths: &[Option<&str>]) -> Vec<Option<PathBuf>> {
        paths.iter().map(|p| p.map(PathBuf::from)).collect()
    }

    fn session(branch: &str, changes: usize, compared: bool) -> Option<SessionGit<'_>> {
        Some(SessionGit { branch: Some(branch), changes, compared })
    }

    #[test]
    fn group_git_prefers_the_active_session() {
        let folders = dirs(&[Some("/w/a"), Some("/w/b"), Some("/w/a")]);
        let groups = group_by_folder(&folders, &[], None);
        // Session 0's status is older: it hasn't seen the checkout and the new file yet.
        let sessions = [session("main", 1, false), session("x", 9, false), session("feature", 2, false)];
        assert_eq!(
            group_git(&groups[0].sessions, &sessions, 2),
            Some(GroupGit { branch: Some("feature"), changes: Some(2) })
        );
        // Active elsewhere: the first session of the group speaks for it.
        assert_eq!(
            group_git(&groups[0].sessions, &sessions, 1),
            Some(GroupGit { branch: Some("main"), changes: Some(1) })
        );
    }

    #[test]
    fn group_changes_skip_sessions_comparing_with_a_branch() {
        let groups = group_by_folder(&dirs(&[Some("/w/a"), Some("/w/a")]), &[], None);
        let sessions = [session("feature", 12, true), session("feature", 3, false)];
        let git = group_git(&groups[0].sessions, &sessions, 0);
        assert_eq!(git, Some(GroupGit { branch: Some("feature"), changes: Some(3) }));
        // The comparing session keeps its own count on its row; the other one doesn't need to.
        assert_eq!(sessions.map(|c| c.and_then(|c| c.own_changes())), [Some(12), None]);

        let all_compared = [session("feature", 12, true), session("feature", 4, true)];
        assert_eq!(group_git(&groups[0].sessions, &all_compared, 0).and_then(|g| g.changes), None);
    }

    #[test]
    fn group_without_statuses_has_no_git() {
        let groups = group_by_folder(&dirs(&[Some("/w/plain"), Some("/w/a")]), &[], None);
        let sessions = [None, session("main", 0, false)];
        assert_eq!(group_git(&groups[0].sessions, &sessions, 0), None);
        // A session that just moved in and hasn't been looked at yet doesn't hide the others'.
        let groups = group_by_folder(&dirs(&[Some("/w/a"), Some("/w/a")]), &[], None);
        let detached = [None, Some(SessionGit { branch: None, changes: 0, compared: false })];
        assert_eq!(
            group_git(&groups[0].sessions, &detached, 0),
            Some(GroupGit { branch: None, changes: Some(0) })
        );
    }
}
