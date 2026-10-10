//! An endless zoom into the Mandelbrot set, toward seahorse valley, with smooth (normalized
//! iteration count) coloring through a cosine palette and a slow turn of the view. Every pixel is
//! its own escape-time loop, so it is the one demo that stresses the benchmark's CPU as much as the
//! terminal: rows are shared out over all cores. When f64 runs out of digits the picture fades and
//! the zoom starts again.

use std::thread;

use crate::canvas::{Pixels, Rgb};
use crate::demo::{Demo, Frame};

const CENTER: (f64, f64) = (-0.743_643_887_037_151, 0.131_825_904_205_330);
/// View height in the plane at the start and at the deepest point (a pixel is still ~1e-14 there,
/// far above f64's epsilon at this magnitude).
const SCALE_START: f64 = 3.0;
const SCALE_END: f64 = 6.0e-12;
/// e-folds of zoom per second.
const RATE: f64 = 0.55;
const FADE_IN: f64 = 1.2;
const FADE_OUT: f64 = 1.5;
/// Squared escape radius; a large one makes the smooth coloring accurate.
const BAILOUT: f64 = 65536.0;

#[derive(Default)]
pub struct Mandelbrot {
    px: Pixels,
}

/// Everything a row needs, copied into each thread.
#[derive(Clone, Copy)]
struct View {
    /// The plane's coordinate of the first pixel's center and the step along a row and a column.
    origin: (f64, f64),
    dx: (f64, f64),
    dy: (f64, f64),
    max_iter: u32,
    /// How close (squared) an orbit must come to an earlier point of itself to count as a cycle.
    tol: f64,
    shift: f64,
    light: f64,
}

/// In the main cardioid or the period-2 bulb: inside, and cheap to rule out.
#[inline]
fn in_bulbs(cr: f64, ci: f64) -> bool {
    let q = (cr - 0.25) * (cr - 0.25) + ci * ci;
    q * (q + (cr - 0.25)) < 0.25 * ci * ci || (cr + 1.0) * (cr + 1.0) + ci * ci < 0.0625
}

/// The smoothed iteration counts of four points, or `None` for those that never escape. The four
/// orbits advance together: one orbit is a chain of dependent multiplications that leaves the
/// core idle most of the time, four side by side fill the gaps (and let the compiler use vectors).
fn escape4(cr: [f64; 4], ci: [f64; 4], max_iter: u32, tol: f64) -> [Option<f64>; 4] {
    let mut res = [None; 4];
    let mut live = [true; 4];
    let mut left = 4;
    for k in 0..4 {
        if in_bulbs(cr[k], ci[k]) {
            live[k] = false;
            left -= 1;
        }
    }
    let (mut x, mut y, mut x2, mut y2) = ([0.0f64; 4], [0.0f64; 4], [0.0f64; 4], [0.0f64; 4]);
    // Brent's cycle detection: a point whose orbit settles into a cycle is inside; stopping there
    // saves running every interior pixel to the limit. Near the boundary orbits linger close to a
    // cycle for ages before escaping, so the tolerance follows the pixel size.
    let (mut sx, mut sy) = ([0.0f64; 4], [0.0f64; 4]);
    for i in 1..=max_iter {
        if left == 0 {
            break;
        }
        for k in 0..4 {
            y[k] = 2.0 * x[k] * y[k] + ci[k];
            x[k] = x2[k] - y2[k] + cr[k];
            x2[k] = x[k] * x[k];
            y2[k] = y[k] * y[k];
        }
        for k in 0..4 {
            if !live[k] {
                continue;
            }
            let m = x2[k] + y2[k];
            if m > BAILOUT {
                // n + 1 - log2(ln|z|), the continuous escape time.
                res[k] = Some(i as f64 + 1.0 - (0.5 * m.ln()).log2());
                live[k] = false;
                left -= 1;
                continue;
            }
            let (ex, ey) = (x[k] - sx[k], y[k] - sy[k]);
            if ex * ex + ey * ey < tol {
                live[k] = false;
                left -= 1;
            }
        }
        if i & (i - 1) == 0 {
            sx = x;
            sy = y;
        }
    }
    res
}

fn color(nu: Option<f64>, shift: f64, light: f64) -> Rgb {
    let Some(nu) = nu else { return Rgb(3, 3, 8).scale(light) };
    // Log scale keeps the bands about the same width on screen as the detail gets finer.
    let t = 0.8 * (nu + 1.0).ln() + 0.012 * nu + shift;
    let c = Rgb::cosine(t, [0.5, 0.45, 0.55], [0.5, 0.5, 0.5], [1.0, 1.0, 1.0], [0.0, 0.12, 0.30]);
    // Points that escape fast are the far-away background: let them sink into the dark.
    let edge = (nu / 14.0).min(1.0);
    c.scale((0.15 + 0.85 * edge) * light)
}

