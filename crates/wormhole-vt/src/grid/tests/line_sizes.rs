//! The hint that says whether any live row has a size (double width or height), which the terminal
//! asks on every write so that a screen that never used the feature pays one flag.

use super::BG;
use crate::grid::{DEFAULT_SCROLLBACK_BYTES, Grid};
use crate::types::{Line, LineSize};

#[test]
fn the_hint_is_recounted_only_when_the_whole_screen_is_blanked() {
    let mut g = Grid::new(6, 4, DEFAULT_SCROLLBACK_BYTES);
    assert!(!g.has_line_sizes());
    g.set_line_size(0, LineSize::DoubleWidth);
    assert!(g.has_line_sizes());

    // Blanking rows takes their size with the rest, but the hint is only a hint: it keeps saying
    // "maybe" until the whole screen is blank, which is when counting is cheap and sure.
    g.erase_rows(0..1, BG, false);
    assert_eq!(g.line_size(Line(0)), LineSize::Single);
    assert!(g.has_line_sizes(), "the first row, not the screen");
    g.erase_rows(1..4, BG, false);
    assert!(g.has_line_sizes(), "the rest of the screen, not all of it");
    g.erase_rows(0..4, BG, false);
    assert!(!g.has_line_sizes(), "the whole screen, and no row has a size");
}
