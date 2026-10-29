//! What a frame paints, kept between frames and redone only where the screen
//! changed. Generic over the shaped-line type so it runs without a window in
//! tests and the benchmark.

use gpui_kit::{Bounds, Hsla, Path, Pixels, Point};
use wormhole_term::ScreenSnapshot;
use wormhole_term::types::LineSize;

use super::cache::{RowCache, SyncStats};
use super::glyphs::GlyphPainter;
use super::rows::{ImageRun, RowBuilder, Segment, Underline};
use super::sized;
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
    /// How big the row is drawn; the painter scales its columns by it.
    pub size: LineSize,
    /// Text on the row blinks.
    blinks: bool,
    /// Inline images' tiles on the row (drawn from the store each frame).
    pub images: Vec<ImageRun>,
    /// Drawn before every word on it was shaped (the frame's budget ran out): built again next frame.
    provisional: bool,
    shapes: Vec<(usize, char, Hsla)>,
    pub quads: Vec<(Bounds<Pixels>, Hsla)>,
    pub strokes: Vec<(Path<Pixels>, Hsla)>,
    /// The row and geometry `quads` and `strokes` were made for.
    placed: Option<(usize, Geometry)>,
}

impl<L> Default for PaintedRow<L> {
    fn default() -> Self {
        Self {
            lines: Vec::new(),
            backgrounds: Vec::new(),
            underlines: Vec::new(),
            struck: Vec::new(),
            overlined: Vec::new(),
            size: LineSize::default(),
            blinks: false,
            images: Vec::new(),
            provisional: false,
            shapes: Vec::new(),
            quads: Vec::new(),
            strokes: Vec::new(),
            placed: None,
        }
    }
}

/// What changed in one `prepare`.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct FrameStats {
    pub rows: SyncStats,
    pub rows_placed: usize,
    /// Rows drawn before all their words were shaped, which the next frame builds again.
    pub provisional: usize,
}

/// `L` is a word as painted.
pub(super) struct FrameCache<K, L> {
    pub snapshot: ScreenSnapshot,
    rows: RowCache<K, PaintedRow<L>>,
    /// Builds each row's words, keeping the buffers of the last row for the next.
    builder: RowBuilder,
    /// Lines of the row being rebuilt that are free to be shaped into again.
    lines: Vec<L>,
    painter: GlyphPainter,
}

impl<K, L> Default for FrameCache<K, L> {
    fn default() -> Self {
        Self {
            snapshot: ScreenSnapshot::default(),
            rows: RowCache::default(),
            builder: RowBuilder::default(),
            lines: Vec::new(),
            painter: GlyphPainter::default(),
        }
    }
}

impl<K: Clone + PartialEq, L> FrameCache<K, L> {
    /// Bring everything in line with `snapshot` (already updated). `key` is what
    /// shaping depends on; `found` is find's matches on each viewport row; `shape` shapes one
    /// segment, and says whether it shaped all of it (`false`: some of it waits for a later frame).
    pub fn prepare(
        &mut self,
        key: K,
        geometry: Geometry,
        palette: &Palette,
        found: &[Vec<(usize, usize, bool)>],
        mut shape: impl FnMut(&Segment, Option<L>) -> (L, bool),
    ) -> FrameStats {
        let mut stats = FrameStats::default();
        let colors = self.snapshot.colors;
        let redo = |painted: &PaintedRow<L>| painted.provisional;
        let Self { rows, snapshot, builder, lines, .. } = self;
        stats.rows = rows.sync(key.clone(), snapshot, found, redo, |cells, extras, marks, spare| {
            let built = builder.build(cells, extras, palette, &colors, marks);
            let mut painted = spare.unwrap_or_default();
            // The words' lines of the row being replaced are the next row's to fill again.
            lines.extend(painted.lines.drain(..).map(|(_, line)| line));
            let mut provisional = false;
            for segment in &built.segments {
                let (line, whole) = shape(segment, lines.pop());
                provisional |= !whole;
                painted.lines.push((segment.col, line));
            }
            lines.clear();
            // Swapped, not copied: the row keeps what the builder made, and the builder the
            // buffers the replaced row had, emptied at its next build.
            std::mem::swap(&mut painted.backgrounds, &mut built.backgrounds);
            std::mem::swap(&mut painted.underlines, &mut built.underlines);
            std::mem::swap(&mut painted.struck, &mut built.strikes);
            std::mem::swap(&mut painted.overlined, &mut built.overlines);
            std::mem::swap(&mut painted.images, &mut built.images);
            std::mem::swap(&mut painted.shapes, &mut built.shapes);
            painted.size = marks.size;
            painted.blinks = built.blinks;
            painted.provisional = provisional;
            painted.quads.clear();
            painted.strokes.clear();
            painted.placed = None;
            painted
        });
        stats.provisional = self.rows.rows().filter(|(_, painted)| painted.provisional).count();
        let painter = &mut self.painter;
        for (row, painted) in self.rows.rows_mut() {
            if painted.shapes.is_empty() || painted.placed == Some((row, geometry)) {
                continue;
            }
            // A double-width or double-height row's shapes are made for its own, bigger cell.
            let cell = sized::shape_metrics(painted.size, &geometry, row);
            painter.reset(
                cell.origin,
                cell.cell_width,
                cell.line_height,
                geometry.scale,
                cell.font_size,
                cell.stretch,
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

    /// Every inline image run in view, with its row. A double-width or double-height row draws
    /// none: a tile is a cell big, and the row's cells are not.
    pub fn image_runs(&self) -> impl Iterator<Item = (usize, &ImageRun)> {
        self.rows
            .rows()
            .filter(|(_, painted)| !painted.size.is_double())
            .flat_map(|(row, painted)| painted.images.iter().map(move |run| (row, run)))
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::{point, px};
    use wormhole_core::theme;
    use wormhole_term::testing::TestTerm;

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
        term.snapshot(&mut cache.snapshot);
        cache.prepare(0, geometry, &palette, &[], |segment, _| (segment.text.clone(), true))
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
}
