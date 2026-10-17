//! Terminals as tabs instead of the sidebar (Settings → Terminals & sidebar → Show terminals in):
//! which tab sets the bar shows. A tab set is what a group is in the list; the tab bar shows the
//! selected terminal's set as tabs and the others one step away.

use serde::{Deserialize, Serialize};

use super::listing::Listing;
use super::urgency::{Urgency, loudest_of};

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

/// The set one step from `current` in the switcher's order, wrapping round: `forward` is the next,
/// otherwise the previous. Sets with no terminal are passed over (a pinned empty group is somewhere to
/// put a new terminal, not somewhere to arrive by pressing a key); `None` when no other set has one.
pub fn step_set(sets: &[TabSet], current: usize, forward: bool) -> Option<usize> {
    let n = sets.len();
    (1..n)
        .map(|offset| if forward { (current + offset) % n } else { (current + n - offset) % n })
        .find(|&at| !sets[at].members.is_empty())
}

/// The set with this number (1 to 9, as the switcher lists them, empty ones included): ⌃⌥1–9.
pub fn set_numbered(sets: &[TabSet], number: usize) -> Option<usize> {
    (1..=NUMBERED_SETS).contains(&number).then(|| number - 1).filter(|&at| at < sets.len())
}

/// How many sets a key reaches.
pub const NUMBERED_SETS: usize = 9;

/// Whether a set other than `current` has a terminal that needs you or failed: the amber dot on the
/// closed switcher. `urgency` is the urgency of the terminal at a session index.
pub fn others_need_you(sets: &[TabSet], current: usize, urgency: impl Fn(usize) -> Urgency) -> bool {
    sets.iter()
        .enumerate()
        .filter(|&(at, _)| at != current)
        .any(|(_, set)| set.members.iter().any(|&member| urgency(member).is_alert()))
}

/// The terminal of a set that speaks loudest (its card's lead), the first among equals.
pub fn loudest_member(set: &TabSet, urgency: impl Fn(usize) -> Urgency) -> Option<usize> {
    loudest_of(set.members.iter().map(|&member| urgency(member))).map(|at| set.members[at])
}

/// What the switcher's search finds: a set by its name, or a terminal by its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hit {
    /// The set at this index of `tab_sets`.
    Set(usize),
    /// The terminal at this session index, in the set at `.1`.
    Terminal(usize, usize),
}

