//! Where a drop in the terminal list puts what was dragged. The pointer is over a terminal's row
//! (its top or bottom quarter reorders, its middle joins a split), a group's head (the top
//! quarter stands before the group, the rest joins it), the gap under a card (it stands after
//! the card, out of any group) or the empty space under the list (last). The answer is a
//! `Landing` in the arrangement (`arrange`), or a join of two terminals in a split.

use super::arrange::Item;
use super::drop::{Part, RowDrop, resolve_drop};
use super::listing::{Card, Listing};
use super::split::SplitId;

/// A place in the arrangement: at the top level, before an item (last without one), or in a group,
/// before one of its terminals (last without one).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Landing {
    Top { before: Option<Item> },
    Group { group: u32, before: Option<i64> },
}

/// What the pointer is over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    /// The terminal at this place in the window, on this part of its row.
    Row { onto: usize, part: Part },
    /// The head of this band of the listing, on this part of it.
    Head { band: usize, part: Part },
    /// The gap under a card.
    After(Card),
    /// The space under the last line.
    Tail,
}

/// What dropping a terminal does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Drop {
    /// It moves to `landing`. `leave_split` when it gets out of its split on the way;
    /// `with_mates` when the rest of its split moves with it (into or out of a group), else it
    /// moves alone and the list gathers the split round wherever its first member stands.
    Move { landing: Landing, leave_split: bool, with_mates: bool },
    /// It makes a split with this terminal, or joins its split.
    Join { target: usize },
}

/// What the drop is worked out from.
pub struct DropInput<'a> {
    pub listing: &'a Listing,
    /// Each terminal's id.
    pub ids: &'a [i64],
    /// Each terminal's split, as the listing was built.
    pub split_of: &'a [Option<SplitId>],
    pub pinned: &'a [bool],
}

impl DropInput<'_> {
    fn is_pinned(&self, ix: usize) -> bool {
        self.pinned.get(ix).copied().unwrap_or(false)
    }

    /// Whether terminal `ix` is in a split (of at least two terminals): dropped away from the
    /// rows, it gets out of it.
    fn in_split(&self, ix: usize) -> bool {
        let Some(id) = super::split_of(self.split_of, ix) else { return false };
        (0..self.ids.len()).filter(|&other| super::split_of(self.split_of, other) == Some(id)).count() > 1
    }

    /// The top level as listed: each terminal in no group and each group, top to bottom.
    fn top(&self) -> Vec<Item> {
        self.listing
            .entries
            .iter()
            .filter_map(|entry| match *entry {
                super::Entry::Band { band, .. } => Some(Item::Group(self.listing.bands[band].group)),
                super::Entry::Row { session, band: None, .. } => Some(Item::Terminal(self.ids[session])),
                super::Entry::Row { .. } => None,
            })
            .collect()
    }

    /// What follows the card at the top level.
    fn after(&self, card: Card) -> Option<Item> {
        let own = match card {
            Card::Group(band) => Item::Group(self.listing.bands.get(band)?.group),
            Card::Loose(session) => Item::Terminal(*self.ids.get(session)?),
        };
        let top = self.top();
        let at = top.iter().position(|&item| item == own)?;
        top.get(at + 1).copied()
    }

    /// Place `at` among `len` others, pushed out of the other kind's block: pinned terminals lead
    /// their group (or the list), `lead` of them, so a pinned one goes no further than the end of
    /// them and an unpinned one no nearer than the end of them.
    fn clamped(&self, lead: usize, len: usize, at: usize, dragged: usize) -> usize {
        if self.is_pinned(dragged) { at.min(lead) } else { at.max(lead.min(len)) }
    }

    /// The landing for a drop that puts `dragged` at slot `to` of the list without it
    /// (`RowDrop`), in the container of `onto`.
    fn landing(&self, dragged: usize, onto: usize, to: usize) -> Landing {
        let listing = self.listing;
        let rest: Vec<usize> = listing.order().iter().copied().filter(|&ix| ix != dragged).collect();
        match listing.band_of(onto) {
            Some(band) => {
                let members: Vec<usize> =
                    listing.bands[band].members.iter().copied().filter(|&ix| ix != dragged).collect();
                let next = rest.get(to).copied().filter(|ix| members.contains(ix));
                let at = next.and_then(|ix| members.iter().position(|&m| m == ix)).unwrap_or(members.len());
                let lead = members.iter().take_while(|&&ix| self.is_pinned(ix)).count();
                let at = self.clamped(lead, members.len(), at, dragged);
                Landing::Group {
                    group: listing.bands[band].group,
                    before: members.get(at).map(|&ix| self.ids[ix]),
                }
            }
            None => {
                let own = Item::Terminal(self.ids[dragged]);
                let top: Vec<Item> = self.top().into_iter().filter(|&item| item != own).collect();
                let next = rest.get(to).map(|&ix| match listing.band_of(ix) {
                    Some(band) => Item::Group(listing.bands[band].group),
                    None => Item::Terminal(self.ids[ix]),
                });
                let at =
                    next.and_then(|item| top.iter().position(|&known| known == item)).unwrap_or(top.len());
                let lead = top
                    .iter()
                    .take_while(|item| matches!(item, Item::Terminal(id) if self.is_id_pinned(*id)))
                    .count();
                let at = self.clamped(lead, top.len(), at, dragged);
                Landing::Top { before: top.get(at).copied() }
            }
        }
    }

    fn is_id_pinned(&self, id: i64) -> bool {
        self.ids.iter().position(|&known| known == id).is_some_and(|ix| self.is_pinned(ix))
    }
}

