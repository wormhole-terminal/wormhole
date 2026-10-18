//! The groups a person makes in the terminal list: a name, a colour and the terminals in it, the
//! way a browser groups its tabs. A group isn't worked out from where its terminals are: it holds
//! whichever terminals were put in it, from any folder, and a terminal in no group stands in the
//! list on its own. A group goes with its last terminal unless it is pinned. A window keeps its
//! own groups, saved with it.

use serde::{Deserialize, Serialize};

use super::group_color::GroupColor;

/// One group of terminals.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TermGroup {
    /// Unique among the window's groups, and never reused while the window lives: what a folded
    /// group and a drag remember it by.
    pub id: u32,
    pub name: String,
    /// A [`GroupColor`]'s name (`blue`, `cyan` …), or empty for none; a name this build doesn't
    /// know is kept and draws nothing.
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
}

fn is_false(value: &bool) -> bool {
    !value
}

/// What the Recovered group is called when it is made.
pub const RECOVERED_NAME: &str = "Recovered";

impl TermGroup {
    pub fn color(&self) -> Option<GroupColor> {
        GroupColor::from_key(&self.color)
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
        self.0.retain(|group| group.pinned || !group.members.is_empty());
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
    use super::{GroupColor, Groups, RECOVERED_NAME, group_key};

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
