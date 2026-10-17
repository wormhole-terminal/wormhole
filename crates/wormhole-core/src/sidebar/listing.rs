//! The terminal list as the sidebar draws it: terminals in no group and the person's groups in
//! the order they gave them (`arrange`), pinned terminals first, a group's terminals under its
//! head (or folded away), and the ⌘1–9 numbers on the first nine terminals you can see.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::arrange::Item;
use super::groups::{Groups, group_key};
use super::order::{SidebarNumbers, with_pins};
use super::split::{self, SplitMarks, Splits};

/// How many terminals get a ⌘ number.
pub const NUMBERED: usize = 9;

/// The sidebar's state that belongs to one window and is saved with it: the groups the person
/// made in it, the order of the list, and which groups are folded. A window saved by an older
/// build also says how it grouped and filtered; both are gone and ignored.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct WindowSidebar {
    /// Folded groups, by `group_key`.
    pub folded: BTreeSet<String>,
    /// The window's own groups.
    pub groups: Groups,
    /// The top level of the list, as last arranged (`WindowSidebar::arranged`).
    pub items: Vec<Item>,
}

impl WindowSidebar {
    pub fn is_group_folded(&self, group: u32) -> bool {
        self.folded.contains(&group_key(group))
    }

    /// Fold an open group or open a folded one.
    pub fn toggle_group_fold(&mut self, group: u32) {
        let key = group_key(group);
        if !self.folded.remove(&key) {
            self.folded.insert(key);
        }
    }
}

/// What the listing is built from. Terminals are counted by their place in the window.
pub struct ListInput<'a> {
    /// Each terminal's id.
    pub ids: &'a [i64],
    /// Which terminals the person pinned.
    pub pinned: &'a [bool],
    pub sidebar: &'a WindowSidebar,
    /// Who is in which split, and which terminal is selected: what makes a split's members sit
    /// together and show their marks. The default has none, so every terminal stands alone.
    pub splits: Splits<'a>,
    /// What ⌘1–9 count along.
    pub numbers: SidebarNumbers,
}

/// One group's head and what it holds.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Band {
    pub group: u32,
    pub label: String,
    /// It stays as a line of its own with no terminal in it.
    pub pinned: bool,
    /// Every terminal in it, pinned first and a split's members together, folded away or not.
    pub members: Vec<usize>,
    pub folded: bool,
}

impl Band {
    /// What the group is remembered by while folded, and known by across frames.
    pub fn key(&self) -> String {
        group_key(self.group)
    }
}

/// One line of the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Entry {
    /// A group's head. `position` of `of` counts the top level: groups and terminals in no group.
    Band { band: usize, position: usize, of: usize },
    /// A terminal: its place among the lines of its group (or, in no group, among the top level),
    /// from 1, its ⌘ number if it has one, and what it shows of its split (nothing, for a terminal
    /// in none).
    Row {
        session: usize,
        band: Option<usize>,
        position: usize,
        of: usize,
        number: Option<usize>,
        marks: SplitMarks,
    },
}

/// The list top to bottom.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Listing {
    pub bands: Vec<Band>,
    pub entries: Vec<Entry>,
    /// Every terminal, as listed without folding (pins applied).
    order: Vec<usize>,
    /// The terminals that carry a ⌘ number, in number order.
    numbered: Vec<usize>,
}

/// The card a line of the list belongs to: a group's (its head and its rows), or a terminal in no
/// group, which is a card of one row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Card {
    Group(usize),
    Loose(usize),
}

/// A place at the top level: a terminal in no group, or a group.
#[derive(Clone, Copy)]
enum Unit {
    Loose(usize),
    Group(u32),
}

