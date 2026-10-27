//! Where a group's new terminals start, the folder worth offering a group that has none, and which
//! terminals a group that keeps only the ones in its folder lets stay.
//!
//! A group's own folder comes first (`Groups::start_folder`). Without one, a new terminal starts where the
//! group was last used, which is the live folder of its most recently used member: the person is working
//! there. A group that has neither is asked about once (`folder_to_ask_about`), with a folder worked out
//! from where its terminals are (`suggest_folder`). The decisions take the facts they need as arguments:
//! the window gathers each member's [`Place`] and makes folders real ([`real_folder`]) first, so a path
//! that climbs with `..` or goes through a link is judged by where it ends up. The only disk access here
//! is that function, and `Groups::start_folder`'s look at the group's own folder.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use super::groups::Groups;
use super::split::SplitId;

/// Where one terminal is, as the window found it: its folder and the repository that is in, both made
/// real (`real_folder`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Place {
    pub folder: PathBuf,
    /// The root of the repository `folder` is in, if it is in one.
    pub repo: Option<PathBuf>,
    /// The main working tree of the repository `folder` is a linked worktree of, as its `.git` file names
    /// it (cleaned of `..` but not looked up on disk), if it is one.
    pub linked_to: Option<PathBuf>,
}

/// What a group that keeps only the terminals in its folder holds them to: the folder (made real) and the
/// repository it is in (the main working tree, when the folder is itself a linked worktree, so its sibling
/// worktrees count too). A linked worktree of that repository counts as inside, so an agent working in a
/// worktree of the group's repository stays in the group. It is the whole repository that is meant, even
/// when the folder is only a part of it (a package of a monorepo): its worktrees count as inside as well.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Fence {
    pub folder: PathBuf,
    pub repo: Option<PathBuf>,
}

impl Fence {
    /// The fence round `folder` (made real already); `repo_of` finds the repository a folder is in (its
    /// main working tree, for a linked worktree). That is a parent of `folder`, so already as real as it.
    pub fn new(folder: PathBuf, repo_of: impl Fn(&Path) -> Option<PathBuf>) -> Self {
        let repo = repo_of(&folder);
        Self { folder, repo }
    }

    /// Whether a terminal at `place` is inside it.
    pub fn holds(&self, place: &Place) -> bool {
        place.folder.starts_with(&self.folder)
            || self.repo.as_ref().is_some_and(|repo| place.linked_to.as_ref() == Some(repo))
    }
}

/// `path` as it really is: links followed and `..` resolved, and only when that is a directory whose name
/// can be stored as text (a folder is kept as a string; a lossy one would name another place). `None`
/// for anything else.
pub fn real_folder(path: &Path) -> Option<PathBuf> {
    let real = path.canonicalize().ok()?;
    (real.is_dir() && real.to_str().is_some()).then_some(real)
}

impl Groups {
    /// Group `id`'s person said no to the question about a folder: it is not asked again. Setting a
    /// folder takes this back (`set_folder`).
    pub fn decline_folder(&mut self, id: u32) {
        if let Some(group) = self.get_mut(id) {
            group.folder_declined = true;
        }
    }

    /// The folder group `id` keeps its terminals in, as the group has it (not made real), when it keeps
    /// only the terminals in it.
    pub fn fenced_folder(&self, id: u32) -> Option<&str> {
        let group = self.get(id)?;
        group.folder.as_deref().filter(|_| group.only_in_folder)
    }

    /// The fence of group `id`: [`fenced_folder`](Self::fenced_folder) made real, with the repository it is
    /// in (`repo_of` finds one). `None` for a group that doesn't keep only its folder's terminals, and for
    /// one whose folder has gone.
    pub fn fence(&self, id: u32, repo_of: impl Fn(&Path) -> Option<PathBuf>) -> Option<Fence> {
        let folder = real_folder(Path::new(self.fenced_folder(id)?))?;
        Some(Fence::new(folder, repo_of))
    }

