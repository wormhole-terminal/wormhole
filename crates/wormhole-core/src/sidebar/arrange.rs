//! The order of the terminal list: terminals in no group and groups, side by side in the order the
//! person gave them, the way a browser lays out its tabs and tab groups. A group's terminals are
//! its own (`Groups`, in the order they sit in it), so the top level only holds the terminals in no
//! group and the groups, and a drag changes one of three things: where a loose terminal or a
//! group stands among the others, which terminal of a group comes first, or which group a
//! terminal is in.
//!
//! The order is saved with the window (`WindowSidebar::items`) as it was last changed; what it
//! doesn't know yet (a terminal made since, a window saved before groups could be placed) comes
//! after, loose terminals first and then the groups, so nothing is ever lost or reshuffled.

use serde::{Deserialize, Serialize};

use super::listing::WindowSidebar;
use super::place::Landing;

/// One thing at the top level of the list.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Item {
    /// A terminal in no group, by its id.
    Terminal(i64),
    /// A group, by its id.
    Group(u32),
}

impl WindowSidebar {
    /// The top level as listed: the saved order without what no longer exists (a closed terminal,
    /// a terminal that is in a group now, a group that went), then what it doesn't know yet.
    /// `ids` are the window's terminals, in the order they were made.
    pub fn arranged(&self, ids: &[i64]) -> Vec<Item> {
        let mut items: Vec<Item> = Vec::with_capacity(ids.len());
        let known = |item: &Item, items: &[Item]| items.contains(item);
        for &item in &self.items {
            let alive = match item {
                Item::Terminal(id) => ids.contains(&id) && self.groups.group_of(id).is_none(),
                Item::Group(id) => self.groups.get(id).is_some(),
            };
            if alive && !known(&item, &items) {
                items.push(item);
            }
        }
        for &id in ids {
            let item = Item::Terminal(id);
            if self.groups.group_of(id).is_none() && !known(&item, &items) {
                items.push(item);
            }
        }
        for group in self.groups.iter() {
            let item = Item::Group(group.id);
            if !known(&item, &items) {
                items.push(item);
            }
        }
        items
    }

    /// Keep the order as it is listed now, so a change below starts from what was on screen.
    fn settle(&mut self, ids: &[i64]) -> Vec<Item> {
        self.items = self.arranged(ids);
        self.items.clone()
    }

    /// Put `item` at the top level, right before `before` (or last without one). A terminal in a
    /// group leaves it, and the group goes with its last terminal unless it is pinned.
    pub fn place_at_top(&mut self, ids: &[i64], item: Item, before: Option<Item>) {
        self.settle(ids);
        if let Item::Terminal(id) = item {
            self.groups.leave(id);
        }
        self.items.retain(|&known| known != item);
        let at = before.and_then(|before| self.items.iter().position(|&known| known == before));
        self.items.insert(at.unwrap_or(self.items.len()), item);
        self.items = self.arranged(ids);
    }

    /// Put terminal `id` in group `group`, right before the group's terminal `before` (or last
    /// without one). `false` if there is no such group.
    pub fn place_in_group(&mut self, ids: &[i64], id: i64, group: u32, before: Option<i64>) -> bool {
        if self.groups.get(group).is_none() {
            return false;
        }
        self.settle(ids);
        self.groups.join(group, id);
        if let Some(group) = self.groups.get_mut(group) {
            group.members.retain(|&member| member != id);
            let at = before.and_then(|before| group.members.iter().position(|&member| member == before));
            group.members.insert(at.unwrap_or(group.members.len()), id);
        }
        self.items = self.arranged(ids);
        true
    }

    /// Seat `joiner`, who just joined a split, with the split's other members `mates` (in the order
    /// the list shows them): into their group, right after the last of them, or out of any group
    /// when they are in none, so a split always stands in one card. Does nothing without mates.
    pub fn seat_beside(&mut self, ids: &[i64], joiner: i64, mates: &[i64]) {
        let list = self.arranged(ids);
        let Some(&anchor) = mates.iter().filter(|&&m| m != joiner).last() else { return };
        if let Some(group) = self.groups.group_of(anchor) {
            self.settle(ids);
            self.groups.join(group, joiner);
            if let Some(group) = self.groups.get_mut(group) {
                group.members.retain(|&member| member != joiner);
                // After the last mate in the group's own order.
                let at = group
                    .members
                    .iter()
                    .rposition(|m| mates.contains(m))
                    .map_or(group.members.len(), |p| p + 1);
                group.members.insert(at, joiner);
            }
            self.items = self.arranged(ids);
        } else if self.groups.group_of(joiner).is_some() {
            // Loose members are gathered at the first one's place by the list; leave right after the last.
            let after = list.iter().position(|&i| i == Item::Terminal(anchor));
            let before =
                after.and_then(|p| list.get(p + 1).copied()).filter(|&i| i != Item::Terminal(joiner));
            self.place_at_top(ids, Item::Terminal(joiner), before);
        }
    }