impl Listing {
    /// The list. A split's members sit together in their group (or among the terminals in no
    /// group), pinning one pins all of them, and each row's marks say how it belongs.
    pub fn build(input: &ListInput) -> Self {
        let ListInput { ids, pinned, sidebar, splits, numbers } = *input;
        let count = ids.len();
        let split_ids = split::normalised(splits.split_of, count);
        let pinned = split::pinned_with_splits(pinned, &split_ids, count);
        let is_pinned = |ix: usize| pinned.get(ix).copied().unwrap_or(false);
        let index_of = |id: i64| ids.iter().position(|&known| known == id);

        // The top level: pinned terminals first, then the person's order. The terminals in no
        // group are pinned and gathered by split as one list, then dealt back to their places.
        let arranged = sidebar.arranged(ids);
        let loose: Vec<usize> = arranged
            .iter()
            .filter_map(|item| match *item {
                Item::Terminal(id) => index_of(id),
                Item::Group(_) => None,
            })
            .collect();
        let loose = split::gather_splits(&with_pins(&loose, is_pinned), &split_ids);
        let lead = loose.iter().take_while(|&&ix| is_pinned(ix)).count();
        let mut loose_left = loose[lead..].iter().copied();
        let mut units: Vec<Unit> = loose[..lead].iter().map(|&ix| Unit::Loose(ix)).collect();
        for item in &arranged {
            match *item {
                Item::Terminal(_) => units.extend(loose_left.next().map(Unit::Loose)),
                Item::Group(id) => units.push(Unit::Group(id)),
            }
        }

        let mut listing = Listing::default();
        // The band of each place in `order` (none for a terminal in no group), for telling whether
        // a split is together.
        let mut order_band: Vec<Option<usize>> = Vec::new();
        let of = units.len();
        for (n, unit) in units.iter().enumerate() {
            match *unit {
                Unit::Loose(ix) => {
                    listing.order.push(ix);
                    order_band.push(None);
                    listing.entries.push(Entry::Row {
                        session: ix,
                        band: None,
                        position: n + 1,
                        of,
                        number: None,
                        marks: SplitMarks::default(),
                    });
                }
                Unit::Group(id) => {
                    let Some(group) = sidebar.groups.get(id) else { continue };
                    let members: Vec<usize> = group.members.iter().filter_map(|&id| index_of(id)).collect();
                    let members = split::gather_splits(&with_pins(&members, is_pinned), &split_ids);
                    let band = listing.bands.len();
                    let folded = sidebar.is_group_folded(id);
                    listing.order.extend(&members);
                    order_band.extend(std::iter::repeat_n(Some(band), members.len()));
                    listing.entries.push(Entry::Band { band, position: n + 1, of });
                    if !folded {
                        let in_group = members.len();
                        for (m, &ix) in members.iter().enumerate() {
                            listing.entries.push(Entry::Row {
                                session: ix,
                                band: Some(band),
                                position: m + 1,
                                of: in_group,
                                number: None,
                                marks: SplitMarks::default(),
                            });
                        }
                    }
                    listing.bands.push(Band {
                        group: id,
                        label: group.name.clone(),
                        pinned: group.pinned,
                        members,
                        folded,
                    });
                }
            }
        }
        listing.numbered = match numbers {
            SidebarNumbers::List => listing.visible().take(NUMBERED).collect(),
            // The app freezes the numbers (`FrozenNumbers`) and hands the frozen ones back
            // through `set_numbers`; a folded group's terminals have none either way.
            SidebarNumbers::Terminal => listing.visible().take(NUMBERED).collect(),
        };
        listing.number_rows();
        let mut shown = vec![false; count];
        for ix in listing.visible() {
            shown[ix] = true;
        }
        let marks = split::marks(&listing.order, &order_band, &shown, &split_ids, splits.selected);
        for entry in &mut listing.entries {
            if let Entry::Row { session, marks: own, .. } = entry {
                *own = marks.get(session).copied().unwrap_or_default();
            }
        }
        listing
    }

    /// The ⌘ numbers go to `numbered` (terminals, in number order) instead of the ones the list dealt:
    /// those that stay with their terminal (see `order::FrozenNumbers`).
    pub fn set_numbers(&mut self, numbered: Vec<usize>) {
        self.numbered = numbered;
        self.number_rows();
    }

    fn number_rows(&mut self) {
        let numbered = &self.numbered;
        for entry in &mut self.entries {
            if let Entry::Row { session, number, .. } = entry {
                *number = numbered.iter().position(|ix| ix == session).map(|at| at + 1);
            }
        }
    }

