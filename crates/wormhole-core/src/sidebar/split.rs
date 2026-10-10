//! Splits in the terminal list. A split groups several terminals that share a window: every one is
//! still a row of its own, and the list shows that they belong together, with a thin bracket in the
//! gutter along the rows when they sit next to each other, or a chain mark on each when they don't
//! (grouped by folder, a terminal stays in its own folder's band).
//!
//! The rules live here and in `Listing`: a split's members sit together in the order, pinning one
//! pins them all, and a filter that keeps one keeps them all. The window's layout of the split is
//! `panes`' business; the list only needs to know who is in which.

use std::collections::{BTreeMap, HashMap, HashSet};

/// A split's identity: only whether two terminals have the same one matters.
pub type SplitId = i64;

/// What the list is told about splits when it is built.
#[derive(Clone, Copy, Debug, Default)]
pub struct Splits<'a> {
    /// Each terminal's split, in the window's order like the rows themselves. `None`, and any
    /// terminal past the end, is in no split; so is one alone under its split's id, which isn't a
    /// split.
    pub split_of: &'a [Option<SplitId>],
    /// The selected terminal: the other members of its split are on screen with it.
    pub selected: Option<usize>,
}

/// Which row of a bracket this is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Bracket {
    Start,
    Mid,
    End,
}

/// What a row shows of its split, next to the terminal's own status.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SplitMarks {
    /// Its place in the bracket along the rows of a split whose members are next to each other.
    pub bracket: Option<Bracket>,
    /// A member of a split that isn't together in this list (another folder's band, or a folded
    /// one, between them): the chain mark stands in for the bracket.
    pub chain: bool,
    /// Another member of the selected terminal's split: it is on screen beside it.
    pub on_screen: bool,
    /// How many terminals the split has, this one included; 0 outside a split.
    pub members: usize,
}

/// The terminal's split, if it is in one.
pub fn split_of(ids: &[Option<SplitId>], session: usize) -> Option<SplitId> {
    ids.get(session).copied().flatten()
}

/// `ids` for `len` terminals, with a split that has only one of them taken as none (a stale
/// record, or the rest of it closed), so what follows can take membership as it is.
pub(super) fn normalised(ids: &[Option<SplitId>], len: usize) -> Vec<Option<SplitId>> {
    let ids: Vec<Option<SplitId>> = (0..len).map(|ix| split_of(ids, ix)).collect();
    let mut sizes: HashMap<SplitId, usize> = HashMap::new();
    for id in ids.iter().flatten() {
        *sizes.entry(*id).or_default() += 1;
    }
    ids.into_iter().map(|id| id.filter(|id| sizes.get(id).is_some_and(|&size| size > 1))).collect()
}

/// `order` with each split's members gathered at the place of its first one, in the order they
/// had. Gather a band's list on its own, and a split with members in several folders keeps each in
/// its own band.
pub fn gather_splits(order: &[usize], ids: &[Option<SplitId>]) -> Vec<usize> {
    let mut members: HashMap<SplitId, Vec<usize>> = HashMap::new();
    for &ix in order {
        if let Some(id) = split_of(ids, ix) {
            members.entry(id).or_default().push(ix);
        }
    }
    let mut gathered = Vec::with_capacity(order.len());
    for &ix in order {
        match split_of(ids, ix) {
            None => gathered.push(ix),
            // The first member brings the rest along; the others find their split gone.
            Some(id) => gathered.extend(members.remove(&id).unwrap_or_default()),
        }
    }
    gathered
}

/// Pinning one member pins the whole split: `pinned` for each of `len` terminals, all of a split's
/// members pinned when any of them is.
pub(super) fn pinned_with_splits(pinned: &[bool], ids: &[Option<SplitId>], len: usize) -> Vec<bool> {
    let is_pinned = |ix: usize| pinned.get(ix).copied().unwrap_or(false);
    let pinned_splits: HashSet<SplitId> =
        (0..len).filter(|&ix| is_pinned(ix)).filter_map(|ix| split_of(ids, ix)).collect();
    (0..len)
        .map(|ix| is_pinned(ix) || split_of(ids, ix).is_some_and(|id| pinned_splits.contains(&id)))
        .collect()
}