fn render_row(v: View, y: usize, row: &mut [Rgb]) {
    let (r0, i0) = (v.origin.0 + v.dy.0 * y as f64, v.origin.1 + v.dy.1 * y as f64);
    for (g, chunk) in row.chunks_mut(4).enumerate() {
        let (mut cr, mut ci) = ([0.0; 4], [0.0; 4]);
        for k in 0..4 {
            // A short last group repeats its last pixel; the extra lanes are dropped.
            let x = (g * 4 + k.min(chunk.len() - 1)) as f64;
            cr[k] = r0 + v.dx.0 * x;
            ci[k] = i0 + v.dx.1 * x;
        }
        let n = escape4(cr, ci, v.max_iter, v.tol);
        for (p, nu) in chunk.iter_mut().zip(n) {
            *p = color(nu, v.shift, v.light);
        }
    }
}

impl Demo for Mandelbrot {
    fn frame(&mut self, f: &mut Frame) {
        self.px.fit(f.size.cols, f.size.rows);
        let (w, h) = (self.px.w, self.px.h);
        if w == 0 || h == 0 {
            return;
        }
        let cycle = (SCALE_START / SCALE_END).ln() / RATE;
        let phase = f.t.rem_euclid(cycle);
        let scale = SCALE_START * (-RATE * phase).exp();
        let light = (phase / FADE_IN).min((cycle - phase) / FADE_OUT).clamp(0.0, 1.0);
        let light = light * light * (3.0 - 2.0 * light);
        let max_iter = (120.0 + 50.0 * (SCALE_START / scale).ln()) as u32;

        // The view turns slowly around the target; `scale` is the height, pixels are square.
        let angle = 0.10 * phase;
        let (sin, cos) = angle.sin_cos();
        let step = scale / h as f64;
        let (dx, dy) = ((cos * step, sin * step), (-sin * step, cos * step));
        let (hx, hy) = (w as f64 / 2.0 - 0.5, h as f64 / 2.0 - 0.5);
        let origin = (CENTER.0 - dx.0 * hx - dy.0 * hy, CENTER.1 - dx.1 * hx - dy.1 * hy);
        let view = View {
            tol: (step * 0.05).powi(2).clamp(1e-30, 1e-18),
            origin,
            dx,
            dy,
            max_iter,
            shift: 0.04 * f.t,
            light,
        };

        let threads = thread::available_parallelism().map(|n| n.get()).unwrap_or(4).clamp(1, 32).min(h);
        // Rows are dealt out round robin: neighbors cost about the same, so each thread gets a fair mix.
        let mut work: Vec<Vec<(usize, &mut [Rgb])>> = (0..threads).map(|_| Vec::new()).collect();
        for (y, row) in self.px.px.chunks_mut(w).enumerate() {
            work[y % threads].push((y, row));
        }
        thread::scope(|s| {
            for rows in work {
                s.spawn(move || {
                    for (y, row) in rows {
                        render_row(view, y, row);
                    }
                });
            }
        });
        self.px.render(f.out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::term::Size;

    #[test]
    fn the_set_and_its_surroundings() {
        let n = escape4([0.0, -1.0, 1.0, -0.75], [0.0, 0.0, 1.0, 0.1], 500, 1e-20);
        assert!(n[0].is_none() && n[1].is_none());
        assert!(n[2].is_some() && n[3].is_some());
    }

    /// `cargo test --release timing -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn timing_across_the_zoom() {
        let cycle = (SCALE_START / SCALE_END).ln() / RATE;
        let mut demo = Mandelbrot::default();
        let mut out = Vec::new();
        for k in 0..10 {
            let t = cycle * k as f64 / 10.0 + 0.3;
            let start = std::time::Instant::now();
            out.clear();
            let size = Size { cols: 300, rows: 80, px_w: 0, px_h: 0 };
            demo.frame(&mut Frame { size, t, dt: 0.016, out: &mut out });
            println!("t={t:5.1}s  {:6.2} ms", start.elapsed().as_secs_f64() * 1e3);
        }
    }

    #[test]
    fn draws_at_any_size_and_through_a_restart() {
        let cycle = (SCALE_START / SCALE_END).ln() / RATE;
        for (cols, rows) in [(1, 1), (2, 1), (20, 5), (80, 24)] {
            let mut demo = Mandelbrot::default();
            let mut out = Vec::new();
            for t in [0.0, 1.0, cycle - 0.1, cycle + 0.1] {
                out.clear();
                let size = Size { cols, rows, px_w: 0, px_h: 0 };
                demo.frame(&mut Frame { size, t, dt: 0.016, out: &mut out });
                assert!(!out.is_empty());
            }
        }
    }
}