    /// Turn on or off that group `id` keeps only the terminals in its folder. A group without a folder
    /// has none to keep them in, so it stays off.
    pub fn set_only_in_folder(&mut self, id: u32, on: bool) {
        if let Some(group) = self.get_mut(id) {
            group.only_in_folder = on && group.folder.is_some();
        }
    }

    /// Group `id`'s terminals, the one in use first: `selected` (the terminal the person is on) if it is in
    /// the group, then `last_selected` (the one of the group they were on last), then the rest, the one that
    /// joined last first.
    fn members_by_use(&self, id: u32, selected: Option<i64>, last_selected: Option<i64>) -> Vec<i64> {
        let Some(group) = self.get(id) else { return Vec::new() };
        let first = [selected, last_selected].into_iter().flatten();
        let mut ordered: Vec<i64> = Vec::with_capacity(group.members.len());
        for terminal in first.chain(group.members.iter().rev().copied()) {
            if group.members.contains(&terminal) && !ordered.contains(&terminal) {
                ordered.push(terminal);
            }
        }
        ordered
    }

    /// Where a new terminal in group `id` starts: its folder when it has one that is still a directory,
    /// else the folder of the terminal in use (`place_of` is asked in that order and stops at the first
    /// terminal that has a place). `None`: nothing to go by, and the terminal starts where it would have
    /// without a group.
    pub fn start_place(
        &self,
        id: u32,
        selected: Option<i64>,
        last_selected: Option<i64>,
        place_of: impl Fn(i64) -> Option<Place>,
    ) -> Option<PathBuf> {
        self.start_folder(id).map(PathBuf::from).or_else(|| {
            let used = self.members_by_use(id, selected, last_selected);
            used.into_iter().find_map(place_of).map(|place| place.folder)
        })
    }

    /// The folder to ask group `id`'s person about before a terminal opens in it, `None` when there is
    /// nothing to ask: the group has a folder, was asked and said no, is the Recovered group (a heap of
    /// strays, not a place), or nothing in it points to a folder worth offering. `home` is the home folder,
    /// made real.
    pub fn folder_to_ask_about(
        &self,
        id: u32,
        selected: Option<i64>,
        last_selected: Option<i64>,
        place_of: impl Fn(i64) -> Option<Place>,
        home: &Path,
    ) -> Option<PathBuf> {
        let group = self.get(id)?;
        if group.folder.is_some() || group.folder_declined || group.recovered {
            return None;
        }
        let used = self.members_by_use(id, selected, last_selected);
        let places: Vec<Place> = used.into_iter().filter_map(place_of).collect();
        suggest_folder(&places, home)
    }

    /// The terminals of group `id` that a group keeping only the terminals in `fence` lets go, for taking
    /// them out. A split stays whole: it counts as inside when any of its panes is, and when none is, all
    /// its panes go. A terminal whose place isn't known stays, and so does a split with one. `place_of` is a
    /// terminal's place, `split_of` its split.
    pub fn members_outside(
        &self,
        id: u32,
        fence: &Fence,
        place_of: impl Fn(i64) -> Option<Place>,
        split_of: impl Fn(i64) -> Option<SplitId>,
    ) -> Vec<i64> {
        let Some(group) = self.get(id) else { return Vec::new() };
        // Each unit: a split's panes in the group, or a terminal on its own.
        let mut units: Vec<Vec<i64>> = Vec::new();
        let mut of_split: HashMap<SplitId, usize> = HashMap::new();
        for &member in &group.members {
            match split_of(member) {
                Some(split) => match of_split.get(&split) {
                    Some(&unit) => units[unit].push(member),
                    None => {
                        of_split.insert(split, units.len());
                        units.push(vec![member]);
                    }
                },
                None => units.push(vec![member]),
            }
        }
        let leaves = |unit: &&Vec<i64>| !keeps(fence, unit.iter().map(|&member| place_of(member)));
        units.iter().filter(leaves).flatten().copied().collect()
    }
}

/// Whether terminals that move together (a terminal and the rest of its split) may stay in, or enter, a
/// group keeping only the terminals in `fence`: any of them inside it (`Fence::holds`), or at a place not
/// known, lets them. `panes` are their places.
pub fn keeps(fence: &Fence, panes: impl IntoIterator<Item = Option<Place>>) -> bool {
    panes.into_iter().any(|pane| pane.is_none_or(|place| fence.holds(&place)))
}

