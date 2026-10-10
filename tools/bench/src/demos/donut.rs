//! The spinning torus from a1k0n's donut.c, in truecolor: luminance picks the ASCII character, and
//! the color blends a warm key light with a cool rim light by surface normal, over a dark
//! background. A z-buffer, aspect-correct projection, scaled to fill the terminal. It stresses
//! small colored glyphs rewritten in nearly every cell every frame.

use std::f64::consts::TAU;

use crate::canvas::{Cells, Rgb};
use crate::demo::{Demo, Frame};

/// Dark to light, as in the original.
const RAMP: &[u8] = b".,-~:;=!*#$@";
const BACKGROUND: Rgb = Rgb(6, 6, 12);
/// Tube radius, the ring's radius, and the viewer's distance from the torus' center.
const R1: f64 = 1.0;
const R2: f64 = 2.0;
const K2: f64 = 5.0;

#[derive(Default)]
pub struct Donut {
    cells: Cells,
    zbuf: Vec<f64>,
}

fn norm(v: [f64; 3]) -> [f64; 3] {
    let l = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt().max(1e-9);
    [v[0] / l, v[1] / l, v[2] / l]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The surface's color: a warm orange-pink where the key light hits, cool blue where the rim light
/// does, a little violet in the shade between.
fn shade(key: f64, rim: f64, facing: f64) -> Rgb {
    let warm = [1.0, 0.36 + 0.30 * key, 0.16 + 0.40 * key];
    let cool = [0.25, 0.50, 1.0];
    let violet = [0.30, 0.16, 0.55];
    let k = key.max(0.0).powf(0.9);
    let r = (rim.max(0.0) * 0.9 + (1.0 - facing).powi(3) * 0.3).min(1.0);
    let mut c = [0.0; 3];
    for i in 0..3 {
        c[i] = warm[i] * k * 1.35 + cool[i] * r * 1.15 + violet[i] * 0.25;
    }
    Rgb::f(c[0], c[1], c[2])
}

impl Demo for Donut {
    fn frame(&mut self, f: &mut Frame) {
        if self.cells.fit(f.size.cols, f.size.rows) {
            self.zbuf = vec![0.0; self.cells.cols * self.cells.rows];
        }
        self.cells.clear(BACKGROUND);
        self.zbuf.fill(0.0);
        let (cols, rows) = (self.cells.cols, self.cells.rows);
        if cols == 0 || rows == 0 {
            return;
        }
        // A cell is about twice as tall as wide. The terminal's pixel size would say more exactly,
        // but it often counts padding and tab bars too, which skews the torus more than 2:1 does.
        let aspect = 2.0;
        // The torus' outer edge, at its nearest, should land near the screen's edge in the smaller
        // dimension (in rows).
        let fit = (rows as f64).min(cols as f64 / aspect);
        let k1 = fit * K2 * 0.75 / (R1 + R2) * 0.55;
        let (cx, cy) = (cols as f64 / 2.0, rows as f64 / 2.0);

        let t = f.t;
        let (a, b) = (t * 0.9, t * 0.55 + 0.4 * (t * 0.3).sin());
        let (sa, ca, sb, cb) = (a.sin(), a.cos(), b.sin(), b.cos());
        let key_dir = norm([-0.35, 0.65, -0.65]);
        let rim_dir = norm([0.75, -0.45, -0.15]);

        // Finer sampling on bigger screens so no cell falls through the surface.
        let scale = (rows as f64 / 22.0).max(1.0);
        let (dtheta, dphi) = ((0.07 / scale).max(0.012), (0.02 / scale).max(0.004));
        let n_phi = (TAU / dphi) as usize + 1;
        let phis: Vec<(f64, f64)> = (0..n_phi).map(|i| (i as f64 * dphi).sin_cos()).collect();

        let mut theta = 0.0;
        while theta < TAU {
            let (st, ct) = theta.sin_cos();
            let ring = R2 + R1 * ct;
            for &(sp, cp) in &phis {
                // The circle revolved around the y axis, then turned by A about x and B about z.
                let (px, py, pz) = (ring * cp, R1 * st, -ring * sp);
                let (nx, ny, nz) = (ct * cp, st, -ct * sp);
                let rot = |x: f64, y: f64, z: f64| {
                    let (y1, z1) = (y * ca - z * sa, y * sa + z * ca);
                    (x * cb - y1 * sb, x * sb + y1 * cb, z1)
                };
                let (x, y, z) = rot(px, py, pz);
                let n = rot(nx, ny, nz);
                let z = z + K2;
                let ooz = 1.0 / z;
                let xp = (cx + k1 * aspect * x * ooz) as isize;
                let yp = (cy - k1 * y * ooz) as isize;
                if xp < 0 || yp < 0 || xp as usize >= cols || yp as usize >= rows {
                    continue;
                }
                let i = yp as usize * cols + xp as usize;
                if ooz <= self.zbuf[i] {
                    continue;
                }
                let n = [n.0, n.1, n.2];
                // Facing away from the viewer: an inner surface hidden behind a nearer one.
                let facing = -n[2];
                let key = dot(n, key_dir);
                let rim = dot(n, rim_dir);
                let lum = (0.85 * key.max(0.0) + 0.35 * rim.max(0.0) + 0.04).clamp(0.0, 0.999);
                self.zbuf[i] = ooz;
                let ch = RAMP[(lum * RAMP.len() as f64) as usize] as char;
                let c = shade(key, rim, facing.max(0.0));
                // The body glows faintly behind its glyph so the shape reads as a solid.
                self.cells.put(xp as usize, yp as usize, ch, c, c.scale(0.10).add(BACKGROUND));
            }
            theta += dtheta;
        }
        self.cells.render(f.out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::term::Size;

    #[test]
    fn draws_at_any_size() {
        for (cols, rows) in [(1, 1), (2, 1), (20, 5), (80, 24), (301, 81)] {
            let mut demo = Donut::default();
            let mut out = Vec::new();
            for n in 0..3 {
                out.clear();
                let size = Size { cols, rows, px_w: 0, px_h: 0 };
                demo.frame(&mut Frame { size, t: n as f64 * 0.7, dt: 0.016, out: &mut out });
                assert!(!out.is_empty());
            }
        }
    }

    #[test]
    fn something_is_drawn_in_the_middle() {
        let mut demo = Donut::default();
        let mut out = Vec::new();
        let size = Size { cols: 80, rows: 24, px_w: 0, px_h: 0 };
        demo.frame(&mut Frame { size, t: 0.0, dt: 0.016, out: &mut out });
        assert!(demo.cells.ch.iter().any(|&c| c != ' '));
    }

    /// `cargo test --release timing -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn timing_at_full_screen() {
        let mut demo = Donut::default();
        let mut out = Vec::new();
        let size = Size { cols: 300, rows: 80, px_w: 0, px_h: 0 };
        let mut worst = 0.0f64;
        for n in 0..120u64 {
            out.clear();
            let start = std::time::Instant::now();
            demo.frame(&mut Frame { size, t: 3.0 + n as f64 / 60.0, dt: 1.0 / 60.0, out: &mut out });
            worst = worst.max(start.elapsed().as_secs_f64() * 1e3);
        }
        println!("donut: worst frame {worst:.2} ms, last {} KB", out.len() / 1000);
    }
}
