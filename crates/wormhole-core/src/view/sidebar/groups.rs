//! The groups a person makes in the terminal list: a name, a colour and the terminals in it, the
//! way a browser groups its tabs. A group isn't worked out from where its terminals are: it holds
//! whichever terminals were put in it, from any folder, and a terminal in no group stands in the
//! list on its own. A group goes with its last terminal unless it is pinned, and may have a folder
//! its new terminals start in. A window keeps its own groups, saved with it.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::group_color::{GroupColor, GroupPaint};

/// One group of terminals.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TermGroup {
    /// Unique among the window's groups, and never reused while the window lives: what a folded
    /// group and a drag remember it by.
    pub id: u32,
    pub name: String,
    /// A [`GroupColor`]'s name (`blue`, `cyan` …), `#rrggbb` for a colour of the person's own, or
    /// empty for none; a name this build doesn't know is kept and draws nothing.
    pub color: String,
    /// The terminals in it by their id, in the order they joined.
    pub members: Vec<i64>,
    /// It stays, as one line, when its last terminal closes.
    pub pinned: bool,
    /// The window's Recovered group: the terminals of session hosts that no saved row named when
    /// the app started (`hosts::LaunchHosts`), adopted rather than left running unseen. Otherwise
    /// a group like any other, which the person may rename, recolour or take apart.
    #[serde(skip_serializing_if = "is_false")]
    pub recovered: bool,
    /// Where a new terminal in the group starts, once the person has set one (never chosen for
    /// them). Saved with the group in the window's layout, so a layout from before it loads with none.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    /// The person was asked whether new terminals should start in a folder and said no (or took the
    /// folder away): not asked again. Setting a folder takes it back. Saved with the group, so a layout
    /// from before it loads as not asked.
    #[serde(skip_serializing_if = "is_false")]
    pub folder_declined: bool,
    /// Only the terminals in the folder stay in the group: one that goes to a folder outside it leaves, and
    /// none from outside can be put in (a split counts as inside when any pane is). Needs a folder, and
    /// clearing the folder turns it off. Like a pinned group, it stays when its last terminal goes, so moving
    /// terminals out never ends it. Saved with the group, so a layout from before it loads with it off.
    #[serde(skip_serializing_if = "is_false")]
    pub only_in_folder: bool,
}

fn is_false(value: &bool) -> bool {
    !value
}

/// What the Recovered group is called when it is made.
pub const RECOVERED_NAME: &str = "Recovered";

impl TermGroup {
    /// The named colour it has, none for a colour of its own.
    pub fn color(&self) -> Option<GroupColor> {
        GroupColor::from_key(&self.color)
    }

    /// What it is painted with: a name or a colour of its own.
    pub fn paint(&self) -> Option<GroupPaint> {
        GroupPaint::from_key(&self.color)
    }
}

/// A window's groups, in the order they are listed.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Groups(pub Vec<TermGroup>);

