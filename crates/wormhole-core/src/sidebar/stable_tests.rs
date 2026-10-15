//! The stable list and the numbers that stay with a terminal: what creating a terminal, a `cd`, a
//! status change, the Show filter or a fold may and may not move, in both modes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::order::settle_bands;
use super::{
    Entry, Filter, Grouping, ListInput, Listing, RowState, Searchable, Show, SidebarNumbers, SidebarOrder,
    Splits, WindowSidebar,
};

const NAMES: [&str; 12] = ["a0", "a1", "a2", "a3", "a4", "a5", "a6", "a7", "a8", "a9", "a10", "a11"];

struct World {
    folders: Vec<Option<PathBuf>>,
    states: Vec<RowState>,
    pinned: Vec<bool>,
    sidebar: WindowSidebar,
    bands: Vec<PathBuf>,
}

impl World {
    fn new(folders: &[&str]) -> Self {
        let mut world = World {
            folders: Vec::new(),
            states: Vec::new(),
            pinned: Vec::new(),
            sidebar: WindowSidebar::default(),
            bands: Vec::new(),
        };
        for folder in folders {
            world.create(folder);
        }
        world
    }

    /// A terminal made now: last in the window's order.
    fn create(&mut self, folder: &str) {
        self.folders.push(Some(PathBuf::from(folder)));
        self.states.push(RowState::Idle);
        self.pinned.push(false);
    }

    fn build(&mut self, order: SidebarOrder, numbers: SidebarNumbers) -> Listing {
        // What the app does every time it builds the list.
        self.bands = settle_bands(&self.bands, &self.folders);
        let rows: Vec<Searchable> = self
            .states
            .iter()
            .enumerate()
            .map(|(ix, &state)| Searchable {
                name: NAMES[ix],
                state,
                place: None,
                branch: None,
                command: None,
            })
            .collect();
        Listing::build(&ListInput {
            grouping: Grouping::Folder,
            folders: &self.folders,
            pinned_folders: &[],
            rows: &rows,
            pinned: &self.pinned,
            sidebar: &self.sidebar,
            home: None,
            splits: Splits::default(),
            order,
            numbers,
            band_order: &self.bands,
        })
    }
}

/// Each numbered terminal's number.
fn numbers(listing: &Listing) -> BTreeMap<usize, usize> {
    listing.numbered().into_iter().enumerate().map(|(at, session)| (session, at + 1)).collect()
}

/// The numbers `after` has for the terminals `before` numbered.
fn kept(before: &BTreeMap<usize, usize>, after: &BTreeMap<usize, usize>) -> bool {
    before.iter().all(|(session, number)| after.get(session) == Some(number))
}

fn band_labels(listing: &Listing) -> Vec<&str> {
    listing.bands.iter().map(|band| band.label.as_str()).collect()
}

const STABLE: (SidebarOrder, SidebarNumbers) = (SidebarOrder::Stable, SidebarNumbers::Terminal);
const CLASSIC: (SidebarOrder, SidebarNumbers) = (SidebarOrder::Classic, SidebarNumbers::List);

#[test]
fn a_new_terminal_takes_the_next_number_and_nobody_else_changes() {
    let mut world = World::new(&["/w/a", "/w/b", "/w/a", "/w/c"]);
    let before = numbers(&world.build(STABLE.0, STABLE.1));
    assert_eq!(before.values().copied().collect::<Vec<_>>().len(), 4);
    // In a folder that already has a band, above others in the list.
    world.create("/w/a");
    let after = numbers(&world.build(STABLE.0, STABLE.1));
    assert!(kept(&before, &after), "{before:?} -> {after:?}");
    assert_eq!(after.get(&4), Some(&5), "the new one is the next number");

    // The classic numbers follow the rows: the new terminal sits in band a and pushes c's down.
    let mut world = World::new(&["/w/a", "/w/b", "/w/a", "/w/c"]);
    let before = numbers(&world.build(CLASSIC.0, CLASSIC.1));
    world.create("/w/a");
    let after = numbers(&world.build(CLASSIC.0, CLASSIC.1));
    assert!(!kept(&before, &after));
}

#[test]
fn a_terminal_that_changes_folder_leaves_every_number_and_every_other_band_alone() {
    let mut world = World::new(&["/w/a", "/w/b", "/w/c", "/w/b"]);
    let before = world.build(STABLE.0, STABLE.1);
    assert_eq!(band_labels(&before), ["a", "b", "c"]);
    // The first terminal goes to a folder of its own.
    world.folders[0] = Some(PathBuf::from("/w/z"));
    let after = world.build(STABLE.0, STABLE.1);
    assert_eq!(numbers(&before), numbers(&after));
    assert_eq!(band_labels(&after), ["b", "c", "z"], "the other bands stay in order; the new one is last");
    // It is back: the band it left had been forgotten, so it is last.
    world.folders[0] = Some(PathBuf::from("/w/a"));
    assert_eq!(band_labels(&world.build(STABLE.0, STABLE.1)), ["b", "c", "a"]);

    // Classic orders bands by their first terminal as they are, so they jump.
    let mut world = World::new(&["/w/a", "/w/b", "/w/c", "/w/b"]);
    world.build(CLASSIC.0, CLASSIC.1);
    world.folders[0] = Some(PathBuf::from("/w/z"));
    assert_eq!(band_labels(&world.build(CLASSIC.0, CLASSIC.1)), ["z", "b", "c"]);
}

