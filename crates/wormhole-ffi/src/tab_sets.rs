//! The tab bar's sets (`wormhole_core::view::sidebar::tabs`): which set a terminal is in, the one a key steps or counts to,
//! and what the switcher's search finds. The window hands over the sets it lists (a group's terminals, the loose ones
//! last) and draws what comes back; the rules are the core's.

use wormhole_core::view::sidebar::tabs::{self, TabSet};

/// One set: its name and its terminals, in the list's order.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TabSetRecord {
    pub label: String,
    pub members: Vec<i64>,
}

/// A terminal and the name the switcher searches it by.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Record)]
pub struct TabNameRecord {
    pub id: i64,
    pub name: String,
}

/// What a search found: a set (`terminal` is `None`), or a terminal in the set at `set`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct TabHitRecord {
    pub set: u32,
    pub terminal: Option<i64>,
}

fn session(id: i64) -> usize {
    usize::try_from(id).unwrap_or(usize::MAX)
}

fn sets(records: &[TabSetRecord]) -> Vec<TabSet> {
    records
        .iter()
        .map(|record| TabSet {
            band: None,
            label: record.label.clone(),
            members: record.members.iter().map(|&id| session(id)).collect(),
        })
        .collect()
}

fn at(index: usize) -> u32 {
    u32::try_from(index).unwrap_or(u32::MAX)
}

/// The set terminal `terminal` is in.
#[uniffi::export]
pub fn tab_sets_current(sets_now: Vec<TabSetRecord>, terminal: i64) -> Option<u32> {
    tabs::current_set(&sets(&sets_now), session(terminal)).map(at)
}

/// The set one step from `current`, wrapping, empty sets passed over; `None` when no other set has a terminal.
#[uniffi::export]
pub fn tab_sets_step(sets_now: Vec<TabSetRecord>, current: u32, forward: bool) -> Option<u32> {
    let sets = sets(&sets_now);
    if current as usize >= sets.len() {
        return None;
    }
    tabs::step_set(&sets, current as usize, forward).map(at)
}

/// The set a key's number (1 to 9) reaches, empty ones counted.
#[uniffi::export]
pub fn tab_sets_numbered(sets_now: Vec<TabSetRecord>, number: u32) -> Option<u32> {
    tabs::set_numbered(&sets(&sets_now), number as usize).map(at)
}

/// The sets whose names contain `query`, then the terminals (`names`) whose names do; nothing for a blank query.
#[uniffi::export]
pub fn tab_sets_search(
    sets_now: Vec<TabSetRecord>,
    query: String,
    names: Vec<TabNameRecord>,
) -> Vec<TabHitRecord> {
    let name_of = |member: usize| {
        names
            .iter()
            .find(|entry| session(entry.id) == member)
            .map(|entry| entry.name.clone())
            .unwrap_or_default()
    };
    tabs::search(&sets(&sets_now), &query, name_of)
        .into_iter()
        .map(|hit| match hit {
            tabs::Hit::Set(set) => TabHitRecord { set: at(set), terminal: None },
            tabs::Hit::Terminal(set, member) => {
                TabHitRecord { set: at(set), terminal: i64::try_from(member).ok() }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(label: &str, members: &[i64]) -> TabSetRecord {
        TabSetRecord { label: label.into(), members: members.to_vec() }
    }

    #[test]
    fn a_key_steps_over_empty_sets_wraps_and_counts_them() {
        let sets = vec![set("A", &[0]), set("Docs", &[]), set("B", &[1, 2]), set("Ungrouped", &[3])];
        assert_eq!(tab_sets_step(sets.clone(), 0, true), Some(2));
        assert_eq!(tab_sets_step(sets.clone(), 3, true), Some(0));
        assert_eq!(tab_sets_step(sets.clone(), 0, false), Some(3));
        assert_eq!(tab_sets_step(vec![set("A", &[0])], 0, true), None);
        assert_eq!(tab_sets_step(sets.clone(), 9, true), None, "a set that is not there");
        assert_eq!(tab_sets_current(sets.clone(), 2), Some(2));
        assert_eq!(tab_sets_current(sets.clone(), 9), None);
        assert_eq!(tab_sets_numbered(sets.clone(), 4), Some(3));
        assert_eq!((tab_sets_numbered(sets.clone(), 5), tab_sets_numbered(sets, 0)), (None, None));
    }

    #[test]
    fn search_finds_sets_by_name_then_terminals_by_name() {
        let sets = vec![set("A", &[0]), set("Docs", &[]), set("B", &[1, 2]), set("Ungrouped", &[3])];
        let names = ["zero", "docs reader", "two", "three"]
            .iter()
            .enumerate()
            .map(|(id, name)| TabNameRecord { id: id as i64, name: (*name).into() })
            .collect::<Vec<_>>();
        let found = tab_sets_search(sets.clone(), "doc".into(), names.clone());
        assert_eq!(
            found,
            [TabHitRecord { set: 1, terminal: None }, TabHitRecord { set: 2, terminal: Some(1) }]
        );
        assert!(tab_sets_search(sets, "  ".into(), names).is_empty());
    }
}