    /// Put terminal `id` where a drop landed it.
    pub fn land(&mut self, ids: &[i64], id: i64, landing: Landing) {
        match landing {
            Landing::Top { before } => self.place_at_top(ids, Item::Terminal(id), before),
            Landing::Group { group, before } => {
                self.place_in_group(ids, id, group, before);
            }
        }
    }

    /// Put group `group` where a drop landed it (a group only ever stands at the top level).
    pub fn land_group(&mut self, ids: &[i64], group: u32, landing: Landing) {
        if let Landing::Top { before } = landing {
            self.place_at_top(ids, Item::Group(group), before);
        }
    }

    /// The terminals in `order` take the places that the same terminals hold now, in that order
    /// (a split's panes were moved: its rows follow them). Only terminals that stand side by side
    /// in one place, the top level or one group, can swap; `false` and nothing changed otherwise.
    pub fn reorder_terminals(&mut self, ids: &[i64], order: &[i64]) -> bool {
        let top = self.settle(ids);
        let slots: Vec<usize> =
            order.iter().filter_map(|&id| top.iter().position(|&item| item == Item::Terminal(id))).collect();
        if slots.len() == order.len() {
            let mut sorted = slots.clone();
            sorted.sort_unstable();
            for (&slot, &id) in sorted.iter().zip(order) {
                self.items[slot] = Item::Terminal(id);
            }
            return true;
        }
        let Some(group) = order.first().and_then(|&id| self.groups.group_of(id)) else { return false };
        let Some(members) = self.groups.get_mut(group) else { return false };
        let slots: Vec<usize> =
            order.iter().filter_map(|id| members.members.iter().position(|m| m == id)).collect();
        if slots.len() != order.len() {
            return false;
        }
        let mut sorted = slots;
        sorted.sort_unstable();
        for (&slot, &id) in sorted.iter().zip(order) {
            members.members[slot] = id;
        }
        true
    }

    /// A new group of `terminals` (from wherever they were), standing where the first of them
    /// stood (after its group, if it was in one). Its id.
    pub fn new_group(&mut self, ids: &[i64], terminals: &[i64]) -> u32 {
        let before = self.settle(ids);
        let anchor = terminals.first().map(|&id| match self.groups.group_of(id) {
            Some(group) => Item::Group(group),
            None => Item::Terminal(id),
        });
        let group = self.groups.create(terminals);
        let mut items = Vec::with_capacity(before.len() + 1);
        for item in before {
            match item {
                Item::Terminal(id) if terminals.contains(&id) => {}
                _ => items.push(item),
            }
            if Some(item) == anchor {
                // A loose terminal's place is the group's; a group's is just before it.
                if matches!(item, Item::Terminal(_)) {
                    items.push(Item::Group(group));
                } else {
                    items.push(item);
                    items.push(Item::Group(group));
                }
            }
        }
        self.items = items;
        self.items = self.arranged(ids);
        group
    }

    /// Take group `group` apart: its terminals stand where it stood, in the order they had in it.
    pub fn ungroup(&mut self, ids: &[i64], group: u32) {
        let before = self.settle(ids);
        let members = self.groups.get(group).map(|group| group.members.clone()).unwrap_or_default();
        self.groups.remove(group);
        self.folded.remove(&super::groups::group_key(group));
        let mut items = Vec::with_capacity(before.len() + members.len());
        for item in before {
            if item == Item::Group(group) {
                items.extend(members.iter().map(|&id| Item::Terminal(id)));
            } else {
                items.push(item);
            }
        }
        self.items = items;
        self.items = self.arranged(ids);
    }
}

