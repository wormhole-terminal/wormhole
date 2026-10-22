//! Frame-build cost of the terminal renderer on a 200×60 screen of agent-style
//! output (colors, bold, dim, a `╭─╮` box, a rule), without a window: the text
//! system's own shaping is left out, but what feeds it (the strings, the runs)
//! is counted. Run with
//! `cargo test -p wormhole --features devctl frame_build -- --ignored --nocapture`.
//!
//! The same screen through the renderer this replaced (one `String`, `Font` and
//! `TextRun` per cell, every cell of every frame) measured 1001 µs and 13,839
//! allocations per frame, 39 lines shaped (7,400 glyphs, half of them trailing
//! blanks), 1,204 box quads and 5,200 blurred glyph paints.

use std::time::Instant;

use gpui_kit::{Font, SharedString, TextRun, font, point, px};
use wormhole_term::testing::TestTerm;

use super::frame::{FrameCache, Geometry};
use super::shaping::word_runs;
use crate::palette::Palette;

/// This thread's allocations so far, from the test build's counting allocator
/// (`crate::perf::allocations`), so tests running in parallel don't disturb a measurement.
pub(super) fn allocations() -> usize {
    usize::try_from(crate::perf::allocations::thread_counts().allocations).unwrap_or(usize::MAX)
}

/// Allocations a frame may make. A full frame builds every row; most of its allocations are this
/// benchmark's stand-in for the text system (`fed`: three per word, and a row of words is many of
/// them since rows are shaped word by word), which real frames don't pay. The steady frames are the
/// ones that matter: nothing changed, or one line of output scrolled the screen.
const FULL_FRAME_ALLOCATIONS: usize = 2600;
const UNCHANGED_FRAME_ALLOCATIONS: usize = 50;
const SCROLLING_FRAME_ALLOCATIONS: usize = 300;

const COLS: usize = 200;
const ROWS: usize = 60;

fn term() -> TestTerm {
    let mut term = TestTerm::new(COLS as u16, ROWS as u16, 10_000);
    let mut bytes = Vec::new();
    for i in 0..400 {
        let line = format!(
            "\x1b[1;34m●\x1b[0m \x1b[2mRead\x1b[0m(\x1b[33msrc/terminal_view/render.rs\x1b[0m) line {i} \x1b[32m+12\x1b[0m \x1b[31m-3\x1b[0m {}\r\n",
            "lorem ipsum dolor sit amet consectetur adipiscing elit sed do eiusmod tempor ".repeat(2)
        );
        bytes.extend_from_slice(&line.as_bytes()[..line.len().min(400)]);
        bytes.extend_from_slice(b"\r\n");
    }
    bytes.extend_from_slice(format!("\x1b[38;5;244m╭{}╮\x1b[0m\r\n", "─".repeat(COLS - 2)).as_bytes());
    bytes.extend_from_slice(
        format!("\x1b[38;5;244m│\x1b[0m > hello{}\x1b[38;5;244m│\x1b[0m\r\n", " ".repeat(COLS - 10))
            .as_bytes(),
    );
    bytes.extend_from_slice(format!("\x1b[38;5;244m╰{}╯\x1b[0m\r\n", "─".repeat(COLS - 2)).as_bytes());
    bytes.extend_from_slice(format!("{}\r\n", "─".repeat(COLS)).as_bytes());
    feed(&mut term, &bytes);
    term
}

fn feed(term: &mut TestTerm, bytes: &[u8]) {
    term.feed(bytes);
}

/// A line as the renderer hands it to the text system: the text moves into a
/// `SharedString` and the text system copies it once more for its cache key
/// (as the old replica counted too); its runs carry a `Font` each.
type Fed = (SharedString, SharedString, Vec<TextRun>);

fn fed(text: String, runs: Vec<TextRun>) -> Fed {
    let text: SharedString = text.into();
    (SharedString::new(text.as_str()), text, runs)
}

#[derive(Debug, Default)]
struct FrameCounts {
    rows_built: usize,
    rows_placed: usize,
    blurred_built: usize,
    lines_shaped: usize,
    glyphs_shaped: usize,
    quads: usize,
}

/// Everything prepaint does except the text system's shaping.
fn frame(
    cache: &mut FrameCache<u8, Fed>,
    term: &mut TestTerm,
    palette: &Palette,
    base_font: &Font,
) -> FrameCounts {
    let geometry = Geometry {
        origin: point(px(10.0), px(42.0)),
        cell_width: px(8.4),
        line_height: px(18.0),
        scale: 2.0,
        font_size: 14.0,
    };
    term.snapshot(&mut cache.snapshot, 3);
    let mut counts = FrameCounts::default();
    let stats = cache.prepare(
        0,
        geometry,
        palette,
        &[],
        |segment| {
            counts.lines_shaped += 1;
            counts.glyphs_shaped += segment.text.chars().count();
            let runs = word_runs(&segment, base_font);
            (fed(segment.text, runs), true)
        },
        |text, runs| {
            let runs = runs
                .into_iter()
                .map(|(len, color)| TextRun {
                    len,
                    font: base_font.clone(),
                    color,
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                })
                .collect();
            fed(text, runs)
        },
    );
    counts.rows_built = stats.rows.built;
    counts.rows_placed = stats.rows_placed;
    counts.blurred_built = stats.blurred_built;
    counts.quads = cache.rows().map(|(_, row)| row.quads.len()).sum();
    counts
}

