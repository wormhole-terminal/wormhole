//! The cursor and its delayed wrap staying in step with the row under it and the margins, whatever
//! changes either: each stream below broke `check_invariants` (a wrap pending where the row no
//! longer wraps, a cursor in a column its row does not show). The check runs after every step, as
//! the fuzz targets do, because the break was visible only between the sequences.

use super::Rig;

/// Feed `steps` one at a time and check the invariants after each.
fn checked(rig: &mut Rig, steps: &[&str]) {
    for step in steps {
        rig.feed(step);
        if let Err(error) = rig.term.check_invariants() {
            panic!("after {step:?}: {error}");
        }
    }
}

#[test]
fn a_soft_reset_ends_a_wrap_due_at_a_margin_it_removed() {
    let mut rig = Rig::new(20, 3);
    checked(&mut rig, &["\x1b[?69h", "\x1b[;4s", "abcd", "\x1b[!p", "X"]);
    // Nothing was due on the full-width row: the next character is written where the cursor stood.
    assert_eq!(rig.row(0), "abcX");
    assert_eq!(rig.row(1), "");
    assert_eq!(rig.cursor(), (0, 4));
}

#[test]
fn a_wrap_still_due_at_the_real_edge_survives_a_soft_reset() {
    let mut rig = Rig::new(5, 3);
    checked(&mut rig, &["abcde", "\x1b[!p", "f"]);
    assert_eq!((rig.row(0), rig.row(1)), ("abcde".into(), "f".into()));
}

#[test]
fn a_scroll_that_puts_a_single_width_row_under_a_waiting_wrap_ends_it() {
    let mut rig = Rig::new(10, 3);
    checked(&mut rig, &["\x1b#6", "abcde", "\x1b[T"]);
    // The row is blank and single-width now: ten columns, and the wrap at the fifth is not due.
    rig.feed("Z");
    assert_eq!(rig.row(0), "    Z");
    let mut rig = Rig::new(10, 3);
    checked(&mut rig, &["\x1b#6", "abcde", "\x1b[S"]);
}

#[test]
fn a_cursor_back_on_a_double_width_row_after_the_alternate_screen_is_inside_its_half() {
    let mut rig = Rig::new(10, 3);
    checked(&mut rig, &["\x1b#6", "\x1b[?47h", "ABCDEFG", "\x1b[?47l"]);
    assert_eq!(rig.cursor(), (0, 4), "the last column the row shows");
    // And the other way, with the wrap due at the end of the alternate screen's row.
    let mut rig = Rig::new(10, 3);
    checked(&mut rig, &["\x1b#6", "\x1b[?47h", "ABCDEFGHIJ", "\x1b[?47l"]);
    let mut rig = Rig::new(10, 3);
    checked(&mut rig, &["\x1b[?1049h", "\x1b#6", "abcde", "\x1b[?1049l"]);
}

#[test]
fn a_picture_placed_on_a_double_width_row_leaves_the_cursor_where_the_row_shows() {
    let mut rig = Rig::new(3, 3);
    checked(&mut rig, &["\x1b#4", "\x1b_Ga=T,f=24,s=1,v=1,i=5;AAAA\x1b\\"]);
    let mut rig = Rig::new(4, 3);
    checked(&mut rig, &["\x1b#6", "\x1b_Ga=T,f=24,s=1,v=1,i=5,c=4;AAAA\x1b\\"]);
}

#[test]
fn a_wrap_below_the_scrolling_region_stays_on_the_last_row() {
    let mut rig = Rig::new(5, 4);
    checked(&mut rig, &["\x1b[1;2r", "\x1b[4;1H", "abcdefg"]);
    // Below the region nothing scrolls and there is no row under the last: the text wraps onto itself.
    assert_eq!(rig.row(3), "fgcde");
    assert_eq!(rig.cursor(), (3, 2));
}

#[test]
fn forward_index_stops_at_the_last_column_a_double_width_row_shows() {
    let mut rig = Rig::new(10, 3);
    // The row shows five columns and the left margin is past them: at the fifth there is no margin
    // to scroll at and no column to move to.
    checked(&mut rig, &["\x1b#6", "\x1b[?69h", "\x1b[8;10s", "\x1b[1;5H", "\x1b9"]);
    assert_eq!(rig.cursor(), (0, 4));
}

