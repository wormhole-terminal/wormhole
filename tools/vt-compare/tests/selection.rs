//! Selecting and copying text, the new engine against alacritty: the same output, the same
//! selection, the same text and the same cells. alacritty is the behaviour core expected
//! before the switch (`Pty`'s selection API), used here only through its public calls.

use alacritty_terminal::event::VoidListener;
use alacritty_terminal::index::{Column, Direction, Line, Point};
use alacritty_terminal::selection::{Selection, SelectionType as TheirType};
use alacritty_terminal::term::test::TermSize;
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi::{Processor, StdSyncHandler};
use wormhole_vt::Terminal;
use wormhole_vt::{Column as OurColumn, Line as OurLine, Point as OurPoint, SelectionType, Side};

use wormhole_vt_bench::rng::Rng;

const COLS: usize = 12;
const ROWS: usize = 8;

struct Pair {
    theirs: Term<VoidListener>,
    ours: Terminal,
}

impl Pair {
    fn new(bytes: &[u8]) -> Pair {
        let config = Config { scrolling_history: 200, ..Config::default() };
        let mut theirs = Term::new(config, &TermSize::new(COLS, ROWS), VoidListener);
        Processor::<StdSyncHandler>::new().advance(&mut theirs, bytes);
        let mut ours = Terminal::builder().size(COLS, ROWS).scrollback(200).build();
        ours.feed(bytes);
        Pair { theirs, ours }
    }

    fn history(&self) -> i32 {
        self.ours.history_size() as i32
    }
}

fn their_type(kind: SelectionType) -> TheirType {
    match kind {
        SelectionType::Simple => TheirType::Simple,
        SelectionType::Block => TheirType::Block,
        SelectionType::Semantic => TheirType::Semantic,
        SelectionType::Lines => TheirType::Lines,
    }
}

fn their_side(side: Side) -> Direction {
    match side {
        Side::Left => Direction::Left,
        Side::Right => Direction::Right,
    }
}

type Anchor = (i32, usize, Side);

/// Select on both and return (their text, their range, our text, our range) as comparable values.
fn select(
    pair: &mut Pair,
    kind: SelectionType,
    a: Anchor,
    b: Anchor,
) -> Option<[(Option<String>, Option<String>); 2]> {
    let mut selection = Selection::new(their_type(kind), Point::new(Line(a.0), Column(a.1)), their_side(a.2));
    selection.update(Point::new(Line(b.0), Column(b.1)), their_side(b.2));
    pair.theirs.selection = Some(selection);
    // alacritty indexes a row one past its end for some selections that end on a wide
    // character in the last column: that is its bug, and nothing to compare against.
    let theirs = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let range = pair.theirs.selection.as_ref().and_then(|s| s.to_range(&pair.theirs)).map(|r| {
            format!(
                "{:?}-{:?} {}",
                (r.start.line.0, r.start.column.0),
                (r.end.line.0, r.end.column.0),
                r.is_block
            )
        });
        (pair.theirs.selection_to_string(), range)
    }));
    let Ok((their_text, their_range)) = theirs else { return None };

    pair.ours.start_selection(kind, OurPoint::new(OurLine(a.0), OurColumn(a.1)), a.2);
    pair.ours.update_selection(OurPoint::new(OurLine(b.0), OurColumn(b.1)), b.2);
    let our_range = pair.ours.selection_range().map(|r| {
        format!(
            "{:?}-{:?} {}",
            (r.start.line.0, r.start.column.0),
            (r.end.line.0, r.end.column.0),
            r.is_block
        )
    });
    let our_text = pair.ours.selection_text();
    Some([(their_text, their_range), (our_text, our_range)])
}

/// Text with the things selection cares about (no tabs: alacritty writes a tab character into the
/// cell a tab starts on, so that copying brings tabs back; the new engine's cells hold what was
/// printed, so a copy has spaces there): words and separators, blank runs, wide
/// characters, combining marks, lines that wrap, empty lines.
fn output(rng: &mut Rng, lines: usize) -> Vec<u8> {
    const PIECES: &[&str] = &[
        "hello",
        "world",
        "foo_bar",
        "a.b.c",
        "x",
        "  ",
        " ",
        "(paren)",
        "[br]",
        "\"q\"",
        "path/to/file",
        "a:b",
        "日本語",
        "日",
        "e\u{301}",
        "│",
        "<tag>",
        ",",
        "ünï",
    ];
    let mut out = Vec::new();
    for _ in 0..lines {
        let pieces = rng.range(0, 9);
        for _ in 0..pieces {
            out.extend_from_slice(rng.pick(PIECES).as_bytes());
        }
        out.extend_from_slice(b"\r\n");
    }
    out
}

