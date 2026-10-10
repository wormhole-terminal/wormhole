//! Warp-speed stars in braille: thousands of stars fly out of a vanishing point, each drawn as a
//! streak from where it was to where it is, bluer and brighter the closer it gets. Every ten seconds
//! the ship jumps and the streaks stretch across the screen. It stresses sparse, high-contrast output
//! where nearly every cell changes glyph and color each frame.

use crate::canvas::{Braille, Rgb, Rng};
use crate::demo::{Demo, Frame};

/// Seconds between jumps to hyperspace, and how long a jump lasts (the width of its bump).
const JUMP_EVERY: f64 = 10.0;
const JUMP_WIDTH: f64 = 0.55;
/// A streak shows this many seconds of the star's path.
const STREAK: f64 = 0.12;
const NEAR: f64 = 0.015;
/// How deep the field goes. Far stars gather around the vanishing point and come out of it, so the
/// screen reads as depth rather than an even spray of dots.
const FAR: f64 = 3.0;

struct Star {
    /// Position across the view at depth 1, in units of half the height.
    x: f64,
    y: f64,
    z: f64,
    hue: f64,
    sat: f64,
}

pub struct Starfield {
    dots: Braille,
    stars: Vec<Star>,
    rng: Rng,
}

impl Default for Starfield {
    fn default() -> Self {
        Self { dots: Braille::new(), stars: Vec::new(), rng: Rng::new(0x57a2_f1e1d) }
    }
}

impl Starfield {
    fn spawn(&mut self, aspect: f64, z: f64) -> Star {
        // Mostly blue-white; now and then an amber or a magenta one.
        let pick = self.rng.f();
        let (hue, sat) = if pick < 0.78 {
            (self.rng.range(0.55, 0.64), self.rng.range(0.08, 0.5))
        } else if pick < 0.9 {
            (self.rng.range(0.07, 0.13), self.rng.range(0.3, 0.6))
        } else {
            (self.rng.range(0.82, 0.92), self.rng.range(0.25, 0.5))
        };
        Star { x: self.rng.range(-1.6 * aspect, 1.6 * aspect), y: self.rng.range(-1.6, 1.6), z, hue, sat }
    }
}

/// Speed in depth units per second: a slow drift with a gentle swell, and a surge at each jump.
fn speed(t: f64) -> (f64, f64) {
    let phase = t.rem_euclid(JUMP_EVERY);
    let jump = (-((phase - (JUMP_EVERY - 1.0)) / JUMP_WIDTH).powi(2)).exp();
    let base = 0.28 + 0.1 * (t * 0.6).sin();
    (FAR * 0.8 * base * (1.0 + 7.0 * jump), jump)
}

impl Demo for Starfield {
    fn frame(&mut self, f: &mut Frame) {
        self.dots.fit(f.size.cols, f.size.rows);
        let (w, h) = (self.dots.w as f64, self.dots.h as f64);
        let aspect = w / h.max(1.0);
        let want = ((w * h) / 90.0).clamp(150.0, 3000.0) as usize;
        if self.stars.len() != want {
            self.stars.clear();
            for _ in 0..want {
                let z = self.rng.range(NEAR, FAR);
                let star = self.spawn(aspect, z);
                self.stars.push(star);
            }
        }

        let (v, jump) = speed(f.t);
        // The vanishing point drifts a little, as if the ship were banking.
        let cx = w * 0.5 + w * 0.04 * (f.t * 0.31).sin();
        let cy = h * 0.5 + h * 0.05 * (f.t * 0.23).cos();
        let focal = h * 0.5;
        let flash = Rgb(190, 215, 255);

        for i in 0..self.stars.len() {
            let s = &mut self.stars[i];
            s.z -= v * f.dt;
            let behind = s.z + (v * STREAK).max(0.002);
            let (x1, y1) = (cx + s.x / s.z.max(NEAR) * focal, cy + s.y / s.z.max(NEAR) * focal);
            let out = x1 < -2.0 || y1 < -2.0 || x1 > w + 2.0 || y1 > h + 2.0;
            if s.z <= NEAR || out {
                let z = FAR + self.rng.f() * 0.1;
                let star = self.spawn(aspect, z);
                self.stars[i] = star;
                continue;
            }
            let (x0, y0) = (cx + s.x / behind * focal, cy + s.y / behind * focal);
            let near = (1.0 - s.z / FAR).clamp(0.0, 1.0);
            let light = near.powi(2);
            if light < 0.06 {
                // Too far to see: drawing it would only gray the sky.
                continue;
            }
            let sat = (s.sat * (1.0 - near * 0.7)).clamp(0.0, 1.0);
            let head = Rgb::hsv(s.hue, sat, light).lerp(flash, jump * near * 0.6);
            let (x0, y0, x1, y1) = (x0 as isize, y0 as isize, x1 as isize, y1 as isize);
            let long = (x1 - x0).abs().max((y1 - y0).abs());
            if long > 6 {
                // The tail is dimmer than the head, so the streak reads as motion, not a line.
                let (mx, my) = ((x0 + x1) / 2, (y0 + y1) / 2);
                self.dots.line(x0, y0, mx, my, head.scale(0.35));
                self.dots.line(mx, my, x1, y1, head);
            } else {
                self.dots.line(x0, y0, x1, y1, head);
            }
        }
        self.dots.render(f.out, Rgb(1, 2, 8));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::term::Size;

    #[test]
    fn draws_at_any_size_and_through_a_jump() {
        for (c, r) in [(0, 0), (1, 1), (20, 5), (80, 24), (300, 80)] {
            let mut d = Starfield::default();
            let mut out = Vec::new();
            for n in 0..700u64 {
                out.clear();
                let size = Size { cols: c, rows: r, px_w: 0, px_h: 0 };
                d.frame(&mut Frame { size, t: n as f64 / 60.0, dt: 1.0 / 60.0, out: &mut out });
            }
            assert!(c == 0 || !out.is_empty());
        }
    }

    #[test]
    fn the_jump_is_faster_than_cruising() {
        assert!(speed(JUMP_EVERY - 1.0).0 > 3.0 * speed(2.0).0);
    }
}
