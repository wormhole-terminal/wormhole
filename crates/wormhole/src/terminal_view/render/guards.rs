//! The work a frame does, counted: words shaped and rows rebuilt for screens that used to make the
//! renderer slow. Counts, not timings, so they hold on any machine and in a debug build; a change
//! that makes a frame shape or rebuild more than these say fails here instead of showing up as a
//! stutter later. `scripts/perf.sh` measures the times.

use std::cell::Cell;

use gpui_kit::{FontId, GlyphId, point, px};
use wormhole_core::alacritty_terminal::event::VoidListener;
use wormhole_core::alacritty_terminal::term::{Config, Term};
use wormhole_core::alacritty_terminal::vte::ansi::{Processor, StdSyncHandler};
use wormhole_core::terminal::GridSize;

use super::frame::{FrameCache, Geometry};
use super::rows::Segment;
use super::words::{Glyph, Shaped, WordCache};
use crate::palette::Palette;

const COLS: usize = 60;
const ROWS: usize = 12;

const GEOMETRY: Geometry = Geometry {
    origin: gpui_kit::Point { x: px(0.0), y: px(0.0) },
    cell_width: px(8.0),
    line_height: px(18.0),
    scale: 2.0,
    font_size: 14.0,
};

/// A seeded generator (xorshift), so every run draws the same screens.
struct Rng(u64);

impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n as u64) as usize
    }
}

/// The renderer's caches with a shaper that counts what it is asked to shape.
struct Renderer {
    term: Term<VoidListener>,
    frame: FrameCache<u8, Vec<Glyph>, String>,
    words: WordCache<u8>,
    palette: Palette,
}

impl Renderer {
    fn new() -> Self {
        let size = GridSize { cols: COLS as u16, rows: ROWS as u16, cell_width: 8.0, cell_height: 18.0 };
        let mut words = WordCache::default();
        words.sync(0);
        Self {
            term: Term::new(Config { scrolling_history: 100, ..Config::default() }, &size, VoidListener),
            frame: FrameCache::default(),
            words,
            palette: Palette::from_theme(&wormhole_core::theme::builtin()[0]),
        }
    }

    /// Feed `bytes` and build the frame; returns how many words (or parts of words) were shaped.
    fn frame(&mut self, bytes: &[u8]) -> usize {
        Processor::<StdSyncHandler>::new().advance(&mut self.term, bytes);
        self.frame.snapshot.update(&mut self.term, 0);
        let shaped = Cell::new(0);
        let words = &mut self.words;
        self.frame.prepare(
            0,
            GEOMETRY,
            &self.palette,
            &[],
            |segment| {
                words.glyphs(&segment, GEOMETRY.cell_width, |segment| {
                    shaped.set(shaped.get() + 1);
                    one_glyph_per_char(segment)
                })
            },
            |text, _| text,
        );
        shaped.get()
    }
}

fn one_glyph_per_char(segment: &Segment) -> Vec<Shaped> {
    segment
        .text
        .char_indices()
        .enumerate()
        .map(|(k, (index, c))| Shaped {
            position: point(px(8.0 * k as f32), px(0.0)),
            index,
            font_id: FontId(0),
            id: GlyphId(c as u32),
            emoji: false,
        })
        .collect()
}

/// A full screen in `cmatrix`'s style: every cell a random half-width katakana in a random green,
/// so every row is a new word every frame.
fn katakana_rain(rng: &mut Rng) -> Vec<u8> {
    let mut out = b"\x1b[H".to_vec();
    for row in 0..ROWS {
        for _ in 0..COLS {
            let c = char::from_u32(0xFF66 + rng.below(0xFF9D - 0xFF66 + 1) as u32).unwrap_or('ｱ');
            out.extend_from_slice(format!("\x1b[38;2;0;{};0m{c}", 80 + rng.below(170)).as_bytes());
        }
        if row + 1 < ROWS {
            out.extend_from_slice(b"\r\n");
        }
    }
    out
}

/// A full screen of truecolor half blocks, every cell its own two colors (the `wormhole` demo).
fn half_blocks(rng: &mut Rng) -> Vec<u8> {
    let mut out = b"\x1b[H".to_vec();
    for row in 0..ROWS {
        for _ in 0..COLS {
            let (r, g, b) = (rng.below(256), rng.below(256), rng.below(256));
            out.extend_from_slice(format!("\x1b[38;2;{r};{g};{b};48;2;{b};{r};{g}m▀").as_bytes());
        }
        if row + 1 < ROWS {
            out.extend_from_slice(b"\r\n");
        }
    }
    out
}

#[test]
fn katakana_rain_shapes_each_character_once_not_each_new_word() {
    let mut renderer = Renderer::new();
    let mut rng = Rng(0x2545_f491_4f6c_dd1d);
    // Seeing every character once takes a few screens; after that nothing is new.
    for _ in 0..12 {
        renderer.frame(&katakana_rain(&mut rng));
    }
    for _ in 0..20 {
        assert_eq!(renderer.frame(&katakana_rain(&mut rng)), 0, "a new screen of seen characters");
    }
}

#[test]
fn truecolor_half_blocks_are_drawn_without_shaping() {
    let mut renderer = Renderer::new();
    let mut rng = Rng(7);
    for _ in 0..5 {
        assert_eq!(renderer.frame(&half_blocks(&mut rng)), 0, "half blocks are shapes, not text");
    }
    let rows_with_quads = renderer.frame.rows().filter(|(_, row)| !row.quads.is_empty()).count();
    assert_eq!(rows_with_quads, ROWS, "every row's half blocks are quads");
}

#[test]
fn a_word_is_shaped_once_whatever_its_color_or_row() {
    let mut renderer = Renderer::new();
    let words = ["cargo", "build", "--release", "warning:", "unused", "->", "!=", "src/main.rs"];
    let mut rng = Rng(11);
    let mut shaped = 0;
    for _ in 0..30 {
        let mut line = String::from("\r\n");
        for _ in 0..6 {
            let word = words[rng.below(words.len())];
            line.push_str(&format!("\x1b[3{}m{word}\x1b[0m ", 1 + rng.below(6)));
        }
        shaped += renderer.frame(line.as_bytes());
    }
    assert_eq!(shaped, words.len(), "each word once, in any color, on any row, as output scrolls");
}

#[test]
fn an_unchanged_screen_rebuilds_nothing() {
    let mut renderer = Renderer::new();
    let mut rng = Rng(3);
    renderer.frame(&katakana_rain(&mut rng));
    for _ in 0..5 {
        let stats = {
            renderer.frame.snapshot.update(&mut renderer.term, 0);
            renderer.frame.prepare(0, GEOMETRY, &renderer.palette, &[], |_| Vec::new(), |text, _| text)
        };
        assert_eq!((stats.rows.built, stats.rows_placed), (0, 0), "{stats:?}");
    }
}
