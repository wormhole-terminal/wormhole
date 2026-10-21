//! The zero-width characters of a page's cells: what an accent, an emoji's joiners or a flag's
//! second half add to the character in the cell.
//!
//! A cell names its marks by its offset in the page (`slot * columns + column`, stable while rows
//! are moved by relinking). A page of plain text has none and a page of emoji has many, so the
//! entries are a hash map; the characters themselves sit in one arena of the page, each cell owning
//! a slot of 2, 4, 8, 16 or 32 of them. A slot that a cell gives up goes onto a list for its size
//! and the next cell that needs that size takes it, so printing marks, overwriting them and
//! scrolling past them allocate nothing once the page has held as many at a time as it will, and a
//! page that is recycled keeps its arena for the next one.

use super::offset_map::ByOffset;

/// The most characters a cell keeps after its first. A cluster of a base, a joiner sequence and
/// a few modifiers is under ten; a program that stacks thousands of combining marks on one cell
/// (zalgo) must not make a page's memory unbounded, so the rest are dropped.
pub const MAX_MARKS: usize = 32;

/// Slot sizes are `2 << class` characters: 2, 4, 8, 16, 32.
const CLASSES: usize = 5;

const fn slot_len(class: u8) -> usize {
    2 << class
}

/// The smallest class whose slots hold `len` characters (1 to [`MAX_MARKS`]).
fn class_for(len: usize) -> u8 {
    (len.max(2).next_power_of_two().trailing_zeros() - 1) as u8
}

/// Where a cell's marks are in the arena: `len` characters from `start`, in a slot of its class.
#[derive(Clone, Copy, Debug)]
struct Entry {
    start: u32,
    len: u8,
    class: u8,
}

/// The characters after the first of the cells that have a cluster, in print order.
#[derive(Clone, Debug, Default)]
pub(super) struct Graphemes {
    entries: ByOffset<Entry>,
    arena: Vec<char>,
    /// Where the slots of each class that no cell uses start.
    free: [Vec<u32>; CLASSES],
}

impl Graphemes {
    #[inline]
    pub(super) fn get(&self, offset: u32) -> &[char] {
        match self.entries.get(&offset) {
            Some(entry) => &self.arena[entry.start as usize..entry.start as usize + usize::from(entry.len)],
            None => &[],
        }
    }

    /// A slot of `class`: one given up earlier, or new at the end of the arena. `None` when the
    /// arena would outgrow its `u32` offsets (a page holds under 65,535 cells, so it never does).
    fn slot(&mut self, class: u8) -> Option<u32> {
        if let Some(start) = self.free[usize::from(class)].pop() {
            return Some(start);
        }
        if self.arena.capacity() == 0 {
            // A page that has any marks usually has a few dozen: skip the first doublings.
            self.arena.reserve(128);
            self.entries.reserve(16);
        }
        let start = self.arena.len();
        let end = start + slot_len(class);
        u32::try_from(end).ok()?;
        self.arena.resize(end, '\0');
        Some(start as u32)
    }

    fn give_up(&mut self, entry: Entry) {
        self.free[usize::from(entry.class)].push(entry.start);
    }

    /// Add `c` to the cell's marks; false when the cell is full.
    pub(super) fn push(&mut self, offset: u32, c: char) -> bool {
        let Some(entry) = self.entries.get_mut(&offset) else {
            let Some(start) = self.slot(0) else { return false };
            self.arena[start as usize] = c;
            self.entries.insert(offset, Entry { start, len: 1, class: 0 });
            return true;
        };
        let len = usize::from(entry.len);
        if len >= MAX_MARKS {
            return false;
        }
        if len < slot_len(entry.class) {
            self.arena[entry.start as usize + len] = c;
            entry.len += 1;
            return true;
        }
        // Full: move to a slot twice the size. (A full slot of the last class is `MAX_MARKS` long,
        // which the check above refused.)
        let old = *entry;
        let Some(start) = self.slot(old.class + 1) else { return false };
        self.arena.copy_within(old.start as usize..old.start as usize + len, start as usize);
        self.arena[start as usize + len] = c;
        self.give_up(old);
        self.entries.insert(offset, Entry { start, len: old.len + 1, class: old.class + 1 });
        true
    }

