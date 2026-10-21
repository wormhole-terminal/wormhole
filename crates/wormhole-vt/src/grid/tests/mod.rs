//! The grid's tests: unit tests per operation (`ops`), arguments past the screen's edge
//! (`clamping`), scrolling (`scrolling`), the history's cap (`history`), reflow (`reflow`) and the
//! other resizes (`resizing`), the size hint of double-width rows (`line_sizes`), picture
//! placements (`images`), the snapshot copy (`snapshot`), text, words and links (`text`) and a
//! randomized comparison with a model
//! (`random`). Every test that changes a grid ends with [`Grid::check_invariants`].

mod clamping;
mod everything;
mod history;
mod images;
mod line_sizes;
mod ops;
mod random;
mod reflow;
mod resizing;
mod scrolling;
mod snapshot;
mod text;

use crate::grid::{Grid, Pen, Region, Style};
use crate::types::{Color, Column, Flags, Line, NamedColor, Point};
use crate::unicode::width;

/// The default background.
pub(super) const BG: Color = Color::Named(NamedColor::Background);

pub(super) fn plain() -> Pen {
    Pen::default()
}

/// A pen with a foreground color.
pub(super) fn colored(n: u8) -> Pen {
    Pen { style: Style { fg: Color::Indexed(n), ..Style::DEFAULT }, ..Pen::default() }
}

pub(super) fn bold() -> Pen {
    Pen { style: Style { attrs: Flags::BOLD, ..Style::DEFAULT }, ..Pen::default() }
}

pub(super) fn full(grid: &Grid) -> Region {
    Region::full(grid.cols(), grid.rows())
}

pub(super) fn at(line: i32, column: usize) -> Point {
    Point::new(Line(line), Column(column))
}

/// The rows of the live screen as text, empty cells as spaces, trailing blanks cut.
pub(super) fn screen(grid: &Grid) -> Vec<String> {
    (0..grid.rows() as i32).map(|y| grid.row_text(Line(y), true)).collect()
}

/// Every row, history included, as text.
pub(super) fn all_rows(grid: &Grid) -> Vec<String> {
    let history = grid.history_len() as i32;
    (-history..grid.rows() as i32).map(|y| grid.row_text(Line(y), true)).collect()
}

/// The lines of the grid, history included, with soft-wrapped rows joined and trailing blanks and
/// blank lines at the end cut: what survives a reflow.
pub(super) fn logical_lines(grid: &Grid) -> Vec<String> {
    let history = grid.history_len() as i32;
    let mut lines = Vec::new();
    let mut current = String::new();
    for y in -history..grid.rows() as i32 {
        let wrapped = grid.is_wrapped(Line(y));
        current.push_str(&grid.row_text(Line(y), !wrapped));
        if !wrapped {
            lines.push(current.trim_end().to_string());
            current.clear();
        }
    }
    if !current.is_empty() {
        lines.push(current.trim_end().to_string());
    }
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines
}

/// A little terminal over a grid: a cursor, autowrap and line feeds, enough to make the wrapped
/// lines and the history real programs make. (The real one is `terminal`.)
pub(super) struct Sim {
    pub grid: Grid,
    pub x: usize,
    pub y: usize,
    pub pen: Pen,
    wrap_pending: bool,
}

impl Sim {
    pub fn new(cols: usize, rows: usize, scrollback: usize) -> Sim {
        Sim { grid: Grid::new(cols, rows, scrollback), x: 0, y: 0, pen: plain(), wrap_pending: false }
    }

    pub fn print(&mut self, text: &str) {
        for c in text.chars() {
            match c {
                '\n' => self.newline(),
                '\r' => {
                    self.x = 0;
                    self.wrap_pending = false;
                }
                c => self.put(c),
            }
        }
    }

    pub fn newline(&mut self) {
        self.x = 0;
        self.wrap_pending = false;
        self.linefeed();
    }

    fn linefeed(&mut self) {
        if self.y + 1 == self.grid.rows() {
            let region = full(&self.grid);
            self.grid.scroll_up(region, 1, BG);
        } else {
            self.y += 1;
        }
    }

    fn put(&mut self, c: char) {
        let cols = self.grid.cols();
        let w = width(c);
        if w == 0 {
            let x = if self.wrap_pending { self.x } else { self.x.saturating_sub(1) };
            self.grid.push_mark(self.y, x, c);
            return;
        }
        let w = w as usize;
        if self.wrap_pending {
            self.grid.set_wrapped(self.y, true);
            self.linefeed();
            self.x = 0;
            self.wrap_pending = false;
        }
        if self.x + w > cols {
            self.grid.write_spacer_head(self.y, &self.pen);
            self.grid.set_wrapped(self.y, true);
            self.linefeed();
            self.x = 0;
        }
        assert!(self.grid.write_cluster(self.y, self.x, c, &[], w as u8, &self.pen));
        self.x += w;
        if self.x >= cols {
            self.x = cols - 1;
            self.wrap_pending = true;
        }
    }

    pub fn check(&self) {
        if let Err(e) = self.grid.check_invariants() {
            panic!("{e}");
        }
    }
}

/// How many random cases a property test runs: `n`, times `GRID_TEST_SEEDS` when that is set (a
/// longer hunt for a rare bug: `GRID_TEST_SEEDS=50 cargo test --release -p wormhole-vt grid`).
pub(super) fn seeds(n: u64) -> u64 {
    n * std::env::var("GRID_TEST_SEEDS").ok().and_then(|v| v.parse().ok()).unwrap_or(1)
}

// A deterministic generator, so a failing seed reproduces: the one the other tests share.
// The same file is included by the parser's and the grid's tests on purpose.
#[allow(clippy::duplicate_mod)]
#[path = "../../../tests/common/rng.rs"]
mod rng;
pub(super) use rng::Rng;
