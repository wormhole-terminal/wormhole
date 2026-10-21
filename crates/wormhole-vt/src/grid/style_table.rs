//! The table that interns a page's styles.
//!
//! A cell names its style by a 16-bit id into its page's table, so a row of colored text is eight
//! bytes a cell however many colors it has, and two cells with the same look share one entry. The
//! table counts how many cells use each entry; when the count reaches zero the entry is freed and
//! its id reused. A page holds fewer than 65,535 cells (see `Page::capacity`), so it can never run
//! out of ids.
//!
//! The table is a flat `Vec` of entries plus an open-addressing index of their ids (linear
//! probing, two bytes a slot, at most half full): interning is one hash of sixteen bytes and a
//! probe, and the common case, the style the last write used, is one comparison.

use super::cell::StyleId;
use super::style::{LAYOUT_FLAGS, Style};

#[cfg(test)]
thread_local! {
    /// Calls of `acquire` on this thread, for the tests that count what an operation costs in
    /// style lookups (a test runs on one thread, so they do not see each other's).
    static ACQUIRES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// The calls of `StyleTable::acquire` so far on this thread.
#[cfg(test)]
pub fn acquires() -> usize {
    ACQUIRES.with(std::cell::Cell::get)
}

#[derive(Clone, Debug)]
pub(super) struct Entry {
    style: Style,
    /// How many cells of the page use the style (0 for a free entry).
    refs: u32,
    /// `style.table_hash(seed)` truncated, kept so that a probe compares one number before it compares
    /// styles, and a deletion does not hash again.
    hash: u32,
}

/// A page's interned styles; see the module docs.
#[derive(Clone, Debug)]
pub(super) struct StyleTable {
    /// By id. Entry 0 is the default style: never counted, never freed, never in the index.
    entries: Vec<Entry>,
    /// Ids that are free to reuse.
    free: Vec<StyleId>,
    /// Open addressing: an id, or 0 for a vacant slot. A power of two long once used.
    index: Vec<StyleId>,
    /// Ids in the index.
    live: usize,
    /// The id the last interning found, tried before the index.
    last: StyleId,
    /// Live styles with an underline color of their own: the snapshot skips the pass that looks for
    /// them when there are none.
    colored_underlines: u32,
    /// The key of the index's hash, drawn at random for each table: what a program draws cannot be
    /// chosen to pile into one slot without knowing it. Nothing but the slots depends on it (ids
    /// are handed out in order of use), so two terminals fed the same bytes are the same.
    seed: u64,
}

/// A key for a new table. `RandomState` is the standard library's source of per-process random
/// keys, drawn from the operating system once per thread and counted up for each use, so this is a
/// few nanoseconds and never allocates.
fn fresh_seed() -> u64 {
    use std::hash::BuildHasher;
    std::hash::RandomState::new().hash_one(0x5EED_u64)
}

impl Default for StyleTable {
    fn default() -> StyleTable {
        StyleTable {
            entries: vec![Entry { style: Style::DEFAULT, refs: 0, hash: 0 }],
            free: Vec::new(),
            index: Vec::new(),
            live: 0,
            last: 0,
            colored_underlines: 0,
            seed: fresh_seed(),
        }
    }
}

impl StyleTable {
    /// A table that holds nothing at all, not even the default style, and allocates nothing: for
    /// the placeholder that stands in for a page taken out of the list. Nothing may look at it.
    pub(super) fn unallocated() -> StyleTable {
        StyleTable {
            entries: Vec::new(),
            free: Vec::new(),
            index: Vec::new(),
            live: 0,
            last: 0,
            colored_underlines: 0,
            seed: 0,
        }
    }

    /// Check that the index finds every style in use and nothing else (for the invariant checker).
    #[cfg(any(test, feature = "internals"))]
    pub(super) fn verify(&self) -> Result<(), String> {
        let mask = self.index.len().wrapping_sub(1);
        let mut indexed = 0;
        for (id, entry) in self.entries.iter().enumerate().skip(1) {
            if entry.refs == 0 {
                if !self.free.contains(&(id as StyleId)) {
                    return Err(format!("style {id} is neither used nor free"));
                }
                continue;
            }
            if entry.hash != entry.style.table_hash(self.seed) as u32 {
                return Err(format!("style {id} has a stale hash"));
            }
            let mut slot = entry.hash as usize & mask;
            loop {
                match self.index.get(slot) {
                    Some(&found) if found as usize == id => break,
                    Some(0) | None => return Err(format!("style {id} is not in the index")),
                    Some(_) => slot = (slot + 1) & mask,
                }
            }
            indexed += 1;
        }
        let in_index = self.index.iter().filter(|&&id| id != 0).count();
        if indexed != self.live || in_index != self.live {
            return Err(format!("{indexed} styles found, {in_index} in the index, {} live", self.live));
        }
        if self.entries.first().is_none_or(|e| e.style != Style::DEFAULT || e.refs != 0) {
            return Err("entry 0 is not the default style".into());
        }
        Ok(())
    }

    /// The style with this id (the default for an id that is not in use).
    #[inline]
    pub(super) fn get(&self, id: StyleId) -> &Style {
        &self.entries[id as usize].style
    }

    /// All entries by id, for the snapshot loop, which indexes it with a cell's id.
    #[inline]
    pub(super) fn styles(&self) -> &[Entry] {
        &self.entries
    }

    #[inline]
    #[cfg(any(test, feature = "internals"))]
    pub(super) fn refs(&self, id: StyleId) -> u32 {
        self.entries[id as usize].refs
    }

    /// Number of ids in use, the default one not counted.
    #[cfg(any(test, feature = "internals"))]
    pub(super) fn live(&self) -> usize {
        self.live
    }

    pub(super) fn has_colored_underlines(&self) -> bool {
        self.colored_underlines != 0
    }

    /// Ids and use counts of every style in use (the checker compares them with the cells).
    #[cfg(any(test, feature = "internals"))]
    pub(super) fn used(&self) -> impl Iterator<Item = (StyleId, u32)> + '_ {
        self.entries
            .iter()
            .enumerate()
            .skip(1)
            .filter(|(_, e)| e.refs > 0)
            .map(|(id, e)| (id as StyleId, e.refs))
    }

    /// Forget every style, keeping the allocations.
    pub(super) fn clear(&mut self) {
        self.entries.truncate(1);
        self.free.clear();
        self.index.fill(0);
        self.live = 0;
        self.last = 0;
        self.colored_underlines = 0;
    }

    /// Bytes the styles take now: the entries (the free ones are kept for reuse) and the index.
    /// Not the capacity, which a recycled page keeps from its last use. A page of history is
    /// measured with this.
    pub(super) fn used_bytes(&self) -> usize {
        self.entries.len() * size_of::<Entry>() + self.index.len() * size_of::<StyleId>()
    }

    /// Bytes the table holds.
    #[cfg(any(test, feature = "internals"))]
    pub(super) fn bytes(&self) -> usize {
        self.entries.capacity() * size_of::<Entry>()
            + self.free.capacity() * size_of::<StyleId>()
            + self.index.capacity() * size_of::<StyleId>()
    }

    /// The id of `style` in this table, counting `n` more cells that use it. The default style is
    /// id 0 and costs nothing. Layout bits in `style.attrs` (wide, spacer, wrap) are not part of a
    /// look and are dropped: they are the cell's own flags, and a table that kept them would hold
    /// a second entry for the same look.
    #[inline]
    pub(super) fn acquire(&mut self, style: &Style, n: u32) -> StyleId {
        #[cfg(test)]
        ACQUIRES.with(|count| count.set(count.get() + 1));
        if style.attrs.intersects(LAYOUT_FLAGS) {
            return self.acquire_without_layout(style, n);
        }
        if style.is_default() {
            return 0;
        }
        let last = self.last;
        if last != 0 {
            let entry = &mut self.entries[last as usize];
            if entry.refs != 0 && entry.style.same(style) {
                entry.refs += n;
                return last;
            }
        }
        self.acquire_slow(style, n)
    }

    #[cold]
    fn acquire_without_layout(&mut self, style: &Style, n: u32) -> StyleId {
        let attrs = style.attrs.difference(LAYOUT_FLAGS);
        self.acquire(&Style { attrs, ..*style }, n)
    }

    #[cold]
    fn acquire_slow(&mut self, style: &Style, n: u32) -> StyleId {
        if self.index.is_empty() {
            self.index = vec![0; 16];
        }
        let mask = self.index.len() - 1;
        let hash = style.table_hash(self.seed) as u32;
        let mut slot = hash as usize & mask;
        loop {
            match self.index[slot] {
                0 => break,
                id => {
                    let entry = &mut self.entries[id as usize];
                    if entry.hash == hash && entry.style.same(style) {
                        entry.refs += n;
                        self.last = id;
                        return id;
                    }
                    slot = (slot + 1) & mask;
                }
            }
        }
        if (self.live + 1) * 2 > self.index.len() {
            self.grow_index();
            let mask = self.index.len() - 1;
            slot = hash as usize & mask;
            while self.index[slot] != 0 {
                slot = (slot + 1) & mask;
            }
        }
        let id = match self.free.pop() {
            Some(id) => {
                self.entries[id as usize] = Entry { style: *style, refs: n, hash };
                id
            }
            None => {
                // A page has fewer cells than ids, so this cannot happen; the default style is
                // the least harm if it ever did.
                let Ok(id) = StyleId::try_from(self.entries.len()) else {
                    debug_assert!(false, "a page ran out of style ids");
                    return 0;
                };
                self.entries.push(Entry { style: *style, refs: n, hash });
                id
            }
        };
        self.index[slot] = id;
        self.live += 1;
        self.last = id;
        self.colored_underlines += u32::from(style.underline_color.is_some());
        id
    }

    fn grow_index(&mut self) {
        let size = (self.index.len() * 2).max(16);
        self.index.clear();
        self.index.resize(size, 0);
        let mask = size - 1;
        for (id, entry) in self.entries.iter().enumerate().skip(1) {
            if entry.refs == 0 {
                continue;
            }
            let mut slot = entry.hash as usize & mask;
            while self.index[slot] != 0 {
                slot = (slot + 1) & mask;
            }
            self.index[slot] = id as StyleId;
        }
    }

    /// `n` more cells use style `id`.
    #[inline]
    pub(super) fn retain(&mut self, id: StyleId, n: u32) {
        if id != 0 {
            self.entries[id as usize].refs += n;
        }
    }

    /// `n` cells stopped using style `id`; the entry goes when none is left.
    #[inline]
    pub(super) fn release(&mut self, id: StyleId, n: u32) {
        if id == 0 {
            return;
        }
        let entry = &mut self.entries[id as usize];
        debug_assert!(entry.refs >= n, "style {id} released more than it was used");
        entry.refs = entry.refs.saturating_sub(n);
        if entry.refs == 0 {
            self.remove(id);
        }
    }

    #[cold]
    fn remove(&mut self, id: StyleId) {
        let mask = self.index.len() - 1;
        let mut slot = self.entries[id as usize].hash as usize & mask;
        while self.index[slot] != id {
            slot = (slot + 1) & mask;
        }
        // Backward-shift deletion: pull later members of the probe run into the hole, so lookups
        // never need tombstones.
        let mut hole = slot;
        let mut next = slot;
        loop {
            next = (next + 1) & mask;
            let moved = self.index[next];
            if moved == 0 {
                break;
            }
            let home = self.entries[moved as usize].hash as usize & mask;
            // Stays put when its home lies cyclically in (hole, next].
            let stays = if hole <= next { hole < home && home <= next } else { hole < home || home <= next };
            if !stays {
                self.index[hole] = moved;
                hole = next;
            }
        }
        self.index[hole] = 0;
        self.live -= 1;
        self.colored_underlines -= u32::from(self.entries[id as usize].style.underline_color.is_some());
        self.free.push(id);
        if self.last == id {
            self.last = 0;
        }
    }
}

