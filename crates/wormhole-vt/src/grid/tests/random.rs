//! Random operations against a model: a grid of characters and looks that does each operation the
//! obvious way, row by row. After every operation the grid's text, the style of every cell, its
//! history and its invariants must agree with the model's. Pages are tiny, so the live screen spans many of them and the
//! operations take their slow (cross-page) paths as often as their fast ones.

use super::{BG, Rng, colored, plain, seeds};
use crate::grid::{Grid, Pen, Region, Style};
use crate::types::{Color, Line, NamedColor};

/// A cell of the model: what is in it and how it looks.
type MCell = (char, Style);

const BLANK: MCell = (' ', Style::DEFAULT);

struct Model {
    cols: usize,
    screen: Vec<Vec<MCell>>,
    history: Vec<String>,
}

impl Model {
    fn new(cols: usize, rows: usize) -> Model {
        Model { cols, screen: vec![vec![BLANK; cols]; rows], history: Vec::new() }
    }

    fn row_text(&self, y: usize) -> String {
        self.screen[y].iter().map(|c| c.0).collect::<String>().trim_end().to_string()
    }

    fn write(&mut self, y: usize, x: usize, bytes: &[u8], style: Style) {
        for (i, &b) in bytes.iter().enumerate() {
            if x + i < self.cols {
                self.screen[y][x + i] = (b as char, style);
            }
        }
    }

    fn erase(&mut self, y: usize, x0: usize, x1: usize, bg: Color) {
        for x in x0..x1.min(self.cols) {
            self.screen[y][x] = (' ', Style::erased(bg));
        }
    }

    fn insert_cells(&mut self, y: usize, x: usize, right: usize, n: usize, bg: Color) {
        if x > right {
            return;
        }
        let n = n.min(right - x + 1);
        for i in (x..=right).rev() {
            self.screen[y][i] = if i >= x + n { self.screen[y][i - n] } else { (' ', Style::erased(bg)) };
        }
    }

    fn delete_cells(&mut self, y: usize, x: usize, right: usize, n: usize, bg: Color) {
        if x > right {
            return;
        }
        let n = n.min(right - x + 1);
        for i in x..=right {
            self.screen[y][i] = if i + n <= right { self.screen[y][i + n] } else { (' ', Style::erased(bg)) };
        }
    }

    /// Rows `from..=region.bottom` move up (or down) by `n` inside the region's columns.
    fn shift(&mut self, region: Region, from: usize, n: usize, up: bool, bg: Color) {
        let (l, r) = (region.left, region.right);
        let new = (' ', Style::erased(bg));
        if up {
            for y in from..=region.bottom {
                for x in l..=r {
                    self.screen[y][x] = if y + n <= region.bottom { self.screen[y + n][x] } else { new };
                }
            }
        } else {
            for y in (from..=region.bottom).rev() {
                for x in l..=r {
                    self.screen[y][x] = if y >= from + n { self.screen[y - n][x] } else { new };
                }
            }
        }
    }

    fn scroll_up(&mut self, region: Region, n: usize, history: bool, bg: Color) {
        let n = n.min(region.bottom - region.top + 1);
        let full_width = region.left == 0 && region.right == self.cols - 1;
        if region.top == 0 && full_width && history {
            for y in 0..n {
                self.history.push(self.row_text(y));
            }
        }
        self.shift(region, region.top, n, true, bg);
    }
}

pub(super) fn random_pen(rng: &mut Rng) -> Pen {
    match rng.below(4) {
        0 => plain(),
        1 => colored(rng.below(6) as u8),
        2 => Pen {
            style: crate::grid::Style { bg: Color::Indexed(rng.below(4) as u8), ..Default::default() },
            ..plain()
        },
        _ => Pen { protected: rng.chance(50), ..colored(1) },
    }
}

pub(super) fn random_region(rng: &mut Rng, cols: usize, rows: usize) -> Region {
    let (top, bottom) = {
        let a = rng.below(rows);
        let b = rng.below(rows);
        if rng.chance(40) { (0, rows - 1) } else { (a.min(b), a.max(b)) }
    };
    let (left, right) = {
        let a = rng.below(cols);
        let b = rng.below(cols);
        if rng.chance(65) { (0, cols - 1) } else { (a.min(b), a.max(b)) }
    };
    Region { top, bottom, left, right }
}

