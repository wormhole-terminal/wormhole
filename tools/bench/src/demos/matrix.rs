//! Digital rain: half-width katakana, digits and a few letters falling in two layers of columns, each
//! with a white-hot head and a truecolor green trail fading to black. Glyphs flip at random. It stresses
//! a full grid of single cells whose foreground changes every frame (the opposite of plasma's block
//! colors) and the terminal's half-width glyph handling. Speeds are in rows per second, so it looks the
//! same at any frame rate.

use crate::canvas::{Cells, Rgb, Rng};
use crate::demo::{Demo, Frame};

/// Glyph flips per second per cell, so a long trail shimmers instead of freezing.
const FLIP_RATE: f64 = 1.6;

struct Stream {
    head: f64,
    speed: f64,
    len: f64,
    /// 1 for the front layer, less for the one behind it.
    depth: f64,
}

impl Stream {
    fn new(rng: &mut Rng, rows: usize, front: bool, fresh: bool) -> Self {
        let rows_f = rows as f64;
        let (speed, depth) = if front { (rng.range(9.0, 30.0), 1.0) } else { (rng.range(5.0, 13.0), 0.42) };
        let len = rng.range(6.0, (rows_f * 0.85).max(10.0));
        // At the start the screen is already raining; afterwards a stream waits a little above the top.
        let head = if fresh { rng.range(-len, rows_f + len * 0.5) } else { -rng.range(0.0, rows_f * 0.7) };
        Stream { head, speed, len, depth }
    }
}

pub struct Matrix {
    cells: Cells,
    glyphs: Vec<char>,
    table: Vec<char>,
    streams: Vec<(usize, Stream)>,
    /// How bright each cell's trail is this frame (0..=1), and how white-hot (0..=1).
    trail: Vec<f32>,
    heat: Vec<f32>,
    palette: Vec<Rgb>,
    rng: Rng,
}

impl Default for Matrix {
    fn default() -> Self {
        let mut table: Vec<char> = Vec::new();
        for _ in 0..3 {
            table.extend((0xFF66u32..=0xFF9D).filter_map(char::from_u32));
        }
        table.extend("0123456789012345".chars());
        table.extend("ZTXAB:.=*+<>|".chars());
        // Brightness to color: black, a dim green, the saturated green of the trail, a mint near the head.
        let palette = (0..256)
            .map(|i| {
                let b = i as f64 / 255.0;
                if b < 0.5 {
                    Rgb(0, 12, 3).lerp(Rgb(0, 105, 30), b * 2.0)
                } else if b < 0.9 {
                    Rgb(0, 105, 30).lerp(Rgb(20, 235, 80), (b - 0.5) / 0.4)
                } else {
                    Rgb(20, 235, 80).lerp(Rgb(150, 255, 170), (b - 0.9) / 0.1)
                }
            })
            .collect();
        Self {
            cells: Cells::new(),
            glyphs: Vec::new(),
            table,
            streams: Vec::new(),
            trail: Vec::new(),
            heat: Vec::new(),
            palette,
            rng: Rng::new(0x4d41_5452_4958),
        }
    }
}

impl Matrix {
    fn glyph(&mut self) -> char {
        self.table[self.rng.below(self.table.len())]
    }

    fn reset(&mut self) {
        let (cols, rows) = (self.cells.cols, self.cells.rows);
        let n = cols * rows;
        self.trail = vec![0.0; n];
        self.heat = vec![0.0; n];
        self.glyphs = (0..n).map(|_| self.glyph()).collect();
        self.streams.clear();
        for col in 0..cols {
            // Two streams per column: a fast one in front and a slower, dimmer one behind.
            for front in [true, false] {
                let s = Stream::new(&mut self.rng, rows, front, true);
                self.streams.push((col, s));
            }
        }
    }
}

impl Demo for Matrix {
    fn frame(&mut self, f: &mut Frame) {
        if self.cells.fit(f.size.cols, f.size.rows) {
            self.reset();
        }
        let (cols, rows) = (self.cells.cols, self.cells.rows);
        if cols == 0 || rows == 0 {
            return;
        }
        let rows_f = rows as f64;

        self.trail.fill(0.0);
        self.heat.fill(0.0);
        for k in 0..self.streams.len() {
            let (col, s) = &mut self.streams[k];
            s.head += s.speed * f.dt;
            if s.head - s.len > rows_f {
                let front = s.depth > 0.9;
                *s = Stream::new(&mut self.rng, rows, front, false);
            }
            let (col, s) = (*col, &self.streams[k].1);
            let head_row = s.head.floor();
            let top = (s.head - s.len).floor().max(0.0) as usize;
            let bottom = head_row.min(rows_f - 1.0);
            if bottom < 0.0 {
                continue;
            }
            for r in top..=(bottom as usize) {
                let i = r * cols + col;
                let d = (s.head - r as f64).max(0.0);
                let b = ((1.0 - d / s.len).powf(1.3) * s.depth) as f32;
                if b > self.trail[i] {
                    self.trail[i] = b;
                }
            }
            if head_row >= 0.0 && head_row < rows_f {
                let i = head_row as usize * cols + col;
                self.heat[i] = self.heat[i].max(s.depth as f32);
                // The head always shows a fresh glyph, like the film's.
                self.glyphs[i] = self.table[self.rng.below(self.table.len())];
                if head_row >= 1.0 {
                    let above = i - cols;
                    self.heat[above] = self.heat[above].max(0.3 * s.depth as f32);
                }
            }
        }

        let flip = (FLIP_RATE * f.dt).min(1.0);
        for i in 0..self.glyphs.len() {
            if self.rng.f() < flip {
                self.glyphs[i] = self.table[self.rng.below(self.table.len())];
            }
            let (b, h) = (self.trail[i], self.heat[i]);
            if b < 0.02 && h == 0.0 {
                self.cells.ch[i] = ' ';
                self.cells.fg[i] = Rgb::BLACK;
            } else {
                let mut c = self.palette[(b.clamp(0.0, 1.0) * 255.0) as usize];
                if h > 0.0 {
                    c = c.lerp(Rgb(235, 255, 240), h as f64);
                }
                self.cells.ch[i] = self.glyphs[i];
                self.cells.fg[i] = c;
            }
            self.cells.bg[i] = Rgb::BLACK;
        }
        self.cells.render(f.out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::term::Size;

    fn run(cols: u16, rows: u16, frames: u64) -> usize {
        let mut m = Matrix::default();
        let mut out = Vec::new();
        for n in 0..frames {
            out.clear();
            let size = Size { cols, rows, px_w: 0, px_h: 0 };
            m.frame(&mut Frame { size, t: n as f64 / 60.0, dt: 1.0 / 60.0, out: &mut out });
        }
        out.len()
    }

    #[test]
    fn draws_at_any_size() {
        for (c, r) in [(0, 0), (1, 1), (20, 5), (80, 24), (300, 80)] {
            run(c, r, 30);
        }
        assert!(run(80, 24, 30) > 80 * 24);
    }

    #[test]
    fn table_has_only_single_column_glyphs() {
        assert!(
            Matrix::default().table.iter().all(|c| c.is_ascii() || ('\u{FF66}'..='\u{FF9D}').contains(c))
        );
    }
}
