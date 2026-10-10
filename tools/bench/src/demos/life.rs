//! Conway's Game of Life on the braille dot grid (every dot is a cell, so a 300×80 terminal runs a
//! 600×320 world), wrapped around the edges and colored by age: newborns glow white-cyan and the old
//! settle into deep violet. Gliders and glider guns are seeded among random soup, and a world that stops
//! changing gets fresh soup dropped into it. It stresses braille output where most cells change glyph
//! every generation.

use crate::canvas::{Braille, Rgb, Rng};
use crate::demo::{Demo, Frame};

/// Generations per second, whatever the frame rate.
const RATE: f64 = 30.0;
/// A full reseed this often, in seconds, so a clip never sits in a settled world.
const RESEED_EVERY: f64 = 30.0;
/// The population is "stuck" when it has stayed within this fraction of the grid for a few seconds.
const STUCK_BAND: f64 = 0.004;
const STUCK_SECONDS: f64 = 3.0;

const GUN: [&str; 9] = [
    "........................O...........",
    "......................O.O...........",
    "............OO......OO............OO",
    "...........O...O....OO............OO",
    "OO........O.....O...OO..............",
    "OO........O...O.OO....O.O...........",
    "..........O.....O.......O...........",
    "...........O...O....................",
    "............OO......................",
];
const GLIDER: [&str; 3] = [".O.", "..O", "OOO"];

pub struct Life {
    dots: Braille,
    w: usize,
    h: usize,
    /// 0 is dead, otherwise the age in generations (saturating).
    now: Vec<u8>,
    next: Vec<u8>,
    palette: Vec<Rgb>,
    rng: Rng,
    owed: f64,
    since_seed: f64,
    pop: u32,
    /// The lowest and highest population since the last check, and when that window began.
    lo: u32,
    hi: u32,
    window: f64,
}

impl Default for Life {
    fn default() -> Self {
        let stops = [
            (0.0, Rgb(210, 255, 255)),
            (2.0, Rgb(0, 235, 255)),
            (12.0, Rgb(40, 130, 255)),
            (50.0, Rgb(130, 60, 230)),
            (255.0, Rgb(75, 25, 150)),
        ];
        let palette = (0..256)
            .map(|age| {
                let a = age as f64;
                let k = stops.iter().position(|s| s.0 >= a).unwrap_or(stops.len() - 1).max(1);
                let ((a0, c0), (a1, c1)) = (stops[k - 1], stops[k]);
                c0.lerp(c1, (a - a0) / (a1 - a0))
            })
            .collect();
        Self {
            dots: Braille::new(),
            w: 0,
            h: 0,
            now: Vec::new(),
            next: Vec::new(),
            palette,
            rng: Rng::new(0x11fe_c0de),
            owed: 0.0,
            since_seed: 0.0,
            pop: 0,
            lo: u32::MAX,
            hi: 0,
            window: 0.0,
        }
    }
}

impl Life {
    fn stamp(&mut self, rows: &[&str], x: usize, y: usize, flip_x: bool, flip_y: bool) {
        let (pw, ph) = (rows.first().map_or(0, |r| r.len()), rows.len());
        for (j, row) in rows.iter().enumerate() {
            for (i, c) in row.bytes().enumerate() {
                if c == b'O' {
                    let (i, j) = (if flip_x { pw - 1 - i } else { i }, if flip_y { ph - 1 - j } else { j });
                    let (px, py) = ((x + i) % self.w, (y + j) % self.h);
                    self.now[py * self.w + px] = 1;
                }
            }
        }
    }

    fn soup(&mut self, x: usize, y: usize, w: usize, h: usize, density: f64) {
        for j in 0..h {
            for i in 0..w {
                let (px, py) = ((x + i) % self.w, (y + j) % self.h);
                self.now[py * self.w + px] = u8::from(self.rng.f() < density);
            }
        }
    }

    fn gliders(&mut self, n: usize) {
        for _ in 0..n {
            let (x, y) = (self.rng.below(self.w), self.rng.below(self.h));
            let (fx, fy) = (self.rng.f() < 0.5, self.rng.f() < 0.5);
            self.stamp(&GLIDER, x, y, fx, fy);
        }
    }

    fn reseed(&mut self) {
        self.now.fill(0);
        self.soup(0, 0, self.w, self.h, 0.22);
        let area = (self.w * self.h) as f64;
        self.gliders((area / 6000.0) as usize + 2);
        // A gun needs room; its glider stream then runs into the soup and the other guns' streams.
        if self.w >= 80 && self.h >= 40 {
            let guns = (area / 40000.0) as usize + 1;
            for _ in 0..guns {
                let (x, y) = (self.rng.below(self.w), self.rng.below(self.h));
                let (fx, fy) = (self.rng.f() < 0.5, self.rng.f() < 0.5);
                self.stamp(&GUN, x, y, fx, fy);
            }
        }
        self.since_seed = 0.0;
        self.reset_window();
    }