impl Groups {
    pub fn iter(&self) -> impl Iterator<Item = &TermGroup> {
        self.0.iter()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn get(&self, id: u32) -> Option<&TermGroup> {
        self.0.iter().find(|group| group.id == id)
    }

    pub(super) fn get_mut(&mut self, id: u32) -> Option<&mut TermGroup> {
        self.0.iter_mut().find(|group| group.id == id)
    }

    /// The group `terminal` is in.
    pub fn group_of(&self, terminal: i64) -> Option<u32> {
        self.0.iter().find(|group| group.members.contains(&terminal)).map(|group| group.id)
    }

    /// A new group holding `members` (taken out of the groups they were in), at the end of the
    /// list: "Group 1", "Group 2" … and the next colour in turn. Returns its id.
    pub fn create(&mut self, members: &[i64]) -> u32 {
        let id = self.0.iter().map(|group| group.id + 1).max().unwrap_or(1);
        let name = (1..).map(|n| format!("Group {n}")).find(|name| self.0.iter().all(|g| &g.name != name));
        let color = GroupColor::IN_TURN[self.0.len() % GroupColor::IN_TURN.len()].key().to_string();
        for &terminal in members {
            self.detach(terminal);
        }
        self.drop_empty();
        self.0.push(TermGroup {
            id,
            name: name.unwrap_or_default(),
            color,
            members: members.to_vec(),
            pinned: false,
            recovered: false,
            folder: None,
            folder_declined: false,
            only_in_folder: false,
        });
        id
    }

    /// Put `members` (terminals adopted from orphaned session hosts) in the window's Recovered
    /// group, after the ones it holds, making the group at the end of the list when there is none.
    /// Its id, `None` when there was nothing to put in it.
    pub fn recover(&mut self, members: &[i64]) -> Option<u32> {
        if members.is_empty() {
            return None;
        }
        let id = match self.0.iter().find(|group| group.recovered) {
            Some(group) => group.id,
            None => {
                let id = self.create(&[]);
                if let Some(group) = self.get_mut(id) {
                    group.name = RECOVERED_NAME.to_string();
                    group.color = GroupColor::Grey.key().to_string();
                    group.recovered = true;
                }
                id
            }
        };
        for &terminal in members {
            self.join(id, terminal);
        }
        Some(id)
    }

    /// Move `terminal` into group `id` (out of the one it was in). `false` if there is no such
    /// group.
    pub fn join(&mut self, id: u32, terminal: i64) -> bool {
        if self.get(id).is_none() {
            return false;
        }
        self.detach(terminal);
        if let Some(group) = self.get_mut(id) {
            group.members.push(terminal);
        }
        self.drop_empty();
        true
    }

    /// Take `terminal` out of its group. A group left with no terminal goes, unless it is pinned.
    pub fn leave(&mut self, terminal: i64) {
        self.detach(terminal);
        self.drop_empty();
    }

    fn detach(&mut self, terminal: i64) {
        for group in &mut self.0 {
            group.members.retain(|&member| member != terminal);
        }
    }

    fn drop_empty(&mut self) {
        self.0.retain(|group| group.pinned || group.only_in_folder || !group.members.is_empty());
    }

    /// Rename group `id`; a blank name keeps the old one.
    pub fn rename(&mut self, id: u32, name: &str) {
        let name = name.trim();
        if let Some(group) = self.get_mut(id).filter(|_| !name.is_empty()) {
            group.name = name.to_string();
        }
    }

    pub fn set_color(&mut self, id: u32, color: Option<GroupColor>) {
        if let Some(group) = self.get_mut(id) {
            group.color = color.map_or("", GroupColor::key).to_string();
        }
    }

    /// Give group `id` a colour of its own (sRGB), in the place of its name.
    pub fn set_custom_color(&mut self, id: u32, rgb: [u8; 3]) {
        if let Some(group) = self.get_mut(id) {
            group.color = GroupPaint::Custom(rgb).key();
        }
    }

    /// Set the folder new terminals in group `id` start in, or take it away (`None`, or a blank
    /// path). A group with a folder is pinned, as a project is: it is a place, so it stays when its
    /// last terminal closes. Taking the folder away leaves it pinned (unpinning is its own choice) and
    /// counts as saying no to having one: the group is not asked about a folder again. A folder set
    /// takes any earlier no back.
    pub fn set_folder(&mut self, id: u32, folder: Option<&str>) {
        let folder = folder.map(str::trim).filter(|folder| !folder.is_empty());
        if let Some(group) = self.get_mut(id) {
            if folder.is_some() {
                group.folder_declined = false;
            } else {
                group.only_in_folder = false;
                group.folder_declined |= group.folder.is_some();
            }
            group.folder = folder.map(str::to_string);
            group.pinned |= folder.is_some();
        }
    }

    /// The folder group `id` itself has, when that is still a directory (`start_place` goes on to
    /// where the group was last used when it isn't).
    pub fn start_folder(&self, id: u32) -> Option<String> {
        let folder = self.get(id)?.folder.as_deref()?;
        Path::new(folder).is_dir().then(|| folder.to_string())
    }

    /// Pin group `id`, or let it go again.
    pub fn set_pinned(&mut self, id: u32, pinned: bool) {
        if let Some(group) = self.get_mut(id) {
            group.pinned = pinned;
        }
        self.drop_empty();
    }

    /// Remove group `id`; its terminals stay, loose.
    pub fn remove(&mut self, id: u32) {
        self.0.retain(|group| group.id != id);
    }

    /// Forget the terminals that are gone (closed while the window was open, or not restored):
    /// `alive` says which ids still exist. A group that has none left goes with them, unless it is
    /// pinned.
    pub fn retain_terminals(&mut self, alive: impl Fn(i64) -> bool) {
        for group in &mut self.0 {
            group.members.retain(|&member| alive(member));
        }
        self.drop_empty();
    }
}

/// How a folded group is remembered (`WindowSidebar::folded`).
pub fn group_key(id: u32) -> String {
    format!("group:{id}")
}

#[cfg(test)]
mod tests {
    use super::{GroupColor, GroupPaint, Groups, RECOVERED_NAME, group_key};

