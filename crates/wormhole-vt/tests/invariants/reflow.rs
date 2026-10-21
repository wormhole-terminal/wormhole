//! Reflow loses nothing: resizing there and back keeps every character.

use wormhole_vt::{Line, Terminal};

use crate::generator::*;
use crate::rng::Rng;

/// The text of a terminal as logical lines: soft-wrapped rows joined, trailing blank lines left out.
fn logical_lines(term: &Terminal, rows: usize) -> Vec<String> {
    let history = term.grid().history_len() as i32;
    let mut lines = Vec::new();
    let mut current = String::new();
    for y in -history..rows as i32 {
        current.push_str(&term.row_text(Line(y), false));
        if !term.is_wrapped(Line(y)) {
            lines.push(current.trim_end().to_string());
            current.clear();
        }
    }
    if !current.is_empty() {
        lines.push(current.trim_end().to_string());
    }
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    lines
}

/// Equal logical lines, or a message that names the first line that differs.
fn assert_lines(got: &[String], expected: &[String], what: String) {
    if got == expected {
        return;
    }
    let at = got.iter().zip(expected).position(|(a, b)| a != b).unwrap_or(got.len().min(expected.len()));
    panic!(
        "{what}: {} lines, expected {}; first difference at line {at}\n  got:      {:?}\n  expected: {:?}",
        got.len(),
        expected.len(),
        got.get(at),
        expected.get(at)
    );
}

fn reflow_text(seed: u64, wide: bool) -> (Vec<String>, Vec<u8>) {
    let mut rng = Rng::new(seed);
    let (mut lines, mut bytes) = (Vec::new(), Vec::new());
    for n in 0..rng.range(30, 90) {
        let mut line = format!("{n}:");
        for _ in 0..rng.range(0, 40) {
            line.push(' ');
            // Words only, or words and the odd wide character or combining mark.
            if wide && rng.chance(20) {
                line.push_str(rng.pick(&["日本語", "😀", "é", "e\u{301}", "한국", "ｱ"]));
            } else {
                line.push_str(rng.pick(WORDS));
            }
        }
        bytes.extend_from_slice(line.as_bytes());
        bytes.extend_from_slice(b"\r\n");
        lines.push(line);
        if rng.chance(8) {
            bytes.extend_from_slice(b"\r\n");
            lines.push(String::new());
        }
    }
    // Blank lines at the end are not text: `logical_lines` leaves them out too.
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
    (lines, bytes)
}

fn reflow_round_trips(from: u64, count: u64) {
    for seed in from..from + count {
        let mut rng = Rng::new(seed ^ 0x5eed);
        let (cols, rows) = (rng.range(20, 110), rng.range(5, 30));
        let (expected, bytes) = reflow_text(seed, false);
        let mut term = Terminal::builder().size(cols, rows).scrollback(100_000).build();
        term.feed(&bytes);
        assert_lines(&logical_lines(&term, rows), &expected, format!("seed {seed}: before any resize"));
        let mut size = (cols, rows);
        for step in 0..8 {
            size = (rng.range(2, 140), rng.range(2, 40));
            term.resize(size.0, size.1);
            assert_lines(
                &logical_lines(&term, size.1),
                &expected,
                format!("seed {seed}: step {step}, resized to {}x{} from {cols}x{rows}", size.0, size.1),
            );
        }
        term.resize(cols, rows);
        assert_lines(
            &logical_lines(&term, rows),
            &expected,
            format!("seed {seed}: back at {cols}x{rows} after {size:?}"),
        );
    }
}

#[test]
fn resizing_through_any_sizes_and_back_keeps_every_line() {
    reflow_round_trips(0, 150);
}

/// Narrower and wider by one column at a time, which moves every wrap point past every word.
#[test]
fn shrinking_one_column_at_a_time_and_growing_back_keeps_every_line() {
    for seed in 0..6u64 {
        let (expected, bytes) = reflow_text(seed + 500, false);
        let mut term = Terminal::builder().size(60, 12).scrollback(100_000).build();
        term.feed(&bytes);
        for width in (3..60).rev().chain(4..=60) {
            term.resize(width, 12);
            assert_lines(&logical_lines(&term, 12), &expected, format!("seed {seed}: at width {width}"));
        }
    }
}

/// Wide characters can leave a blank cell at the end of a row, and a character that does not fit a
/// narrow screen cannot stay; the letters and marks must all still be there, in order.
#[test]
fn resizing_text_with_wide_characters_keeps_every_character() {
    let letters =
        |lines: &[String]| -> String { lines.concat().chars().filter(|c| !c.is_whitespace()).collect() };
    for seed in 0..40u64 {
        let mut rng = Rng::new(seed);
        let (expected, bytes) = reflow_text(seed + 900, true);
        let mut term = Terminal::builder().size(50, 10).scrollback(100_000).build();
        term.feed(&bytes);
        let want = letters(&expected);
        assert_eq!(letters(&logical_lines(&term, 10)), want, "seed {seed}: before any resize");
        for step in 0..6 {
            let width = rng.range(3, 90);
            term.resize(width, 10);
            assert_eq!(letters(&logical_lines(&term, 10)), want, "seed {seed}: step {step}, width {width}");
        }
        term.resize(50, 10);
        assert_eq!(letters(&logical_lines(&term, 10)), want, "seed {seed}: back at 50");
    }
}

#[test]
#[ignore = "many more seeds: cargo test -p wormhole-vt --test invariants -- --ignored"]
fn reflow_many_seeds() {
    reflow_round_trips(1_000, 1_000);
}