fn run(seed: u64, steps: usize) {
    let mut rng = Rng::new(seed);
    let cols = rng.range(2, 14);
    let rows = rng.range(1, 9);
    let keep_history = rng.chance(75);
    let limit = if keep_history { 1 << 30 } else { 0 };
    let mut grid = Grid::with_page_rows(cols, rows, limit, rng.range(1, 6));
    let mut model = Model::new(cols, rows);
    let bg = |rng: &mut Rng| if rng.chance(70) { BG } else { Color::Named(NamedColor::Blue) };
    for step in 0..steps {
        let op = rng.below(12);
        let desc = match op {
            0..=2 => {
                let (y, x) = (rng.below(rows), rng.below(cols));
                let len = rng.range(1, cols + 2);
                let bytes: Vec<u8> = (0..len).map(|_| b'a' + rng.below(26) as u8).collect();
                let pen = random_pen(&mut rng);
                grid.write_ascii(y, x, &bytes, &pen);
                model.write(y, x, &bytes, pen.style);
                format!("write {y},{x} {len}")
            }
            3 => {
                let y = rng.below(rows);
                let (a, b) = (rng.below(cols + 1), rng.below(cols + 1));
                let background = bg(&mut rng);
                grid.erase(y, a.min(b)..a.max(b), background, false);
                model.erase(y, a.min(b), a.max(b), background);
                format!("erase {y} {}..{}", a.min(b), a.max(b))
            }
            4 => {
                let (a, b) = (rng.below(rows + 1), rng.below(rows + 1));
                let background = bg(&mut rng);
                grid.erase_rows(a.min(b)..a.max(b), background, false);
                for y in a.min(b)..a.max(b).min(rows) {
                    model.erase(y, 0, cols, background);
                }
                format!("erase rows {}..{}", a.min(b), a.max(b))
            }
            5 | 6 => {
                let y = rng.below(rows);
                let (a, b) = (rng.below(cols), rng.below(cols));
                let n = rng.range(0, cols + 1);
                let background = bg(&mut rng);
                if op == 5 {
                    grid.insert_cells(y, a.min(b), a.max(b), n, background);
                    if n > 0 {
                        model.insert_cells(y, a.min(b), a.max(b), n, background);
                    }
                } else {
                    grid.delete_cells(y, a.min(b), a.max(b), n, background);
                    if n > 0 {
                        model.delete_cells(y, a.min(b), a.max(b), n, background);
                    }
                }
                format!(
                    "{} cells {y} {}..={} n {n}",
                    if op == 5 { "insert" } else { "delete" },
                    a.min(b),
                    a.max(b)
                )
            }
            7 | 8 => {
                let region = random_region(&mut rng, cols, rows);
                let n = rng.range(0, rows + 1);
                let background = bg(&mut rng);
                if op == 7 {
                    grid.scroll_up(region, n, background);
                    if n > 0 {
                        model.scroll_up(region, n, keep_history, background);
                    }
                } else {
                    grid.scroll_down(region, n, background);
                    if n > 0 {
                        let n = n.min(region.bottom - region.top + 1);
                        model.shift(region, region.top, n, false, background);
                    }
                }
                format!("scroll {} {region:?} n {n}", if op == 7 { "up" } else { "down" })
            }
            9 | 10 => {
                let region = random_region(&mut rng, cols, rows);
                let y = rng.range(region.top, region.bottom);
                let n = rng.range(0, rows + 1);
                let background = bg(&mut rng);
                if op == 9 {
                    grid.insert_lines(y, region, n, background);
                } else {
                    grid.delete_lines(y, region, n, background);
                }
                if n > 0 {
                    let n = n.min(region.bottom - y + 1);
                    model.shift(region, y, n, op == 10, background);
                }
                format!("{} lines {y} {region:?} n {n}", if op == 9 { "insert" } else { "delete" })
            }
            _ => {
                if rng.chance(20) {
                    grid.scroll_view(crate::types::Scroll::Delta(rng.below(5) as i32 - 2));
                    "scroll view".to_string()
                } else {
                    let y = rng.below(rows);
                    let wrapped = rng.chance(50);
                    grid.set_wrapped(y, wrapped);
                    format!("wrap {y} {wrapped}")
                }
            }
        };
        let context = || format!("seed {seed} step {step} ({desc}) on {cols}x{rows}, history {keep_history}");
        if let Err(e) = grid.check_invariants() {
            panic!("{e}: {}", context());
        }
        for y in 0..rows {
            assert_eq!(grid.row_text(Line(y as i32), true), model.row_text(y), "row {y}: {}", context());
            for x in 0..cols {
                let style = grid.style_at(crate::types::Point::new(Line(y as i32), crate::types::Column(x)));
                assert_eq!(style, Some(model.screen[y][x].1), "style of {y},{x}: {}", context());
            }
        }
        let history: Vec<String> = (0..grid.history_len())
            .map(|i| grid.row_text(Line(i as i32 - grid.history_len() as i32), true))
            .collect();
        assert_eq!(history, model.history, "history: {}", context());
    }
}

#[test]
fn random_operations_agree_with_the_model() {
    for seed in 1..=seeds(300) {
        run(seed * 7919, 120);
    }
}

#[test]
fn long_runs_agree_with_the_model() {
    for seed in 1..=seeds(12) {
        run(seed * 104_729, 3000);
    }
}
