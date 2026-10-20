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
