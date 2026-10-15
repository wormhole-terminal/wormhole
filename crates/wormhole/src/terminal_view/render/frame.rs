//! What a frame paints, kept between frames and redone only where the screen
//! changed. Generic over the shaped-line type so it runs without a window in
//! tests and the benchmark.

use gpui_kit::{Bounds, Hsla, Path, Pixels, Point};
use wormhole_core::terminal::ScreenSnapshot;
use wormhole_core::terminal::types::Cell;

use super::blur::blurred_row;
use super::cache::{RowCache, SyncStats};
use super::glyphs::GlyphPainter;
use super::rows::{ImageRun, Segment, Underline, build_row};
use crate::palette::Palette;

/// Where the grid is on screen, which box-drawing quads depend on.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Geometry {
    pub origin: Point<Pixels>,
    pub cell_width: Pixels,
    pub line_height: Pixels,
    pub scale: f32,
    pub font_size: f32,
}

/// One row ready to paint. Text and backgrounds are in columns, so the row can
/// move (output scrolled) without being shaped again; its quads are redone then.
pub(super) struct PaintedRow<L> {
    pub lines: Vec<(usize, L)>,
    pub backgrounds: Vec<(usize, usize, Hsla)>,
    pub underlines: Vec<Underline>,
    /// `[first, end)` columns struck through.
    pub struck: Vec<(usize, usize, Hsla)>,
    /// `[first, end)` columns with a line along the top of the cell.
    pub overlined: Vec<(usize, usize, Hsla)>,
    /// Text on the row blinks.
    blinks: bool,
    /// Inline images' tiles on the row (drawn from the store each frame).
    pub images: Vec<ImageRun>,
    shapes: Vec<(usize, char, Hsla)>,
    pub quads: Vec<(Bounds<Pixels>, Hsla)>,
    pub strokes: Vec<(Path<Pixels>, Hsla)>,
    /// The row and geometry `quads` and `strokes` were made for.
    placed: Option<(usize, Geometry)>,
}

struct Blurred<B> {
    cells: Vec<Cell>,
    line: Option<B>,
}

/// What changed in one `prepare`.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct FrameStats {
    pub rows: SyncStats,
    pub rows_placed: usize,
    pub blurred_built: usize,
}

/// `L` is a word as painted, `B` a faint row above the viewport.
pub(super) struct FrameCache<K, L, B = L> {
    pub snapshot: ScreenSnapshot,
    rows: RowCache<K, PaintedRow<L>>,
    above: Vec<Blurred<B>>,
    above_key: Option<K>,
    painter: GlyphPainter,
}

impl<K, L, B> Default for FrameCache<K, L, B> {
    fn default() -> Self {
        Self {
            snapshot: ScreenSnapshot::default(),
            rows: RowCache::default(),
            above: Vec::new(),
            above_key: None,
            painter: GlyphPainter::default(),
        }
    }
}

impl<K: Clone + PartialEq, L, B> FrameCache<K, L, B> {
    /// Bring everything in line with `snapshot` (already updated). `key` is what
    /// shaping depends on; `found` is find's matches on each viewport row; `shape` shapes one
    /// word, `shape_blurred` one faint row above the viewport.
    #[allow(clippy::too_many_arguments)]
    pub fn prepare(
        &mut self,
        key: K,
        geometry: Geometry,
        palette: &Palette,
        found: &[Vec<(usize, usize, bool)>],
        mut shape: impl FnMut(Segment) -> L,
        mut shape_blurred: impl FnMut(String, Vec<(usize, Hsla)>) -> B,
    ) -> FrameStats {
        let mut stats = FrameStats::default();
        let colors = self.snapshot.colors;
        stats.rows = self.rows.sync(key.clone(), &self.snapshot, found, |cells, extras, marks| {
            let built = build_row(cells, extras, palette, &colors, marks);
            PaintedRow {
                lines: built.segments.into_iter().map(|segment| (segment.col, shape(segment))).collect(),
                backgrounds: built.backgrounds,
                underlines: built.underlines,
                struck: built.strikes,
                overlined: built.overlines,
                blinks: built.blinks,
                images: built.images,
                shapes: built.shapes,
                quads: Vec::new(),
                strokes: Vec::new(),
                placed: None,
            }
        });
        let painter = &mut self.painter;
        for (row, painted) in self.rows.rows_mut() {
            if painted.shapes.is_empty() || painted.placed == Some((row, geometry)) {
                continue;
            }
            painter.reset(
                geometry.origin,
                geometry.cell_width,
                geometry.line_height,
                geometry.scale,
                geometry.font_size,
            );
            for (col, c, fg) in &painted.shapes {
                painter.add(*c, *col, row, *fg);
            }
            // Swap rather than move, so both sides keep their allocations.
            std::mem::swap(&mut painted.quads, &mut painter.rects);
            std::mem::swap(&mut painted.strokes, &mut painter.strokes);
            painted.placed = Some((row, geometry));
            stats.rows_placed += 1;
        }

        if self.above_key.as_ref() != Some(&key) {
            self.above.clear();
            self.above_key = Some(key);
        }
        let above = self.snapshot.above();
        self.above.truncate(above.len());
        for (k, cells) in above.iter().enumerate() {
            if self.above.get(k).is_some_and(|blurred| blurred.cells == *cells) {
                continue;
            }
            let line = blurred_row(cells, palette, &colors).map(|(text, runs)| shape_blurred(text, runs));
            stats.blurred_built += 1;
            let blurred = Blurred { cells: cells.clone(), line };
            match self.above.get_mut(k) {
                Some(slot) => *slot = blurred,
                None => self.above.push(blurred),
            }
        }
        stats
    }