impl Entry {
    #[inline]
    pub(super) fn style(&self) -> &Style {
        &self.style
    }
}

#[cfg(test)]
mod audit;

#[cfg(test)]
mod tests {
    use super::{Style, StyleTable};
    use crate::types::{Color, Flags, NamedColor, Rgb};

    fn style(n: u8) -> Style {
        Style { fg: Color::Indexed(n), ..Style::DEFAULT }
    }

    #[test]
    fn the_default_style_is_id_zero_and_free() {
        let mut table = StyleTable::default();
        assert_eq!(table.acquire(&Style::DEFAULT, 100), 0);
        assert_eq!(table.live(), 0);
        table.release(0, 100);
        assert_eq!(table.get(0), &Style::DEFAULT);
    }

    #[test]
    fn equal_styles_share_an_id_and_count_their_cells() {
        let mut table = StyleTable::default();
        let a = table.acquire(&style(1), 3);
        let b = table.acquire(&style(2), 1);
        assert_ne!(a, b);
        assert_eq!(table.acquire(&style(1), 2), a);
        assert_eq!((table.refs(a), table.refs(b)), (5, 1));
        table.release(a, 4);
        assert_eq!(table.refs(a), 1);
        assert_eq!(table.get(a), &style(1));
    }

    #[test]
    fn layout_bits_are_not_part_of_a_look() {
        let mut table = StyleTable::default();
        let layout =
            Flags::WIDE_CHAR | Flags::WIDE_CHAR_SPACER | Flags::LEADING_WIDE_CHAR_SPACER | Flags::WRAPLINE;
        // Only layout bits: the default style, which costs nothing.
        assert_eq!(table.acquire(&Style { attrs: layout, ..Style::DEFAULT }, 2), 0);
        assert_eq!(table.live(), 0);
        // Mixed with a real attribute: the same entry as the attribute alone.
        let bold = Style { attrs: Flags::BOLD, ..style(3) };
        let id = table.acquire(&bold, 1);
        assert_eq!(table.acquire(&Style { attrs: Flags::BOLD | layout, ..style(3) }, 1), id);
        assert_eq!(table.get(id).attrs, Flags::BOLD);
        assert_eq!(table.refs(id), 2);
        assert_eq!(table.live(), 1);
        assert!(table.verify().is_ok());
    }

