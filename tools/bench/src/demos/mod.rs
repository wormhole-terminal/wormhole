//! The animations, each in a file of its own. `ALL` is the order `list` and `showcase` use.

mod dashboard;
mod donut;
mod fire;
mod life;
mod mandelbrot;
mod matrix;
mod plasma;
mod starfield;
mod unicode;
mod wormhole;

use crate::demo::{Demo, Entry};

fn boxed<D: Demo + Default + 'static>() -> Box<dyn Demo> {
    Box::new(D::default())
}

#[rustfmt::skip]
pub const ALL: &[Entry] = &[
    Entry { name: "wormhole", about: "a truecolor tunnel, two pixels per cell, every cell new every frame", make: boxed::<wormhole::Wormhole> },
    Entry { name: "plasma", about: "full-screen truecolor plasma, every cell new every frame", make: boxed::<plasma::Plasma> },
    Entry { name: "starfield", about: "warp-speed stars in braille, eight dots per cell", make: boxed::<starfield::Starfield> },
    Entry { name: "fire", about: "the Doom fire, a palette of 37 colors climbing the screen", make: boxed::<fire::Fire> },
    Entry { name: "matrix", about: "digital rain in half-width katakana with truecolor trails", make: boxed::<matrix::Matrix> },
    Entry { name: "donut", about: "a spinning lit torus, ASCII shading in truecolor", make: boxed::<donut::Donut> },
    Entry { name: "mandelbrot", about: "an endless zoom into the Mandelbrot set, smooth coloring", make: boxed::<mandelbrot::Mandelbrot> },
    Entry { name: "life", about: "Conway's Game of Life in braille, colored by age", make: boxed::<life::Life> },
    Entry { name: "unicode", about: "emoji, CJK, combining marks and box drawing at full speed", make: boxed::<unicode::Unicode> },
    Entry { name: "dashboard", about: "a busy system monitor: sparklines, gauges, a scrolling log", make: boxed::<dashboard::Dashboard> },
];

pub fn find(name: &str) -> Option<&'static Entry> {
    ALL.iter().find(|e| e.name == name)
}
