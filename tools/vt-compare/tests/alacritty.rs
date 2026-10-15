//! A mistake of alacritty_terminal that the differential harness found, kept as the reason the
//! parity tests (`parity/`) feed alacritty whole.

use vt_compare::engines::AlacrittyEngine;
use wormhole_vt_bench::Engine;

/// A `feed` that completes a UTF-8 character begun by the one before, holds one more character
/// and ends in the start of another, loses the one in the middle. The same bytes fed in one piece
/// show all four.
#[test]
fn alacritty_drops_a_character_after_a_split_utf8_sequence() {
    let mut whole = AlacrittyEngine::new(10, 2, 0);
    whole.feed(b"x\xc3\xa9a\xc3\xa9");
    assert_eq!(whole.row_text(0), "xéaé");
    let mut cut = AlacrittyEngine::new(10, 2, 0);
    cut.feed(b"x\xc3");
    cut.feed(b"\xa9a\xc3");
    cut.feed(b"\xa9");
    // When alacritty is fixed this reads "xéaé".
    assert_eq!(cut.row_text(0), "xéé");
}