#[test]
fn a_status_change_moves_nothing() {
    let mut world = World::new(&["/w/a", "/w/b", "/w/a"]);
    let before = world.build(STABLE.0, STABLE.1);
    for state in
        [RowState::Working, RowState::NeedsYou, RowState::Finished, RowState::Failed, RowState::Exited]
    {
        world.states[1] = state;
        let after = world.build(STABLE.0, STABLE.1);
        assert_eq!(after.entries.len(), before.entries.len());
        assert_eq!(after.order(), before.order());
        assert_eq!(numbers(&after), numbers(&before));
    }
}

#[test]
fn the_filter_and_folds_hide_rows_but_not_their_numbers() {
    let mut world = World::new(&["/w/a", "/w/b", "/w/a", "/w/c"]);
    let before = numbers(&world.build(STABLE.0, STABLE.1));
    world.states[2] = RowState::Working;
    world.sidebar.filter = Filter { show: Show::Working, ..Filter::default() };
    let filtered = world.build(STABLE.0, STABLE.1);
    assert_eq!(numbers(&filtered), before, "hidden terminals keep theirs");
    let rows = filtered.entries.iter().filter(|entry| matches!(entry, Entry::Row { .. })).count();
    assert_eq!(rows, 1);
    assert!(filtered.entries.iter().any(|e| matches!(e, Entry::Row { session: 2, number: Some(3), .. })));

    world.sidebar.filter = Filter::default();
    world.sidebar.toggle_fold(Some(Path::new("/w/a")));
    let folded = world.build(STABLE.0, STABLE.1);
    assert_eq!(numbers(&folded), before, "a folded band's terminals keep theirs");

    // Classic: a fold hands the numbers to the next rows.
    let mut world = World::new(&["/w/a", "/w/b", "/w/a", "/w/c"]);
    world.sidebar.toggle_fold(Some(Path::new("/w/a")));
    assert_eq!(world.build(CLASSIC.0, CLASSIC.1).numbered(), [1, 3]);
}

#[test]
fn pins_and_closing_are_the_ways_a_number_changes() {
    let mut world = World::new(&["/w/a", "/w/b", "/w/a"]);
    let before = numbers(&world.build(STABLE.0, STABLE.1));
    world.pinned[2] = true;
    let pinned = numbers(&world.build(STABLE.0, STABLE.1));
    assert_eq!(pinned.get(&2), Some(&1), "pinned first");
    assert_eq!((pinned.get(&0), pinned.get(&1)), (Some(&2), Some(&3)));
    world.pinned[2] = false;
    assert_eq!(numbers(&world.build(STABLE.0, STABLE.1)), before, "unpinned: back where it was");
    // The terminal before it is closed.
    world.folders.remove(0);
    world.states.remove(0);
    world.pinned.remove(0);
    let closed = numbers(&world.build(STABLE.0, STABLE.1));
    assert_eq!(closed.values().copied().collect::<Vec<_>>(), [1, 2]);
}

#[test]
fn only_nine_get_a_number_and_the_tenth_is_none() {
    let mut world = World::new(&["/w/a"; 12]);
    let listing = world.build(STABLE.0, STABLE.1);
    assert_eq!(listing.numbered(), [0, 1, 2, 3, 4, 5, 6, 7, 8]);
    assert!(listing.entries.iter().any(|e| matches!(e, Entry::Row { session: 9, number: None, .. })));
}

#[test]
fn with_no_remembered_order_bands_follow_their_first_terminal() {
    let folders = [Some(PathBuf::from("/w/b")), Some(PathBuf::from("/w/a")), None];
    let rows: Vec<Searchable> = (0..3)
        .map(|ix| Searchable {
            name: NAMES[ix],
            state: RowState::Idle,
            place: None,
            branch: None,
            command: None,
        })
        .collect();
    let listing = Listing::build(&ListInput {
        grouping: Grouping::Folder,
        folders: &folders,
        pinned_folders: &[],
        rows: &rows,
        pinned: &[false; 3],
        sidebar: &WindowSidebar::default(),
        home: None,
        splits: Splits::default(),
        order: SidebarOrder::Stable,
        numbers: SidebarNumbers::Terminal,
        band_order: &[],
    });
    assert_eq!(band_labels(&listing), ["b", "a", "No folder"]);
}
