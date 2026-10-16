//! Terminals as tabs instead of the sidebar (Settings → Terminals & sidebar → Show terminals in):
//! where the tab bar goes, and which tab sets it shows. A tab set is what a band is in the list (a
//! folder's terminals, or one of the person's groups, or every terminal of a flat list); the tab
//! bar shows the selected terminal's set as tabs and the others as chips, piles or segments,
//! depending on the layout.

use serde::{Deserialize, Serialize};

use super::listing::Listing;

/// Where a window lists its terminals.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TerminalsIn {
    /// The list on the left.
    #[default]
    Sidebar,
    /// A tab bar (`TabLayout`), and no sidebar.
    Tabs,
}

impl TerminalsIn {
    pub const ALL: [TerminalsIn; 2] = [TerminalsIn::Sidebar, TerminalsIn::Tabs];
}

/// How the tab bar is laid out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TabLayout {
    /// Across the top: the sets as chips on the first row, the selected one's tabs on a second.
    #[default]
    TwoRows,
    /// Across the top, one row: the selected set's tabs, the other sets piled up beside them.
    Stacks,
    /// Along the bottom: every set a segment, the selected one opened into its tabs.
    Bottom,
}

impl TabLayout {
    pub const ALL: [TabLayout; 3] = [TabLayout::TwoRows, TabLayout::Stacks, TabLayout::Bottom];

    pub fn label(self) -> &'static str {
        match self {
            TabLayout::TwoRows => "Two rows",
            TabLayout::Stacks => "Stacks",
            TabLayout::Bottom => "Bottom",
        }
    }

    /// The bar is across the top, where the title row would be: the title row moves under it.
    pub fn on_top(self) -> bool {
        self != TabLayout::Bottom
    }
}

/// What the terminals of a set without a band are called.
pub const UNGROUPED: &str = "Ungrouped";
/// The one set of a flat list.
pub const ALL_TERMINALS: &str = "Terminals";

/// One set of tabs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabSet {
    /// The listing's band it is; `None` for the terminals in none (or a flat list's).
    pub band: Option<usize>,
    pub label: String,
    /// Its terminals in the list's order, folded and filtered ones too: the tab bar has neither.
    pub members: Vec<usize>,
}

/// The tab sets of a listing: one per band that has terminals, and every one of the person's
/// groups (an empty group is somewhere to put a new terminal), then the terminals in no band as
/// "Ungrouped"; a flat list is one set.
pub fn tab_sets(listing: &Listing) -> Vec<TabSet> {
    let order = listing.order();
    if listing.bands.is_empty() {
        return vec![TabSet { band: None, label: ALL_TERMINALS.to_string(), members: order.to_vec() }];
    }
    let mut sets: Vec<TabSet> = listing
        .bands
        .iter()
        .enumerate()
        .filter(|(_, band)| !band.members.is_empty() || band.group.is_some())
        .map(|(ix, band)| TabSet {
            band: Some(ix),
            label: band.label.clone(),
            // In the list's order, which pins and splits have settled.
            members: order.iter().copied().filter(|session| band.members.contains(session)).collect(),
        })
        .collect();
    let loose: Vec<usize> =
        order.iter().copied().filter(|&session| listing.band_of(session).is_none()).collect();
    if !loose.is_empty() {
        sets.push(TabSet { band: None, label: UNGROUPED.to_string(), members: loose });
    }
    sets
}

/// The set `active` is in.
pub fn current_set(sets: &[TabSet], active: usize) -> Option<usize> {
    sets.iter().position(|set| set.members.contains(&active))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::{ALL_TERMINALS, TabLayout, TabSet, TerminalsIn, UNGROUPED, current_set, tab_sets};
    use crate::sidebar::filter::{RowState, Searchable};
    use crate::sidebar::{Grouping, ListInput, Listing, SidebarNumbers, SidebarOrder, Splits, WindowSidebar};

    fn rows(n: usize) -> Vec<Searchable<'static>> {
        (0..n)
            .map(|_| Searchable {
                name: "t",
                state: RowState::Idle,
                place: None,
                branch: None,
                command: None,
            })
            .collect()
    }

    fn listing(
        grouping: Grouping,
        folders: &[Option<PathBuf>],
        group_of: &[Option<u32>],
        sidebar: &WindowSidebar,
    ) -> Listing {
        let n = folders.len().max(group_of.len());
        let rows = rows(n);
        Listing::build(&ListInput {
            grouping,
            folders,
            pinned_folders: &[PathBuf::from("/w/empty")],
            rows: &rows,
            pinned: &vec![false; n],
            sidebar,
            home: None,
            splits: Splits::default(),
            order: SidebarOrder::Classic,
            numbers: SidebarNumbers::List,
            band_order: &[],
            group_of,
        })
    }

    fn shape(sets: &[TabSet]) -> Vec<(String, Vec<usize>)> {
        sets.iter().map(|set| (set.label.clone(), set.members.clone())).collect()
    }

    #[test]
    fn a_flat_list_is_one_set() {
        let folders = vec![None, None, None];
        let sets = tab_sets(&listing(Grouping::None, &folders, &[], &WindowSidebar::default()));
        assert_eq!(shape(&sets), [(ALL_TERMINALS.to_string(), vec![0, 1, 2])]);
        assert_eq!(current_set(&sets, 2), Some(0));
    }

    #[test]
    fn folders_are_sets_and_an_empty_pinned_folder_is_not() {
        let folders =
            vec![Some(PathBuf::from("/w/a")), Some(PathBuf::from("/w/b")), Some(PathBuf::from("/w/a"))];
        let sets = tab_sets(&listing(Grouping::Folder, &folders, &[], &WindowSidebar::default()));
        assert_eq!(shape(&sets), [("a".to_string(), vec![0, 2]), ("b".to_string(), vec![1])]);
        assert_eq!(current_set(&sets, 1), Some(1));
    }

    #[test]
    fn groups_are_sets_even_empty_or_folded_and_loose_terminals_come_last() {
        let mut sidebar = WindowSidebar::default();
        let a = sidebar.groups.create(&[]);
        let empty = sidebar.groups.create(&[]);
        sidebar.toggle_group_fold(a);
        let folders = vec![None, None, None];
        let sets = tab_sets(&listing(Grouping::Custom, &folders, &[Some(a), None, Some(a)], &sidebar));
        assert_eq!(
            shape(&sets),
            [
                ("Group 1".to_string(), vec![0, 2]),
                ("Group 2".to_string(), vec![]),
                (UNGROUPED.to_string(), vec![1])
            ]
        );
        assert!(sets[1].band.is_some(), "group {empty} has a band");
        assert_eq!(current_set(&sets, 1), Some(2));
        assert_eq!(current_set(&sets, 9), None);
    }

    #[test]
    fn the_settings_read_and_write_by_their_names() {
        assert_eq!(serde_json::to_string(&TabLayout::TwoRows).ok().as_deref(), Some("\"two_rows\""));
        assert_eq!(serde_json::from_str::<TerminalsIn>("\"tabs\"").ok(), Some(TerminalsIn::Tabs));
        assert!(TabLayout::Stacks.on_top() && !TabLayout::Bottom.on_top());
    }
}
