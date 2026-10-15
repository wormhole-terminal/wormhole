//! The stable list and the numbers that stay with a terminal: what creating a terminal, a `cd`, a
//! status change, the Show filter or a fold may and may not move, in both modes.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use super::order::{FrozenNumbers, settle_bands};
use super::{
    Entry, Filter, Grouping, ListInput, Listing, RowState, Searchable, Show, SidebarNumbers, SidebarOrder,
    Splits, WindowSidebar,
};

const NAMES: [&str; 12] = ["a0", "a1", "a2", "a3", "a4", "a5", "a6", "a7", "a8", "a9", "a10", "a11"];

struct World {
    /// Each terminal's id, in the window's order (an index is a session, an id is who it is).
    ids: Vec<i64>,
    next: i64,
    folders: Vec<Option<PathBuf>>,
    states: Vec<RowState>,
    pinned: Vec<bool>,
    sidebar: WindowSidebar,
    bands: Vec<PathBuf>,
    frozen: FrozenNumbers<i64>,
}

impl World {
    fn new(folders: &[&str]) -> Self {
        let mut world = World {
            ids: Vec::new(),
            next: 1,
            folders: Vec::new(),
            states: Vec::new(),
            pinned: Vec::new(),
            sidebar: WindowSidebar::default(),
            bands: Vec::new(),
            frozen: FrozenNumbers::default(),
        };
        for folder in folders {
            world.create(folder);
        }
        world
    }

    /// A terminal made now: last in the window's order.
    fn create(&mut self, folder: &str) {
        self.ids.push(self.next);
        self.next += 1;
        self.folders.push(Some(PathBuf::from(folder)));
        self.states.push(RowState::Idle);
        self.pinned.push(false);
    }

    /// A terminal closed.
    fn close(&mut self, ix: usize) {
        self.ids.remove(ix);
        self.folders.remove(ix);
        self.states.remove(ix);
        self.pinned.remove(ix);
    }

    /// The list as the app builds it every time: the bands settled, the listing, then the
    /// numbers frozen when they stay with the terminal.
    fn build(&mut self, order: SidebarOrder, numbers: SidebarNumbers) -> Listing {
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
        let mut listing = Listing::build(&ListInput {
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
        });
        if numbers == SidebarNumbers::Terminal {
            let window: Vec<(i64, bool)> =
                self.ids.iter().copied().zip(self.pinned.iter().copied()).collect();
            let dealt: Vec<i64> = listing.order().iter().map(|&ix| self.ids[ix]).collect();
            let held = self.frozen.settle(&window, 0, &dealt, 9);
            let at = |id: &i64| self.ids.iter().position(|known| known == id);
            listing.set_numbers(held.iter().filter_map(at).collect());
        } else {
            self.frozen.reset();
        }
        listing
    }
}

/// Each numbered terminal's number, by its id.
fn numbers(world: &World, listing: &Listing) -> BTreeMap<i64, usize> {
    listing.numbered().into_iter().enumerate().map(|(at, session)| (world.ids[session], at + 1)).collect()
}

/// The numbers `after` has for the terminals `before` numbered.
fn kept(before: &BTreeMap<i64, usize>, after: &BTreeMap<i64, usize>) -> bool {
    before.iter().all(|(session, number)| after.get(session) == Some(number))
}

fn band_labels(listing: &Listing) -> Vec<&str> {
    listing.bands.iter().map(|band| band.label.as_str()).collect()
}

const STABLE: (SidebarOrder, SidebarNumbers) = (SidebarOrder::Stable, SidebarNumbers::Terminal);
const CLASSIC: (SidebarOrder, SidebarNumbers) = (SidebarOrder::Classic, SidebarNumbers::List);

#[test]
fn the_numbers_are_dealt_down_the_list_and_a_new_terminal_takes_the_next_free_one() {
    // alpha: 1, 3; beta: 2; gamma: 4: dealt down the list they read 1, 3 | 2 | 4.
    let mut world = World::new(&["/w/a", "/w/b", "/w/a", "/w/c"]);
    let listing = world.build(STABLE.0, STABLE.1);
    let before = numbers(&world, &listing);
    assert_eq!(before, BTreeMap::from([(1, 1), (3, 2), (2, 3), (4, 4)]));
    // One made in the middle band: it keeps the next free number, nobody else changes.
    world.create("/w/b");
    let listing = world.build(STABLE.0, STABLE.1);
    let after = numbers(&world, &listing);
    assert!(kept(&before, &after), "{before:?} -> {after:?}");
    assert_eq!(after.get(&5), Some(&5));
    // The row says so too.
    assert!(listing.entries.iter().any(|e| matches!(e, Entry::Row { session: 4, number: Some(5), .. })));

    // Following the list, it sits in the middle and pushes the numbers below it.
    let mut world = World::new(&["/w/a", "/w/b", "/w/a", "/w/c"]);
    let listing = world.build(CLASSIC.0, CLASSIC.1);
    let before = numbers(&world, &listing);
    world.create("/w/b");
    let listing = world.build(CLASSIC.0, CLASSIC.1);
    assert!(!kept(&before, &numbers(&world, &listing)));
}

