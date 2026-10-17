//! Terminals as tabs instead of the sidebar (Settings → Terminals & sidebar → Show terminals in):
//! which tab sets the bar shows. A tab set is what a group is in the list; the tab bar shows the
//! selected terminal's set as tabs and the others one step away.

use serde::{Deserialize, Serialize};

use super::listing::Listing;

/// Where a window lists its terminals.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalsIn {
    /// The list on the left.
    #[default]
    Sidebar,
    /// A tab bar across the top, and no sidebar.
    Tabs,
}

impl TerminalsIn {
    pub const ALL: [TerminalsIn; 2] = [TerminalsIn::Sidebar, TerminalsIn::Tabs];
}

/// What the terminals in no group are called as a set.
pub const IN_NO_GROUP: &str = "In no group";

/// One set of tabs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabSet {
    /// The listing's band it is; `None` for the terminals in no group.
    pub band: Option<usize>,
    pub label: String,
    /// Its terminals in the list's order, folded ones too: the tab bar has no folds.
    pub members: Vec<usize>,
}

/// The tab sets of a listing: every group (an empty pinned one is somewhere to put a new
/// terminal), then the terminals in no group as "In no group".
pub fn tab_sets(listing: &Listing) -> Vec<TabSet> {
    let mut sets: Vec<TabSet> = listing
        .bands
        .iter()
        .enumerate()
        .map(|(ix, band)| TabSet { band: Some(ix), label: band.label.clone(), members: band.members.clone() })
        .collect();
    let loose: Vec<usize> =
        listing.order().iter().copied().filter(|&session| listing.band_of(session).is_none()).collect();
    if !loose.is_empty() {
        sets.push(TabSet { band: None, label: IN_NO_GROUP.to_string(), members: loose });
    }
    sets
}

/// The set `active` is in.
pub fn current_set(sets: &[TabSet], active: usize) -> Option<usize> {
    sets.iter().position(|set| set.members.contains(&active))
}

#[cfg(test)]
mod tests {
    use super::{IN_NO_GROUP, TabSet, TerminalsIn, current_set, tab_sets};
    use crate::sidebar::{ListInput, Listing, SidebarNumbers, Splits, WindowSidebar};

    fn listing(n: usize, sidebar: &WindowSidebar) -> Listing {
        let ids: Vec<i64> = (1..=n as i64).collect();
        Listing::build(&ListInput {
            ids: &ids,
            pinned: &vec![false; n],
            sidebar,
            splits: Splits::default(),
            numbers: SidebarNumbers::List,
        })
    }

    fn shape(sets: &[TabSet]) -> Vec<(String, Vec<usize>)> {
        sets.iter().map(|set| (set.label.clone(), set.members.clone())).collect()
    }

    #[test]
    fn terminals_in_no_group_are_one_set() {
        let sets = tab_sets(&listing(3, &WindowSidebar::default()));
        assert_eq!(shape(&sets), [(IN_NO_GROUP.to_string(), vec![0, 1, 2])]);
        assert_eq!(current_set(&sets, 2), Some(0));
    }

    #[test]
    fn groups_are_sets_even_folded_or_empty_and_the_rest_comes_last() {
        let ids = [1, 2, 3];
        let mut sidebar = WindowSidebar::default();
        let a = sidebar.new_group(&ids, &[1, 3]);
        let empty = sidebar.new_group(&ids, &[2]);
        sidebar.groups.set_pinned(empty, true);
        sidebar.groups.leave(2);
        sidebar.toggle_group_fold(a);
        let sets = tab_sets(&listing(3, &sidebar));
        assert_eq!(
            shape(&sets),
            [
                ("Group 1".to_string(), vec![0, 2]),
                ("Group 2".to_string(), vec![]),
                (IN_NO_GROUP.to_string(), vec![1])
            ]
        );
        assert!(sets[1].band.is_some(), "the pinned empty group has a band");
        assert_eq!(current_set(&sets, 1), Some(2));
        assert_eq!(current_set(&sets, 9), None);
    }

    #[test]
    fn the_setting_reads_and_writes_by_its_name() {
        assert_eq!(serde_json::from_str::<TerminalsIn>("\"tabs\"").ok(), Some(TerminalsIn::Tabs));
        assert_eq!(serde_json::to_string(&TerminalsIn::Sidebar).ok().as_deref(), Some("\"sidebar\""));
    }
}
