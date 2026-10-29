//! Rows built for earlier frames, kept until their content changes.
//!
//! The snapshot says which rows changed. A row that didn't, with the same
//! marks (selection, find's matches), keeps what was built for it. A changed row first looks for its
//! exact content among the rows being replaced: when output scrolls, every row
//! "changes" but all except the new ones were built a frame ago, one line up.
//! Only rows with new content are built (and shaped) again.

use wormhole_term::ScreenSnapshot;
use wormhole_term::types::{Cell, Color, RowExtras};

use super::rows::Marks;

struct Entry<T> {
    /// What the row was built from: compared before reusing it elsewhere. (A row without extras,
    /// nearly all of them, keeps an empty `RowExtras`, which owns no memory.)
    cells: Vec<Cell>,
    extras: RowExtras,
    marks: Marks,
    hash: u64,
    value: T,
}

/// What one `sync` did, for tests and the benchmark.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct SyncStats {
    pub built: usize,
    /// Reused from another row (scrolled output) or after a change that
    /// changed nothing (the cursor's row is reported every frame).
    pub moved: usize,
    pub kept: usize,
}

pub(super) struct RowCache<K, T> {
    /// What every entry was built with (fonts, colors, cell size).
    key: Option<K>,
    rows: Vec<Option<Entry<T>>>,
    /// Scratch for the entries being replaced.
    pool: Vec<Entry<T>>,
    /// Scratch for the rows that changed, and for those among them that are built anew.
    stale: Vec<usize>,
    unmatched: Vec<(usize, u64, Marks)>,
}

impl<K, T> Default for RowCache<K, T> {
    fn default() -> Self {
        Self { key: None, rows: Vec::new(), pool: Vec::new(), stale: Vec::new(), unmatched: Vec::new() }
    }
}

impl<K: PartialEq, T> RowCache<K, T> {
    /// Bring the cache in line with `snapshot`, building rows it can't reuse. `found` is find's
    /// matches on each row (rows past its end have none); a row `redo` says of is built again,
    /// changed or not (one drawn before all its words were shaped).
    ///
    /// `build` is given the spare of a row that was replaced and found no match, if one is left,
    /// to fill again in place of allocating its buffers afresh. A spare is only handed out after
    /// every changed row had its chance to match one (a scrolled row is a replaced row's
    /// content), so recycling never turns a move into a build.
    pub fn sync(
        &mut self,
        key: K,
        snapshot: &ScreenSnapshot,
        found: &[Vec<(usize, usize, bool)>],
        redo: impl Fn(&T) -> bool,
        mut build: impl FnMut(&[Cell], &RowExtras, &Marks, Option<T>) -> T,
    ) -> SyncStats {
        if self.key.as_ref() != Some(&key) {
            self.rows.clear();
            self.key = Some(key);
        }
        let rows = snapshot.rows();
        self.rows.resize_with(rows.len(), || None);

        let marks = |row: usize| Marks {
            size: rows[row].size(),
            selected: snapshot.selected_columns(row),
            found: found.get(row).cloned().unwrap_or_default(),
        };
        let mut stats = SyncStats::default();
        let mut stale = std::mem::take(&mut self.stale);
        stale.clear();
        for (row, entry) in self.rows.iter_mut().enumerate() {
            let same_marks = |e: &Entry<T>| {
                e.marks.size == rows[row].size()
                    && e.marks.selected == snapshot.selected_columns(row)
                    && e.marks.found.as_slice() == found.get(row).map_or(&[][..], Vec::as_slice)
            };
            let keep =
                !rows[row].changed() && entry.as_ref().is_some_and(|e| same_marks(e) && !redo(&e.value));
            if keep {
                stats.kept += 1;
            } else {
                self.pool.extend(entry.take());
                stale.push(row);
            }
        }
        let mut unmatched = std::mem::take(&mut self.unmatched);
        unmatched.clear();
        for &row in &stale {
            let (cells, extras) = (rows[row].cells(), rows[row].extras());
            let marks = marks(row);
            let hash = row_hash(cells, extras, &marks);
            let reusable = self.pool.iter().position(|e| {
                e.hash == hash
                    && e.marks == marks
                    && e.cells == cells
                    && e.extras == *extras
                    && !redo(&e.value)
            });
            match reusable {
                Some(index) => {
                    stats.moved += 1;
                    self.rows[row] = Some(self.pool.swap_remove(index));
                }
                None => unmatched.push((row, hash, marks)),
            }
        }
        for (row, hash, marks) in unmatched.drain(..) {
            stats.built += 1;
            let (cells, extras) = (rows[row].cells(), rows[row].extras());
            let (mut kept_cells, spare) = match self.pool.pop() {
                Some(spare) => (spare.cells, Some(spare.value)),
                None => (Vec::with_capacity(cells.len()), None),
            };
            let value = build(cells, extras, &marks, spare);
            kept_cells.clear();
            kept_cells.extend_from_slice(cells);
            self.rows[row] = Some(Entry { cells: kept_cells, extras: extras.clone(), marks, hash, value });
        }
        self.pool.clear();
        (self.stale, self.unmatched) = (stale, unmatched);
        stats
    }