    /// Every row with what was built for it.
    pub fn rows(&self) -> impl Iterator<Item = (usize, &PaintedRow<L>)> {
        self.rows.rows()
    }

    /// Whether any row in view has blinking text: only then does the blink clock run.
    pub fn any_blinks(&self) -> bool {
        self.rows.rows().any(|(_, painted)| painted.blinks)
    }

    /// Every inline image run in view, with its row.
    pub fn image_runs(&self) -> impl Iterator<Item = (usize, &ImageRun)> {
        self.rows.rows().flat_map(|(row, painted)| painted.images.iter().map(move |run| (row, run)))
    }

    /// The faint rows above the viewport, nearest first, by distance (1 = just above).
    pub fn blurred(&self) -> impl Iterator<Item = (usize, &B)> {
        self.above.iter().enumerate().filter_map(|(k, blurred)| Some((k + 1, blurred.line.as_ref()?)))
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::{point, px};
    use wormhole_core::terminal::testing::TestTerm;
    use wormhole_core::theme;

    use super::{FrameCache, FrameStats, Geometry};
    use crate::palette::Palette;

    const GEOMETRY: Geometry = Geometry {
        origin: gpui_kit::Point { x: px(10.0), y: px(42.0) },
        cell_width: px(8.0),
        line_height: px(18.0),
        scale: 2.0,
        font_size: 14.0,
    };

    fn term() -> TestTerm {
        TestTerm::new(20, 4, 100)
    }

    fn feed(term: &mut TestTerm, bytes: &[u8]) {
        term.feed(bytes);
    }

    fn prepare(cache: &mut FrameCache<u8, String>, term: &mut TestTerm, geometry: Geometry) -> FrameStats {
        let palette = Palette::from_theme(&theme::builtin()[0]);
        term.snapshot(&mut cache.snapshot, 2);
        cache.prepare(0, geometry, &palette, &[], |segment| segment.text, |text, _| text)
    }

    #[test]
    fn box_quads_are_redone_only_for_rows_that_moved() {
        let mut term = term();
        let mut cache = FrameCache::default();
        feed(&mut term, "a\r\n────\r\nb".as_bytes());
        assert_eq!(prepare(&mut cache, &mut term, GEOMETRY).rows_placed, 1);
        assert_eq!(prepare(&mut cache, &mut term, GEOMETRY).rows_placed, 0);
        let quads: Vec<usize> = cache.rows().map(|(_, row)| row.quads.len()).collect();
        assert_eq!(quads, [0, 1, 0, 0], "the rule is one quad on its row");
        // Output scrolls the rule up a row: same text, new quads.
        feed(&mut term, b"\r\nc\r\nd");
        let stats = prepare(&mut cache, &mut term, GEOMETRY);
        assert_eq!((stats.rows.built, stats.rows_placed), (2, 1), "{stats:?}");
        let (row, rule) = cache.rows().find(|(_, row)| !row.quads.is_empty()).expect("the rule");
        assert_eq!(row, 0);
        let top = rule.quads[0].0.top();
        assert!(top >= GEOMETRY.origin.y && top < GEOMETRY.origin.y + GEOMETRY.line_height, "drawn on row 0");
        // The pane moved: every row with shapes is placed again.
        let shifted = Geometry { origin: point(px(20.0), px(42.0)), ..GEOMETRY };
        assert_eq!(prepare(&mut cache, &mut term, shifted).rows_placed, 1);
    }

    #[test]
    fn blurred_rows_are_built_once_per_content() {
        let mut term = term();
        let mut cache = FrameCache::default();
        feed(&mut term, b"1\r\n2\r\n3\r\n4\r\n5\r\n6");
        assert_eq!(prepare(&mut cache, &mut term, GEOMETRY).blurred_built, 2);
        assert_eq!(prepare(&mut cache, &mut term, GEOMETRY).blurred_built, 0);
        let blurred: Vec<(usize, String)> = cache.blurred().map(|(k, text)| (k, text.clone())).collect();
        assert_eq!(blurred, [(1, "2".to_string()), (2, "1".to_string())]);
        feed(&mut term, b"\r\n7");
        assert_eq!(prepare(&mut cache, &mut term, GEOMETRY).blurred_built, 2);
    }
}