#[test]
fn a_cursor_beyond_the_right_margin_goes_no_further_than_the_last_column() {
    let mut rig = Rig::new(10, 3);
    checked(&mut rig, &["\x1b[?69h", "\x1b[1;5s", "\x1b[1;10H"]);
    assert_eq!(rig.cursor(), (0, 9));
    // Forward, tab and forward index have nowhere to go; the margin is not the edge for it.
    for step in ["\x1b[5C", "\t", "\x1b9"] {
        checked(&mut rig, &[step]);
        assert_eq!(rig.cursor(), (0, 9), "{step:?}");
    }
}

#[test]
fn back_index_at_the_left_edge_outside_a_left_margin_stays_there() {
    let mut rig = Rig::new(10, 3);
    checked(&mut rig, &["\x1b[?69h", "\x1b[3;8s", "\x1b[1;1H", "\x1b6"]);
    assert_eq!(rig.cursor(), (0, 0));
}

#[test]
fn reverse_wraparound_goes_round_the_top_to_the_bottom_margin_or_the_screen() {
    // Extended reverse wraparound at the top row: a region that starts below it sends the cursor
    // to the last row of the screen (there is no line above, and the region does not hold the
    // cursor), one that starts at it to the region's last row.
    let mut rig = Rig::new(10, 6);
    checked(&mut rig, &["\x1b[?1045h", "\x1b[3;4r", "\x1b[1;1H", "\x08"]);
    assert_eq!(rig.cursor(), (5, 9));
    let mut rig = Rig::new(10, 6);
    checked(&mut rig, &["\x1b[?1045h", "\x1b[1;3r", "\x1b[1;1H", "\x08"]);
    assert_eq!(rig.cursor(), (2, 9));
}

#[test]
fn a_column_in_origin_mode_counts_from_the_left_margin() {
    let mut rig = Rig::new(10, 3);
    checked(&mut rig, &["\x1b[?6h", "\x1b[5G"]);
    assert_eq!(rig.cursor(), (0, 4));
    checked(&mut rig, &["\x1b[?69h", "\x1b[3;8s", "\x1b[?6h", "\x1b[2G"]);
    assert_eq!(rig.cursor(), (0, 3), "the left margin is the third column: the second from it is the fourth");
    checked(&mut rig, &["\x1b[9G"]);
    assert_eq!(rig.cursor(), (0, 7), "and no further than the right margin");
}

#[test]
fn leaving_the_alternate_screen_that_was_not_entered_restores_nothing() {
    let mut rig = Rig::new(10, 5);
    checked(&mut rig, &["\x1b[2;2H", "\x1b7", "\x1b[4;5H", "\x1b[?1049l"]);
    assert_eq!(rig.cursor(), (3, 4), "the cursor DECSC saved is for 1049 to give back, not 1049 l alone");
    checked(&mut rig, &["\x1b[?1049h", "\x1b[1;1H", "\x1b[?1049l"]);
    assert_eq!(rig.cursor(), (3, 4), "entering saved this one");
}

#[test]
fn screen_alignment_leaves_origin_mode_and_the_margins_modes() {
    let mut rig = Rig::new(10, 3);
    checked(&mut rig, &["\x1b[?69h", "\x1b[3;8s", "\x1b[?6h", "\x1b#8"]);
    let modes = rig.term.modes();
    assert!(!modes.contains(crate::types::Modes::ORIGIN));
    assert!(!modes.contains(crate::types::Modes::LEFT_RIGHT_MARGINS));
}

#[test]
fn inserting_and_deleting_columns_work_only_with_the_cursor_inside_the_region() {
    // The rows outside the scrolling region.
    let mut rig = Rig::new(8, 4);
    checked(&mut rig, &["\x1b[2;3r", "\x1b[2;1Habcdefgh", "\x1b[3;1Hijklmnop", "\x1b[1;3H"]);
    checked(&mut rig, &["\x1b[1'}", "\x1b[1'~"]);
    assert_eq!((rig.row(1), rig.row(2)), ("abcdefgh".into(), "ijklmnop".into()));
    // The columns outside the margins.
    let mut rig = Rig::new(8, 4);
    checked(&mut rig, &["\x1b[?69h", "\x1b[3;6s", "abcdef", "\x1b[1;1H"]);
    checked(&mut rig, &["\x1b[1'}", "\x1b[1'~"]);
    assert_eq!(rig.row(0), "abcdef");
    // Inside both they do act: the f is pushed past the right margin.
    checked(&mut rig, &["\x1b[1;4H", "\x1b[1'}"]);
    assert_eq!(rig.row(0), "abc de");
}