    #[test]
    fn a_style_nobody_uses_is_freed_and_its_id_reused() {
        let mut table = StyleTable::default();
        let a = table.acquire(&style(1), 1);
        table.release(a, 1);
        assert_eq!(table.live(), 0);
        let b = table.acquire(&style(2), 1);
        assert_eq!(a, b, "the freed id is the next one");
        assert_eq!(table.get(b), &style(2));
        // The old style is gone: asking for it again makes a new entry.
        let c = table.acquire(&style(1), 1);
        assert_ne!(b, c);
    }

    #[test]
    fn every_field_tells_styles_apart() {
        let mut table = StyleTable::default();
        let base = style(9);
        let variants = [
            base,
            Style { fg: Color::Named(NamedColor::Red), ..base },
            Style { bg: Color::Rgb(Rgb { r: 1, g: 2, b: 3 }), ..base },
            Style { underline_color: Some(Color::Indexed(9)), ..base },
            Style { attrs: Flags::BOLD, ..base },
            Style { attrs: Flags::UNDERCURL, ..base },
        ];
        let ids: Vec<_> = variants.iter().map(|s| table.acquire(s, 1)).collect();
        for (i, a) in ids.iter().enumerate() {
            for b in &ids[i + 1..] {
                assert_ne!(a, b);
            }
        }
        assert!(table.has_colored_underlines());
        table.release(ids[3], 1);
        assert!(!table.has_colored_underlines());
    }

