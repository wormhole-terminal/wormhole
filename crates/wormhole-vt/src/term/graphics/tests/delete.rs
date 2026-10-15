//! Deleting: kitty, "Deleting images". Lower case deletes placements, upper case also frees the
//! pictures' data.

use super::{apc, apc_bare, freed, placed, rgb, rig, spots, uploads};
use crate::term::tests::Rig;

/// Three pictures (ids 1, 2, 3; number 7 on picture 2) and four placements, all with `C=1`:
///
/// - A: picture 1, placement 1, line 0 column 0, 1x1, z 0
/// - B: picture 1, placement 2, line 0 column 5, 1x1, z 1
/// - C: picture 2, line 3 column 10, 2x2, z 0
/// - D: picture 3, line 6 column 20, 2x2, z -1
fn scene() -> Rig {
    let mut r = rig(40, 10);
    r.feed(&apc("a=t,f=24,s=8,v=16,i=1,q=2", &rgb(8, 16)));
    r.feed(&apc("a=t,f=24,s=16,v=32,I=7,q=2", &rgb(16, 32)));
    r.feed(&apc("a=t,f=24,s=16,v=32,i=3,q=2", &rgb(16, 32)));
    for (at, command) in [
        ("\x1b[1;1H", "a=p,i=1,p=1,C=1"),
        ("\x1b[1;6H", "a=p,i=1,p=2,C=1,z=1"),
        ("\x1b[4;11H", "a=p,i=2,C=1"),
        ("\x1b[7;21H", "a=p,i=3,C=1,z=-1"),
    ] {
        r.feed(at);
        r.feed(&apc_bare(&format!("{command},q=2")));
    }
    r.events();
    r
}

/// The placements left, by (picture id, line, column).
fn left(r: &Rig) -> Vec<(u32, i32, usize)> {
    let mut left: Vec<_> = spots(r).into_iter().map(|(id, line, col, ..)| (id, line, col)).collect();
    left.sort_by_key(|&(_, line, col)| (line, col));
    left
}

#[test]
fn the_default_is_everything_on_the_screen_and_keeps_the_data() {
    let mut r = scene();
    assert_eq!(left(&r).len(), 4);
    r.feed(&apc_bare("a=d"));
    assert!(left(&r).is_empty());
    assert!(freed(&mut r).is_empty(), "lower case keeps the pictures");
    r.feed(&apc_bare("a=p,i=3,q=2"));
    assert_eq!(left(&r).len(), 1, "a stored picture can be placed again");
}

#[test]
fn a_capital_also_frees_the_data() {
    let mut r = scene();
    r.feed(&apc_bare("a=d,d=A"));
    assert_eq!(freed(&mut r).len(), 3, "all three pictures");
    r.feed(&apc_bare("a=p,i=3"));
    assert!(r.replies().contains("ENOENT"));
}

#[test]
fn only_what_is_on_the_screen_goes() {
    let mut r = scene();
    // Scroll everything up and a bit more: A and B are in the history now.
    r.feed("\x1b[10;1H\n\n\n\n\n\n\n");
    assert_eq!(r.term.history_size(), 7);
    r.feed("\x1b[1;1H");
    r.feed(&apc_bare("a=p,i=1,p=3,C=1,q=2"));
    r.feed(&apc_bare("a=d,d=a"));
    r.term.scroll_display(crate::types::Scroll::Top);
    let in_history = left(&r);
    assert_eq!(in_history.len(), 4 - 1, "the one placed on the screen went; the others had scrolled off");
    assert!(freed(&mut r).is_empty());
}

#[test]
fn by_id_with_or_without_a_placement_id() {
    let mut r = scene();
    r.feed(&apc_bare("a=d,d=i,i=1,p=2"));
    assert_eq!(left(&r), [(1, 0, 0), (2, 3, 10), (3, 6, 20)]);
    r.feed(&apc_bare("a=d,d=i,i=1"));
    assert_eq!(left(&r), [(2, 3, 10), (3, 6, 20)]);
    r.replies();
    r.feed(&apc_bare("a=d,d=i"));
    assert_eq!(r.replies(), "", "no id to answer to");
    assert_eq!(left(&r).len(), 2, "and nothing named");
    // A picture that does not exist is nothing to delete.
    r.feed(&apc_bare("a=d,d=i,i=99"));
    assert_eq!(left(&r).len(), 2);
    assert_eq!(r.replies(), "\x1b_Gi=99;OK\x1b\\");
}