    /// Every row with what was built for it.
    pub fn rows(&self) -> impl Iterator<Item = (usize, &T)> {
        self.rows.iter().enumerate().filter_map(|(row, entry)| Some((row, &entry.as_ref()?.value)))
    }

    /// [`Self::rows`], to finish what depends on where a row is.
    pub fn rows_mut(&mut self) -> impl Iterator<Item = (usize, &mut T)> {
        self.rows.iter_mut().enumerate().filter_map(|(row, entry)| Some((row, &mut entry.as_mut()?.value)))
    }
}

/// A quick fingerprint of a row (FxHash-style mixing): a full hash per changed
/// row would cost ten times more, and every hit is confirmed by comparing cells.
fn row_hash(cells: &[Cell], extras: &RowExtras, marks: &Marks) -> u64 {
    let mut hash = 0u64;
    let mut mix = |value: u64| hash = (hash.rotate_left(5) ^ value).wrapping_mul(0x517c_c1b7_2722_0a95);
    let color = |color: Color| match color {
        Color::Named(named) => named as u64,
        Color::Indexed(index) => 0x1000 | u64::from(index),
        Color::Rgb(rgb) => 0x0100_0000 | u64::from(rgb.r) << 16 | u64::from(rgb.g) << 8 | u64::from(rgb.b),
    };
    for cell in cells {
        mix(cell.c as u64 | u64::from(cell.flags.bits()) << 32);
        mix(color(cell.fg) << 32 | color(cell.bg));
    }
    // Hyperlinks are left out: they change nothing that is drawn, and a hit is confirmed by
    // comparing the rows whole anyway.
    for (col, extra) in extras.cells() {
        mix(col as u64);
        if let Some(c) = extra.underline_color {
            mix(color(c));
        }
        for c in extra.zerowidth {
            mix(*c as u64);
        }
    }
    mix(marks.size as u64);
    if let Some((first, last)) = marks.selected {
        mix(first as u64 | (last as u64) << 32);
    }
    for &(first, last, current) in &marks.found {
        mix(first as u64 | (last as u64) << 32 | u64::from(current) << 63);
    }
    hash
}

#[cfg(test)]
mod tests {
    use wormhole_term::ScreenSnapshot;
    use wormhole_term::testing::TestTerm;
    use wormhole_term::types::{Cell, Column, Line, Point, RowExtras, SelectionType, Side};

    use super::super::rows::Marks;
    use super::{RowCache, SyncStats};

    fn term(cols: u16, rows: u16) -> TestTerm {
        TestTerm::new(cols, rows, 100)
    }

    fn feed(term: &mut TestTerm, bytes: &[u8]) {
        term.feed(bytes);
    }

