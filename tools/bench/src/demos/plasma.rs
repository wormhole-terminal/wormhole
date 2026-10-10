//! The classic demoscene plasma: a sum of sines per pixel, through a cosine palette. Every cell
//! changes both colors every frame, so it is the densest truecolor picture there is.

use crate::canvas::{Pixels, Rgb};
use crate::demo::{Demo, Frame};

#[derive(Default)]
pub struct Plasma {
    px: Pixels,
}

impl Demo for Plasma {
    fn frame(&mut self, f: &mut Frame) {
        self.px.fit(f.size.cols, f.size.rows);
        let t = f.t;
        self.px.shade(|u, v, aspect| {
            let (x, y) = ((u - 0.5) * aspect * 6.0, (v - 0.5) * 6.0);
            let mut s = (x + t).sin();
            s += ((y + t) * 0.5).sin();
            s += ((x + y + t) * 0.5).sin();
            let (cx, cy) = (x + 2.0 * (t / 3.0).sin(), y + 2.0 * (t / 2.0).cos());
            s += ((cx * cx + cy * cy + 1.0).sqrt() + t).sin();
            Rgb::cosine(s * 0.25 + t * 0.05, [0.5; 3], [0.5; 3], [1.0, 1.0, 1.0], [0.0, 0.10, 0.20])
        });
        self.px.render(f.out);
    }
}