    #[test]
    fn a_colour_of_its_own_is_saved_with_the_group_and_a_name_replaces_it() {
        let mut groups = Groups::default();
        let id = groups.create(&[1]);
        groups.set_custom_color(id, [0x33, 0x99, 0xcc]);
        let saved = serde_json::to_string(&groups).expect("serializes");
        assert!(saved.contains("#3399cc"), "{saved}");
        let back: Groups = serde_json::from_str(&saved).expect("reads back");
        assert_eq!(back.get(id).and_then(|g| g.paint()), Some(GroupPaint::Custom([0x33, 0x99, 0xcc])));
        assert_eq!(back.get(id).and_then(|g| g.color()), None, "it is no named colour");
        groups.set_color(id, Some(GroupColor::Red));
        assert_eq!(groups.get(id).and_then(|g| g.paint()), Some(GroupPaint::Named(GroupColor::Red)));
    }

    #[test]
    fn a_new_group_takes_its_members_from_the_groups_they_were_in() {
        let mut groups = Groups::default();
        let a = groups.create(&[1, 2]);
        let b = groups.create(&[2, 3]);
        assert_ne!(a, b);
        assert_eq!(groups.get(a).map(|g| g.members.clone()), Some(vec![1]));
        assert_eq!(groups.get(b).map(|g| g.members.clone()), Some(vec![2, 3]));
        assert_eq!(groups.group_of(2), Some(b));
        assert_eq!(groups.group_of(9), None);
    }

    #[test]
    fn new_groups_are_numbered_and_coloured_in_turn() {
        let mut groups = Groups::default();
        let a = groups.create(&[1]);
        let b = groups.create(&[2]);
        let names: Vec<_> = groups.iter().map(|g| (g.name.as_str(), g.color.as_str())).collect();
        assert_eq!(
            names,
            [("Group 1", GroupColor::IN_TURN[0].key()), ("Group 2", GroupColor::IN_TURN[1].key())]
        );
        groups.remove(a);
        // The free name is taken again, the id never is.
        let c = groups.create(&[3]);
        assert_eq!(groups.get(c).map(|g| g.name.as_str()), Some("Group 1"));
        assert!(c > b);
    }

    #[test]
    fn joining_moves_a_terminal_and_a_group_left_empty_goes() {
        let mut groups = Groups::default();
        let a = groups.create(&[1]);
        let b = groups.create(&[2]);
        assert!(groups.join(b, 1));
        assert_eq!(groups.group_of(1), Some(b));
        assert!(groups.get(a).is_none(), "its last terminal left, so it went");
        assert!(!groups.join(99, 1), "no such group");
        groups.leave(1);
        assert_eq!(groups.group_of(1), None);
        assert!(groups.get(b).is_some(), "one terminal is still in it");
    }

    #[test]
    fn a_pinned_group_stays_empty_and_an_unpinned_one_goes_with_its_last_terminal() {
        let mut groups = Groups::default();
        let a = groups.create(&[1]);
        let b = groups.create(&[2]);
        groups.set_pinned(b, true);
        groups.retain_terminals(|_| false);
        assert!(groups.get(a).is_none());
        assert_eq!(groups.get(b).map(|g| (g.members.len(), g.pinned)), Some((0, true)));
        // Unpinned while empty, it goes at once.
        groups.set_pinned(b, false);
        assert!(groups.is_empty());
        // A terminal can join a pinned empty group.
        let c = groups.create(&[5]);
        groups.set_pinned(c, true);
        groups.leave(5);
        assert!(groups.join(c, 6));
        assert_eq!(groups.get(c).map(|g| g.members.clone()), Some(vec![6]));
    }

    #[test]
    fn rename_ignores_a_blank_name_and_colours_are_the_nine() {
        let mut groups = Groups::default();
        let a = groups.create(&[1]);
        groups.rename(a, "  Release  ");
        groups.rename(a, "   ");
        assert_eq!(groups.get(a).map(|g| g.name.as_str()), Some("Release"));
        groups.set_color(a, Some(GroupColor::Cyan));
        assert_eq!(groups.get(a).and_then(|g| g.color()), Some(GroupColor::Cyan));
        groups.set_color(a, None);
        assert_eq!(groups.get(a).and_then(|g| g.color()), None);
    }