    /// Syncs a cache of row texts and returns what it did.
    fn sync(cache: &mut RowCache<u32, String>, term: &mut TestTerm, key: u32) -> SyncStats {
        let mut snapshot = ScreenSnapshot::default();
        term.snapshot(&mut snapshot);
        let stats =
            cache.sync(key, &snapshot, &[], |_| false, |cells, _, _, _| cells.iter().map(|c| c.c).collect());
        for (row, text) in cache.rows() {
            let expected: String = snapshot.rows()[row].cells().iter().map(|c| c.c).collect();
            assert_eq!(*text, expected, "row {row} shows stale content");
        }
        stats
    }

    #[test]
    fn unchanged_rows_are_not_built_again() {
        let mut term = term(10, 4);
        let mut cache = RowCache::default();
        feed(&mut term, b"one\r\ntwo\r\n$ ");
        assert_eq!(sync(&mut cache, &mut term, 0).built, 4);
        assert_eq!(sync(&mut cache, &mut term, 0).built, 0);
        feed(&mut term, b"ls");
        let stats = sync(&mut cache, &mut term, 0);
        assert_eq!(stats.built, 1, "only the prompt row: {stats:?}");
    }

    #[test]
    fn scrolled_output_reuses_rows_built_a_line_higher() {
        let mut term = term(10, 4);
        let mut cache = RowCache::default();
        feed(&mut term, b"a\r\nb\r\nc\r\nd");
        sync(&mut cache, &mut term, 0);
        // One more line scrolls everything up: only the new bottom line is new.
        feed(&mut term, b"\r\ne");
        let stats = sync(&mut cache, &mut term, 0);
        assert_eq!(stats.built, 1, "{stats:?}");
        assert_eq!(stats.moved, 3, "{stats:?}");
    }

    #[test]
    fn a_selection_rebuilds_only_the_rows_it_touches() {
        let mut term = term(10, 4);
        let mut cache = RowCache::default();
        feed(&mut term, b"a\r\nb\r\nc\r\nd");
        sync(&mut cache, &mut term, 0);
        term.start_selection(SelectionType::Simple, Point::new(Line(1), Column(0)), Side::Left);
        term.update_selection(Point::new(Line(1), Column(3)), Side::Right);
        assert_eq!(sync(&mut cache, &mut term, 0).built, 1);
        term.clear_selection();
        assert_eq!(sync(&mut cache, &mut term, 0).built, 1);
    }

    #[test]
    fn find_rebuilds_only_the_rows_whose_matches_changed() {
        let mut term = term(10, 4);
        let mut cache: RowCache<u32, String> = RowCache::default();
        feed(&mut term, b"a\r\nb\r\nc\r\nd");
        let mut snapshot = ScreenSnapshot::default();
        term.snapshot(&mut snapshot);
        let mut sync = |found: &[Vec<(usize, usize, bool)>]| {
            term.snapshot(&mut snapshot);
            cache
                .sync(0, &snapshot, found, |_| false, |cells, _, _, _| cells.iter().map(|c| c.c).collect())
                .built
        };
        sync(&[]);
        assert_eq!(sync(&[vec![], vec![(0, 0, true)]]), 1, "the row with a match");
        assert_eq!(sync(&[vec![], vec![(0, 0, true)]]), 0);
        // Stepping to another match moves "current" between two rows.
        assert_eq!(sync(&[vec![], vec![(0, 0, false)], vec![(0, 0, true)]]), 2);
        assert_eq!(sync(&[]), 2, "closing find clears them");
    }