    /// Give the cell exactly these marks (at most [`MAX_MARKS`] of them are kept).
    pub(super) fn set_from(&mut self, offset: u32, marks: &[char]) {
        let marks = &marks[..marks.len().min(MAX_MARKS)];
        if marks.is_empty() {
            self.remove(offset);
            return;
        }
        let entry = match self.entries.get(&offset) {
            // The slot it has is big enough: keep it.
            Some(&entry) if marks.len() <= slot_len(entry.class) => Some(entry),
            Some(&entry) => {
                self.give_up(entry);
                None
            }
            None => None,
        };
        let entry = match entry {
            Some(entry) => entry,
            None => {
                let class = class_for(marks.len());
                let Some(start) = self.slot(class) else {
                    self.entries.remove(&offset);
                    return;
                };
                Entry { start, len: 0, class }
            }
        };
        let start = entry.start as usize;
        self.arena[start..start + marks.len()].copy_from_slice(marks);
        self.entries.insert(offset, Entry { len: marks.len() as u8, ..entry });
    }

    /// Give the cell `to` a copy of the marks of the cell `from`.
    pub(super) fn copy(&mut self, from: u32, to: u32) {
        let Some(&source) = self.entries.get(&from) else { return };
        let Some(start) = self.slot(source.class) else {
            self.remove(to);
            return;
        };
        let (a, len) = (source.start as usize, usize::from(source.len));
        self.arena.copy_within(a..a + len, start as usize);
        if let Some(old) = self.entries.insert(to, Entry { start, ..source }) {
            self.give_up(old);
        }
    }

    /// Move the marks of the cell `from` to the cell `to`, whose own (if it had any) are dropped.
    pub(super) fn relocate(&mut self, from: u32, to: u32) {
        let Some(entry) = self.entries.remove(&from) else { return };
        if let Some(old) = self.entries.insert(to, entry) {
            self.give_up(old);
        }
    }

    pub(super) fn remove(&mut self, offset: u32) {
        if let Some(old) = self.entries.remove(&offset) {
            self.give_up(old);
        }
    }

    /// How many cells have marks.
    #[cfg(any(test, feature = "internals"))]
    pub(super) fn len(&self) -> usize {
        self.entries.len()
    }

    /// Forget every cell's marks, keeping the arena and the tables for the page's next life.
    pub(super) fn clear(&mut self) {
        self.entries.clear();
        self.arena.clear();
        self.free.iter_mut().for_each(Vec::clear);
    }

    pub(super) fn bytes(&self) -> usize {
        // Per entry: the key, the entry and the map's control byte.
        self.entries.capacity() * (size_of::<u32>() + size_of::<Entry>() + 1)
            + self.arena.capacity() * size_of::<char>()
            + self.free.iter().map(|list| list.capacity() * size_of::<u32>()).sum::<usize>()
    }