/// What dropping terminal `dragged` (its place in the window) on `target` does; `None` when
/// nothing would change.
pub fn terminal_drop(input: &DropInput, dragged: usize, target: Target) -> Option<Drop> {
    let listing = input.listing;
    match target {
        Target::Row { onto, part } => {
            let across = listing.band_of(dragged) != listing.band_of(onto);
            let drop = resolve_drop(listing.order(), input.split_of, dragged, onto, part, across)?;
            Some(match drop {
                RowDrop::Join { target, .. } => Drop::Join { target },
                RowDrop::Reorder { to } => Drop::Move {
                    landing: input.landing(dragged, onto, to),
                    leave_split: false,
                    with_mates: across,
                },
                RowDrop::Leave { to } => Drop::Move {
                    landing: input.landing(dragged, onto, to),
                    leave_split: true,
                    with_mates: false,
                },
            })
        }
        Target::Head { band, part } => {
            let group = listing.bands.get(band)?;
            if part == Part::Top {
                return Some(Drop::Move {
                    landing: Landing::Top { before: Some(Item::Group(group.group)) },
                    leave_split: false,
                    with_mates: true,
                });
            }
            // An open group takes it at the top, under the head; a folded or empty one at its end.
            let rest: Vec<usize> = group.members.iter().copied().filter(|&ix| ix != dragged).collect();
            let lead = rest.iter().take_while(|&&ix| input.is_pinned(ix)).count();
            let at = if group.folded { rest.len() } else { 0 };
            let at = input.clamped(lead, rest.len(), at, dragged);
            Some(Drop::Move {
                landing: Landing::Group { group: group.group, before: rest.get(at).map(|&ix| input.ids[ix]) },
                leave_split: false,
                with_mates: true,
            })
        }
        Target::After(card) => {
            if card == Card::Loose(dragged) {
                return None;
            }
            let landing = Landing::Top { before: input.after(card) };
            let leave_split = input.in_split(dragged);
            Some(Drop::Move { landing, leave_split, with_mates: !leave_split })
        }
        Target::Tail => {
            let last = input.top().last().copied();
            if last == Some(Item::Terminal(input.ids[dragged]))
                && listing.band_of(dragged).is_none()
                && !input.in_split(dragged)
            {
                return None;
            }
            Some(Drop::Move {
                landing: Landing::Top { before: None },
                leave_split: input.in_split(dragged),
                with_mates: false,
            })
        }
    }
}