    #[test]
    fn a_row_drawn_before_its_words_were_shaped_is_built_again_unchanged() {
        let mut term = term(10, 3);
        feed(&mut term, b"a\r\nb\r\nc");
        let mut cache: RowCache<u32, String> = RowCache::default();
        let mut snapshot = ScreenSnapshot::default();
        term.snapshot(&mut snapshot);
        // Row 1 ("b") is drawn provisionally.
        fn build(cells: &[Cell], _: &RowExtras, _: &Marks, _: Option<String>) -> String {
            let text: String = cells.iter().map(|c| c.c).collect();
            if text.starts_with('b') { format!("{text}?") } else { text }
        }
        let redo = |value: &String| value.ends_with('?');
        assert_eq!(cache.sync(0, &snapshot, &[], redo, build).built, 3);
        term.snapshot(&mut snapshot);
        let stats = cache.sync(0, &snapshot, &[], redo, |cells, _, _, _| cells.iter().map(|c| c.c).collect());
        assert_eq!((stats.built, stats.kept + stats.moved), (1, 2), "the provisional row, though unchanged");
        term.snapshot(&mut snapshot);
        assert_eq!(cache.sync(0, &snapshot, &[], redo, build).built, 0, "built in full now");
    }

    #[test]
    fn a_rows_size_is_part_of_what_it_was_built_from() {
        let mut term = term(10, 4);
        let mut cache = RowCache::default();
        feed(&mut term, b"a\r\nb\r\nc");
        sync(&mut cache, &mut term, 0);
        // The same text at another size is another row: built again, not found by its content.
        feed(&mut term, b"\x1b[2;1H\x1b#6");
        let stats = sync(&mut cache, &mut term, 0);
        assert_eq!((stats.built, stats.kept + stats.moved), (1, 3), "{stats:?}");
        assert_eq!(sync(&mut cache, &mut term, 0).built, 0);
        // The row scrolls up with its size, and rows of the same text but another size don't match.
        feed(&mut term, b"\x1b[4;1H\n\n");
        let stats = sync(&mut cache, &mut term, 0);
        assert!(stats.built <= 2, "the new blank rows at most: {stats:?}");
    }

    #[test]
    fn a_new_key_rebuilds_everything() {
        let mut term = term(10, 3);
        let mut cache = RowCache::default();
        sync(&mut cache, &mut term, 0);
        assert_eq!(sync(&mut cache, &mut term, 1).built, 3);
    }

    /// A row built again is handed a replaced row's value to fill in place of a new one, and a
    /// row that scrolled is still found by its content, never built.
    #[test]
    fn a_rebuilt_row_gets_a_replaced_rows_buffer_and_a_scrolled_row_is_still_moved() {
        let mut term = term(10, 4);
        let mut cache: RowCache<u32, String> = RowCache::default();
        feed(&mut term, b"one\r\ntwo\r\nthree\r\nfour");
        let mut spares = Vec::new();
        // Syncs, noting for each build whether it was handed a spare.
        fn sync(cache: &mut RowCache<u32, String>, term: &mut TestTerm, spares: &mut Vec<bool>) -> SyncStats {
            let mut snapshot = ScreenSnapshot::default();
            term.snapshot(&mut snapshot);
            cache.sync(
                0,
                &snapshot,
                &[],
                |_| false,
                |cells, _, _, spare| {
                    spares.push(spare.is_some());
                    let mut text = spare.unwrap_or_default();
                    text.clear();
                    text.extend(cells.iter().map(|c| c.c));
                    text
                },
            )
        }
        assert_eq!(sync(&mut cache, &mut term, &mut spares).built, 4);
        assert_eq!(spares, [false; 4], "nothing to recycle in the first frame");
        spares.clear();
        // One more line scrolls everything up: three rows move, the new one reuses the spare.
        feed(&mut term, b"\r\nfive");
        let stats = sync(&mut cache, &mut term, &mut spares);
        assert_eq!((stats.built, stats.moved), (1, 3));
        assert_eq!(spares, [true], "the row that scrolled off is the spare");
        spares.clear();
        // Every row replaced by new content: every build has a spare.
        feed(&mut term, b"\x1b[H\x1b[2Jaaa\r\nbbb\r\nccc\r\nddd");
        let stats = sync(&mut cache, &mut term, &mut spares);
        assert_eq!(stats.built, 4);
        assert_eq!(spares, [true; 4]);
        for (row, text) in cache.rows() {
            assert_eq!(text.trim_end(), ["aaa", "bbb", "ccc", "ddd"][row]);
        }
    }
}
