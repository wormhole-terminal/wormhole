//! The signature clip: a flight down a bending tunnel of violet, electric blue and cyan, rings
//! rushing past, a white-hot light at the far end and sparks streaking by. Classic tunnel mapping
//! (angle and 1/distance) with the axis swaying over depth, tone mapped and dithered so the dark
//! gradients stay smooth. It stresses the densest case there is: every cell changes both colors
//! every frame, with long runs of near-identical but never equal values.

use std::f64::consts::TAU;

use crate::canvas::{Pixels, Rgb, Rng};
use crate::demo::{Demo, Frame};

/// The tunnel's radius in screen-height units at depth 1; smaller makes a narrower, deeper tunnel.
const RADIUS: f64 = 0.40;
/// Depth where the light at the end takes over and the texture is fogged away.
const Z_MAX: f64 = 40.0;
const TABLE: usize = 256;
const STARS: usize = 56;

/// A 4×4 Bayer matrix: ordered dithering is free per pixel and hides 8-bit banding in the glow.
const BAYER: [f64; 16] = [0., 8., 2., 10., 12., 4., 14., 6., 3., 11., 1., 9., 15., 7., 13., 5.];

/// Two ramps from the dark end to the bright end; the tunnel drifts between them so the walls shift
/// from violet to cyan as you fly.
const VIOLET: [[f64; 3]; 5] =
    [[0.01, 0.0, 0.05], [0.14, 0.02, 0.38], [0.52, 0.12, 0.95], [0.80, 0.32, 1.0], [0.95, 0.66, 1.0]];
const CYAN: [[f64; 3]; 5] =
    [[0.0, 0.01, 0.06], [0.02, 0.10, 0.42], [0.06, 0.42, 1.0], [0.26, 0.88, 1.0], [0.62, 1.0, 1.0]];

struct Star {
    angle: f64,
    speed: f64,
    phase: f64,
    len: f64,
    tint: f64,
}

pub struct Wormhole {
    px: Pixels,
    stars: Vec<Star>,
    /// The axis' screen offset by depth, filled each frame so pixels only interpolate.
    axis: [(f64, f64); TABLE],
}

impl Default for Wormhole {
    fn default() -> Self {
        let mut rng = Rng::new(0x57_0123_4567);
        let stars = (0..STARS)
            .map(|_| Star {
                angle: rng.range(0.0, TAU),
                speed: rng.range(0.10, 0.42),
                phase: rng.f(),
                len: rng.range(0.10, 0.30),
                tint: rng.f(),
            })
            .collect();
        Self { px: Pixels::new(), stars, axis: [(0.0, 0.0); TABLE] }
    }
}

/// Where the tunnel's axis lies at a distance along the flight: two slow sines per direction, so the
/// path wanders without repeating soon.
fn path(w: f64) -> (f64, f64) {
    (
        1.5 * (0.21 * w).sin() + 0.9 * (0.113 * w + 2.0).sin(),
        1.2 * (0.17 * w + 1.0).cos() + 0.7 * (0.29 * w).sin(),
    )
}

fn ramp(stops: &[[f64; 3]; 5], x: f64) -> [f64; 3] {
    let x = x.clamp(0.0, 1.0) * 4.0;
    let i = (x as usize).min(3);
    let f = x - i as f64;
    let f = f * f * (3.0 - 2.0 * f);
    let (a, b) = (stops[i], stops[i + 1]);
    [a[0] + (b[0] - a[0]) * f, a[1] + (b[1] - a[1]) * f, a[2] + (b[2] - a[2]) * f]
}

impl Wormhole {
    fn fill_axis(&mut self, travel: f64) {
        let (x0, y0) = path(travel);
        for (i, a) in self.axis.iter_mut().enumerate() {
            let z = i as f64 * Z_MAX / (TABLE - 1) as f64;
            let (x, y) = path(travel + z);
            // Seen in perspective a bend far down the tunnel moves the wall less than a near one.
            let k = z / (z + 1.0) / (z + 2.5) * 2.2;
            *a = ((x - x0) * k, (y - y0) * k);
        }
    }

    #[inline]
    fn axis_at(&self, z: f64) -> (f64, f64) {
        let f = (z / Z_MAX).clamp(0.0, 1.0) * (TABLE - 1) as f64;
        let i = (f as usize).min(TABLE - 2);
        let k = f - i as f64;
        let (a, b) = (self.axis[i], self.axis[i + 1]);
        (a.0 + (b.0 - a.0) * k, a.1 + (b.1 - a.1) * k)
    }