fn measure(label: &str, iterations: usize, mut frame: impl FnMut(usize) -> FrameCounts) -> (f64, usize) {
    let counts = frame(0);
    let before = allocations();
    let start = Instant::now();
    for i in 1..=iterations {
        std::hint::black_box(frame(i));
    }
    let micros = start.elapsed().as_secs_f64() * 1e6 / iterations as f64;
    let allocs = (allocations() - before) / iterations;
    eprintln!("{label}: {micros:.1} µs/frame, {allocs} allocations/frame; {counts:?}");
    (micros, allocs)
}

#[test]
#[ignore = "benchmark; run with --ignored --nocapture"]
fn frame_build_200x60() {
    let palette = Palette::from_theme(&wormhole_core::theme::builtin()[0]);
    let mut base_font = font("JetBrains Mono");
    base_font.features = crate::fonts::ligature_features(false);
    let mut term = term();

    // A frame with nothing cached: the first one (after a resize or a theme
    // change the buffers are reused, so those cost a little less).
    let (_, full) =
        measure("full frame", 200, |_| frame(&mut FrameCache::default(), &mut term, &palette, &base_font));
    let mut cache = FrameCache::default();
    frame(&mut cache, &mut term, &palette, &base_font);
    // Nothing changed (another view caused the frame; with `.cached` it doesn't even run).
    let (_, idle) = measure("unchanged frame", 2000, |_| frame(&mut cache, &mut term, &palette, &base_font));
    // A key typed at the prompt.
    measure("typing frame", 2000, |i| {
        let c = (b'a' + (i % 26) as u8) as char;
        feed(&mut term, format!("\x08{c}").as_bytes());
        frame(&mut cache, &mut term, &palette, &base_font)
    });
    // Streaming output: one more line scrolls the screen.
    let (_, scroll) = measure("scrolling frame", 2000, |i| {
        feed(&mut term, format!("\r\n\x1b[32m+\x1b[0m streamed line {i}").as_bytes());
        frame(&mut cache, &mut term, &palette, &base_font)
    });
    assert!(full < FULL_FRAME_ALLOCATIONS, "a full frame allocates {full} times");
    assert!(
        idle < UNCHANGED_FRAME_ALLOCATIONS && scroll < SCROLLING_FRAME_ALLOCATIONS,
        "steady frames allocate {idle} / {scroll} times"
    );
}

/// The benchmark's allocation limits as a check that always runs: counts don't depend on the
/// machine, so a change that makes a steady frame allocate per cell again fails here.
#[test]
fn frames_stay_within_their_allocations() {
    let palette = Palette::from_theme(&wormhole_core::theme::builtin()[0]);
    let mut base_font = font("JetBrains Mono");
    base_font.features = crate::fonts::ligature_features(false);
    let mut term = term();
    let (_, full) =
        measure("full frame", 5, |_| frame(&mut FrameCache::default(), &mut term, &palette, &base_font));
    let mut cache = FrameCache::default();
    frame(&mut cache, &mut term, &palette, &base_font);
    let (_, idle) = measure("unchanged frame", 20, |_| frame(&mut cache, &mut term, &palette, &base_font));
    let (_, scroll) = measure("scrolling frame", 20, |i| {
        feed(&mut term, format!("\r\n\x1b[32m+\x1b[0m streamed line {i}").as_bytes());
        frame(&mut cache, &mut term, &palette, &base_font)
    });
    assert!(full < FULL_FRAME_ALLOCATIONS, "a full frame allocates {full} times");
    assert!(
        idle < UNCHANGED_FRAME_ALLOCATIONS && scroll < SCROLLING_FRAME_ALLOCATIONS,
        "steady frames allocate {idle} / {scroll} times"
    );
}

/// Double-width and double-height rows are built like any other: once a screen with them has
/// been built, a frame where nothing changed stays within the same limit (the stretched glyphs'
/// sprites are counted in `outline`'s tests).
#[test]
fn a_screen_with_sized_rows_is_as_cheap_when_idle() {
    let palette = Palette::from_theme(&wormhole_core::theme::builtin()[0]);
    let mut base_font = font("JetBrains Mono");
    base_font.features = crate::fonts::ligature_features(false);
    let mut term = term();
    let rows = b"\x1b[H\x1b#6double width \x1b[31mred\x1b[0m\r\n\x1b#3tall\r\n\x1b#4tall\r\n\x1b#6\x1b[4munderlined\x1b[0m\r\n";
    feed(&mut term, rows);
    let mut cache = FrameCache::default();
    frame(&mut cache, &mut term, &palette, &base_font);
    let sized = cache.rows().filter(|(_, row)| row.size.is_double()).count();
    assert!(sized >= 4, "the screen has {sized} sized rows");
    let (_, idle) = measure("unchanged frame with sized rows", 20, |_| {
        frame(&mut cache, &mut term, &palette, &base_font)
    });
    assert!(idle < UNCHANGED_FRAME_ALLOCATIONS, "an idle frame with sized rows allocates {idle} times");
}
