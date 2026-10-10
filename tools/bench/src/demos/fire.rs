//! The Doom PSX fire: a grid of heat that climbs the screen, losing a little at random each row and
//! drifting with the wind, read through the original 37-color palette (blended up to 256 steps so
//! it stays smooth at full-screen size). Now and then the word WORMHOLE is held as a heat source
//! and burns. Every cell changes every frame, in a smooth gradient with a lot of flicker.

use crate::canvas::{Pixels, Rgb, Rng};
use crate::demo::{Demo, Frame};

/// Doom's palette, black to white.
const PALETTE: [Rgb; 37] = [
    Rgb(7, 7, 7),
    Rgb(31, 7, 7),
    Rgb(47, 15, 7),
    Rgb(71, 15, 7),
    Rgb(87, 23, 7),
    Rgb(103, 31, 7),
    Rgb(119, 31, 7),
    Rgb(143, 39, 7),
    Rgb(159, 47, 7),
    Rgb(175, 63, 7),
    Rgb(191, 71, 7),
    Rgb(199, 71, 7),
    Rgb(223, 79, 7),
    Rgb(223, 87, 7),
    Rgb(223, 87, 7),
    Rgb(215, 95, 7),
    Rgb(215, 95, 7),
    Rgb(215, 103, 15),
    Rgb(207, 111, 15),
    Rgb(207, 119, 15),
    Rgb(207, 127, 15),
    Rgb(207, 135, 23),
    Rgb(199, 135, 23),
    Rgb(199, 143, 23),
    Rgb(199, 151, 31),
    Rgb(191, 159, 31),
    Rgb(191, 159, 31),
    Rgb(191, 167, 39),
    Rgb(191, 167, 39),
    Rgb(191, 175, 47),
    Rgb(183, 175, 47),
    Rgb(183, 183, 47),
    Rgb(183, 183, 55),
    Rgb(207, 207, 111),
    Rgb(223, 223, 159),
    Rgb(239, 239, 199),
    Rgb(255, 255, 255),
];

/// Seconds between the word's appearances, and when in each period it burns.
const PERIOD: f64 = 12.0;
const WORD_ON: f64 = 2.5;
const WORD_OFF: f64 = 8.5;
const WORD: [&str; 8] = ["W", "O", "R", "M", "H", "O", "L", "E"];
/// The simulation's own clock, so the flames look the same at any frame rate.
const STEP: f64 = 1.0 / 60.0;

/// 5 × 7 glyphs, one row per byte, the lowest five bits (the left pixel is bit 4).
fn glyph(c: &str) -> [u8; 7] {
    match c {
        "W" => [0b10001, 0b10001, 0b10001, 0b10101, 0b10101, 0b11011, 0b10001],
        "O" => [0b01110, 0b10001, 0b10001, 0b10001, 0b10001, 0b10001, 0b01110],
        "R" => [0b11110, 0b10001, 0b10001, 0b11110, 0b10100, 0b10010, 0b10001],
        "M" => [0b10001, 0b11011, 0b10101, 0b10101, 0b10001, 0b10001, 0b10001],
        "H" => [0b10001, 0b10001, 0b10001, 0b11111, 0b10001, 0b10001, 0b10001],
        "L" => [0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b10000, 0b11111],
        _ => [0b11111, 0b10000, 0b10000, 0b11110, 0b10000, 0b10000, 0b11111],
    }
}

pub struct Fire {
    px: Pixels,
    heat: Vec<u8>,
    lut: [Rgb; 256],
    rng: Rng,
    acc: f64,
    /// Pixels of the word for the current size.
    word: Vec<usize>,
}

impl Default for Fire {
    fn default() -> Self {
        let mut lut = [Rgb::BLACK; 256];
        for (i, c) in lut.iter_mut().enumerate() {
            let x = i as f64 / 255.0 * 36.0;
            let k = (x as usize).min(35);
            *c = if i == 0 { Rgb::BLACK } else { PALETTE[k].lerp(PALETTE[k + 1], x - k as f64) };
        }
        Self { px: Pixels::new(), heat: Vec::new(), lut, rng: Rng::new(0xF1_2E), acc: 0.0, word: Vec::new() }
    }
}

impl Fire {
    /// The word's pixels, about 2/3 of the width and centered a little above the middle.
    fn layout_word(&mut self) {
        let (w, h) = (self.px.w, self.px.h);
        self.word.clear();
        let glyph_w = 5 + 1;
        let total = WORD.len() * glyph_w - 1;
        let scale = (w * 2 / 3 / total).min(h * 2 / 5 / 7).max(1);
        if total * scale > w || 7 * scale > h {
            return;
        }
        let x0 = (w - total * scale) / 2;
        let y0 = (h * 50 / 100).saturating_sub(7 * scale / 2);
        for (k, c) in WORD.iter().enumerate() {
            for (row, bits) in glyph(c).iter().enumerate() {
                for col in 0..5 {
                    if bits >> (4 - col) & 1 == 0 {
                        continue;
                    }
                    for dy in 0..scale {
                        for dx in 0..scale {
                            let (x, y) = (x0 + (k * glyph_w + col) * scale + dx, y0 + row * scale + dy);
                            if x < w && y < h {
                                self.word.push(y * w + x);
                            }
                        }
                    }
                }
            }
        }
    }