    fn tunnel(&mut self, t: f64, travel: f64) {
        let (w, h) = (self.px.w, self.px.h);
        let aspect = w as f64 / h.max(1) as f64;
        let core = self.axis_at(Z_MAX);
        let roll = 0.5 * (t * 0.23).sin() + 0.25 * (t * 0.11 + 1.0).sin();
        let mix = |k: f64| 0.5 + 0.5 * k;
        for y in 0..h {
            let py = ((y as f64 + 0.5) / h as f64 - 0.5) * 2.0;
            for x in 0..w {
                let pxx = ((x as f64 + 0.5) / w as f64 - 0.5) * aspect * 2.0;
                // Find the wall the pixel sees: the axis offset depends on the depth, which depends on
                // the offset. Two rounds are enough for a gentle bend.
                let (mut qx, mut qy) = (pxx, py);
                let mut z = (RADIUS / (qx * qx + qy * qy).sqrt().max(1e-3)).min(Z_MAX);
                for _ in 0..2 {
                    let (ox, oy) = self.axis_at(z);
                    qx = pxx - ox;
                    qy = py - oy;
                    z = (RADIUS / (qx * qx + qy * qy).sqrt().max(1e-3)).min(Z_MAX);
                }
                let r = (qx * qx + qy * qy).sqrt();
                let a = qy.atan2(qx) + roll + z * 0.045;
                let d = z + travel;

                // Texture: thin bright rings, a wide soft band every few of them, a slow swirl and
                // spiral spokes. Multiples of `a` by whole numbers keep the seam at ±π invisible.
                let ring = {
                    let u = (d * 2.0).fract() - 0.5;
                    (-u * u * 70.0).exp()
                };
                let band = {
                    let u = (d * 0.4).fract() - 0.5;
                    (-u * u * 22.0).exp()
                };
                let swirl = mix((a * 3.0 + d * 1.0 + 1.8 * (d * 0.19).sin()).sin());
                let swirl2 = mix((a * 5.0 - d * 1.6 + 2.0).sin());
                let spokes = mix((a * 14.0 + d * 1.4).cos()).powi(8);
                let cloud = swirl * swirl * 0.55 + swirl2 * swirl * 0.25;
                let mut i =
                    0.06 + cloud * 0.55 + ring * (0.25 + 0.55 * swirl) + band * 0.22 + spokes * 0.30 * swirl;

                // Fog toward the far end, and a dark vignette on the near walls at the screen's edge.
                let fog = 1.0 / (1.0 + z * z / 30.0);
                let vignette = 1.0 - ((r - 0.6) / 1.0).clamp(0.0, 1.0).powi(2);
                i *= fog * vignette * 0.85;

                let depth = (z / 0.6).ln() / 4.0;
                let tint = (depth + 0.25 * (t * 0.2 + a * 2.0 + d * 0.05).sin()).clamp(0.0, 1.0);
                let (c1, c2) = (ramp(&VIOLET, i), ramp(&CYAN, i));
                let mut c = [0.0; 3];
                for k in 0..3 {
                    c[k] = c1[k] + (c2[k] - c1[k]) * tint;
                }

                // The light at the far end: a haze that gathers with depth and a white-hot core
                // around the point the tunnel runs to.
                let haze = (1.0 - (-z * 0.06).exp()).powi(2);
                let (cx, cy) = (pxx - core.0, py - core.1);
                let dist2 = cx * cx + cy * cy;
                let hot = 1.0 / (1.0 + dist2 * 140.0);
                let glow = 0.60 * haze * haze + 1.6 * hot * hot + 0.30 / (1.0 + dist2 * 14.0);
                let (gr, gg, gb) = (0.55 + 0.45 * hot, 0.62 + 0.38 * hot, 1.0);
                c[0] += glow * gr;
                c[1] += glow * gg;
                c[2] += glow * gb;

                // Tone map (a soft shoulder keeps the rings from clipping to flat color), then dither.
                let dither = (BAYER[(y & 3) * 4 + (x & 3)] / 16.0 - 0.47) / 255.0;
                let tone = |v: f64| 1.0 - (-v * 1.6).exp();
                self.px.px[y * w + x] = Rgb::f(tone(c[0]) + dither, tone(c[1]) + dither, tone(c[2]) + dither);
            }
        }
    }