    /// Fresh soup in a few patches and some new gliders, leaving the rest of the world alone.
    fn stir(&mut self) {
        let patches = 3 + self.w * self.h / 30000;
        for _ in 0..patches {
            let pw = (self.w / 6).max(8).min(self.w);
            let ph = (self.h / 4).max(8).min(self.h);
            let (x, y) = (self.rng.below(self.w), self.rng.below(self.h));
            self.soup(x, y, pw, ph, 0.4);
        }
        self.gliders(4);
        self.reset_window();
    }

    fn reset_window(&mut self) {
        self.lo = u32::MAX;
        self.hi = 0;
        self.window = 0.0;
    }

    fn step(&mut self) {
        let (w, h) = (self.w, self.h);
        let mut pop = 0u32;
        for y in 0..h {
            let up = ((y + h - 1) % h) * w;
            let mid = y * w;
            let down = ((y + 1) % h) * w;
            for x in 0..w {
                let (l, r) = (if x == 0 { w - 1 } else { x - 1 }, if x + 1 == w { 0 } else { x + 1 });
                let n = [up + l, up + x, up + r, mid + l, mid + r, down + l, down + x, down + r]
                    .iter()
                    .filter(|&&i| self.now[i] > 0)
                    .count();
                let age = self.now[mid + x];
                let born = match (age > 0, n) {
                    (true, 2) | (true, 3) => age.saturating_add(1),
                    (false, 3) => 1,
                    _ => 0,
                };
                pop += u32::from(born > 0);
                self.next[mid + x] = born;
            }
        }
        std::mem::swap(&mut self.now, &mut self.next);
        self.pop = pop;
        self.lo = self.lo.min(pop);
        self.hi = self.hi.max(pop);
    }
}

impl Demo for Life {
    fn frame(&mut self, f: &mut Frame) {
        self.dots.fit(f.size.cols, f.size.rows);
        if (self.dots.w, self.dots.h) != (self.w, self.h) {
            self.w = self.dots.w;
            self.h = self.dots.h;
            self.now = vec![0; self.w * self.h];
            self.next = vec![0; self.w * self.h];
            if self.w >= 3 && self.h >= 3 {
                self.reseed();
            }
        }
        if self.w >= 3 && self.h >= 3 {
            self.since_seed += f.dt;
            // Never more than a few steps per frame: a stall shouldn't turn into a burst.
            self.owed = (self.owed + f.dt * RATE).min(4.0);
            while self.owed >= 1.0 {
                self.owed -= 1.0;
                self.step();
                self.window += 1.0 / RATE;
            }
            if self.since_seed >= RESEED_EVERY {
                self.reseed();
            } else if self.window >= STUCK_SECONDS {
                let band = (self.hi - self.lo) as f64;
                if band <= STUCK_BAND * (self.w * self.h) as f64 || self.pop == 0 {
                    self.stir();
                } else {
                    self.reset_window();
                }
            }
        }
        for y in 0..self.h {
            let row = &self.now[y * self.w..(y + 1) * self.w];
            for (x, &age) in row.iter().enumerate() {
                if age > 0 {
                    self.dots.dot(x as isize, y as isize, self.palette[age as usize]);
                }
            }
        }
        self.dots.render(f.out, Rgb(2, 2, 8));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::term::Size;

    #[test]
    fn gun_rows_are_all_the_same_width() {
        assert!(GUN.iter().all(|r| r.len() == 36));
    }

    #[test]
    fn a_glider_survives_a_lap() {
        let mut l = Life::default();
        (l.w, l.h) = (16, 16);
        l.now = vec![0; 256];
        l.next = vec![0; 256];
        l.stamp(&GLIDER, 2, 2, false, false);
        for _ in 0..64 {
            l.step();
        }
        // Four generations move a glider one dot diagonally, so 64 is a lap of the 16-wide torus.
        assert_eq!(l.pop, 5);
        assert!(l.now[(2 + 1) * 16 + 2 + 2] > 0);
    }

    #[test]
    fn draws_at_any_size_and_resizes() {
        let mut d = Life::default();
        let mut out = Vec::new();
        for (n, (c, r)) in [(0, 0), (1, 1), (20, 5), (80, 24), (300, 80), (60, 20)].into_iter().enumerate() {
            for k in 0..200u64 {
                out.clear();
                let size = Size { cols: c, rows: r, px_w: 0, px_h: 0 };
                let t = (n as u64 * 200 + k) as f64 / 60.0;
                d.frame(&mut Frame { size, t, dt: 1.0 / 60.0, out: &mut out });
            }
        }
        assert!(!out.is_empty());
    }
}
