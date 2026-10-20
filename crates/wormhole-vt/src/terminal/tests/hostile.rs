//! Input nobody should send: nothing panics, the grid stays consistent, memory stays bounded.

use super::Rig;
use crate::terminal::{Policy, Terminal};

/// A tiny generator of bytes (xorshift), so that the run is the same every time.
struct Bytes(u64);

impl Bytes {
    fn next(&mut self) -> u8 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 24) as u8
    }
}

#[test]
fn random_bytes_leave_a_consistent_terminal() {
    for seed in 1..40u64 {
        let mut rng = Bytes(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let (cols, rows) = [(2, 1), (3, 2), (5, 4), (20, 6), (80, 24)][(seed % 5) as usize];
        let mut rig = Rig::new(cols, rows);
        let mut data = vec![0u8; 20_000];
        for byte in &mut data {
            // Lean toward escape sequences and text, so that the state machine is exercised.
            *byte = match rng.next() % 8 {
                0 => 0x1b,
                1 => {
                    const PUNCTUATION: &[u8] = b"[]();?>=<!#$ \"'0123456789";
                    PUNCTUATION[usize::from(rng.next()) % PUNCTUATION.len()]
                }
                2 => {
                    const FINALS: &[u8] = b"HJKmhlrstuABCDGdfLMPXZ@`bcqn~}|";
                    FINALS[usize::from(rng.next()) % FINALS.len()]
                }
                3 => rng.next() | 0x80,
                _ => rng.next(),
            };
        }
        for piece in data.chunks(1 + (seed as usize % 97)) {
            rig.feed_bytes(piece);
            rig.term.events().for_each(drop);
            rig.term.clear_replies();
        }
        rig.term.check_invariants().unwrap_or_else(|why| panic!("seed {seed}: {why}"));
        let (y, x) = rig.cursor();
        assert!(y < rig.term.rows() && x < rig.term.cols(), "seed {seed}: cursor out of the screen");
        rig.term.resize(cols + 3, rows + 1);
        rig.term.check_invariants().unwrap_or_else(|why| panic!("seed {seed} after a resize: {why}"));
    }
}

#[test]
fn huge_numbers_everywhere_are_clamped() {
    let mut rig = Rig::new(20, 6);
    rig.feed("\x1b[99999999;99999999H\x1b[99999999A\x1b[99999999C\x1b[99999999L\x1b[99999999M\x1b[99999999@\x1b[99999999P");
    rig.feed("\x1b[99999999X\x1b[99999999S\x1b[99999999T\x1b[99999999b\x1b[99999999I\x1b[99999999Z\x1b[99999999;99999999r");
    rig.feed("\x1b[?69h\x1b[99999999;99999999s\x1b[99999999'}\x1b[99999999'~\x1b[0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0;0m");
    rig.term.check_invariants().expect("consistent");
    let (y, x) = rig.cursor();
    assert!(y < 6 && x < 20);
}

#[test]
fn rectangular_operations_with_wild_numbers_keep_the_grid_consistent() {
    // Text with wide characters, marks, links and attributes, then rectangles of every kind with
    // numbers that are zero, small, off the screen or the largest a parameter holds.
    const NUMBERS: [&str; 9] = ["", "0", "1", "2", "3", "5", "9", "99", "65535"];
    const FINALS: [&str; 6] = ["$r", "$t", "$v", "$x", "$z", "${"];
    for seed in 1..40u64 {
        let mut rng = Bytes(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let (cols, rows) = [(2, 1), (3, 3), (6, 4), (9, 5)][(seed % 4) as usize];
        let mut rig = Rig::new(cols, rows);
        rig.term.set_policy(Policy { screen_checksum: true, ..Policy::default() });
        for step in 0..300 {
            let mut pick = |n: usize| usize::from(rng.next()) % n;
            let seq = match pick(6) {
                0 => "日本e\u{301}x\x1b]8;;http://a.test\x07y\x1b]8;;\x07\x1b[1;4;7mz\x1b[0m".to_string(),
                1 => ["\x1b[?69h", "\x1b[?69l", "\x1b[?6h", "\x1b[?6l", "\x1b[2;4r", "\x1b[2;5s"][pick(6)]
                    .to_string(),
                2 => ["\x1b[2*x", "\x1b[0*x", "\x1b[1\"q", "\x1b[0\"q", "\x1b[3;3H"][pick(5)].to_string(),
                _ => {
                    let mut seq = String::from("\x1b[");
                    for i in 0..pick(9) {
                        if i > 0 {
                            seq.push(';');
                        }
                        seq.push_str(NUMBERS[pick(NUMBERS.len())]);
                    }
                    seq.push_str(if pick(8) == 0 { "*y" } else { FINALS[pick(FINALS.len())] });
                    seq
                }
            };
            rig.feed(&seq);
            rig.term.clear_replies();
            rig.term
                .check_invariants()
                .unwrap_or_else(|why| panic!("seed {seed} step {step} after {seq:?}: {why}"));
        }
    }
}

#[test]
fn a_repeat_count_is_bounded_by_what_a_parameter_can_say() {
    let mut rig = Rig::new(80, 24);
    rig.feed("x\x1b[65535b\x1b[65535b");
    rig.term.check_invariants().expect("consistent");
}

#[test]
fn a_pile_of_unfinished_strings_costs_no_memory() {
    let mut term = Terminal::builder().size(20, 5).scrollback(10).build();
    for _ in 0..2000 {
        term.feed(b"\x1b]0;");
        term.feed(&[b'x'; 5000]);
        term.feed(b"\x1bP$q");
        term.feed(&[b'x'; 5000]);
        term.feed(b"\x1b_G");
        term.feed(&[b'x'; 5000]);
        term.feed(b"\x18");
    }
    assert!(term.events().count() <= 16_384);
    term.feed(b"ok");
    assert_eq!(term.row_text(crate::types::Line(0), true), "ok");
}

#[test]
fn control_characters_inside_sequences_and_strings_are_harmless() {
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b[1\x00\x07\x7f;2\x08H\x1b]0;a\x00\x01b\x07\x1bP$q\x00m\x1b\\ok");
    assert_eq!(rig.term.title(), Some("ab"));
    assert!(rig.row(1).ends_with("ok") || rig.row(0).contains("ok"));
}

#[test]
fn replies_do_not_pile_up_for_a_host_that_never_collects_them() {
    let mut rig = Rig::new(20, 5);
    for _ in 0..100_000 {
        rig.feed("\x1b[c\x1b[6n\x1b]4;1;?\x07");
    }
    assert!(rig.term.replies().len() < (1 << 20) + 100);
}

/// A point on the screen, in the history, or far beyond either.
fn wild_point(rng: &mut Bytes, term: &Terminal) -> crate::types::Point {
    use crate::types::{Column, Line, Point};
    let (a, b) = (usize::from(rng.next()), usize::from(rng.next()));
    let history = term.history_len() as i32;
    let line = match a % 5 {
        0 => -history - b as i32,
        1 => term.rows() as i32 + b as i32,
        2 => i32::MIN + b as i32,
        3 => -(b as i32 % (history + 1)),
        _ => b as i32 % term.rows() as i32,
    };
    let column = if a % 7 == 0 { usize::MAX - b } else { b % (term.cols() + 3) };
    Point::new(Line(line), Column(column))
}

#[test]
fn host_calls_with_wild_points_between_random_bytes_keep_the_terminal_consistent() {
    use crate::grid::{TextCopy, TextShape};
    use crate::types::{Scroll, SelectionType, Side};
    // What a host does around the stream (the `term` fuzz target does the same with the fuzzer
    // choosing): resize, scroll the view, select, copy, search and pin, with points on the screen,
    // in the history and far beyond both.
    for seed in 1..30u64 {
        let mut rng = Bytes(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15));
        let (cols, rows) = [(2, 1), (5, 3), (12, 4), (40, 10)][(seed % 4) as usize];
        let mut rig = Rig::new(cols, rows);
        rig.feed("\x1b[?2027h");
        let mut copy = TextCopy::new();
        let mut pins = Vec::new();
        for step in 0..150 {
            let p = wild_point(&mut rng, &rig.term);
            match rng.next() % 12 {
                0 => rig.term.resize(1 + usize::from(rng.next()) % 30, 1 + usize::from(rng.next()) % 12),
                1 => rig.term.scroll(Scroll::Delta(i32::from(rng.next() as i8))),
                2 => rig.term.start_selection(SelectionType::Block, p, Side::Left),
                3 => rig.term.start_selection(SelectionType::Semantic, p, Side::Right),
                4 => {
                    let q = wild_point(&mut rng, &rig.term);
                    rig.term.update_selection(q, Side::Right);
                    let _ = (rig.term.selection_text(), rig.term.selection_range());
                }
                5 => {
                    let q = wild_point(&mut rng, &rig.term);
                    let _ = rig.term.text_between(p, q, TextShape::Block);
                    let _ = rig.term.text_between(p, q, TextShape::Stream);
                }
                6 => {
                    let _ = rig.term.copy_text(p.line, usize::from(rng.next()), &mut copy);
                }
                7 => pins.push(rig.term.pin(p)),
                8 => {
                    if let Some(pin) = pins.pop() {
                        rig.term.move_pin(pin, p);
                        let _ = rig.term.pin_point(pin);
                        rig.term.unpin(pin);
                    }
                }
                9 => {
                    let _ = (rig.term.grid().bracket_match(p), rig.term.grid().line_extent(p.line));
                    let _ = rig.term.link_span(p);
                }
                10 => rig.term.clear_history(),
                _ => {
                    let mut data = [0u8; 40];
                    data.iter_mut()
                        .for_each(|b| *b = rng.next() | if rng.next().is_multiple_of(3) { 0x80 } else { 0 });
                    rig.feed_bytes(&data);
                    rig.feed("\nsome text\x1b[1m wide: \u{65e5}\u{672c}\r\n");
                }
            }
            rig.term.events().for_each(drop);
            rig.term.clear_replies();
            rig.term.check_invariants().unwrap_or_else(|why| panic!("seed {seed} step {step}: {why}"));
        }
    }
}

#[test]
fn a_long_cursor_back_with_reverse_wrap_is_as_many_backspaces() {
    // CUB with reverse wraparound walks as BS does; it only takes the steps along a row at once.
    let mut rng = Bytes(0x5EED);
    let pick = |rng: &mut Bytes, below: usize| rng.next() as usize % below;
    for _ in 0..400 {
        let (cols, rows) = (2 + pick(&mut rng, 7), 1 + pick(&mut rng, 6));
        let mut setup = String::new();
        for _ in 0..pick(&mut rng, 4) {
            let (y, x) = (1 + pick(&mut rng, rows), 1 + pick(&mut rng, cols));
            setup += &format!("\x1b[{y};{x}H{}", "w".repeat(pick(&mut rng, 2 * cols)));
        }
        if pick(&mut rng, 2) == 0 {
            let top = 1 + pick(&mut rng, rows);
            setup += &format!("\x1b[{top};{}r", top + pick(&mut rng, rows + 1 - top));
        }
        if pick(&mut rng, 2) == 0 {
            let left = 1 + pick(&mut rng, cols);
            setup += &format!("\x1b[?69h\x1b[{left};{}s", left + pick(&mut rng, cols + 1 - left));
        }
        setup += ["\x1b[?45h", "\x1b[?1045h", "\x1b[?45h\x1b[?1045h"][pick(&mut rng, 3)];
        setup += &format!("\x1b[{};{}H", 1 + pick(&mut rng, rows), 1 + pick(&mut rng, cols));
        if pick(&mut rng, 3) == 0 {
            setup += &"z".repeat(cols);
        }
        let n = 1 + pick(&mut rng, cols * rows + 1);
        let (mut cub, mut bs) = (Rig::new(cols, rows), Rig::new(cols, rows));
        cub.feed(&setup).feed(&format!("\x1b[{n}D\x1b[1$w"));
        bs.feed(&setup).feed(&"\x08".repeat(n)).feed("\x1b[1$w");
        assert_eq!(cub.replies(), bs.replies(), "{setup:?} then {n} left");
    }
}

#[test]
fn styles_do_not_grow_the_history_past_its_cap() {
    // A history of 2,000 lines of 80 cells, each cell in a colour of its own: the table of styles
    // of a page is about four times the size of its cells, which the cap did not count.
    let mut rig = Rig::with_scrollback(80, 24, 2_000);
    let mut color = 0u32;
    for _ in 0..4_000 {
        let mut line = String::new();
        for _ in 0..80 {
            color += 1;
            line += &format!("\x1b[38;2;{};{};{}mx", color >> 16 & 255, color >> 8 & 255, color & 255);
        }
        rig.feed(&(line + "\r\n"));
    }
    let grid = rig.term.grid();
    let (held, limit) = (grid.memory_bytes(), grid.scrollback_limit());
    // The cells' cap, as much again for the styles, and the screen's pages.
    assert!(held < 2 * limit + 2 * 1024 * 1024, "{held} bytes held for a limit of {limit}");
    assert!(grid.history_len() > 0, "some history stays");
    assert_eq!(grid.check_invariants(), Ok(()));
}

#[test]
fn repeated_long_cursor_backs_with_reverse_wrap_stay_cheap() {
    let mut rig = Rig::new(500, 200);
    rig.feed("\x1b[?45h\x1b[?1045h\x1b[200;500H");
    let start = std::time::Instant::now();
    rig.feed(&"\x1b[65535D".repeat(10_000));
    assert!(start.elapsed() < std::time::Duration::from_secs(1), "took {:?}", start.elapsed());
}

#[test]
fn marks_and_links_do_not_grow_the_history_past_its_cap() {
    // A history of 2,000 lines, then many more of what costs most beside the cells: every cell
    // with eight combining marks, or under a link of its own.
    let marked = format!("{}\r\n", format!("a{}", "\u{301}".repeat(8)).repeat(80));
    let linked = |line: usize| -> String {
        let mut out = String::new();
        for col in 0..80 {
            out += &format!("\x1b]8;;https://example.com/{line}/{col}/{}\x1b\\b", "x".repeat(256));
        }
        out + "\x1b]8;;\x1b\\\r\n"
    };
    for (lines, kind) in [(4_000, "marks"), (600, "links")] {
        let mut rig = Rig::with_scrollback(80, 24, 2_000);
        for line in 0..lines {
            rig.feed(&if kind == "marks" { marked.clone() } else { linked(line) });
        }
        let grid = rig.term.grid();
        let (held, limit) = (grid.memory_bytes(), grid.scrollback_limit());
        // The cells' cap, as much again for the marks and links, and the screen's pages.
        assert!(held < 2 * limit + 2 * 1024 * 1024, "{kind}: {held} bytes held for a limit of {limit}");
        assert!(grid.history_len() > 0, "{kind}: some history stays");
        assert_eq!(grid.check_invariants(), Ok(()), "{kind}");
    }
}