#[test]
fn a_terminal_that_changes_folder_leaves_every_number_and_every_other_band_alone() {
    let mut world = World::new(&["/w/a", "/w/b", "/w/c", "/w/b"]);
    let before = world.build(STABLE.0, STABLE.1);
    assert_eq!(band_labels(&before), ["a", "b", "c"]);
    let numbers_before = numbers(&world, &before);
    // The first terminal goes to a folder of its own.
    world.folders[0] = Some(PathBuf::from("/w/z"));
    let after = world.build(STABLE.0, STABLE.1);
    assert_eq!(numbers_before, numbers(&world, &after), "it keeps ⌘1 in the last band");
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
    let numbers_before = numbers(&world, &before);
    for state in
        [RowState::Working, RowState::NeedsYou, RowState::Finished, RowState::Failed, RowState::Exited]
    {
        world.states[1] = state;
        let after = world.build(STABLE.0, STABLE.1);
        assert_eq!(after.entries.len(), before.entries.len());
        assert_eq!(after.order(), before.order());
        assert_eq!(numbers(&world, &after), numbers_before);
    }
}

#[test]
fn the_filter_and_folds_hide_rows_but_not_their_numbers() {
    let mut world = World::new(&["/w/a", "/w/b", "/w/a", "/w/c"]);
    let first = world.build(STABLE.0, STABLE.1);
    let before = numbers(&world, &first);
    world.states[2] = RowState::Working;
    world.sidebar.filter = Filter { show: Show::Working, ..Filter::default() };
    let filtered = world.build(STABLE.0, STABLE.1);
    assert_eq!(numbers(&world, &filtered), before, "hidden terminals keep theirs");
    let rows = filtered.entries.iter().filter(|entry| matches!(entry, Entry::Row { .. })).count();
    assert_eq!(rows, 1);
    assert!(filtered.entries.iter().any(|e| matches!(e, Entry::Row { session: 2, number: Some(2), .. })));

    world.sidebar.filter = Filter::default();
    world.sidebar.toggle_fold(Some(Path::new("/w/a")));
    let folded = world.build(STABLE.0, STABLE.1);
    assert_eq!(numbers(&world, &folded), before, "a folded band's terminals keep theirs");

    // Classic: a fold hands the numbers to the next rows.
    let mut world = World::new(&["/w/a", "/w/b", "/w/a", "/w/c"]);
    world.sidebar.toggle_fold(Some(Path::new("/w/a")));
    assert_eq!(world.build(CLASSIC.0, CLASSIC.1).numbered(), [1, 3]);
}

#[test]
fn a_pin_a_drag_a_close_and_a_setting_deal_the_numbers_again() {
    let mut world = World::new(&["/w/a", "/w/b", "/w/a", "/w/b"]);
    let listing = world.build(STABLE.0, STABLE.1);
    let dealt = numbers(&world, &listing);
    // A terminal made in the first band, so the frozen numbers differ from a fresh deal.
    world.create("/w/a");
    let listing = world.build(STABLE.0, STABLE.1);
    assert_eq!(numbers(&world, &listing).get(&5), Some(&5));
    // A pin: pinned first within its band, and the numbers are dealt down the list again.
    world.pinned[2] = true;
    let listing = world.build(STABLE.0, STABLE.1);
    let pinned = numbers(&world, &listing);
    assert_eq!(pinned.get(&3), Some(&1), "the pinned terminal leads its band");
    assert_eq!(pinned.get(&5), Some(&3), "and the one made later is dealt its place in the list");
    // Unpinned: dealt again, so the same as a fresh deal of the list, not the old frozen numbers.
    world.pinned[2] = false;
    let listing = world.build(STABLE.0, STABLE.1);
    assert_eq!(numbers(&world, &listing).get(&5), Some(&3));
    assert_ne!(numbers(&world, &listing).get(&5), dealt.get(&5));
    // A terminal closed: no gap.
    world.close(0);
    let listing = world.build(STABLE.0, STABLE.1);
    let mut closed: Vec<usize> = numbers(&world, &listing).values().copied().collect();
    closed.sort_unstable();
    assert_eq!(closed, [1, 2, 3, 4]);
    // Following the list and back deals again.
    world.build(CLASSIC.0, CLASSIC.1);
    let listing = world.build(STABLE.0, STABLE.1);
    let fresh: Vec<i64> = listing.numbered().into_iter().map(|ix| world.ids[ix]).collect();
    let down: Vec<i64> = listing.order().iter().map(|&ix| world.ids[ix]).collect();
    assert_eq!(fresh, down);
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
