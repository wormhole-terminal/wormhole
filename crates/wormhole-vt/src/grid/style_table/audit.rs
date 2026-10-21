//! The table's checker, its byte counts and how its index grows: what they say about a table that
//! is sound, and about one made unsound on purpose.

use super::{Entry, Style, StyleTable};
use crate::grid::StyleId;
use crate::types::Color;

fn style(n: u8) -> Style {
    Style { fg: Color::Indexed(n), ..Style::DEFAULT }
}

/// A table with one style in use, id 1, and the slot of the index that is its own.
fn one_style() -> (StyleTable, usize) {
    let mut table = StyleTable::default();
    assert_eq!(table.acquire(&style(1), 2), 1);
    let slot = table.entries[1].hash as usize & (table.index.len() - 1);
    (table, slot)
}

#[test]
fn a_table_that_holds_nothing_allocates_nothing() {
    let table = StyleTable::unallocated();
    assert!(table.styles().is_empty());
    assert_eq!(table.bytes(), 0);
}

#[test]
fn the_checker_accepts_a_sound_table() {
    assert!(one_style().0.verify().is_ok());
}

#[test]
fn the_checker_refuses_a_style_the_index_cannot_find() {
    // Its own slot is vacant, and it sits further on: a lookup stops at the vacant slot.
    let (mut table, slot) = one_style();
    let mask = table.index.len() - 1;
    table.index[slot] = 0;
    table.index[(slot + 2) & mask] = 1;
    assert!(table.verify().is_err(), "out of its probe run");

    // Another style at its slot and none of its own: a lookup runs on to the vacant slot after it.
    let (mut table, slot) = one_style();
    table.index[slot] = 2;
    assert!(table.verify().is_err(), "behind another");
}

#[test]
fn the_checker_refuses_an_index_with_more_in_it_than_there_are_styles_in_use() {
    let (mut table, slot) = one_style();
    let mask = table.index.len() - 1;
    table.index[(slot + 3) & mask] = 1;
    assert!(table.verify().is_err());
}

#[test]
fn the_checker_refuses_a_first_entry_that_is_not_the_default_style() {
    let (mut table, _) = one_style();
    table.entries[0].style = style(2);
    assert!(table.verify().is_err());
}

#[test]
fn the_styles_in_use_are_listed_and_the_ones_given_back_are_not() {
    let mut table = StyleTable::default();
    let (a, b) = (table.acquire(&style(1), 3), table.acquire(&style(2), 1));
    table.release(b, 1);
    assert_eq!(table.used().collect::<Vec<_>>(), [(a, 3)]);
}

#[test]
fn the_bytes_of_a_table_are_its_entries_free_ids_and_index() {
    let mut table = StyleTable::default();
    let a = table.acquire(&style(1), 1);
    table.acquire(&style(2), 1);
    table.release(a, 1); // an id to reuse
    assert!(table.free.capacity() > 0 && table.index.capacity() > 0);
    let id = size_of::<StyleId>();
    assert_eq!(table.used_bytes(), table.entries.len() * size_of::<Entry>() + table.index.len() * id);
    assert_eq!(
        table.bytes(),
        table.entries.capacity() * size_of::<Entry>()
            + table.free.capacity() * id
            + table.index.capacity() * id
    );
}

#[test]
fn the_index_doubles_when_one_more_style_would_fill_over_half_of_it() {
    let mut table = StyleTable::default();
    let lengths: Vec<usize> = (1..=17)
        .map(|n| {
            table.acquire(&style(n), 1);
            table.index.len()
        })
        .collect();
    assert_eq!(lengths[0], 16);
    assert_eq!(lengths[7], 16, "eight styles are half of sixteen slots");
    assert_eq!(lengths[8], 32);
    assert_eq!(lengths[15], 32);
    assert_eq!(lengths[16], 64);
    assert!(table.verify().is_ok());
}