/// Where a terminal counts when the place with most of them is looked for: its repository, so one repository
/// is one place however deep they are in it, or its folder when it is in none or its repository is the home
/// folder or above (a repository of dotfiles in `~` is not a place for a group).
#[derive(Clone, Debug, PartialEq, Eq)]
enum Anchor {
    Repo(PathBuf),
    Folder(PathBuf),
}

/// The folder worth offering for terminals at `places` (the one in use first). One repository holding all
/// of them: its root. None of them in a repository: the deepest folder that holds them all, when that is
/// acceptable. Otherwise (several repositories, or some in one and some in none, or the shared folder no
/// good) the place that holds the most of them, the one in use winning only a tie: the person's last
/// terminal doesn't outvote the rest. A repository's root is offered whatever its depth; a folder that is
/// not one must not be the home folder or above it, nor a top-level system folder such as `/private/tmp` or
/// `/Volumes/disk`, which two unrelated folders under it would share by chance. `None` when nothing
/// qualifies.
fn suggest_folder(places: &[Place], home: &Path) -> Option<PathBuf> {
    places.first()?;
    let outside_home = |folder: &Path| !home.starts_with(folder);
    let top_level = |folder: &Path| {
        folder.components().filter(|part| matches!(part, std::path::Component::Normal(_))).count() <= 2
    };
    let usable_folder = |folder: &Path| outside_home(folder) && !top_level(folder);
    let anchors: Vec<Anchor> = places
        .iter()
        .map(|place| match &place.repo {
            Some(repo) if outside_home(repo) => Anchor::Repo(repo.clone()),
            _ => Anchor::Folder(place.folder.clone()),
        })
        .collect();
    if let Some(Anchor::Repo(root)) = anchors.first()
        && anchors.iter().all(|anchor| anchor == &anchors[0])
    {
        return Some(root.clone());
    }
    if anchors.iter().all(|anchor| matches!(anchor, Anchor::Folder(_))) {
        let folders: Vec<&Path> = places.iter().map(|place| place.folder.as_path()).collect();
        if let Some(shared) = common_ancestor(&folders).filter(|shared| usable_folder(shared)) {
            return Some(shared);
        }
    }
    // Each place with how many terminals are in it, in the order they were first seen (the one in use first).
    let mut counted: Vec<(Anchor, usize)> = Vec::new();
    for anchor in anchors {
        match counted.iter_mut().find(|(seen, _)| *seen == anchor) {
            Some((_, count)) => *count += 1,
            None => counted.push((anchor, 1)),
        }
    }
    // A stable sort keeps the first seen first among equals.
    counted.sort_by_key(|&(_, count)| std::cmp::Reverse(count));
    counted.into_iter().find_map(|(anchor, _)| match anchor {
        Anchor::Repo(root) => Some(root),
        Anchor::Folder(folder) => usable_folder(&folder).then_some(folder),
    })
}