/// The sets whose names contain `query` (case does not matter), then the terminals whose names do,
/// each in the order the switcher lists them. Nothing for a blank query: the switcher shows its
/// cards then.
pub fn search(sets: &[TabSet], query: &str, name_of: impl Fn(usize) -> String) -> Vec<Hit> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Vec::new();
    }
    let found = |text: &str| text.to_lowercase().contains(&query);
    let named = sets.iter().enumerate().filter(|(_, set)| found(&set.label)).map(|(at, _)| Hit::Set(at));
    let terminals =
        sets.iter().enumerate().flat_map(|(at, set)| set.members.iter().map(move |&member| (at, member)));
    named
        .chain(
            terminals
                .filter(|&(_, member)| found(&name_of(member)))
                .map(|(at, member)| Hit::Terminal(at, member)),
        )
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        Hit, IN_NO_GROUP, TabSet, TerminalsIn, current_set, loudest_member, others_need_you, search,
        set_numbered, step_set, tab_sets,
    };
    use crate::sidebar::Urgency;
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

    fn sets_of(spec: &[(&str, &[usize])]) -> Vec<TabSet> {
        spec.iter()
            .enumerate()
            .map(|(at, (label, members))| TabSet {
                band: Some(at),
                label: label.to_string(),
                members: members.to_vec(),
            })
            .collect()
    }

    #[test]
    fn stepping_wraps_and_passes_over_empty_sets() {
        let sets = sets_of(&[("A", &[0]), ("Docs", &[]), ("B", &[1, 2]), (IN_NO_GROUP, &[3])]);
        assert_eq!(step_set(&sets, 0, true), Some(2), "Docs is empty");
        assert_eq!(step_set(&sets, 2, true), Some(3));
        assert_eq!(step_set(&sets, 3, true), Some(0), "wraps");
        assert_eq!(step_set(&sets, 0, false), Some(3), "wraps back");
        assert_eq!(step_set(&sets, 2, false), Some(0));
        assert_eq!(step_set(&sets_of(&[("A", &[0])]), 0, true), None, "nowhere else to go");
        assert_eq!(step_set(&sets_of(&[("A", &[0]), ("B", &[])]), 0, true), None);
    }

    #[test]
    fn numbers_reach_the_first_nine_sets_as_listed() {
        let sets = sets_of(&[("A", &[0]), ("Docs", &[]), ("B", &[1])]);
        assert_eq!(set_numbered(&sets, 1), Some(0));
        assert_eq!(set_numbered(&sets, 2), Some(1), "an empty group is reached too: it gets a terminal");
        assert_eq!(set_numbered(&sets, 3), Some(2));
        assert_eq!(set_numbered(&sets, 4), None);
        assert_eq!(set_numbered(&sets, 0), None);
        let many: Vec<(String, Vec<usize>)> = (0..12).map(|n| (format!("G{n}"), vec![n])).collect();
        let many: Vec<TabSet> = many
            .iter()
            .enumerate()
            .map(|(at, (label, members))| TabSet {
                band: Some(at),
                label: label.clone(),
                members: members.clone(),
            })
            .collect();
        assert_eq!(set_numbered(&many, 9), Some(8));
        assert_eq!(set_numbered(&many, 10), None, "only nine have a key");
    }

    #[test]
    fn the_dot_says_another_set_needs_you_never_the_open_one() {
        let sets = sets_of(&[("A", &[0, 1]), ("B", &[2])]);
        let urgency =
            |loud: usize| move |ix: usize| if ix == loud { Urgency::NeedsYou } else { Urgency::Idle };
        assert!(others_need_you(&sets, 0, urgency(2)));
        assert!(!others_need_you(&sets, 0, urgency(1)), "the open set has its own tiles");
        assert!(!others_need_you(&sets, 1, urgency(2)));
        let working = |_: usize| Urgency::Working;
        assert!(!others_need_you(&sets, 0, working), "working is not asking");
        let failed = |ix: usize| if ix == 2 { Urgency::Failed } else { Urgency::Idle };
        assert!(others_need_you(&sets, 0, failed));
    }

    #[test]
    fn a_set_leads_with_its_loudest_terminal() {
        let set = &sets_of(&[("A", &[4, 5, 6])])[0];
        let urgency = |ix: usize| match ix {
            5 => Urgency::Working,
            6 => Urgency::NeedsYou,
            _ => Urgency::Idle,
        };
        assert_eq!(loudest_member(set, urgency), Some(6));
        assert_eq!(loudest_member(&sets_of(&[("E", &[])])[0], urgency), None);
    }

    #[test]
    fn search_finds_sets_then_terminals_and_nothing_for_a_blank_query() {
        let sets = sets_of(&[("Release", &[0, 1]), ("Review", &[2]), (IN_NO_GROUP, &[3])]);
        let names = |ix: usize| ["cargo test", "release notes", "zsh", "Review queue"][ix].to_string();
        assert_eq!(search(&sets, "  ", names), []);
        assert_eq!(
            search(&sets, "rele", names),
            [Hit::Set(0), Hit::Terminal(0, 1)],
            "the group, then the terminal named so"
        );
        assert_eq!(search(&sets, "REVIEW", names), [Hit::Set(1), Hit::Terminal(2, 3)]);
        assert_eq!(search(&sets, "nothing", names), []);
    }

    #[test]
    fn the_setting_reads_and_writes_by_its_name() {
        assert_eq!(serde_json::from_str::<TerminalsIn>("\"tabs\"").ok(), Some(TerminalsIn::Tabs));
        assert_eq!(serde_json::to_string(&TerminalsIn::Sidebar).ok().as_deref(), Some("\"sidebar\""));
    }
}