    /// Check that every cell's slot is inside the arena, big enough, and shared with no other
    /// cell nor with a free slot (for the checker).
    #[cfg(any(test, feature = "internals"))]
    pub(super) fn verify(&self) -> Result<(), String> {
        let mut slots: Vec<(u32, u8, bool)> =
            self.entries.values().map(|entry| (entry.start, entry.class, true)).collect();
        for (class, list) in self.free.iter().enumerate() {
            slots.extend(list.iter().map(|&start| (start, class as u8, false)));
        }
        for entry in self.entries.values() {
            if entry.len == 0 || usize::from(entry.len) > slot_len(entry.class) {
                return Err(format!(
                    "marks of {} characters in a slot of {}",
                    entry.len,
                    slot_len(entry.class)
                ));
            }
        }
        slots.sort_unstable();
        let mut end = 0;
        for (start, class, used) in slots {
            if start < end {
                return Err(format!("the mark slot at {start} overlaps the one before it"));
            }
            end = start + slot_len(class) as u32;
            if end as usize > self.arena.len() {
                return Err(format!("the mark slot at {start} (used: {used}) is outside the arena"));
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{Graphemes, MAX_MARKS, class_for};

    #[test]
    fn marks_pile_up_in_order_and_stop_at_the_cap() {
        let mut graphemes = Graphemes::default();
        assert_eq!(graphemes.get(7), &[] as &[char]);
        assert!(graphemes.push(7, '\u{301}'));
        assert!(graphemes.push(7, '\u{200D}'));
        assert_eq!(graphemes.get(7), &['\u{301}', '\u{200D}']);
        for i in 2..MAX_MARKS {
            assert!(graphemes.push(7, 'x'));
            assert_eq!(graphemes.get(7).len(), i + 1, "growing into a bigger slot keeps the marks");
        }
        assert!(!graphemes.push(7, 'y'), "a cell keeps {MAX_MARKS} marks, no more");
        assert_eq!(&graphemes.get(7)[..2], &['\u{301}', '\u{200D}']);
        assert!(graphemes.verify().is_ok());
        graphemes.remove(7);
        assert_eq!(graphemes.len(), 0);
    }

    #[test]
    fn a_slot_fits_its_class() {
        assert_eq!([1, 2, 3, 4, 5, 8, 9, 16, 17, 32].map(class_for), [0, 0, 1, 1, 2, 2, 3, 3, 4, 4]);
    }

    #[test]
    fn cells_grow_side_by_side_without_mixing_their_marks() {
        let mut graphemes = Graphemes::default();
        for round in 0..10u32 {
            for cell in 0..5u32 {
                assert!(graphemes.push(cell, char::from_u32(0x300 + cell * 16 + round).unwrap_or('x')));
            }
        }
        for cell in 0..5u32 {
            let want: Vec<char> =
                (0..10).map(|round| char::from_u32(0x300 + cell * 16 + round).unwrap_or('x')).collect();
            assert_eq!(graphemes.get(cell), want.as_slice());
        }
        assert!(graphemes.verify().is_ok());
    }

    #[test]
    fn set_copy_and_relocate_move_marks_between_cells() {
        let mut graphemes = Graphemes::default();
        graphemes.set_from(1, &['a', 'b', 'c']);
        graphemes.copy(1, 9);
        graphemes.set_from(1, &['z']);
        assert_eq!((graphemes.get(1), graphemes.get(9)), (&['z'][..], &['a', 'b', 'c'][..]));
        graphemes.relocate(9, 2);
        assert_eq!((graphemes.get(9), graphemes.get(2)), (&[][..], &['a', 'b', 'c'][..]));
        // Onto a cell that has marks: its own are dropped.
        graphemes.relocate(1, 2);
        assert_eq!((graphemes.get(1), graphemes.get(2)), (&[][..], &['z'][..]));
        // Fewer marks than the slot holds, and more.
        graphemes.set_from(2, &[]);
        assert_eq!(graphemes.len(), 0);
        graphemes.set_from(3, &['a'; 40]);
        assert_eq!(graphemes.get(3).len(), MAX_MARKS, "at most {MAX_MARKS} are kept");
        graphemes.set_from(3, &['q', 'r']);
        assert_eq!(graphemes.get(3), &['q', 'r']);
        assert!(graphemes.verify().is_ok());
    }

    #[test]
    fn slots_given_up_are_taken_again_and_clearing_keeps_the_arena() {
        let mut graphemes = Graphemes::default();
        for cell in 0..50 {
            graphemes.push(cell, 'a');
            graphemes.push(cell, 'b');
            graphemes.push(cell, 'c');
        }
        let (len, capacity) = (graphemes.arena.len(), graphemes.arena.capacity());
        for _ in 0..20 {
            for cell in 0..50 {
                graphemes.remove(cell);
                graphemes.push(cell, 'x');
                graphemes.push(cell, 'y');
                graphemes.push(cell, 'z');
            }
        }
        for cell in 0..50 {
            graphemes.remove(cell);
        }
        assert_eq!(graphemes.arena.len(), len, "churn reuses the slots it has");
        graphemes.clear();
        assert_eq!(graphemes.len(), 0);
        assert_eq!(graphemes.arena.capacity(), capacity, "the next page reuses the arena");
        assert!(graphemes.verify().is_ok());
    }
}