/// `admitted` for each terminal, a split kept whole: if the filter keeps one member it keeps all,
/// so a split never shows in part.
pub(super) fn admitted_with_splits(admitted: &[bool], ids: &[Option<SplitId>]) -> Vec<bool> {
    let kept_splits: HashSet<SplitId> = admitted
        .iter()
        .enumerate()
        .filter(|&(_, &kept)| kept)
        .filter_map(|(ix, _)| split_of(ids, ix))
        .collect();
    admitted
        .iter()
        .enumerate()
        .map(|(ix, &kept)| kept || split_of(ids, ix).is_some_and(|id| kept_splits.contains(&id)))
        .collect()
}

/// What each visible row of a split shows. `order` is every terminal as listed, `band` the band each
/// is in (all `None` in a flat list) and `shown` whether its row is on screen (not folded away or
/// filtered out), both by position in `order` and by terminal respectively. A split is together
/// when its members are next to each other in `order` and in one band: a bracket then runs along
/// its rows, and any other split gets the chain mark.
pub(super) fn marks(
    order: &[usize],
    band: &[Option<usize>],
    shown: &[bool],
    ids: &[Option<SplitId>],
    selected: Option<usize>,
) -> BTreeMap<usize, SplitMarks> {
    let mut places: BTreeMap<SplitId, Vec<usize>> = BTreeMap::new();
    for (at, &ix) in order.iter().enumerate() {
        if let Some(id) = split_of(ids, ix) {
            places.entry(id).or_default().push(at);
        }
    }
    let on_screen_split = selected.and_then(|ix| split_of(ids, ix));
    let mut marks = BTreeMap::new();
    for (id, places) in places {
        let together = places.windows(2).all(|pair| pair[1] == pair[0] + 1 && band[pair[0]] == band[pair[1]]);
        let rows: Vec<usize> = places.iter().map(|&at| order[at]).filter(|&ix| shown[ix]).collect();
        for (n, &ix) in rows.iter().enumerate() {
            let bracket = (together && rows.len() > 1).then(|| match n {
                0 => Bracket::Start,
                n if n + 1 == rows.len() => Bracket::End,
                _ => Bracket::Mid,
            });
            marks.insert(
                ix,
                SplitMarks {
                    bracket,
                    chain: !together,
                    on_screen: on_screen_split == Some(id) && selected != Some(ix),
                    members: places.len(),
                },
            );
        }
    }
    marks
}

#[cfg(test)]
mod tests {
    use super::{
        Bracket, SplitMarks, Splits, admitted_with_splits, gather_splits, normalised, pinned_with_splits,
    };
    use super::{marks as marks_of, split_of};
    use crate::sidebar::{Filter, Grouping, ListInput, Listing, RowState, Searchable, Show, WindowSidebar};
    use std::path::{Path, PathBuf};

    const NAMES: [&str; 8] = ["t0", "t1", "t2", "t3", "t4", "t5", "t6", "t7"];

