//! Rows built for earlier frames, kept until their content changes.
//!
//! The snapshot says which rows changed. A row that didn't, with the same
//! marks (selection, find's matches), keeps what was built for it. A changed row first looks for its
//! exact content among the rows being replaced: when output scrolls, every row
//! "changes" but all except the new ones were built a frame ago, one line up.
//! Only rows with new content are built (and shaped) again.

use wormhole_core::terminal::ScreenSnapshot;
use wormhole_core::terminal::types::{Cell, Color, RowExtras};

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
}

impl<K, T> Default for RowCache<K, T> {
    fn default() -> Self {
        Self { key: None, rows: Vec::new(), pool: Vec::new() }
    }
}

impl<K: PartialEq, T> RowCache<K, T> {
    /// Bring the cache in line with `snapshot`, building rows it can't reuse. `found` is find's
    /// matches on each row (rows past its end have none).
    pub fn sync(
        &mut self,
        key: K,
        snapshot: &ScreenSnapshot,
        found: &[Vec<(usize, usize, bool)>],
        mut build: impl FnMut(&[Cell], &RowExtras, &Marks) -> T,
    ) -> SyncStats {
        if self.key.as_ref() != Some(&key) {
            self.rows.clear();
            self.key = Some(key);
        }
        let rows = snapshot.rows();
        self.rows.resize_with(rows.len(), || None);

        let marks = |row: usize| Marks {
            selected: snapshot.selected_columns(row),
            found: found.get(row).cloned().unwrap_or_default(),
        };
        let mut stats = SyncStats::default();
        let mut stale = Vec::new();
        for (row, entry) in self.rows.iter_mut().enumerate() {
            let same_marks = |e: &Entry<T>| {
                e.marks.selected == snapshot.selected_columns(row)
                    && e.marks.found.as_slice() == found.get(row).map_or(&[][..], Vec::as_slice)
            };
            let keep = !rows[row].changed() && entry.as_ref().is_some_and(same_marks);
            if keep {
                stats.kept += 1;
            } else {
                self.pool.extend(entry.take());
                stale.push(row);
            }
        }
        for row in stale {
            let (cells, extras) = (rows[row].cells(), rows[row].extras());
            let marks = marks(row);
            let hash = row_hash(cells, extras, &marks);
            let reusable = self
                .pool
                .iter()
                .position(|e| e.hash == hash && e.marks == marks && e.cells == cells && e.extras == *extras);
            self.rows[row] = Some(match reusable {
                Some(index) => {
                    stats.moved += 1;
                    self.pool.swap_remove(index)
                }
                None => {
                    stats.built += 1;
                    let value = build(cells, extras, &marks);
                    Entry { cells: cells.to_vec(), extras: extras.clone(), marks, hash, value }
                }
            });
        }
        self.pool.clear();
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
    use wormhole_core::terminal::ScreenSnapshot;
    use wormhole_core::terminal::testing::TestTerm;
    use wormhole_core::terminal::types::{Column, Line, Point, SelectionType, Side};

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
        term.snapshot(&mut snapshot, 0);
        let stats = cache.sync(key, &snapshot, &[], |cells, _, _| cells.iter().map(|c| c.c).collect());
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
        term.snapshot(&mut snapshot, 0);
        let mut sync = |found: &[Vec<(usize, usize, bool)>]| {
            term.snapshot(&mut snapshot, 0);
            cache.sync(0, &snapshot, found, |cells, _, _| cells.iter().map(|c| c.c).collect()).built
        };
        sync(&[]);
        assert_eq!(sync(&[vec![], vec![(0, 0, true)]]), 1, "the row with a match");
        assert_eq!(sync(&[vec![], vec![(0, 0, true)]]), 0);
        // Stepping to another match moves "current" between two rows.
        assert_eq!(sync(&[vec![], vec![(0, 0, false)], vec![(0, 0, true)]]), 2);
        assert_eq!(sync(&[]), 2, "closing find clears them");
    }

    #[test]
    fn a_new_key_rebuilds_everything() {
        let mut term = term(10, 3);
        let mut cache = RowCache::default();
        sync(&mut cache, &mut term, 0);
        assert_eq!(sync(&mut cache, &mut term, 1).built, 3);
    }
}