/// Where dropping group `band` (its place in the listing) on `target` puts it: always at the top
/// level. Over another group the pointer in its upper half stands before it, the lower half
/// after; over a terminal in no group, its top half before it, the bottom half after.
pub fn group_drop(input: &DropInput, band: usize, target: Target) -> Option<Landing> {
    let listing = input.listing;
    let own = Item::Group(listing.bands.get(band)?.group);
    let top = input.top();
    let item_at = |item: Item| top.iter().position(|&known| known == item);
    let own_at = item_at(own)?;
    let before_item = |at: usize, after: bool| -> Option<Item> {
        let next = at + usize::from(after);
        top.iter().enumerate().find(|&(n, &item)| n >= next && item != own).map(|(_, &item)| item)
    };
    let landing = match target {
        Target::Row { onto, part } => match listing.band_of(onto) {
            Some(other) if other == band => return None,
            Some(other) => {
                let group = listing.bands[other].group;
                let rows = &listing.bands[other].members;
                let place = rows.iter().position(|&ix| ix == onto).unwrap_or(0);
                let after = place * 2 >= rows.len() && part != Part::Top || place * 2 > rows.len();
                before_item(item_at(Item::Group(group))?, after)
            }
            None => {
                let at = item_at(Item::Terminal(input.ids[onto]))?;
                before_item(at, part == Part::Bottom || part == Part::Middle)
            }
        },
        Target::Head { band: other, part } => {
            if other == band {
                return None;
            }
            let at = item_at(Item::Group(listing.bands.get(other)?.group))?;
            before_item(at, part == Part::Bottom)
        }
        Target::After(card) => {
            let card_item = match card {
                Card::Group(other) if other == band => return None,
                Card::Group(other) => Item::Group(listing.bands.get(other)?.group),
                Card::Loose(session) => Item::Terminal(*input.ids.get(session)?),
            };
            before_item(item_at(card_item)?, true)
        }
        Target::Tail => None,
    };
    // Dropped where it already is: before its own next neighbour.
    let current_next = top.get(own_at + 1).copied();
    (landing != current_next || matches!(target, Target::Tail) && current_next.is_some())
        .then_some(Landing::Top { before: landing })
}

#[cfg(test)]
mod tests {
    use super::{Drop, DropInput, Landing, Target, group_drop, terminal_drop};
    use crate::sidebar::{Card, Item, ListInput, Listing, Part, SidebarNumbers, Splits, WindowSidebar};

    struct World {
        ids: Vec<i64>,
        pinned: Vec<bool>,
        sidebar: WindowSidebar,
        splits: Vec<Option<i64>>,
    }

    impl World {
        fn new(n: i64) -> Self {
            let ids: Vec<i64> = (1..=n).collect();
            World {
                pinned: vec![false; ids.len()],
                splits: vec![None; ids.len()],
                ids,
                sidebar: WindowSidebar::default(),
            }
        }

        fn group(&mut self, members: &[i64]) -> u32 {
            let ids = self.ids.clone();
            self.sidebar.new_group(&ids, members)
        }

        fn listing(&self) -> Listing {
            Listing::build(&ListInput {
                ids: &self.ids,
                pinned: &self.pinned,
                sidebar: &self.sidebar,
                splits: Splits { split_of: &self.splits, selected: None },
                numbers: SidebarNumbers::List,
            })
        }

        fn drop(&self, dragged: usize, target: Target) -> Option<Drop> {
            let listing = self.listing();
            terminal_drop(
                &DropInput {
                    listing: &listing,
                    ids: &self.ids,
                    split_of: &self.splits,
                    pinned: &self.pinned,
                },
                dragged,
                target,
            )
        }

        /// Where the drop lands the terminal, if it moves it.
        fn lands(&self, dragged: usize, target: Target) -> Option<Landing> {
            match self.drop(dragged, target)? {
                Drop::Move { landing, .. } => Some(landing),
                Drop::Join { .. } => None,
            }
        }

        fn group_drop(&self, band: usize, target: Target) -> Option<Landing> {
            let listing = self.listing();
            group_drop(
                &DropInput {
                    listing: &listing,
                    ids: &self.ids,
                    split_of: &self.splits,
                    pinned: &self.pinned,
                },
                band,
                target,
            )
        }
    }

    use Item::{Group, Terminal};

    fn moves(landing: Landing) -> Option<Landing> {
        Some(landing)
    }

    #[test]
    fn dropped_on_the_top_of_a_row_of_a_group_it_joins_the_group_before_it() {
        // 0 | [A] 1 2 | 3
        let mut world = World::new(4);
        let a = world.group(&[2, 3]);
        let drop = world.lands(0, Target::Row { onto: 2, part: Part::Top });
        assert_eq!(drop, moves(Landing::Group { group: a, before: Some(3) }));
        let drop = world.lands(0, Target::Row { onto: 1, part: Part::Top });
        assert_eq!(
            drop,
            moves(Landing::Group { group: a, before: Some(2) }),
            "the first row, though it keeps its place"
        );
        let drop = world.lands(0, Target::Row { onto: 2, part: Part::Bottom });
        assert_eq!(drop, moves(Landing::Group { group: a, before: None }), "after the last");
    }