/// The deepest folder every one of `folders` is in (or is), `None` if they share none.
fn common_ancestor(folders: &[&Path]) -> Option<PathBuf> {
    let (first, rest) = folders.split_first()?;
    let mut common: PathBuf = first.to_path_buf();
    for folder in rest {
        while !folder.starts_with(&common) {
            if !common.pop() {
                return None;
            }
        }
    }
    Some(common)
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use super::{Fence, Place, common_ancestor, keeps, real_folder, suggest_folder};
    use crate::view::sidebar::Groups;

    fn p(path: &str) -> PathBuf {
        PathBuf::from(path)
    }

    fn at(folder: &str, repo: Option<&str>) -> Place {
        Place { folder: p(folder), repo: repo.map(p), linked_to: None }
    }

    #[test]
    fn terminals_in_one_repository_suggest_its_root_not_their_common_folder() {
        let home = p("/Users/me");
        let places = [
            at("/Users/me/work/repo/crates/a", Some("/Users/me/work/repo")),
            at("/Users/me/work/repo/crates/a/src", Some("/Users/me/work/repo")),
        ];
        assert_eq!(suggest_folder(&places, &home), Some(p("/Users/me/work/repo")));
    }

    #[test]
    fn terminals_in_different_places_suggest_the_deepest_folder_holding_them_all() {
        let home = p("/Users/me");
        let places = [at("/Users/me/work/a/src", None), at("/Users/me/work/b", None)];
        assert_eq!(suggest_folder(&places, &home), Some(p("/Users/me/work")));
        assert_eq!(suggest_folder(&places[..1], &home), Some(p("/Users/me/work/a/src")));
    }

    #[test]
    fn the_home_folder_and_everything_above_it_is_never_offered() {
        let home = p("/Users/me");
        // Their common folder is home: the one in use stands in, by its repository first.
        let places =
            [at("/Users/me/work/repo/src", Some("/Users/me/work/repo")), at("/Users/me/notes", None)];
        assert_eq!(suggest_folder(&places, &home), Some(p("/Users/me/work/repo")));
        let places = [at("/Users/me/notes", None), at("/Users/me/work/a", None)];
        assert_eq!(suggest_folder(&places, &home), Some(p("/Users/me/notes")));
        // Across homes, the common folder is `/Users`.
        let places = [at("/Users/me/notes", None), at("/Users/other/x", None)];
        assert_eq!(suggest_folder(&places, &home), Some(p("/Users/me/notes")));
        // Only home to go by: nothing.
        assert_eq!(suggest_folder(&[at("/Users/me", None)], &home), None);
        assert_eq!(suggest_folder(&[at("/", None)], &home), None);
        // A repository that is home itself isn't offered; the folder in it is.
        assert_eq!(
            suggest_folder(&[at("/Users/me/dots", Some("/Users/me"))], &home),
            Some(p("/Users/me/dots"))
        );
        assert_eq!(suggest_folder(&[], &home), None, "no terminals, no suggestion");
    }

    #[test]
    fn the_place_with_most_of_the_terminals_wins_over_the_one_in_use() {
        let home = p("/Users/me");
        let repo = "/Users/me/work/repo";
        let in_repo = [
            at(repo, Some(repo)),
            at("/Users/me/work/repo/src", Some(repo)),
            at("/Users/me/work/repo/docs", Some(repo)),
        ];
        // Three in a repository and the one in use elsewhere: their common folder is home, so the
        // repository, which holds three of the four, is offered, not the folder of the last terminal.
        let mut places = vec![at("/Users/me/notes", None)];
        places.extend(in_repo.clone());
        assert_eq!(suggest_folder(&places, &home), Some(p(repo)));
        // A tie goes to the place of the one in use.
        let tie = [
            at("/Users/me/a", None),
            at("/Users/me/b", None),
            at("/Users/me/b", None),
            at("/Users/me/a", None),
        ];
        assert_eq!(suggest_folder(&tie, &home), Some(p("/Users/me/a")), "two each, the one in use is in a");
        let tie = [
            at("/Users/me/b", None),
            at("/Users/me/a", None),
            at("/Users/me/a", None),
            at("/Users/me/b", None),
        ];
        assert_eq!(suggest_folder(&tie, &home), Some(p("/Users/me/b")));
    }

    #[test]
    fn a_top_level_system_folder_is_not_a_place_two_folders_share_by_chance() {
        let home = p("/Users/me");
        let places = [at("/private/tmp/a", None), at("/private/tmp/b", None)];
        assert_eq!(suggest_folder(&places, &home), Some(p("/private/tmp/a")), "not /private/tmp");
        assert_eq!(suggest_folder(&[at("/Volumes/disk", None)], &home), None);
        assert_eq!(suggest_folder(&[at("/private/tmp/a/b", None)], &home), Some(p("/private/tmp/a/b")));
        // A folder outside home is not above it.
        assert_eq!(
            suggest_folder(&[at("/opt/work/app", None), at("/opt/work/app/src", None)], &home),
            Some(p("/opt/work/app"))
        );
    }

    #[test]
    fn a_folder_outside_home_is_fine() {
        let home = p("/Users/me");
        assert_eq!(suggest_folder(&[at("/opt/app/src", None)], &home), Some(p("/opt/app/src")));
        assert_eq!(common_ancestor(&[Path::new("/a/b"), Path::new("/c")]), Some(p("/")));
        assert_eq!(common_ancestor(&[]), None);
    }

    #[test]
    fn a_folder_is_judged_by_where_it_ends_up() {
        let dir = tempfile::tempdir().expect("temp dir");
        let real = real_folder(dir.path()).expect("a real folder");
        let (app, other) = (real.join("app"), real.join("other"));
        std::fs::create_dir_all(app.join("src")).expect("dirs");
        std::fs::create_dir_all(&other).expect("dirs");
        // Climbing out of a folder with `..` leads somewhere else; reaching it by a path with `..` in it
        // leads there.
        assert_eq!(real_folder(&app.join("../other")), Some(other.clone()));
        assert_eq!(real_folder(&app.join("src/..")), Some(app.clone()));
        let beyond = dir.path().join("app/../other");
        assert!(
            !keeps(&fence_at(&app), [real_folder(&beyond).map(|f| here(&f))]),
            "`app/../other` is not under `app`"
        );
        // The home folder reached by climbing is still the home folder.
        let home = real.join("home");
        std::fs::create_dir_all(home.join("x")).expect("dirs");
        let climbed = real_folder(&home.join("x/..")).expect("real");
        assert_eq!(suggest_folder(&[Place { folder: climbed, repo: None, linked_to: None }], &home), None);
        // A temp dir is behind a link on macOS: its lexical path and its real one are the same folder.
        assert_eq!(real_folder(&dir.path().join("app")), Some(app));
        // Missing folders and files are no folder.
        assert_eq!(real_folder(&real.join("gone")), None);
        std::fs::write(real.join("a-file"), "x").expect("write");
        assert_eq!(real_folder(&real.join("a-file")), None);
    }

    fn fence_at(folder: &Path) -> Fence {
        Fence { folder: folder.to_path_buf(), repo: None }
    }

    #[test]
    fn a_split_stays_while_any_pane_is_inside_and_unknown_ones_stay() {
        let fence = fence_at(Path::new("/w/app"));
        let pane = |folder: &str| Some(here(Path::new(folder)));
        assert!(keeps(&fence, [pane("/w/app/src")]));
        assert!(keeps(&fence, [pane("/w/app")]));
        assert!(!keeps(&fence, [pane("/w/other")]));
        assert!(!keeps(&fence, [pane("/w/app-two")]), "whole names only");
        assert!(keeps(&fence, [pane("/w/other"), pane("/w/app")]), "a mixed split stays");
        assert!(!keeps(&fence, [pane("/w/other"), pane("/w/elsewhere")]));
        assert!(keeps(&fence, [None]), "unknown");
        assert!(keeps(&fence, [pane("/w/other"), None]), "a split with an unknown pane stays");
    }

    #[test]
    fn a_linked_worktree_of_the_groups_repository_is_inside_and_another_repositorys_is_not() {
        let fence = Fence { folder: p("/w/app"), repo: Some(p("/w/app")) };
        let worktree = |of: &str| Place { folder: p("/w/app-feature"), repo: None, linked_to: Some(p(of)) };
        assert!(keeps(&fence, [Some(worktree("/w/app"))]), "a worktree of the repository");
        assert!(!keeps(&fence, [Some(worktree("/w/other"))]), "a worktree of another one");
        // A fence in no repository has no worktrees.
        let loose = Fence { folder: p("/w/app"), repo: None };
        assert!(!keeps(&loose, [Some(worktree("/w/app"))]));
    }

    /// A group of the terminals `ids`.
    fn group_of(ids: &[i64]) -> (Groups, u32) {
        let mut groups = Groups::default();
        let id = groups.create(ids);
        (groups, id)
    }

    fn here(folder: &Path) -> Place {
        Place { folder: folder.to_path_buf(), repo: None, linked_to: None }
    }

    #[test]
    fn a_new_terminal_starts_in_the_folder_then_where_the_group_was_last_used_then_nowhere() {
        let a = p("/a");
        let b = p("/b");
        let place = |id: i64| match id {
            1 => Some(here(&a)),
            2 => Some(here(&b)),
            _ => None,
        };
        let (mut groups, group) = group_of(&[1, 2]);
        let start = |groups: &Groups, selected, last| groups.start_place(group, selected, last, place);
        // Nothing used yet: the member that joined last.
        assert_eq!(start(&groups, None, None), Some(b.clone()));
        assert_eq!(start(&groups, None, Some(1)), Some(a.clone()), "the one used last");
        assert_eq!(
            start(&groups, Some(2), Some(1)),
            Some(b.clone()),
            "the selected terminal is the one in use"
        );
        assert_eq!(start(&groups, Some(9), Some(1)), Some(a.clone()), "a terminal of another group is not");
        // A member with no place is passed over, and the walk stops at the first that has one.
        let asked = std::cell::RefCell::new(Vec::new());
        let found = groups.start_place(group, None, Some(1), |id| {
            asked.borrow_mut().push(id);
            place(id)
        });
        assert_eq!((found, asked.into_inner()), (Some(a.clone()), vec![1]), "no need to look at the rest");
        assert_eq!(groups.start_place(group, None, None, |_| None), None);
        assert_eq!(groups.start_place(99, None, None, place), None, "no such group");
        // A folder of the group's own comes first, but only while it is a directory.
        let dir = tempfile::tempdir().expect("temp dir");
        groups.set_folder(group, Some(&dir.path().to_string_lossy()));
        assert_eq!(start(&groups, Some(2), None), Some(dir.path().to_path_buf()));
        drop(dir);
        assert_eq!(start(&groups, Some(2), None), Some(b), "gone, so the members' places stand in");
    }

    #[test]
    fn a_group_is_asked_about_once_and_only_when_there_is_something_to_offer() {
        let home = p("/home/me");
        let app = p("/home/me/app");
        let place = |id: i64| Some(if id == 1 { here(&app.join("src")) } else { here(&app) });
        let (mut groups, group) = group_of(&[1, 2]);
        let ask = |groups: &Groups| groups.folder_to_ask_about(group, None, None, place, &home);
        assert_eq!(ask(&groups), Some(app.clone()));
        groups.decline_folder(group);
        assert_eq!(ask(&groups), None, "declined");
        // A folder set later takes the no back, and a cleared one is not asked about either.
        groups.set_folder(group, Some("/home/me/app"));
        assert_eq!(ask(&groups), None, "it has one");
        assert!(groups.get(group).is_some_and(|g| !g.folder_declined), "setting one takes the no back");
        groups.set_folder(group, None);
        assert_eq!(ask(&groups), None, "taking the folder away was a no too");
        // Terminals that are all in the home folder have nothing to offer.
        let (home_group, id) = group_of(&[1]);
        assert_eq!(home_group.folder_to_ask_about(id, None, None, |_| Some(here(&home)), &home), None);
        // The Recovered group isn't a place.
        let mut recovered = Groups::default();
        let id = recovered.recover(&[1, 2]).expect("made");
        assert_eq!(recovered.folder_to_ask_about(id, None, None, place, &home), None);
    }

    #[test]
    fn the_fence_needs_a_folder_that_is_there_and_goes_with_it() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut groups, group) = group_of(&[1]);
        groups.set_only_in_folder(group, true);
        assert!(groups.get(group).is_some_and(|g| !g.only_in_folder), "no folder to keep them in");
        groups.set_folder(group, Some(&dir.path().to_string_lossy()));
        assert_eq!(groups.fence(group, |_| None), None, "off until it is turned on");
        groups.set_only_in_folder(group, true);
        assert_eq!(
            groups.fence(group, |_| None),
            real_folder(dir.path()).map(|folder| Fence { folder, repo: None })
        );
        groups.set_folder(group, None);
        assert!(groups.get(group).is_some_and(|g| !g.only_in_folder), "clearing the folder turns it off");
        groups.set_folder(group, Some(&dir.path().to_string_lossy()));
        groups.set_only_in_folder(group, true);
        drop(dir);
        assert_eq!(groups.fence(group, |_| None), None, "a folder that has gone keeps no one out");
    }

    #[test]
    fn members_in_other_folders_are_the_ones_outside_and_splits_leave_whole_or_not_at_all() {
        let fence = fence_at(Path::new("/w/app"));
        let folder = |id: i64| match id {
            1 => Some(p("/w/app")),
            2 => Some(p("/w/app/src")),
            3 => Some(p("/w/other")),
            // 5 and 6 are one split with a pane inside; 7 and 8 one split with none.
            5 => Some(p("/w/other")),
            6 => Some(p("/w/app")),
            7 | 8 => Some(p("/w/elsewhere")),
            _ => None,
        };
        let split = |id: i64| match id {
            5 | 6 => Some(1),
            7 | 8 => Some(2),
            _ => None,
        };
        let place = |id: i64| folder(id).map(|folder| Place { folder, repo: None, linked_to: None });
        let (groups, group) = group_of(&[1, 2, 3, 4, 5, 6, 7, 8]);
        // 4 has no known folder and stays; the mixed split 5, 6 stays; the split 7, 8 goes together.
        assert_eq!(groups.members_outside(group, &fence, place, split), [3, 7, 8]);
        assert_eq!(groups.members_outside(99, &fence, place, split), Vec::<i64>::new());
    }

    #[test]
    fn one_repository_is_its_root_whatever_its_depth_and_several_are_the_biggest_of_them() {
        let home = p("/Users/me");
        // Three in `a` and one in `b`: not their shared parent, `a`.
        let (a, b) = ("/Users/me/Projects/a", "/Users/me/Projects/b");
        let places =
            [at(b, Some(b)), at(a, Some(a)), at("/Users/me/Projects/a/src", Some(a)), at(a, Some(a))];
        assert_eq!(suggest_folder(&places, &home), Some(p(a)));
        // Two and two: the one in use.
        let places = [at(b, Some(b)), at(a, Some(a)), at(b, Some(b)), at(a, Some(a))];
        assert_eq!(suggest_folder(&places, &home), Some(p(b)));
        // A repository at the top of the disk is still a repository.
        assert_eq!(suggest_folder(&[at("/opt/app/src", Some("/opt/app"))], &home), Some(p("/opt/app")));
        assert_eq!(
            suggest_folder(&[at("/Volumes/Disk", Some("/Volumes/Disk"))], &home),
            Some(p("/Volumes/Disk"))
        );
        // Terminals in one repository and one in no repository: the repository, which holds most.
        let places = [at("/Users/me/notes", None), at(a, Some(a)), at("/Users/me/Projects/a/src", Some(a))];
        assert_eq!(suggest_folder(&places, &home), Some(p(a)));
        // A repository of dotfiles in home isn't a place: its terminals count by their folders.
        let places = [at("/Users/me/dots", Some("/Users/me")), at("/Users/me/dots/x", Some("/Users/me"))];
        assert_eq!(suggest_folder(&places, &home), Some(p("/Users/me/dots")));
        let places = [
            at("/Users/me/dots", Some("/Users/me")),
            at(a, Some(a)),
            at("/Users/me/dots/x", Some("/Users/me")),
        ];
        assert_eq!(
            suggest_folder(&places, &home),
            Some(p("/Users/me/dots")),
            "two terminals there, one in `a`"
        );
    }

    #[test]
    fn a_group_that_keeps_only_its_folder_stays_when_it_is_emptied() {
        let (mut groups, group) = group_of(&[1]);
        groups.set_folder(group, Some("/tmp/work"));
        groups.set_pinned(group, false);
        groups.set_only_in_folder(group, true);
        groups.leave(1);
        assert!(groups.get(group).is_some(), "left empty, it stays: it is a place");
        groups.set_only_in_folder(group, false);
        assert!(groups.get(group).is_some(), "set_only_in_folder doesn't end it");
        groups.retain_terminals(|_| false);
        assert!(groups.get(group).is_none(), "off and unpinned, it goes as any group does");
    }
}