#[cfg(test)]
mod tests {
    use super::Item::{Group, Terminal};
    use crate::sidebar::WindowSidebar;

    const IDS: [i64; 5] = [1, 2, 3, 4, 5];

    #[test]
    fn a_terminal_joining_a_split_goes_into_the_splits_group_after_its_last_member() {
        let mut sidebar = view();
        let g = sidebar.groups.create(&[1, 2, 3]);
        sidebar.seat_beside(&IDS, 5, &[1, 2]);
        assert_eq!(sidebar.groups.get(g).map(|g| g.members.clone()), Some(vec![1, 2, 5, 3]));
        sidebar.seat_beside(&IDS, 4, &[1]);
        assert_eq!(sidebar.groups.get(g).map(|g| g.members.clone()), Some(vec![1, 4, 2, 5, 3]));
    }

    #[test]
    fn a_terminal_joining_a_loose_split_leaves_its_group() {
        let mut sidebar = view();
        let g = sidebar.groups.create(&[4, 5]);
        sidebar.seat_beside(&IDS, 4, &[1, 2]);
        assert_eq!(sidebar.groups.group_of(4), None);
        assert_eq!(sidebar.groups.group_of(5), Some(g));
        sidebar.seat_beside(&IDS, 5, &[]);
        assert_eq!(sidebar.groups.group_of(5), Some(g), "no mates, no move");
    }

    fn view() -> WindowSidebar {
        WindowSidebar::default()
    }

    #[test]
    fn a_window_that_never_placed_anything_lists_terminals_then_groups() {
        let mut sidebar = view();
        let g = sidebar.groups.create(&[2, 4]);
        assert_eq!(
            sidebar.arranged(&IDS),
            [Terminal(1), Terminal(3), Terminal(5), Group(g)],
            "as an old window with groups was drawn"
        );
    }

    #[test]
    fn what_is_gone_is_dropped_and_what_is_new_comes_last() {
        let mut sidebar = view();
        sidebar.items = vec![Terminal(3), Terminal(9), Group(7), Terminal(1), Terminal(3)];
        assert_eq!(
            sidebar.arranged(&[1, 2, 3]),
            [Terminal(3), Terminal(1), Terminal(2)],
            "9 and group 7 don't exist, 3 twice is once, 2 is new"
        );
        let g = sidebar.groups.create(&[3]);
        assert_eq!(sidebar.arranged(&[1, 2, 3]), [Terminal(1), Terminal(2), Group(g)], "3 is in a group now");
    }

    #[test]
    fn a_group_and_a_terminal_swap_places_by_a_drag() {
        let mut sidebar = view();
        let g = sidebar.new_group(&IDS, &[2, 3]);
        assert_eq!(
            sidebar.arranged(&IDS),
            [Terminal(1), Group(g), Terminal(4), Terminal(5)],
            "where 2 stood"
        );
        sidebar.place_at_top(&IDS, Group(g), Some(Terminal(1)));
        assert_eq!(sidebar.arranged(&IDS), [Group(g), Terminal(1), Terminal(4), Terminal(5)]);
        sidebar.place_at_top(&IDS, Terminal(5), Some(Group(g)));
        assert_eq!(sidebar.arranged(&IDS), [Terminal(5), Group(g), Terminal(1), Terminal(4)]);
        sidebar.place_at_top(&IDS, Terminal(1), None);
        assert_eq!(sidebar.arranged(&IDS), [Terminal(5), Group(g), Terminal(4), Terminal(1)]);
    }

    #[test]
    fn a_terminal_dropped_out_of_a_group_stays_where_it_was_put() {
        let mut sidebar = view();
        let g = sidebar.new_group(&IDS, &[2, 3]);
        sidebar.place_at_top(&IDS, Terminal(3), Some(Terminal(4)));
        assert_eq!(sidebar.arranged(&IDS), [Terminal(1), Group(g), Terminal(3), Terminal(4), Terminal(5)]);
        assert_eq!(sidebar.groups.get(g).map(|g| g.members.clone()), Some(vec![2]));
        // The last one out takes an unpinned group with it.
        sidebar.place_at_top(&IDS, Terminal(2), None);
        assert_eq!(sidebar.arranged(&IDS), [Terminal(1), Terminal(3), Terminal(4), Terminal(5), Terminal(2)]);
        assert!(sidebar.groups.is_empty());
    }

