//! The numbers that stay with a terminal: what creating a terminal, a status change, a fold or a
//! drag may and may not move.

use std::collections::BTreeMap;

use super::order::FrozenNumbers;
use super::{Entry, Item, ListInput, Listing, SidebarNumbers, Splits, WindowSidebar};

struct World {
    /// Each terminal's id, in the order they were made.
    ids: Vec<i64>,
    next: i64,
    pinned: Vec<bool>,
    sidebar: WindowSidebar,
    frozen: FrozenNumbers<i64>,
}

impl World {
    fn new(n: usize) -> Self {
        let mut world = World {
            ids: Vec::new(),
            next: 1,
            pinned: Vec::new(),
            sidebar: WindowSidebar::default(),
            frozen: FrozenNumbers::default(),
        };
        for _ in 0..n {
            world.create();
        }
        world
    }

    /// A terminal made now: last in the order they were made, and last in the list.
    fn create(&mut self) -> i64 {
        self.ids.push(self.next);
        self.next += 1;
        self.pinned.push(false);
        self.next - 1
    }

    /// A terminal closed.
    fn close(&mut self, ix: usize) {
        let id = self.ids.remove(ix);
        self.pinned.remove(ix);
        self.sidebar.groups.retain_terminals(|known| known != id);
    }

    /// The list as the app builds it every time: the listing, then the numbers frozen when they
    /// stay with the terminal. `tag` stands for what re-deals them (here the folds and the rest).
    fn build(&mut self, numbers: SidebarNumbers) -> Listing {
        let mut listing = Listing::build(&ListInput {
            ids: &self.ids,
            pinned: &self.pinned,
            sidebar: &self.sidebar,
            splits: Splits::default(),
            numbers,
        });
        if numbers == SidebarNumbers::Terminal {
            let window: Vec<(i64, bool)> =
                self.ids.iter().copied().zip(self.pinned.iter().copied()).collect();
            let dealt: Vec<i64> = listing.visible().map(|ix| self.ids[ix]).collect();
            let tag = self.sidebar.folded.len() as u64;
            let held = self.frozen.settle(&window, tag, &dealt, 9);
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

const STAY: SidebarNumbers = SidebarNumbers::Terminal;
const FOLLOW: SidebarNumbers = SidebarNumbers::List;

#[test]
fn a_new_terminal_takes_the_next_free_number_and_nobody_else_changes() {
    let mut world = World::new(4);
    let ids = world.ids.clone();
    world.sidebar.new_group(&ids, &[2, 3]);
    let listing = world.build(STAY);
    let before = numbers(&world, &listing);
    assert_eq!(before, BTreeMap::from([(1, 1), (2, 2), (3, 3), (4, 4)]));
    // One made later joins the end of the list with the next free number.
    world.create();
    let listing = world.build(STAY);
    let after = numbers(&world, &listing);
    assert!(kept(&before, &after), "{before:?} -> {after:?}");
    assert_eq!(after.get(&5), Some(&5));
    assert!(listing.entries.iter().any(|e| matches!(e, Entry::Row { session: 4, number: Some(5), .. })));
}

#[test]
fn a_status_change_moves_nothing() {
    // Nothing the listing is built from changes with a terminal's status, so the same input is
    // the same list and the same numbers.
    let mut world = World::new(3);
    let first = world.build(STAY);
    let again = world.build(STAY);
    assert_eq!(first.entries, again.entries);
    assert_eq!(numbers(&world, &first), numbers(&world, &again));
}

#[test]
fn a_fold_hands_its_numbers_on_and_opening_takes_them_back() {
    let mut world = World::new(5);
    let ids = world.ids.clone();
    let group = world.sidebar.new_group(&ids, &[1, 2]);
    let listing = world.build(STAY);
    let open = numbers(&world, &listing);
    assert_eq!(open.get(&1), Some(&1));
    world.sidebar.toggle_group_fold(group);
    let folded = world.build(STAY);
    let shut = numbers(&world, &folded);
    assert_eq!((shut.get(&1), shut.get(&2)), (None, None), "a folded group's terminals have none");
    assert_eq!(shut.get(&3), Some(&1), "the next ones take them");
    world.sidebar.toggle_group_fold(group);
    let listing = world.build(STAY);
    assert_eq!(numbers(&world, &listing), open);
    // Following the list says the same.
    world.sidebar.toggle_group_fold(group);
    assert_eq!(world.build(FOLLOW).numbered(), [2, 3, 4]);
}

#[test]
fn a_pin_a_drag_a_close_and_a_setting_deal_the_numbers_again() {
    let mut world = World::new(4);
    let listing = world.build(STAY);
    let dealt = numbers(&world, &listing);
    world.create();
    let listing = world.build(STAY);
    assert_eq!(numbers(&world, &listing).get(&5), Some(&5));
    // A pin: pinned first, and the numbers are dealt down the list again.
    world.pinned[2] = true;
    let listing = world.build(STAY);
    let pinned = numbers(&world, &listing);
    assert_eq!(pinned.get(&3), Some(&1), "the pinned terminal leads");
    // Unpinned: dealt again, so the same as a fresh deal of the list.
    world.pinned[2] = false;
    let listing = world.build(STAY);
    assert_eq!(numbers(&world, &listing).get(&5), Some(&5));
    assert_eq!(numbers(&world, &listing).get(&1), dealt.get(&1));
    // A terminal closed: no gap.
    world.close(0);
    let listing = world.build(STAY);
    let mut closed: Vec<usize> = numbers(&world, &listing).values().copied().collect();
    closed.sort_unstable();
    assert_eq!(closed, [1, 2, 3, 4]);
    // A drag in the list reaches the numbers through the order it gives: dealt top to bottom.
    let ids = world.ids.clone();
    world.sidebar.place_at_top(&ids, Item::Terminal(5), Some(Item::Terminal(2)));
    world.frozen.reset();
    let listing = world.build(STAY);
    assert_eq!(numbers(&world, &listing).get(&5), Some(&1));
}

#[test]
fn only_nine_get_a_number_and_the_tenth_is_none() {
    let mut world = World::new(12);
    let listing = world.build(STAY);
    assert_eq!(listing.numbered(), [0, 1, 2, 3, 4, 5, 6, 7, 8]);
    assert!(listing.entries.iter().any(|e| matches!(e, Entry::Row { session: 9, number: None, .. })));
}
