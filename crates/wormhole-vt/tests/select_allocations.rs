//! Selecting, finding and reading text allocate only what they hand back. Moving a selection
//! (every mouse move of a drag), asking its range (every frame), finding a word, a bracket or a
//! link under the pointer, and copying a window of text for a search into buffers that have been
//! used once allocate nothing; the text of a copied selection is a `String` of its own, and
//! comparing a stored match with the cells needs one scratch string.
//!
//! Its own test binary, because the allocator is global and a test elsewhere must not count here.

use wormhole_vt::grid::{TextCopy, TextShape};
use wormhole_vt::parser::Parser;
use wormhole_vt::term::Terminal;
use wormhole_vt::types::{Column, Line, Point, SelectionType, Side};

#[path = "common/mod.rs"]
mod common;
use common::alloc::{Counting, allocations};

#[global_allocator]
static ALLOCATOR: Counting = Counting;

fn at(line: i32, column: usize) -> Point {
    Point::new(Line(line), Column(column))
}

/// A busy screen with history: words, wide characters, marks, a hyperlink, wrapped lines.
fn terminal() -> Terminal {
    let mut term = Terminal::new(80, 24, 500);
    let mut parser = Parser::new();
    for i in 0..300 {
        let line = format!(
            "{i:04} the quick (brown) fox 日本語 cafe\u{301} \x1b]8;;https://example.com/{i}\x1b\\link {i}\x1b]8;;\x1b\\ {}\r\n",
            "and again ".repeat(i % 12)
        );
        term.advance(&mut parser, line.as_bytes());
    }
    term
}

fn count<R>(what: &str, f: impl FnOnce() -> R) -> (R, usize) {
    let before = allocations();
    let result = f();
    let calls = allocations() - before;
    eprintln!("{what}: {calls} allocations");
    (result, calls)
}

#[test]
fn moving_and_reading_a_selection_allocates_nothing() {
    let mut term = terminal();
    for kind in [SelectionType::Simple, SelectionType::Block, SelectionType::Semantic, SelectionType::Lines] {
        term.start_selection(kind, at(-40, 5), Side::Left);
        let (_, calls) = count(&format!("{kind:?}: 200 updates and ranges"), || {
            for i in 0..200 {
                term.update_selection(at(-30 + (i % 20) as i32, i % 80), Side::Right);
                assert!(term.selection_range().is_some() || kind == SelectionType::Block);
            }
        });
        assert_eq!(calls, 0, "{kind:?}");
    }
}

#[test]
fn words_brackets_and_links_under_the_pointer_allocate_nothing() {
    let term = terminal();
    let grid = term.grid();
    let (_, calls) = count("word, bracket and link at 100 points", || {
        for i in 0..100 {
            let p = at(-((i % 30) as i32), (i * 7) % 80);
            let _ = grid.word_start(p, |c| " ()".contains(c));
            let _ = grid.word_end(p, |c| " ()".contains(c));
            let _ = grid.bracket_match(p);
            let _ = grid.line_extent(p.line);
        }
    });
    assert_eq!(calls, 0);
    let (found, calls) = count("a link", || grid.link_span(at(-1, 60)).is_some());
    assert!(found || grid.link_span(at(-1, 44)).is_some(), "the screen holds links");
    assert_eq!(calls, 0, "the span shares the link's strings");
}

#[test]
fn a_window_of_text_for_a_search_allocates_nothing_once_the_copy_is_warm() {
    let term = terminal();
    let mut copy = TextCopy::new();
    // The history's lines are of all shapes; a first walk sizes the buffers for the biggest.
    let walk = |term: &Terminal, copy: &mut TextCopy| {
        let mut from = Line(-(term.history_size() as i32));
        loop {
            let next = term.copy_text(from, 64, copy);
            assert!(copy.lines().len() > 0);
            let Some(next) = next else { break };
            from = next;
        }
    };
    walk(&term, &mut copy);
    let (_, calls) = count("the whole history in windows of 64 rows", || walk(&term, &mut copy));
    assert_eq!(calls, 0);
}

#[test]
fn copying_text_allocates_its_result_and_nothing_else() {
    let term = terminal();
    let (start, end) = (at(-250, 3), at(-100, 70));
    let mut buffer = String::new();
    term.grid().push_text_between(start, end, TextShape::Stream, &mut buffer);
    buffer.clear();
    let (_, calls) = count("a stream into a buffer that was used", || {
        term.grid().push_text_between(start, end, TextShape::Stream, &mut buffer);
    });
    assert_eq!(calls, 0);
    let (text, calls) =
        count("a stream into a new string", || term.text_between(start, end, TextShape::Stream));
    assert!(text.len() > 5_000 && calls < 40, "{} bytes in {calls} allocations (doubling)", text.len());
}

#[test]
fn comparing_a_stored_match_needs_one_scratch_string() {
    let term = terminal();
    let text = term.grid().span_text(at(-50, 5), at(-50, 20));
    let (held, calls) = count("a stored match of one row", || term.span_is(at(-50, 5), at(-50, 20), &text));
    assert!(held);
    assert!(calls <= 4, "{calls}");
}