    #[test]
    fn thousands_of_styles_come_and_go_and_stay_findable() {
        let mut table = StyleTable::default();
        let mut ids = Vec::new();
        for n in 0..=255u8 {
            for r in 0..16u8 {
                let s =
                    Style { fg: Color::Indexed(n), bg: Color::Rgb(Rgb { r, g: n, b: 1 }), ..Style::DEFAULT };
                ids.push((s, table.acquire(&s, 1)));
            }
        }
        assert_eq!(table.live(), 4096);
        // Free every other one, which makes holes in probe runs, then look all of them up again.
        for (i, (_, id)) in ids.iter().enumerate() {
            if i % 2 == 0 {
                table.release(*id, 1);
            }
        }
        for (i, (s, id)) in ids.iter().enumerate() {
            if i % 2 == 1 {
                assert_eq!(table.acquire(s, 1), *id, "style {i} is still found");
                table.release(*id, 1);
            }
        }
        assert_eq!(table.live(), 2048);
        assert_eq!(table.used().count(), 2048);
    }

    fn rgb_style(n: u32) -> Style {
        Style { fg: Color::Rgb(Rgb { r: (n >> 16) as u8, g: (n >> 8) as u8, b: n as u8 }), ..Style::DEFAULT }
    }

    #[test]
    fn every_table_draws_its_own_seed() {
        let seeds: std::collections::HashSet<u64> = (0..8).map(|_| StyleTable::default().seed).collect();
        assert!(seeds.len() > 1, "{seeds:?}");
    }