    #[test]
    fn dropped_on_a_loose_terminal_it_stands_at_the_top_level_by_it() {
        // 0 | [A] 1 2 | 3
        let mut world = World::new(4);
        let a = world.group(&[2, 3]);
        let _ = a;
        // 1 is in the group; dropped below the loose 3 it leaves, last.
        let drop = world.lands(1, Target::Row { onto: 3, part: Part::Bottom });
        assert_eq!(drop, moves(Landing::Top { before: None }));
        let drop = world.lands(1, Target::Row { onto: 0, part: Part::Bottom });
        assert_eq!(
            drop,
            moves(Landing::Top { before: Some(Group(1)) }),
            "between the terminal and the group"
        );
        let drop = world.lands(1, Target::Row { onto: 0, part: Part::Top });
        assert_eq!(drop, moves(Landing::Top { before: Some(Terminal(1)) }));
    }

    #[test]
    fn dropped_in_the_gap_under_a_card_it_leaves_the_group_and_stays_there() {
        let mut world = World::new(4);
        let a = world.group(&[2, 3]);
        let drop = world.lands(1, Target::After(Card::Group(0)));
        assert_eq!(drop, moves(Landing::Top { before: Some(Terminal(4)) }));
        assert_eq!(world.lands(3, Target::After(Card::Loose(3))), None, "on its own card");
        let _ = a;
        assert_eq!(world.lands(0, Target::Tail), moves(Landing::Top { before: None }));
        assert_eq!(world.lands(3, Target::Tail), None, "already last");
    }

    #[test]
    fn dropped_on_a_head_it_joins_or_stands_before_the_group() {
        let mut world = World::new(4);
        let a = world.group(&[2, 3]);
        assert_eq!(
            world.lands(0, Target::Head { band: 0, part: Part::Top }),
            moves(Landing::Top { before: Some(Group(a)) })
        );
        assert_eq!(
            world.lands(0, Target::Head { band: 0, part: Part::Middle }),
            moves(Landing::Group { group: a, before: Some(2) }),
            "an open group takes it under its head"
        );
        world.sidebar.toggle_group_fold(a);
        assert_eq!(
            world.lands(0, Target::Head { band: 0, part: Part::Middle }),
            moves(Landing::Group { group: a, before: None }),
            "a folded one at its end"
        );
    }

    #[test]
    fn a_pinned_terminal_stays_among_the_pinned() {
        let mut world = World::new(4);
        let a = world.group(&[1, 2, 3]);
        world.pinned[0] = true;
        // 1 is pinned (first); dragging 3 above it lands after the pinned.
        let drop = world.lands(2, Target::Row { onto: 0, part: Part::Top });
        assert_eq!(drop, moves(Landing::Group { group: a, before: Some(2) }));
        // And a pinned one dragged to the end stays in front of the unpinned.
        let drop = world.lands(0, Target::Row { onto: 2, part: Part::Bottom });
        assert_eq!(drop, moves(Landing::Group { group: a, before: Some(2) }));
    }

    #[test]
    fn the_middle_of_a_row_joins_a_split_and_leaving_one_is_said() {
        let mut world = World::new(3);
        assert_eq!(
            world.drop(0, Target::Row { onto: 2, part: Part::Middle }),
            Some(Drop::Join { target: 2 })
        );
        world.splits = vec![Some(7), Some(7), None];
        let leaving = world.drop(0, Target::Row { onto: 2, part: Part::Bottom });
        assert_eq!(
            leaving,
            Some(Drop::Move { landing: Landing::Top { before: None }, leave_split: true, with_mates: false }),
            "dropped outside the bracket it left"
        );
    }

    #[test]
    fn a_group_dropped_over_another_stands_before_or_after_it() {
        // [A] 0 1 | 2 | [B] 3 4
        let mut world = World::new(5);
        let _a = world.group(&[1, 2]);
        let b = world.group(&[4, 5]);
        assert_eq!(
            world.group_drop(0, Target::Head { band: 1, part: Part::Top }),
            Some(Landing::Top { before: Some(Group(b)) }),
            "the terminal between them stays behind"
        );
        assert_eq!(
            world.group_drop(0, Target::Head { band: 1, part: Part::Bottom }),
            Some(Landing::Top { before: None }),
            "after B, the end"
        );
        assert_eq!(
            world.group_drop(1, Target::Head { band: 0, part: Part::Top }),
            Some(Landing::Top { before: Some(Group(1)) })
        );
        assert_eq!(
            world.group_drop(0, Target::Row { onto: 2, part: Part::Top }),
            None,
            "before the terminal after it: where it is"
        );
        assert_eq!(
            world.group_drop(0, Target::Row { onto: 2, part: Part::Bottom }),
            Some(Landing::Top { before: Some(Group(b)) })
        );
        assert_eq!(world.group_drop(0, Target::Tail), Some(Landing::Top { before: None }));
        assert_eq!(world.group_drop(1, Target::Tail), None);
    }
}