    /// The terminals you can see, top to bottom.
    pub fn visible(&self) -> impl Iterator<Item = usize> + '_ {
        self.entries.iter().filter_map(|entry| match entry {
            Entry::Row { session, .. } => Some(*session),
            Entry::Band { .. } => None,
        })
    }

    /// The terminals that carry a ⌘ number, in number order.
    pub fn numbered(&self) -> Vec<usize> {
        self.numbered.clone()
    }

    /// Every terminal top to bottom, folded ones included (pins applied).
    pub fn order(&self) -> &[usize] {
        &self.order
    }

    /// The terminal `delta` places from `active` (⌘⇧] / ⌘⇧[), wrapping around the terminals you
    /// can see; the whole list when `active` itself is folded away.
    pub fn step(&self, active: usize, delta: isize) -> Option<usize> {
        let visible: Vec<usize> = self.visible().collect();
        let along = if visible.contains(&active) { visible } else { self.order.clone() };
        let place = along.iter().position(|&ix| ix == active)?;
        let next = (place as isize + delta).rem_euclid(along.len() as isize) as usize;
        along.get(next).copied()
    }

    /// What `session`'s row shows of its split (nothing, for a terminal in none or not on screen).
    pub fn marks(&self, session: usize) -> SplitMarks {
        self.entries
            .iter()
            .find_map(|entry| match entry {
                Entry::Row { session: own, marks, .. } if *own == session => Some(*marks),
                _ => None,
            })
            .unwrap_or_default()
    }

    /// The card the line `at` is a stretch of.
    pub fn card(&self, at: usize) -> Option<Card> {
        match *self.entries.get(at)? {
            Entry::Band { band, .. } | Entry::Row { band: Some(band), .. } => Some(Card::Group(band)),
            Entry::Row { session, band: None, .. } => Some(Card::Loose(session)),
        }
    }

    /// Whether the line `at` is the first and the last of its card: a group's head starts its card,
    /// the line before the next card (or the end) ends it, and a terminal in no group is both.
    pub fn card_edges(&self, at: usize) -> (bool, bool) {
        let own = self.card(at);
        let top = at == 0 || self.card(at - 1) != own || matches!(self.entries[at], Entry::Band { .. });
        let bottom = self.card(at + 1) != own || matches!(self.entries.get(at + 1), Some(Entry::Band { .. }));
        (top, bottom)
    }

    /// The band `session` is in.
    pub fn band_of(&self, session: usize) -> Option<usize> {
        self.bands.iter().position(|band| band.members.contains(&session))
    }
}

#[cfg(test)]
mod tests {
    use super::{Card, Entry, ListInput, Listing, WindowSidebar};
    use crate::sidebar::{SidebarNumbers, Splits};

    fn build(n: usize, pinned: &[bool], sidebar: &WindowSidebar) -> Listing {
        let ids: Vec<i64> = (1..=n as i64).collect();
        let pinned: Vec<bool> = (0..n).map(|ix| pinned.get(ix).copied().unwrap_or(false)).collect();
        Listing::build(&ListInput {
            ids: &ids,
            pinned: &pinned,
            sidebar,
            splits: Splits::default(),
            numbers: SidebarNumbers::List,
        })
    }

    /// The list as text: groups as `[name]`, rows as their session index and ⌘ number.
    fn shape(listing: &Listing) -> Vec<String> {
        listing
            .entries
            .iter()
            .map(|entry| match *entry {
                Entry::Band { band, .. } => format!("[{}]", listing.bands[band].label),
                Entry::Row { session, number: Some(n), .. } => format!("{session}:{n}"),
                Entry::Row { session, number: None, .. } => format!("{session}"),
            })
            .collect()
    }

    fn named(sidebar: &mut WindowSidebar, ids: &[i64], name: &str, members: &[i64]) -> u32 {
        let group = sidebar.new_group(ids, members);
        sidebar.groups.rename(group, name);
        group
    }

    #[test]
    fn terminals_in_no_group_and_groups_stand_in_the_order_given() {
        let ids = [1, 2, 3, 4, 5];
        let mut sidebar = WindowSidebar::default();
        named(&mut sidebar, &ids, "Docs", &[2, 3]);
        let listing = build(5, &[], &sidebar);
        // Docs stands where terminal 2 stood: between 1 and 4.
        assert_eq!(shape(&listing), ["0:1", "[Docs]", "1:2", "2:3", "3:4", "4:5"]);
        assert_eq!(listing.order(), [0, 1, 2, 3, 4]);
        assert_eq!(listing.bands[0].members, [1, 2]);
        let Entry::Row { position, of, band, .. } = listing.entries[0] else { panic!("a row") };
        assert_eq!((position, of, band), (1, 4, None), "1 of 4 at the top level: 3 terminals and a group");
        let Entry::Band { position, of, .. } = listing.entries[1] else { panic!("a head") };
        assert_eq!((position, of), (2, 4));
    }