    /// Sparks that fly out of the far light and past the viewer, drawn as short additive streaks
    /// that lengthen and brighten as they near.
    fn sparks(&mut self, travel: f64) {
        let (w, h) = (self.px.w as f64, self.px.h as f64);
        if w < 2.0 || h < 2.0 {
            return;
        }
        let aspect = w / h;
        let (cx, cy) = self.axis_at(Z_MAX);
        for s in &self.stars {
            let p = (s.phase + travel * s.speed * 0.12).fract();
            let fade = (p * 6.0).min(1.0) * (1.0 - ((p - 0.85) / 0.15).clamp(0.0, 1.0));
            if fade <= 0.0 {
                continue;
            }
            let (dx, dy) = (s.angle.cos(), s.angle.sin());
            let steps = 14;
            let r1 = 0.03 + p.powf(2.4) * 2.4;
            let r0 = (r1 - s.len * (0.15 + p * 1.2)).max(0.02);
            let hue = if s.tint < 0.4 {
                [0.75, 0.80, 1.0]
            } else if s.tint < 0.8 {
                [0.55, 0.95, 1.0]
            } else {
                [0.85, 0.60, 1.0]
            };
            for k in 0..=steps {
                let f = k as f64 / steps as f64;
                let r = r0 + (r1 - r0) * f;
                // The head is brightest and the tail fades out.
                let b = fade * f * f * (0.35 + 0.65 * p);
                let sx = ((cx + dx * r) / (2.0 * aspect) + 0.5) * w;
                let sy = ((cy + dy * r) / 2.0 + 0.5) * h;
                splat(&mut self.px, sx, sy, hue, b);
            }
        }
    }
}

/// Adds light to the four pixels around a point, weighted by how close each is.
fn splat(px: &mut Pixels, x: f64, y: f64, hue: [f64; 3], b: f64) {
    let (x, y) = (x - 0.5, y - 0.5);
    if x < -1.0 || y < -1.0 || x >= px.w as f64 || y >= px.h as f64 {
        return;
    }
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    for (ix, iy, wgt) in [
        (0.0, 0.0, (1.0 - fx) * (1.0 - fy)),
        (1.0, 0.0, fx * (1.0 - fy)),
        (0.0, 1.0, (1.0 - fx) * fy),
        (1.0, 1.0, fx * fy),
    ] {
        let (qx, qy) = (x0 + ix, y0 + iy);
        if qx < 0.0 || qy < 0.0 || qx >= px.w as f64 || qy >= px.h as f64 {
            continue;
        }
        let k = wgt * b * 255.0;
        let add = Rgb((hue[0] * k) as u8, (hue[1] * k) as u8, (hue[2] * k) as u8);
        let (qx, qy) = (qx as usize, qy as usize);
        let cur = px.get(qx, qy);
        px.set(qx, qy, cur.add(add));
    }
}

impl Demo for Wormhole {
    fn frame(&mut self, f: &mut Frame) {
        self.px.fit(f.size.cols, f.size.rows);
        // The flight surges a little so the rings don't tick past at a fixed beat.
        let travel = f.t * 1.6 + 0.5 * (f.t * 0.35).sin();
        self.fill_axis(travel);
        self.tunnel(f.t, travel);
        self.sparks(travel);
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
            let mut demo = Wormhole::default();
            let mut out = Vec::new();
            for n in 0..3 {
                out.clear();
                let size = Size { cols, rows, px_w: 0, px_h: 0 };
                demo.frame(&mut Frame { size, t: n as f64 * 0.7, dt: 0.016, out: &mut out });
                assert!(!out.is_empty());
            }
        }
    }

    /// `cargo test --release timing -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn timing_at_full_screen() {
        let mut demo = Wormhole::default();
        let mut out = Vec::new();
        let size = Size { cols: 300, rows: 80, px_w: 0, px_h: 0 };
        let mut worst = 0.0f64;
        for n in 0..120u64 {
            out.clear();
            let start = std::time::Instant::now();
            demo.frame(&mut Frame { size, t: 3.0 + n as f64 / 60.0, dt: 1.0 / 60.0, out: &mut out });
            worst = worst.max(start.elapsed().as_secs_f64() * 1e3);
        }
        println!("wormhole: worst frame {worst:.2} ms, last {} KB", out.len() / 1000);
    }
}
