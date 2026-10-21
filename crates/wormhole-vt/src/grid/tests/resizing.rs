//! Height changes and what a resize leaves alone: where the screen sits in the rows it is made of,
//! and the view. (Width changes, which reflow, are in `reflow`.)

use super::{Sim, at, screen};
use crate::grid::DEFAULT_SCROLLBACK_BYTES;

#[test]
fn shrinking_a_screen_that_shows_the_live_rows_keeps_it_there() {
    let mut sim = Sim::new(6, 4, DEFAULT_SCROLLBACK_BYTES);
    sim.print("a\nb\nc\nd");
    // The rows that no longer fit go into the history; the view stays on the live screen, it is
    // not scrolled back to where the old top row went.
    let cursor = sim.grid.resize(6, 2, at(3, 1), true);
    assert_eq!(screen(&sim.grid), ["c", "d"]);
    assert_eq!(sim.grid.history_len(), 2);
    assert_eq!(sim.grid.display_offset(), 0);
    assert_eq!(cursor, at(1, 1));
    if let Err(e) = sim.grid.check_invariants() {
        panic!("{e}");
    }
}

#[test]
fn text_below_the_cursor_pushes_rows_into_the_history_when_it_does_not_fit() {
    let mut sim = Sim::new(6, 4, DEFAULT_SCROLLBACK_BYTES);
    sim.print("a\nb\nc");
    // The cursor is on the second line, with a third below it: of two rows, the last two lines
    // are kept and the first goes to the history, rather than the line under the cursor being lost.
    let cursor = sim.grid.resize(6, 2, at(1, 0), true);
    assert_eq!(sim.grid.history_len(), 1);
    assert_eq!(screen(&sim.grid), ["b", "c"]);
    assert_eq!(cursor, at(0, 0));
    if let Err(e) = sim.grid.check_invariants() {
        panic!("{e}");
    }
}