    #[test]
    fn a_folded_group_is_its_head_only_and_takes_no_number() {
        let ids: Vec<i64> = (1..=12).collect();
        let mut sidebar = WindowSidebar::default();
        let group = named(&mut sidebar, &ids, "Docs", &[1, 2]);
        let open = build(12, &[], &sidebar);
        assert_eq!(open.numbered(), [0, 1, 2, 3, 4, 5, 6, 7, 8]);
        sidebar.toggle_group_fold(group);
        let folded = build(12, &[], &sidebar);
        assert_eq!(folded.numbered(), [2, 3, 4, 5, 6, 7, 8, 9, 10], "a folded group takes no number");
        assert_eq!(shape(&folded)[..3], ["[Docs]", "2:1", "3:2"]);
        // Its terminals are still in the order (⌘⇧] from one of them goes on from there).
        assert_eq!(folded.order().len(), 12);
        assert!(folded.bands[0].folded);
    }

    #[test]
    fn pinned_terminals_lead_the_list_and_their_group() {
        let ids = [1, 2, 3, 4];
        let mut sidebar = WindowSidebar::default();
        named(&mut sidebar, &ids, "Docs", &[1, 2, 3]);
        let listing = build(4, &[false, false, true, false], &sidebar);
        assert_eq!(shape(&listing), ["[Docs]", "2:1", "0:2", "1:3", "3:4"]);
        let listing = build(4, &[false, false, false, true], &sidebar);
        assert_eq!(
            shape(&listing),
            ["3:1", "[Docs]", "0:2", "1:3", "2:4"],
            "a loose pin goes above the groups"
        );
    }

    #[test]
    fn a_pinned_empty_group_is_a_head_with_nothing_under_it() {
        let ids = [1, 2];
        let mut sidebar = WindowSidebar::default();
        let docs = named(&mut sidebar, &ids, "Docs", &[1]);
        sidebar.groups.set_pinned(docs, true);
        sidebar.groups.leave(1);
        let listing = build(2, &[], &sidebar);
        assert_eq!(shape(&listing), ["[Docs]", "1:1", "0:2"], "the empty group keeps its place");
        assert!(listing.bands[0].members.is_empty() && listing.bands[0].pinned);
        assert_eq!(listing.band_of(0), None);
    }

    #[test]
    fn every_line_is_a_stretch_of_a_card_and_a_loose_terminal_is_a_card_of_one() {
        let ids = [1, 2, 3, 4];
        let mut sidebar = WindowSidebar::default();
        let docs = named(&mut sidebar, &ids, "Docs", &[2, 3]);
        let listing = build(4, &[], &sidebar);
        // 0 | [Docs] 1 2 | 3
        let edges: Vec<(bool, bool)> = (0..listing.entries.len()).map(|at| listing.card_edges(at)).collect();
        assert_eq!(edges, [(true, true), (true, false), (false, false), (false, true), (true, true)]);
        assert_eq!(listing.card(0), Some(Card::Loose(0)));
        assert_eq!(listing.card(2), Some(Card::Group(0)));
        assert_eq!(listing.card(9), None);
        // Folded, the head is the whole card.
        sidebar.toggle_group_fold(docs);
        let listing = build(4, &[], &sidebar);
        assert_eq!(listing.card_edges(1), (true, true));
        // A pinned group with nothing in it is a head and nothing more.
        let mut sidebar = WindowSidebar::default();
        let pinned = named(&mut sidebar, &ids, "Infra", &[1]);
        sidebar.groups.set_pinned(pinned, true);
        sidebar.groups.leave(1);
        let listing = build(4, &[], &sidebar);
        assert_eq!(listing.card_edges(0), (true, true));
    }

    #[test]
    fn stepping_goes_through_what_is_visible() {
        let ids = [1, 2, 3, 4];
        let mut sidebar = WindowSidebar::default();
        let group = named(&mut sidebar, &ids, "Docs", &[2, 3]);
        sidebar.toggle_group_fold(group);
        let listing = build(4, &[], &sidebar);
        // 0, [Docs folded: 1 and 2], 3 → the folded ones are skipped.
        assert_eq!(listing.step(0, 1), Some(3));
        assert_eq!(listing.step(3, -1), Some(0));
        // From a folded one the whole list is walked.
        assert_eq!(listing.step(1, 1), Some(2));
    }
}