    #[test]
    fn colors_picked_to_collide_without_the_seed_spread_out_with_it() {
        // What an attacker who knew the hash without its key could do: 300 colors whose index
        // slot, in a table of 4,096, is the same.
        const MASK: u64 = 4095;
        let colliding: Vec<Style> =
            (0..1 << 22).map(rgb_style).filter(|s| s.table_hash(0) & MASK == 7).take(300).collect();
        assert_eq!(colliding.len(), 300);
        let seed = super::fresh_seed();
        let mut load = std::collections::HashMap::<u64, usize>::new();
        for style in &colliding {
            *load.entry(style.table_hash(seed) & MASK).or_default() += 1;
        }
        let worst = load.values().max().copied().unwrap_or(0);
        assert!(worst <= 12, "{worst} of 300 colors share a slot under a random seed");
    }

    #[test]
    fn the_seed_changes_where_styles_sit_in_the_index_and_nothing_a_terminal_can_see() {
        let mut a = StyleTable { seed: 1, ..StyleTable::default() };
        let mut b = StyleTable { seed: 0xDEAD_BEEF_F00D, ..StyleTable::default() };
        let mut ids = (Vec::new(), Vec::new());
        for n in 0..3000u32 {
            let s = rgb_style(n * 7919);
            ids.0.push(a.acquire(&s, 1 + n % 3));
            ids.1.push(b.acquire(&s, 1 + n % 3));
            if n % 3 == 0 {
                a.release(ids.0[n as usize], 1);
                b.release(ids.1[n as usize], 1);
            }
        }
        assert_eq!(ids.0, ids.1, "ids come in order of use");
        assert_eq!(a.used().collect::<Vec<_>>(), b.used().collect::<Vec<_>>());
        assert_eq!((a.live(), a.bytes()), (b.live(), b.bytes()));
        assert!(a.verify().is_ok() && b.verify().is_ok());
    }

    #[test]
    fn two_styles_whose_hashes_collide_stay_two_entries() {
        // What the seed makes a stranger's guess: two looks with the same 32 bits of hash, found by
        // brute force for one seed.
        let seed = 7;
        let mut seen = std::collections::HashMap::<u32, u32>::new();
        let (first, second) = (0..1_u32 << 20)
            .find_map(|n| {
                let hash = rgb_style(n).table_hash(seed) as u32;
                seen.insert(hash, n).map(|earlier| (rgb_style(earlier), rgb_style(n)))
            })
            .expect("two of a million looks share 32 bits of hash");
        let mut table = StyleTable { seed, ..StyleTable::default() };
        let a = table.acquire(&first, 1);
        let b = table.acquire(&second, 1);
        assert_ne!(a, b);
        assert_eq!((table.get(a), table.get(b)), (&first, &second));
        assert_eq!(table.acquire(&second, 1), b, "and each is still found");
        assert!(table.verify().is_ok());
    }

    #[test]
    fn clearing_keeps_nothing_but_the_default() {
        let mut table = StyleTable::default();
        table.acquire(&style(1), 4);
        table.clear();
        assert_eq!(table.live(), 0);
        assert_eq!(table.used().count(), 0);
        assert_eq!(table.acquire(&style(1), 1), 1);
    }
}