/// Whether a wide character was moved to the next row, leaving a spacer in the last column.
fn has_spacer_head(pair: &Pair) -> bool {
    let grid = pair.ours.grid();
    (-pair.history()..ROWS as i32).any(|y| {
        grid.cell(OurPoint::new(OurLine(y), OurColumn(COLS - 1))).is_some_and(|cell| cell.is_spacer_head())
    })
}

fn random_anchor(rng: &mut Rng, history: i32) -> Anchor {
    let line = rng.range(0, (history + ROWS as i32 - 1) as usize) as i32 - history;
    let side = if rng.one_in(2) { Side::Left } else { Side::Right };
    (line, rng.below(COLS), side)
}

#[test]
fn selections_copy_what_alacritty_copies() {
    let mut rng = Rng::new(7);
    let (mut checked, mut crashed) = (0, 0);
    for round in 0..300 {
        let lines = rng.range(2, 30);
        let bytes = output(&mut rng, lines);
        let mut pair = Pair::new(&bytes);
        for _ in 0..40 {
            let history = pair.history();
            let kind = *rng.pick(&[
                SelectionType::Simple,
                SelectionType::Block,
                SelectionType::Semantic,
                SelectionType::Lines,
            ]);
            let (a, b) = (random_anchor(&mut rng, history), random_anchor(&mut rng, history));
            let Some([theirs, ours]) = select(&mut pair, kind, a, b) else {
                crashed += 1;
                continue;
            };
            // A block between one cell's right edge and the next cell's left edge is no cells
            // wide. alacritty still reports it (a range whose first column is past its last,
            // copying a newline per row); the new engine says nothing is selected.
            //
            // The same goes for a drag across the two halves of one cell: alacritty orders a
            // block's columns by cell and then adjusts for the sides, which turns the drag from a
            // cell's right half to its left half into no cells; the new engine orders the edges, so
            // that is the cell.
            //
            // And a block's row that ends in a spacer: alacritty adds the wide character from the
            // next row to this one's text, though it is not in the block's columns there.
            if kind == SelectionType::Block
                && ((ours.1.is_none() && theirs.1.is_some())
                    || (a.1 == b.1 && a.2 != b.2)
                    || has_spacer_head(&pair))
            {
                continue;
            }
            // A selection that ends on the blank a wide character left at the end of a row:
            // alacritty copies a space there, or the character from the next row, or panics.
            let ends_on_spacer = pair
                .ours
                .selection_range()
                .is_some_and(|r| pair.ours.grid().cell(r.end).is_some_and(|cell| cell.is_spacer_head()));
            if ends_on_spacer {
                continue;
            }
            assert_eq!(
                theirs,
                ours,
                "round {round}, {kind:?} from {a:?} to {b:?} over {:?}",
                String::from_utf8_lossy(&bytes)
            );
            checked += 1;
        }
    }
    assert!(checked > 10_000, "{checked} compared, {crashed} where alacritty panics");
}

/// `Screen::text_between` (core's block output copy, `lines_text`) is alacritty's
/// `bounds_to_string`.
#[test]
fn text_between_is_what_alacritty_gives() {
    let mut rng = Rng::new(11);
    let mut checked = 0;
    for round in 0..300 {
        let lines = rng.range(2, 30);
        let bytes = output(&mut rng, lines);
        let pair = Pair::new(&bytes);
        for _ in 0..40 {
            let history = pair.history();
            let (a, b) = (random_anchor(&mut rng, history), random_anchor(&mut rng, history));
            let (start, end) = if (a.0, a.1) <= (b.0, b.1) { (a, b) } else { (b, a) };
            let (s, e) = (
                OurPoint::new(OurLine(start.0), OurColumn(start.1)),
                OurPoint::new(OurLine(end.0), OurColumn(end.1)),
            );
            if pair.ours.grid().cell(e).is_some_and(|cell| cell.is_spacer_head()) {
                continue;
            }
            let theirs = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                pair.theirs.bounds_to_string(
                    Point::new(Line(start.0), Column(start.1)),
                    Point::new(Line(end.0), Column(end.1)),
                )
            }));
            let Ok(theirs) = theirs else { continue };
            let ours = pair.ours.text_between(s, e, wormhole_vt::TextShape::Stream);
            assert_eq!(
                theirs,
                ours,
                "round {round}: {start:?} to {end:?} over {:?}",
                String::from_utf8_lossy(&bytes)
            );
            checked += 1;
        }
    }
    assert!(checked > 10_000, "{checked} compared");
}