    fn rows(states: &[RowState]) -> Vec<Searchable<'static>> {
        states
            .iter()
            .enumerate()
            .map(|(ix, &state)| Searchable {
                name: NAMES[ix],
                state,
                place: None,
                branch: None,
                command: None,
            })
            .collect()
    }

    fn idle(n: usize) -> Vec<Searchable<'static>> {
        rows(&vec![RowState::Idle; n])
    }

    struct Case<'a> {
        grouping: Grouping,
        folders: Vec<Option<PathBuf>>,
        pinned: Vec<bool>,
        sidebar: WindowSidebar,
        selected: Option<usize>,
        ids: &'a [Option<i64>],
    }

    fn flat(ids: &[Option<i64>]) -> Case<'_> {
        Case {
            grouping: Grouping::None,
            folders: Vec::new(),
            pinned: Vec::new(),
            sidebar: WindowSidebar::default(),
            selected: None,
            ids,
        }
    }

    fn by_folder<'a>(folders: &[&str], ids: &'a [Option<i64>]) -> Case<'a> {
        let folders = folders.iter().map(|folder| Some(PathBuf::from(folder))).collect();
        Case { grouping: Grouping::Folder, folders, ..flat(ids) }
    }

    fn build(case: &Case, rows: &[Searchable]) -> Listing {
        let input = ListInput {
            grouping: case.grouping,
            folders: &case.folders,
            pinned_folders: &[],
            rows,
            pinned: &case.pinned,
            sidebar: &case.sidebar,
            home: None,
            splits: Splits { split_of: case.ids, selected: case.selected },
        };
        Listing::build(&input)
    }

    fn marks(listing: &Listing, session: usize) -> (Option<Bracket>, bool, bool) {
        let marks = listing.marks(session);
        (marks.bracket, marks.chain, marks.on_screen)
    }

    // ------------------------------------------------------------ gathering and the rules

    #[test]
    fn a_split_is_gathered_at_its_first_member_keeping_the_order_it_had() {
        let ids = [None, Some(1), None, Some(2), Some(1), None, Some(2)];
        assert_eq!(gather_splits(&[0, 1, 2, 3, 4, 5, 6], &ids), [0, 1, 4, 2, 3, 6, 5]);
        assert_eq!(gather_splits(&[6, 4, 3, 1], &ids), [6, 3, 4, 1], "whatever order the list is in");
        assert_eq!(gather_splits(&[0, 2, 5], &ids), [0, 2, 5], "no splits, no change");
        assert_eq!(gather_splits(&[], &ids), Vec::<usize>::new());
        assert_eq!(gather_splits(&[3, 1, 9], &ids), [3, 1, 9], "past the ids: in none");
    }

    #[test]
    fn a_band_is_gathered_on_its_own_so_members_in_other_bands_stay_there() {
        let ids = [Some(1), None, Some(1), Some(1)];
        // Band a lists 0 and 2, band b lists 1 and 3: only the members of a band are brought together.
        assert_eq!(gather_splits(&[0, 1, 2], &ids), [0, 2, 1]);
        assert_eq!(gather_splits(&[3, 1], &ids), [3, 1]);
    }

    #[test]
    fn a_split_with_one_member_is_not_a_split() {
        let ids = [Some(1), None, Some(2), Some(1)];
        assert_eq!(normalised(&ids, 4), [Some(1), None, None, Some(1)]);
        assert_eq!(normalised(&ids, 2), [None, None], "its other member isn't in the list");
        assert_eq!(normalised(&[], 3), [None, None, None]);
        assert_eq!(split_of(&ids, 0), Some(1));
        assert_eq!(split_of(&ids, 9), None);
    }

    #[test]
    fn pinning_one_member_pins_the_split() {
        let ids = [Some(1), None, Some(1), Some(2)];
        assert_eq!(pinned_with_splits(&[false, false, true], &ids, 4), [true, false, true, false]);
        assert_eq!(pinned_with_splits(&[], &ids, 4), [false; 4]);
        assert_eq!(pinned_with_splits(&[false, true, false, false], &ids, 4), [false, true, false, false]);
    }

    #[test]
    fn a_filter_that_keeps_one_member_keeps_the_split() {
        let ids = [Some(1), None, Some(1), Some(2), Some(2)];
        assert_eq!(
            admitted_with_splits(&[false, false, true, false, false], &ids),
            [true, false, true, false, false]
        );
        assert_eq!(admitted_with_splits(&[false; 5], &ids), [false; 5]);
        assert_eq!(
            admitted_with_splits(&[true, true, false, true, false], &ids),
            [true, true, true, true, true]
        );
    }

    // ------------------------------------------------------------ marks

    #[test]
    fn marks_follow_the_listing_by_position_and_band_and_what_is_shown() {
        let order = [0, 1, 2, 3, 4];
        let ids = [None, Some(1), Some(1), Some(1), None];
        let shown = [true; 5];
        let marks = marks_of(&order, &[None; 5], &shown, &ids, Some(2));
        assert_eq!(marks.get(&1).map(|m| m.bracket), Some(Some(Bracket::Start)));
        assert_eq!(marks.get(&2).map(|m| m.bracket), Some(Some(Bracket::Mid)));
        assert_eq!(marks.get(&3).map(|m| m.bracket), Some(Some(Bracket::End)));
        assert!(!marks.contains_key(&0) && !marks.contains_key(&4));
        assert_eq!(
            marks[&1],
            SplitMarks { bracket: Some(Bracket::Start), chain: false, on_screen: true, members: 3 }
        );
        assert!(!marks[&2].on_screen, "the selected one is on screen by being selected");
        // The same three in two bands: a chain, not a bracket.
        let banded = marks_of(&order, &[None, Some(0), Some(0), Some(1), Some(1)], &shown, &ids, None);
        assert!([1, 2, 3].iter().all(|ix| banded[ix].chain && banded[ix].bracket.is_none()));
        // A row that isn't shown has no mark, and the rest are not left with half a bracket.
        let hidden = marks_of(&order, &[None; 5], &[true, true, false, true, true], &ids, None);
        assert!(!hidden.contains_key(&2));
        assert_eq!(hidden[&1].bracket, Some(Bracket::Start));
        assert_eq!(hidden[&3].bracket, Some(Bracket::End));
        let alone = marks_of(&order, &[None; 5], &[false, true, false, false, true], &ids, None);
        assert_eq!(alone[&1].bracket, None, "a bracket needs two rows");
        assert!(!alone[&1].chain);
    }

    // ------------------------------------------------------------ the listing

    #[test]
    fn a_flat_list_gathers_a_split_where_its_first_member_is_and_brackets_its_rows() {
        let ids = [None, Some(7), None, Some(7), None];
        let listing = build(&flat(&ids), &idle(5));
        assert_eq!(listing.order(), [0, 1, 3, 2, 4]);
        assert_eq!(listing.visible().collect::<Vec<_>>(), [0, 1, 3, 2, 4]);
        assert_eq!(marks(&listing, 1), (Some(Bracket::Start), false, false));
        assert_eq!(marks(&listing, 3), (Some(Bracket::End), false, false));
        assert_eq!(listing.marks(0), SplitMarks::default());
        assert_eq!(listing.marks(3).members, 2);
        // The ⌘ numbers count the rows as they are listed.
        assert_eq!(listing.numbered(), [0, 1, 3, 2, 4]);
    }

    #[test]
    fn a_bracket_of_three_has_a_middle_and_the_siblings_of_the_selected_are_on_screen() {
        let ids = [Some(1), None, Some(1), None, Some(1)];
        let listing = build(&Case { selected: Some(2), ..flat(&ids) }, &idle(5));
        assert_eq!(listing.order(), [0, 2, 4, 1, 3]);
        assert_eq!(marks(&listing, 0), (Some(Bracket::Start), false, true));
        assert_eq!(marks(&listing, 2), (Some(Bracket::Mid), false, false), "selected, not marked on screen");
        assert_eq!(marks(&listing, 4), (Some(Bracket::End), false, true));
        assert_eq!(marks(&listing, 1), (None, false, false));
        // Selecting a terminal outside it leaves them all unmarked.
        let listing = build(&Case { selected: Some(1), ..flat(&ids) }, &idle(5));
        assert!([0, 2, 4].iter().all(|&ix| !listing.marks(ix).on_screen));
    }

    #[test]
    fn a_split_with_one_member_in_the_list_is_an_ordinary_terminal() {
        let ids = [Some(3), None, None];
        let listing = build(&flat(&ids), &idle(3));
        assert_eq!(listing.order(), [0, 1, 2]);
        assert_eq!(listing.marks(0), SplitMarks::default());
    }

    #[test]
    fn pinning_one_member_lifts_the_whole_split_to_the_top() {
        let ids = [None, Some(1), None, Some(1), None];
        let listing = build(&Case { pinned: vec![false, false, false, true, false], ..flat(&ids) }, &idle(5));
        assert_eq!(listing.order(), [1, 3, 0, 2, 4]);
        assert_eq!(marks(&listing, 1).0, Some(Bracket::Start));
        // Pinned, it leads its band: with folders, in its own band only.
        let ids = [None, Some(1), None, Some(1)];
        let case = by_folder(&["/w/a", "/w/a", "/w/a", "/w/a"], &ids);
        let listing = build(&Case { pinned: vec![false, false, false, true], ..case }, &idle(4));
        assert_eq!(listing.bands[0].members, [1, 3, 0, 2]);
    }

    #[test]
    fn a_filter_keeps_a_whole_split_when_one_member_matches() {
        let ids = [None, Some(1), None, Some(1), None];
        let states = [RowState::Idle, RowState::Idle, RowState::Idle, RowState::NeedsYou, RowState::Idle];
        let sidebar = WindowSidebar {
            filter: Filter { show: Show::NeedsYou, ..Filter::default() },
            ..WindowSidebar::default()
        };
        let listing = build(&Case { sidebar, ..flat(&ids) }, &rows(&states));
        assert_eq!(listing.visible().collect::<Vec<_>>(), [1, 3], "the quiet member comes with it");
        assert_eq!(listing.hidden, 3);
        assert_eq!(marks(&listing, 1).0, Some(Bracket::Start));

        // The search the same way: one member's name is enough.
        let sidebar = WindowSidebar {
            filter: Filter { search: "t3".into(), ..Filter::default() },
            ..WindowSidebar::default()
        };
        let listing = build(&Case { sidebar, ..flat(&ids) }, &idle(5));
        assert_eq!(listing.visible().collect::<Vec<_>>(), [1, 3]);

        // Nothing matches: nothing shows.
        let sidebar = WindowSidebar {
            filter: Filter { search: "zzz".into(), ..Filter::default() },
            ..WindowSidebar::default()
        };
        assert!(build(&Case { sidebar, ..flat(&ids) }, &idle(5)).entries.is_empty());
    }

    #[test]
    fn grouped_by_folder_each_terminal_stays_in_its_band_and_a_split_across_bands_is_a_chain() {
        // a: 0, 2, 4, 5   b: 1, 3. Split 1 is in band a only, split 2 in both.
        let folders = ["/w/a", "/w/b", "/w/a", "/w/b", "/w/a", "/w/a"];
        let ids = [Some(1), Some(2), Some(2), None, Some(1), None];
        let listing = build(&by_folder(&folders, &ids), &idle(6));
        assert_eq!(listing.bands[0].members, [0, 4, 2, 5], "0 and 4 gathered inside band a");
        assert_eq!(listing.bands[1].members, [1, 3]);
        assert_eq!(listing.order(), [0, 4, 2, 5, 1, 3]);
        assert_eq!(marks(&listing, 0), (Some(Bracket::Start), false, false));
        assert_eq!(marks(&listing, 4), (Some(Bracket::End), false, false));
        // Split 2 has 2 in band a and 1 in band b: neither is next to the other.
        assert_eq!(marks(&listing, 2), (None, true, false));
        assert_eq!(marks(&listing, 1), (None, true, false));
        assert_eq!(listing.marks(1).members, 2);
    }

    #[test]
    fn members_next_to_each_other_across_a_band_boundary_are_a_chain_not_a_bracket() {
        // 1 is the last row of band a and 2 the first of band b: next to each other in the order,
        // with a band's header between them.
        let folders = ["/w/a", "/w/a", "/w/b", "/w/b"];
        let ids = [None, Some(9), Some(9), None];
        let listing = build(&by_folder(&folders, &ids), &idle(4));
        assert_eq!(listing.order(), [0, 1, 2, 3]);
        assert_eq!((marks(&listing, 1).1, marks(&listing, 2).1), (true, true));
        assert!(listing.marks(1).bracket.is_none() && listing.marks(2).bracket.is_none());
        // Both in one band they are a bracket.
        let folders = ["/w/a", "/w/b", "/w/b", "/w/a"];
        let listing = build(&by_folder(&folders, &ids), &idle(4));
        assert_eq!((marks(&listing, 1).0, marks(&listing, 2).0), (Some(Bracket::Start), Some(Bracket::End)));
    }

    #[test]
    fn a_folded_band_takes_its_rows_and_its_members_marks_with_it() {
        let folders = ["/w/a", "/w/b", "/w/a", "/w/b"];
        let ids = [Some(1), Some(2), Some(1), Some(2)];
        let mut sidebar = WindowSidebar::default();
        sidebar.toggle_fold(Some(Path::new("/w/b")));
        let listing = build(&Case { sidebar: sidebar.clone(), ..by_folder(&folders, &ids) }, &idle(4));
        assert_eq!(listing.visible().collect::<Vec<_>>(), [0, 2]);
        assert_eq!((marks(&listing, 0).0, marks(&listing, 2).0), (Some(Bracket::Start), Some(Bracket::End)));
        assert_eq!(listing.marks(1), SplitMarks::default(), "folded away: nothing to mark");
        // A split with a member in the folded band: the one still on screen is chained to it.
        let ids = [Some(1), Some(1), None, None];
        let folders = ["/w/a", "/w/b", "/w/a", "/w/b"];
        let listing = build(&Case { sidebar, ..by_folder(&folders, &ids) }, &idle(4));
        assert_eq!(marks(&listing, 0), (None, true, false));
    }

    #[test]
    fn without_splits_the_list_is_what_it_was() {
        let folders = ["/w/b", "/w/a", "/w/b"];
        let none: [Option<i64>; 0] = [];
        let case = by_folder(&folders, &none);
        let rows = idle(3);
        let with = build(&case, &rows);
        let input = ListInput {
            grouping: case.grouping,
            folders: &case.folders,
            pinned_folders: &[],
            rows: &rows,
            pinned: &case.pinned,
            sidebar: &case.sidebar,
            home: None,
            splits: Splits::default(),
        };
        assert_eq!(Listing::build(&input).entries, with.entries);
        assert_eq!(Listing::build(&input).order(), with.order());
        assert_eq!(with.marks(0), SplitMarks::default());
    }
}