    fn step(&mut self, t: f64, word: f64) {
        let (w, h) = (self.px.w, self.px.h);
        if w == 0 || h < 2 {
            return;
        }
        // Flames reach about two thirds up; heat is 0..=255, so each row costs this much on average.
        let flame = (h as f64 * 0.52).max(8.0);
        let decay_q = (2.0 * 255.0 / flame * 256.0) as u32;
        let wind = (t * 0.45).sin() * 0.6 + (t * 1.7).sin() * 0.2;
        let bias = wind.abs();
        let sign = if wind < 0.0 { -1isize } else { 1 };
        for x in 0..w {
            // The base breathes a little so the flames' roots aren't all one height.
            let flicker = 235 + (self.rng.below(21)) as u8;
            self.heat[(h - 1) * w + x] = flicker;
        }
        for y in 1..h {
            for x in 0..w {
                let src = self.heat[y * w + x];
                let r = self.rng.next_u64();
                let mut dx = [1isize, 0, -1, 0][(r & 3) as usize];
                if ((r >> 8) & 255) as f64 / 255.0 < bias {
                    dx += sign;
                }
                let dst_x = (x as isize + dx).clamp(0, w as isize - 1) as usize;
                let dec = (((((r >> 16) & 255) as u32 * decay_q) + 0x8000) >> 16) as u8;
                self.heat[(y - 1) * w + dst_x] = src.saturating_sub(dec);
            }
        }
        if word > 0.0 {
            let v = (word * 165.0) as u8;
            for &i in &self.word {
                self.heat[i] = v;
            }
        }
    }
}

/// How much of the word is lit at this time: fades in, holds, fades out.
fn word_level(t: f64) -> f64 {
    let p = t.rem_euclid(PERIOD);
    let ramp = |x: f64| x.clamp(0.0, 1.0);
    let k = ramp((p - WORD_ON) / 0.9).min(ramp((WORD_OFF - p) / 1.2));
    k * k * (3.0 - 2.0 * k)
}

impl Demo for Fire {
    fn frame(&mut self, f: &mut Frame) {
        if self.px.fit(f.size.cols, f.size.rows) {
            self.heat = vec![0; self.px.w * self.px.h];
            self.layout_word();
        }
        self.acc = (self.acc + f.dt).min(0.2);
        let level = word_level(f.t);
        while self.acc >= STEP {
            self.acc -= STEP;
            self.step(f.t, level);
        }
        for (p, &h) in self.px.px.iter_mut().zip(&self.heat) {
            *p = self.lut[h as usize];
        }
        // The letters themselves shine white-hot over the flames they feed, so the word stays
        // readable instead of dissolving into the fire above it.
        for &i in &self.word {
            self.px.px[i] = self.px.px[i].lerp(Rgb(255, 250, 225), level);
        }
        self.px.render(f.out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::term::Size;

    #[test]
    fn draws_at_any_size() {
        for (cols, rows) in [(1, 1), (2, 1), (20, 5), (80, 24), (301, 81)] {
            let mut demo = Fire::default();
            let mut out = Vec::new();
            for n in 0..4 {
                out.clear();
                let size = Size { cols, rows, px_w: 0, px_h: 0 };
                demo.frame(&mut Frame { size, t: 3.0 + n as f64 * 0.7, dt: 0.1, out: &mut out });
                assert!(!out.is_empty());
            }
        }
    }

    #[test]
    fn the_word_fits_a_wide_screen() {
        let mut demo = Fire::default();
        demo.px.fit(200, 40);
        demo.layout_word();
        assert!(!demo.word.is_empty());
    }

    /// `cargo test --release timing -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn timing_at_full_screen() {
        let mut demo = Fire::default();
        let mut out = Vec::new();
        let size = Size { cols: 300, rows: 80, px_w: 0, px_h: 0 };
        let mut worst = 0.0f64;
        for n in 0..120u64 {
            out.clear();
            let start = std::time::Instant::now();
            demo.frame(&mut Frame { size, t: 3.0 + n as f64 / 60.0, dt: 1.0 / 60.0, out: &mut out });
            worst = worst.max(start.elapsed().as_secs_f64() * 1e3);
        }
        println!("fire: worst frame {worst:.2} ms, last {} KB", out.len() / 1000);
    }
}
