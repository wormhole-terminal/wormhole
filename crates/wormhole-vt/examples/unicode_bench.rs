//! How fast the Unicode tables answer: lookups per second for the width and the grapheme break of
//! ASCII, CJK, emoji-heavy and accented text.
//!
//! ```sh
//! cargo run --release -p wormhole-vt --example unicode_bench [-- --millis 500]
//! ```
//!
//! The text is made up front as `char`s (deterministic, no `rand`), so the numbers are the lookups and
//! not the UTF-8 decoding. `sum` is the floor: adding up the code points.

use std::hint::black_box;
use std::io::{self, Write};
use std::time::{Duration, Instant};

use wormhole_vt::unicode::{GraphemeState, breaks, width};

#[path = "../tests/common/rng.rs"]
mod rng;
use rng::Rng;

/// A character with a code point in `first..=last`.
trait Scalars {
    fn scalar(&mut self, first: u32, last: u32) -> char;
}

impl Scalars for Rng {
    fn scalar(&mut self, first: u32, last: u32) -> char {
        char::from_u32(self.range(first as usize, last as usize) as u32).unwrap_or('?')
    }
}

const LEN: usize = 1 << 20;

fn ascii(rng: &mut Rng) -> Vec<char> {
    (0..LEN).map(|_| rng.scalar(0x20, 0x7E)).collect()
}

fn cjk(rng: &mut Rng) -> Vec<char> {
    (0..LEN)
        .map(|_| match rng.below(10) {
            0 => rng.scalar(0x3041, 0x3096), // hiragana
            1 => rng.scalar(0x30A1, 0x30FA), // katakana
            2 => rng.scalar(0xFF01, 0xFF60), // fullwidth forms
            3 => rng.scalar(0xAC00, 0xD7A3), // Hangul syllables
            _ => rng.scalar(0x4E00, 0x9FFF), // ideographs
        })
        .collect()
}

fn emoji(rng: &mut Rng) -> Vec<char> {
    let skin = ['\u{1F3FB}', '\u{1F3FC}', '\u{1F3FD}', '\u{1F3FE}', '\u{1F3FF}'];
    let hearts = ['\u{2764}', '\u{263A}', '\u{2600}', '\u{231A}', '\u{1F441}'];
    let mut text = Vec::with_capacity(LEN + 16);
    while text.len() < LEN {
        match rng.below(10) {
            // A pictograph, maybe with a skin tone.
            0..=3 => {
                text.push(rng.scalar(0x1F300, 0x1FAFF));
                if rng.below(4) == 0 {
                    text.push(rng.pick(&skin));
                }
            }
            // A ZWJ sequence of two or three pictographs.
            4 | 5 => {
                text.push(rng.scalar(0x1F466, 0x1F469));
                for _ in 0..1 + rng.below(2) {
                    text.extend(['\u{200D}', rng.scalar(0x1F466, 0x1F469)]);
                }
            }
            // A flag.
            6 => text.extend([rng.scalar(0x1F1E6, 0x1F1FF), rng.scalar(0x1F1E6, 0x1F1FF)]),
            // A text-default symbol with a variation selector.
            7 | 8 => {
                text.extend([rng.pick(&hearts), if rng.below(2) == 0 { '\u{FE0F}' } else { '\u{FE0E}' }])
            }
            // A space between them.
            _ => text.push(' '),
        }
    }
    text.truncate(LEN);
    text
}

/// Mostly ASCII with Latin letters, accents and combining marks, as prose and code.
fn accented(rng: &mut Rng) -> Vec<char> {
    (0..LEN)
        .map(|_| match rng.below(20) {
            0 => rng.scalar(0xC0, 0x17F),    // Latin-1 supplement and Latin Extended-A
            1 => rng.scalar(0x300, 0x36F),   // combining marks
            2 => rng.scalar(0x2500, 0x257F), // box drawing
            3 => rng.scalar(0x2190, 0x21FF), // arrows
            _ => rng.scalar(0x20, 0x7E),
        })
        .collect()
}

/// Runs `work` over `text` for about `budget` and returns the million characters per second.
fn measure(text: &[char], budget: Duration, work: impl Fn(&[char]) -> u64) -> f64 {
    black_box(work(text));
    let start = Instant::now();
    let mut passes = 0u64;
    while start.elapsed() < budget {
        black_box(work(black_box(text)));
        passes += 1;
    }
    (passes * text.len() as u64) as f64 / start.elapsed().as_secs_f64() / 1e6
}

fn main() -> io::Result<()> {
    let mut millis = 400;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--millis" {
            millis = args.next().and_then(|value| value.parse().ok()).unwrap_or(millis);
        }
    }
    let budget = Duration::from_millis(millis);
    let mut rng = Rng::new(0x9E37_79B9_7F4A_7C15);
    let texts: [(&str, Vec<char>); 4] = [
        ("ascii", ascii(&mut rng)),
        ("cjk", cjk(&mut rng)),
        ("emoji", emoji(&mut rng)),
        ("accented", accented(&mut rng)),
    ];

    let sum = |text: &[char]| text.iter().map(|&c| u64::from(c)).sum::<u64>();
    let widths = |text: &[char]| text.iter().map(|&c| u64::from(width(c))).sum::<u64>();
    let break_count = |text: &[char]| {
        let mut state = GraphemeState::new();
        text.iter().filter(|&&c| breaks(&mut state, c)).count() as u64
    };
    // What the print path will do in grapheme mode: both answers from one lookup.
    let step = |text: &[char]| {
        let mut state = GraphemeState::new();
        let mut total = 0u64;
        for &c in text {
            let step = state.step(c);
            total += u64::from(step.width) + u64::from(step.breaks);
        }
        total
    };

    let mut out = io::stdout().lock();
    writeln!(out, "million characters per second ({} ms each, {LEN} characters per text)", millis)?;
    writeln!(out, "{:<10} {:>10} {:>10} {:>10} {:>10}", "", "sum", "width", "breaks", "step")?;
    for (name, text) in &texts {
        writeln!(
            out,
            "{:<10} {:>10.0} {:>10.0} {:>10.0} {:>10.0}",
            name,
            measure(text, budget, sum),
            measure(text, budget, widths),
            measure(text, budget, break_count),
            measure(text, budget, step),
        )?;
    }
    Ok(())
}