    #[test]
    fn orphans_gather_in_one_recovered_group_at_the_end() {
        let mut groups = Groups::default();
        let mine = groups.create(&[1]);
        assert_eq!(groups.recover(&[]), None, "nothing to recover makes no group");
        let recovered = groups.recover(&[7, 8]).expect("made");
        assert_ne!(recovered, mine);
        let group = groups.get(recovered).expect("listed");
        assert_eq!((group.name.as_str(), group.recovered), (RECOVERED_NAME, true));
        assert_eq!(group.members, [7, 8]);
        assert_eq!(groups.iter().last().map(|g| g.id), Some(recovered), "after the person's own");
        // More orphans join the same group; one that was in a group of its own moves there.
        groups.join(mine, 9);
        assert_eq!(groups.recover(&[9]), Some(recovered));
        assert_eq!(groups.get(recovered).map(|g| g.members.clone()), Some(vec![7, 8, 9]));
        assert_eq!(groups.get(mine).map(|g| g.members.clone()), Some(vec![1]));
        assert_eq!(groups.iter().filter(|g| g.recovered).count(), 1);
    }

    #[test]
    fn the_recovered_mark_round_trips_and_plain_groups_save_as_before() {
        let mut groups = Groups::default();
        groups.create(&[1]);
        let plain = serde_json::to_string(&groups).unwrap_or_default();
        assert!(!plain.contains("recovered"), "{plain}");
        groups.recover(&[2]);
        let json = serde_json::to_string(&groups).unwrap_or_default();
        let back: Groups = serde_json::from_str(&json).unwrap_or_default();
        assert_eq!(back.iter().filter(|g| g.recovered).count(), 1, "{json}");
    }

    #[test]
    fn a_folder_is_only_there_once_set_and_cleared_by_none_or_a_blank_path() {
        let mut groups = Groups::default();
        let a = groups.create(&[1]);
        assert_eq!(groups.get(a).and_then(|g| g.folder.clone()), None, "no folder until the person sets one");
        groups.set_folder(a, Some("  /tmp/work  "));
        assert_eq!(groups.get(a).and_then(|g| g.folder.as_deref()), Some("/tmp/work"));
        groups.set_folder(a, Some("   "));
        assert_eq!(groups.get(a).and_then(|g| g.folder.clone()), None);
        groups.set_folder(a, Some("/tmp/work"));
        groups.set_folder(a, None);
        assert_eq!(groups.get(a).and_then(|g| g.folder.clone()), None);
        groups.set_folder(99, Some("/tmp"));
        assert_eq!(groups.iter().count(), 1, "no such group, nothing made");
    }

    #[test]
    fn setting_a_folder_pins_the_group_and_clearing_it_leaves_it_pinned() {
        let mut groups = Groups::default();
        let a = groups.create(&[1]);
        groups.set_folder(a, Some("   "));
        assert!(!groups.get(a).is_some_and(|g| g.pinned), "a blank path is no folder");
        groups.set_folder(a, Some("/tmp/work"));
        assert!(groups.get(a).is_some_and(|g| g.pinned));
        groups.set_folder(a, None);
        assert!(groups.get(a).is_some_and(|g| g.pinned), "clearing the folder does not unpin");
        groups.set_pinned(a, false);
        groups.retain_terminals(|_| false);
        assert_eq!(groups.iter().count(), 0, "unpinned, it goes with its last terminal");
        let b = groups.create(&[2]);
        groups.set_folder(b, Some("/tmp/work"));
        groups.retain_terminals(|_| false);
        assert_eq!(groups.iter().count(), 1, "pinned by its folder, it stays");
    }

    #[test]
    fn a_folder_that_is_gone_or_is_a_file_is_not_a_start_folder() {
        let dir = tempfile::tempdir().expect("temp dir");
        let file = dir.path().join("a-file");
        std::fs::write(&file, "x").expect("write");
        let mut groups = Groups::default();
        let a = groups.create(&[1]);
        assert_eq!(groups.start_folder(a), None, "none set");
        let path = dir.path().to_string_lossy().into_owned();
        groups.set_folder(a, Some(&path));
        assert_eq!(groups.start_folder(a), Some(path));
        groups.set_folder(a, Some(&file.to_string_lossy()));
        assert_eq!(groups.start_folder(a), None, "a file");
        groups.set_folder(a, Some(&dir.path().join("gone").to_string_lossy()));
        assert_eq!(groups.start_folder(a), None, "missing");
        assert_eq!(groups.start_folder(99), None, "no such group");
    }