#[test]
fn by_id_capital_frees_only_pictures_nothing_is_placed_from() {
    let mut r = scene();
    r.feed(&apc_bare("a=d,d=I,i=1,p=2"));
    assert!(freed(&mut r).is_empty(), "placement 1 of picture 1 is still there");
    r.feed(&apc_bare("a=d,d=I,i=1"));
    assert_eq!(freed(&mut r).len(), 1);
    r.feed(&apc_bare("a=p,i=1"));
    assert!(r.replies().contains("ENOENT"));
}

#[test]
fn by_number() {
    let mut r = scene();
    r.feed(&apc_bare("a=d,d=n,I=7"));
    assert_eq!(left(&r), [(1, 0, 0), (1, 0, 5), (3, 6, 20)]);
    r.feed(&apc_bare("a=d,d=N,I=7"));
    assert_eq!(freed(&mut r).len(), 1, "picture 2 is freed");
}

#[test]
fn at_the_cursor_and_at_a_cell() {
    let mut r = scene();
    r.feed("\x1b[5;12H");
    r.feed(&apc_bare("a=d,d=c"));
    assert_eq!(left(&r).len(), 3, "C covers rows 3 and 4, columns 10 and 11");
    let mut r = scene();
    r.feed("\x1b[1;1H");
    r.feed(&apc_bare("a=d,d=c"));
    assert_eq!(left(&r), [(1, 0, 5), (2, 3, 10), (3, 6, 20)], "A is under the cursor");
    // `p`: the cell x, y counted from 1 at the screen's top left.
    let mut r = scene();
    r.feed(&apc_bare("a=d,d=p,x=11,y=5"));
    assert_eq!(left(&r), [(1, 0, 0), (1, 0, 5), (3, 6, 20)]);
    r.feed(&apc_bare("a=d,d=p,x=12,y=5"));
    assert_eq!(left(&r).len(), 3, "no placement there");
}

#[test]
fn at_a_cell_with_a_z_index() {
    let mut r = scene();
    r.feed(&apc_bare("a=d,d=q,x=11,y=4,z=3"));
    assert_eq!(left(&r).len(), 4, "the z does not match");
    r.feed(&apc_bare("a=d,d=q,x=11,y=4,z=0"));
    assert_eq!(left(&r).len(), 3);
    r.feed(&apc_bare("a=d,d=Q,x=1,y=1,z=0"));
    assert_eq!(freed(&mut r).len(), 0, "picture 1 still has placement 2");
}

#[test]
fn by_a_range_of_ids() {
    let mut r = scene();
    r.feed(&apc_bare("a=d,d=r,x=2,y=3"));
    assert_eq!(left(&r), [(1, 0, 0), (1, 0, 5)]);
    let mut r = scene();
    r.feed(&apc_bare("a=d,d=R,x=1,y=2"));
    assert_eq!(left(&r), [(3, 6, 20)]);
    assert_eq!(freed(&mut r).len(), 2);
}

#[test]
fn in_a_column_or_a_row() {
    let mut r = scene();
    r.feed(&apc_bare("a=d,d=x,x=6"));
    assert_eq!(left(&r), [(1, 0, 0), (2, 3, 10), (3, 6, 20)]);
    r.feed(&apc_bare("a=d,d=x,x=22"));
    assert_eq!(left(&r), [(1, 0, 0), (2, 3, 10)], "the second column of D");
    r.feed(&apc_bare("a=d,d=y,y=5"));
    assert_eq!(left(&r), [(1, 0, 0)], "the second row of C");
    r.feed(&apc_bare("a=d,d=Y,y=1"));
    assert!(left(&r).is_empty());
    assert_eq!(freed(&mut r).len(), 1, "picture 1 had nothing left to show");
}

#[test]
fn by_z_index() {
    let mut r = scene();
    r.feed(&apc_bare("a=d,d=z,z=-1"));
    assert_eq!(left(&r).len(), 3);
    r.feed(&apc_bare("a=d,d=Z,z=0"));
    assert_eq!(left(&r), [(1, 0, 5)]);
    assert_eq!(freed(&mut r).len(), 1, "picture 2; picture 3 was only deleted from the screen before");
}

#[test]
fn animation_frames_are_none_so_there_is_nothing_to_delete() {
    let mut r = scene();
    r.feed(&apc_bare("a=d,d=f,i=1"));
    assert_eq!(left(&r).len(), 4);
}

#[test]
fn an_answer_names_the_ids_of_the_command() {
    let mut r = scene();
    r.feed(&apc_bare("a=d,d=i,i=2"));
    assert_eq!(r.replies(), "\x1b_Gi=2;OK\x1b\\");
    r.feed(&apc_bare("a=d,d=a"));
    assert_eq!(r.replies(), "", "no id, no answer");
    let up = uploads(&mut r);
    assert!(up.is_empty());
    let _ = placed(&r);
}