    #[test]
    fn a_terminal_joins_a_group_before_a_member_or_last() {
        let mut sidebar = view();
        let g = sidebar.new_group(&IDS, &[2, 3]);
        assert!(sidebar.place_in_group(&IDS, 5, g, Some(3)));
        assert_eq!(sidebar.groups.get(g).map(|g| g.members.clone()), Some(vec![2, 5, 3]));
        assert!(sidebar.place_in_group(&IDS, 1, g, None));
        assert_eq!(sidebar.groups.get(g).map(|g| g.members.clone()), Some(vec![2, 5, 3, 1]));
        assert_eq!(sidebar.arranged(&IDS), [Group(g), Terminal(4)]);
        // Within the group it is a reorder.
        assert!(sidebar.place_in_group(&IDS, 1, g, Some(2)));
        assert_eq!(sidebar.groups.get(g).map(|g| g.members.clone()), Some(vec![1, 2, 5, 3]));
        assert!(!sidebar.place_in_group(&IDS, 4, 99, None), "no such group");
    }

    #[test]
    fn a_group_made_of_terminals_from_two_places_stands_at_the_first() {
        let mut sidebar = view();
        let a = sidebar.new_group(&IDS, &[1, 2]);
        let b = sidebar.new_group(&IDS, &[2, 4]);
        // 2 left group a (still holding 1) for b, which stands where 2 stood: after a.
        assert_eq!(sidebar.arranged(&IDS), [Group(a), Group(b), Terminal(3), Terminal(5)]);
        assert_eq!(sidebar.groups.get(a).map(|g| g.members.clone()), Some(vec![1]));
    }

    #[test]
    fn terminals_swap_the_places_they_hold_among_themselves() {
        let mut sidebar = view();
        assert!(sidebar.reorder_terminals(&IDS, &[4, 2]));
        assert_eq!(
            sidebar.arranged(&IDS),
            [Terminal(1), Terminal(4), Terminal(3), Terminal(2), Terminal(5)],
            "2 and 4 trade places, the rest stay"
        );
        let g = sidebar.new_group(&IDS, &[1, 3, 5]);
        assert!(sidebar.reorder_terminals(&IDS, &[5, 1]));
        assert_eq!(sidebar.groups.get(g).map(|g| g.members.clone()), Some(vec![5, 3, 1]));
        assert!(!sidebar.reorder_terminals(&IDS, &[5, 2]), "one is in a group, the other is not");
        assert_eq!(sidebar.groups.get(g).map(|g| g.members.clone()), Some(vec![5, 3, 1]));
    }

    #[test]
    fn ungrouping_puts_its_terminals_where_it_stood() {
        let mut sidebar = view();
        let g = sidebar.new_group(&IDS, &[4, 2]);
        sidebar.place_at_top(&IDS, Group(g), Some(Terminal(3)));
        assert_eq!(sidebar.arranged(&IDS), [Terminal(1), Group(g), Terminal(3), Terminal(5)]);
        sidebar.ungroup(&IDS, g);
        assert_eq!(
            sidebar.arranged(&IDS),
            [Terminal(1), Terminal(4), Terminal(2), Terminal(3), Terminal(5)],
            "in the order they had in the group"
        );
        assert!(sidebar.groups.is_empty());
    }

    #[test]
    fn the_order_round_trips_through_json_and_an_old_window_loads() {
        let mut sidebar = view();
        let g = sidebar.new_group(&IDS, &[2]);
        let json = serde_json::to_string(&sidebar).unwrap_or_default();
        assert!(json.contains(r#""items":[{"terminal":1},{"group":"#), "{json}");
        assert_eq!(serde_json::from_str::<WindowSidebar>(&json).ok(), Some(sidebar));
        // Saved by 1.1: a grouping, a filter and groups, with no order of their own.
        let old = r#"{"grouping":"custom","filter":{"show":"all"},"folded":["group:1"],
                      "groups":[{"id":1,"name":"Docs","color":"teal","members":[2]}]}"#;
        let loaded: WindowSidebar = serde_json::from_str(old).unwrap_or_default();
        assert_eq!(loaded.arranged(&IDS), [Terminal(1), Terminal(3), Terminal(4), Terminal(5), Group(1)]);
        assert!(loaded.is_group_folded(1));
        let _ = g;
    }
}