    #[test]
    fn the_folder_round_trips_and_groups_without_one_save_as_before() {
        let mut groups = Groups::default();
        let a = groups.create(&[1]);
        let plain = serde_json::to_string(&groups).unwrap_or_default();
        assert!(!plain.contains("folder"), "{plain}");
        groups.set_folder(a, Some("/tmp/work"));
        let json = serde_json::to_string(&groups).unwrap_or_default();
        assert!(json.contains(r#""folder":"/tmp/work""#), "{json}");
        assert_eq!(serde_json::from_str::<Groups>(&json).ok(), Some(groups));
        // Saved before groups had a folder.
        let old: Groups = serde_json::from_str(r#"[{"id":2,"name":"Docs","color":"blue","members":[1]}]"#)
            .unwrap_or_default();
        assert_eq!(old.get(2).map(|g| g.folder.clone()), Some(None));
    }

    #[test]
    fn a_no_to_the_folder_question_is_saved_and_a_layout_from_before_loads_as_not_asked() {
        let mut groups = Groups::default();
        let a = groups.create(&[1]);
        let plain = serde_json::to_string(&groups).unwrap_or_default();
        assert!(!plain.contains("declined"), "{plain}");
        groups.decline_folder(a);
        let json = serde_json::to_string(&groups).unwrap_or_default();
        assert!(json.contains(r#""folder_declined":true"#), "{json}");
        let back: Groups = serde_json::from_str(&json).unwrap_or_default();
        assert_eq!(back.get(a).map(|g| g.folder_declined), Some(true));
        let old: Groups = serde_json::from_str(r#"[{"id":2,"name":"Docs","color":"blue","members":[1]}]"#)
            .unwrap_or_default();
        assert_eq!(old.get(2).map(|g| (g.folder_declined, g.only_in_folder)), Some((false, false)));
    }

    #[test]
    fn keeping_only_the_folders_terminals_is_saved_with_the_group() {
        let mut groups = Groups::default();
        let a = groups.create(&[1]);
        groups.set_folder(a, Some("/tmp/work"));
        let plain = serde_json::to_string(&groups).unwrap_or_default();
        assert!(!plain.contains("only_in_folder"), "{plain}");
        groups.set_only_in_folder(a, true);
        let json = serde_json::to_string(&groups).unwrap_or_default();
        assert!(json.contains(r#""only_in_folder":true"#), "{json}");
        assert_eq!(serde_json::from_str::<Groups>(&json).ok(), Some(groups));
    }

    #[test]
    fn setting_a_folder_takes_a_no_back_and_taking_it_away_is_a_no() {
        let mut groups = Groups::default();
        let a = groups.create(&[1]);
        groups.decline_folder(a);
        groups.set_folder(a, Some("/tmp/work"));
        assert!(groups.get(a).is_some_and(|g| !g.folder_declined));
        groups.set_folder(a, None);
        assert!(groups.get(a).is_some_and(|g| g.folder_declined));
        // Nothing to take away is not a no.
        let b = groups.create(&[2]);
        groups.set_folder(b, None);
        assert!(groups.get(b).is_some_and(|g| !g.folder_declined));
    }

    #[test]
    fn closed_terminals_are_forgotten() {
        let mut groups = Groups::default();
        let a = groups.create(&[1, 2, 3]);
        groups.retain_terminals(|id| id != 2);
        assert_eq!(groups.get(a).map(|g| g.members.clone()), Some(vec![1, 3]));
    }

    #[test]
    fn groups_round_trip_as_a_plain_list_and_old_ones_load() {
        let mut groups = Groups::default();
        groups.create(&[4]);
        let json = serde_json::to_string(&groups).unwrap_or_default();
        assert!(json.starts_with('['), "{json}");
        assert_eq!(serde_json::from_str::<Groups>(&json).ok(), Some(groups));
        assert_eq!(group_key(3), "group:3");
        // Saved before groups could be pinned, when "teal" was a colour.
        let old: Groups = serde_json::from_str(r#"[{"id":2,"name":"Docs","color":"teal","members":[1]}]"#)
            .unwrap_or_default();
        let group = old.get(2).expect("loaded");
        assert!(!group.pinned);
        assert_eq!(group.color(), Some(GroupColor::Cyan));
    }
}
