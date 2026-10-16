//! The groups a person makes in the terminal list (grouping "Your groups"): a name, a colour and the
//! terminals in it, the way a browser groups its tabs. Unlike a folder band a group isn't worked
//! out from where its terminals are: it holds whichever terminals were put in it, from any folder,
//! and a terminal in no group stays loose in the list. A window keeps its own groups, saved with it.

use serde::{Deserialize, Serialize};

/// One group of terminals.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct TermGroup {
    /// Unique among the window's groups, and never reused while the window lives: what a folded
    /// group and a drag remember it by.
    pub id: u32,
    pub name: String,
    /// A folder colour's name (`blue`, `teal` … see `config::FolderColor`), or empty for none.
    pub color: String,
    /// The terminals in it by their id, in the order they joined.
    pub members: Vec<i64>,
}

/// The colours new groups take in turn, so two groups made one after the other don't look alike.
pub const GROUP_COLORS: [&str; 8] = ["blue", "green", "orange", "purple", "pink", "teal", "yellow", "red"];

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

    fn get_mut(&mut self, id: u32) -> Option<&mut TermGroup> {
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
        let color = GROUP_COLORS[self.0.len() % GROUP_COLORS.len()].to_string();
        for &terminal in members {
            self.leave(terminal);
        }
        self.0.push(TermGroup { id, name: name.unwrap_or_default(), color, members: members.to_vec() });
        id
    }

    /// Move `terminal` into group `id` (out of the one it was in). `false` if there is no such
    /// group.
    pub fn join(&mut self, id: u32, terminal: i64) -> bool {
        if self.get(id).is_none() {
            return false;
        }
        self.leave(terminal);
        if let Some(group) = self.get_mut(id) {
            group.members.push(terminal);
        }
        true
    }

    /// Take `terminal` out of its group; the group stays, even empty, until it is removed.
    pub fn leave(&mut self, terminal: i64) {
        for group in &mut self.0 {
            group.members.retain(|&member| member != terminal);
        }
    }

    /// Rename group `id`; a blank name keeps the old one.
    pub fn rename(&mut self, id: u32, name: &str) {
        let name = name.trim();
        if let Some(group) = self.get_mut(id).filter(|_| !name.is_empty()) {
            group.name = name.to_string();
        }
    }

    pub fn set_color(&mut self, id: u32, color: &str) {
        if let Some(group) = self.get_mut(id) {
            group.color = color.to_string();
        }
    }

    /// Remove group `id`; its terminals stay, loose.
    pub fn remove(&mut self, id: u32) {
        self.0.retain(|group| group.id != id);
    }

    /// Forget the terminals that are gone (closed while the window was open, or not restored):
    /// `alive` says which ids still exist. Empty groups stay; the person removes those.
    pub fn retain_terminals(&mut self, alive: impl Fn(i64) -> bool) {
        for group in &mut self.0 {
            group.members.retain(|&member| alive(member));
        }
    }
}

/// How a folded group is remembered among the folded bands (`WindowSidebar::folded`). A folder's
/// key is its absolute path, so the two never meet.
pub fn group_key(id: u32) -> String {
    format!("group:{id}")
}

#[cfg(test)]
mod tests {
    use super::{GROUP_COLORS, Groups, group_key};

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
        let a = groups.create(&[]);
        let b = groups.create(&[]);
        let names: Vec<_> = groups.iter().map(|g| (g.name.as_str(), g.color.as_str())).collect();
        assert_eq!(names, [("Group 1", GROUP_COLORS[0]), ("Group 2", GROUP_COLORS[1])]);
        groups.remove(a);
        // The free name is taken again, the id never is.
        let c = groups.create(&[]);
        assert_eq!(groups.get(c).map(|g| g.name.as_str()), Some("Group 1"));
        assert!(c > b);
    }

    #[test]
    fn joining_moves_a_terminal_and_leaving_keeps_the_group() {
        let mut groups = Groups::default();
        let a = groups.create(&[1]);
        let b = groups.create(&[]);
        assert!(groups.join(b, 1));
        assert_eq!(groups.group_of(1), Some(b));
        assert!(!groups.join(99, 1), "no such group");
        groups.leave(1);
        assert_eq!(groups.group_of(1), None);
        assert!(groups.get(a).is_some() && groups.get(b).is_some());
    }

    #[test]
    fn rename_ignores_a_blank_name() {
        let mut groups = Groups::default();
        let a = groups.create(&[]);
        groups.rename(a, "  Release  ");
        groups.rename(a, "   ");
        assert_eq!(groups.get(a).map(|g| g.name.as_str()), Some("Release"));
    }

    #[test]
    fn closed_terminals_are_forgotten() {
        let mut groups = Groups::default();
        let a = groups.create(&[1, 2, 3]);
        groups.retain_terminals(|id| id != 2);
        assert_eq!(groups.get(a).map(|g| g.members.clone()), Some(vec![1, 3]));
    }

    #[test]
    fn groups_round_trip_as_a_plain_list() {
        let mut groups = Groups::default();
        groups.create(&[4]);
        let json = serde_json::to_string(&groups).unwrap_or_default();
        assert!(json.starts_with('['), "{json}");
        assert_eq!(serde_json::from_str::<Groups>(&json).ok(), Some(groups));
        assert_eq!(group_key(3), "group:3");
    }
}
