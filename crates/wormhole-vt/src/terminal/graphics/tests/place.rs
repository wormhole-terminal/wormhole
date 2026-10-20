//! Placing: kitty, "Display images on screen" and "Controlling displayed image layout".

use super::{apc, apc_bare, placed, rgb, rig, spots, uploads};
use crate::terminal::VirtualPlacement;
use crate::types::{Extent, SourceRect};

/// A picture of `w` by `h` pixels stored as image `id` (not yet placed).
fn store(r: &mut crate::terminal::tests::Rig, id: u32, w: usize, h: usize) {
    r.feed(&apc(&format!("a=t,f=24,s={w},v={h},i={id},q=2"), &rgb(w, h)));
}

#[test]
fn transmit_and_display_puts_the_picture_at_the_cursor_and_the_cursor_after_it() {
    // kitty: "the cursor is moved to the position right after the last cell of the image".
    let mut r = rig(40, 10);
    r.feed("ab");
    r.feed(&apc("a=T,f=24,s=16,v=32,i=1,q=2", &rgb(16, 32)));
    assert_eq!(spots(&r), [(1, 0, 2, 2, 2)], "16x32 pixels are 2 cells by 2 rows, at the cursor");
    assert_eq!(r.cursor(), (1, 4), "right of its last row");
    // A pixel more than a cell takes another.
    let mut r = rig(40, 10);
    r.feed(&apc("a=T,f=24,s=17,v=33,i=1,q=2", &rgb(17, 33)));
    assert_eq!(spots(&r), [(1, 0, 0, 3, 3)]);
    assert_eq!(r.cursor(), (2, 3));
}

#[test]
fn a_stored_picture_is_put_where_the_cursor_is_when_asked_to() {
    let mut r = rig(40, 10);
    store(&mut r, 3, 24, 16);
    r.feed("\x1b[4;6H");
    r.feed(&apc_bare("a=p,i=3,p=1"));
    r.feed("\x1b[1;1H");
    r.feed(&apc_bare("a=p,i=3,p=2"));
    assert_eq!(spots(&r), [(3, 3, 5, 3, 1), (3, 0, 0, 3, 1)]);
    assert_eq!(placed(&r).iter().map(|p| p.placement_id).collect::<Vec<_>>(), [1, 2]);
    // The same placement id again moves the placement instead of adding one.
    r.feed("\x1b[9;1H");
    r.feed(&apc_bare("a=p,i=3,p=1"));
    assert_eq!(spots(&r), [(3, 0, 0, 3, 1), (3, 8, 0, 3, 1)]);
}

#[test]
fn c_1_leaves_the_cursor_and_scrolls_nothing() {
    let mut r = rig(40, 5);
    r.feed("\x1b[4;3H");
    r.feed(&apc("a=T,f=24,s=8,v=64,i=1,C=1,q=2", &rgb(8, 64)));
    assert_eq!(r.cursor(), (3, 2), "where it was");
    assert_eq!(r.term.history_len(), 0, "nothing scrolled");
    assert_eq!(spots(&r), [(1, 3, 2, 1, 2)], "cut at the bottom of the screen");
    let p = &placed(&r)[0];
    assert_eq!(p.source.height, 32, "and the picture with it: two of its four rows");
    assert_eq!(p.extent, Extent::Pixels { width: 8, height: 32 });
}

#[test]
fn the_screen_scrolls_to_make_room_for_the_picture() {
    let mut r = rig(40, 6);
    r.feed("one\r\ntwo\r\nthree\r\nfour\r\nfive\r\n> ");
    assert_eq!(r.cursor(), (5, 2));
    r.feed(&apc("a=T,f=24,s=8,v=48,i=1,q=2", &rgb(8, 48)));
    // Three rows from the bottom row: two lines scroll away.
    assert_eq!(r.term.history_len(), 2);
    assert_eq!(spots(&r), [(1, 3, 2, 1, 3)]);
    assert_eq!(r.cursor(), (5, 3));
    assert_eq!(r.row(2), "five");
}

#[test]
fn a_picture_taller_than_the_screen_scrolls_its_top_into_the_history() {
    let mut r = rig(40, 5);
    r.feed(&apc("a=T,f=24,s=8,v=128,i=1,q=2", &rgb(8, 128)));
    assert_eq!(spots(&r), [(1, -3, 0, 1, 8)], "eight rows, the first three above the screen");
    assert_eq!(r.cursor(), (4, 1));
    assert_eq!(r.term.history_len(), 3);
}

#[test]
fn a_scroll_region_without_history_cuts_the_picture_to_the_region() {
    let mut r = rig(40, 10);
    r.feed("\x1b[3;7r\x1b[7;1H");
    r.feed(&apc("a=T,f=24,s=8,v=160,i=1,q=2", &rgb(8, 160)));
    let [p] = placed(&r)[..] else { panic!("{:?}", spots(&r)) };
    assert_eq!((p.line.0, p.rows), (2, 5), "the five rows of the region");
    assert_eq!(r.term.history_len(), 0);
    assert_eq!(r.cursor(), (6, 1));
}

#[test]
fn a_picture_wider_than_the_rest_of_the_line_is_cut_not_scaled() {
    // The old placeholders put the last tile in the last column; the picture is clipped.
    let mut r = rig(40, 10);
    r.feed("\x1b[1;36H");
    r.feed(&apc("a=T,f=24,s=80,v=16,i=1,q=2", &rgb(80, 16)));
    let [p] = placed(&r)[..] else { panic!() };
    assert_eq!((p.column, p.cols, p.rows), (35, 5, 1));
    assert_eq!(p.source, SourceRect { x: 0, y: 0, width: 40, height: 16 }, "the left half of it");
    assert_eq!(p.extent, Extent::Pixels { width: 40, height: 16 });
    // The cursor is in the last column with the wrap waiting.
    assert_eq!(r.cursor(), (0, 39));
    r.feed("x");
    assert_eq!(r.row(1), "x");
}

#[test]
fn a_picture_as_wide_as_the_screen_leaves_the_wrap_pending() {
    let mut r = rig(40, 10);
    r.feed(&apc("a=T,f=24,s=320,v=16,i=1,q=2", &rgb(320, 16)));
    assert_eq!(spots(&r), [(1, 0, 0, 40, 1)]);
    r.feed("y");
    assert_eq!(r.row(1), "y", "the next character starts the next line");
}

#[test]
fn c_and_r_size_the_placement_in_cells() {
    // kitty: "c, r … if only one is given the other is computed to keep the aspect ratio".
    let mut r = rig(40, 10);
    store(&mut r, 1, 80, 40);
    r.feed(&apc_bare("a=p,i=1,c=4"));
    let p = placed(&r)[0];
    assert_eq!((p.cols, p.rows, p.extent), (4, 1, Extent::Pixels { width: 32, height: 16 }));
    r.feed("\x1b[5;1H");
    r.feed(&apc_bare("a=p,i=1,r=4"));
    let p = placed(&r)[1];
    assert_eq!((p.cols, p.rows, p.extent), (16, 4, Extent::Pixels { width: 128, height: 64 }));
    r.feed("\x1b[1;30H");
    r.feed(&apc_bare("a=p,i=1,c=3,r=5,p=9"));
    let p = *placed(&r).last().unwrap_or_else(|| panic!());
    assert_eq!((p.cols, p.rows, p.extent), (3, 5, Extent::Fill), "both: stretched to fit");
}

#[test]
fn a_part_of_the_picture_and_an_offset_inside_the_first_cell() {
    // kitty: "x, y, w, h: the source rectangle; X, Y: offset within the first cell".
    let mut r = rig(40, 10);
    store(&mut r, 1, 100, 50);
    r.feed(&apc_bare("a=p,i=1,x=10,y=5,w=30,h=20,X=3,Y=4"));
    let p = placed(&r)[0];
    assert_eq!(p.source, SourceRect { x: 10, y: 5, width: 30, height: 20 });
    assert_eq!((p.offset_x, p.offset_y), (3, 4));
    assert_eq!((p.cols, p.rows), (5, 2), "33 pixels across is 5 cells, 24 down is 2 rows");
    // A rectangle over the edge is cut to the picture; one that starts outside is refused.
    r.feed(&apc_bare("a=p,i=1,x=90,y=40,w=100,h=100"));
    assert_eq!(placed(&r)[1].source, SourceRect { x: 90, y: 40, width: 10, height: 10 });
    r.feed(&apc_bare("a=p,i=1,x=100"));
    assert!(r.replies().contains("EINVAL"));
    // An offset is less than a cell.
    r.feed(&apc_bare("a=p,i=1,X=99,Y=99"));
    let p = placed(&r)[2];
    assert_eq!((p.offset_x, p.offset_y), (7, 15));
}

#[test]
fn z_orders_the_placements_for_drawing() {
    // kitty: "z … negative values are drawn under the text; lower z first".
    let mut r = rig(40, 10);
    store(&mut r, 1, 8, 16);
    for (z, col) in [(5, 1), (-2, 2), (0, 3), (-2, 4)] {
        r.feed(&format!("\x1b[1;{col}H"));
        r.feed(&apc_bare(&format!("a=p,i=1,z={z},C=1")));
    }
    let order: Vec<(i32, usize)> = placed(&r).iter().map(|p| (p.z, p.column)).collect();
    assert_eq!(order, [(-2, 1), (-2, 3), (0, 2), (5, 0)]);
}

#[test]
fn a_virtual_placement_is_declared_not_drawn() {
    // kitty, "Unicode placeholders": `a=p,U=1` makes a virtual placement; nothing is drawn or moved.
    let mut r = rig(40, 10);
    store(&mut r, 5, 80, 48);
    r.feed("ab");
    r.feed(&apc_bare("a=p,i=5,U=1,c=10,r=3,p=2"));
    assert!(placed(&r).is_empty());
    assert_eq!(r.cursor(), (0, 2), "the cursor stays");
    let handle = uploads(&mut r)[0].image;
    assert_eq!(
        r.term.virtual_placement(5, 2),
        Some(VirtualPlacement { image: handle, image_id: 5, placement_id: 2, cols: 10, rows: 3 })
    );
    // Without a placement id a cell uses the newest placement; an unknown picture has none.
    assert_eq!(r.term.virtual_placement(5, 0).map(|v| v.placement_id), Some(2));
    assert_eq!(r.term.virtual_placement(6, 0), None);
    // Sizes default to the picture's own, in cells.
    r.feed(&apc_bare("a=p,i=5,U=1,p=3"));
    assert_eq!(r.term.virtual_placement(5, 3).map(|v| (v.cols, v.rows)), Some((10, 3)));
    // The same placement id replaces; deleting the picture takes them along.
    r.feed(&apc_bare("a=p,i=5,U=1,p=3,c=1,r=1"));
    assert_eq!(r.term.virtual_placement(5, 3).map(|v| (v.cols, v.rows)), Some((1, 1)));
    r.feed(&apc_bare("a=d,d=i,i=5"));
    assert_eq!(r.term.virtual_placement(5, 3), None, "a=d,d=i removes the virtual placements too");
}

#[test]
fn transmit_and_display_with_u_1_declares_a_virtual_placement() {
    let mut r = rig(40, 10);
    r.feed(&apc("a=T,f=24,s=16,v=16,i=8,U=1,c=2,r=1,q=2", &rgb(16, 16)));
    assert!(placed(&r).is_empty());
    assert_eq!(r.term.virtual_placement(8, 0).map(|v| (v.cols, v.rows)), Some((2, 1)));
}

#[test]
fn a_picture_that_leaves_the_cursor_leaves_a_pending_wrap_too() {
    let mut r = rig(10, 4);
    r.feed("0123456789");
    r.feed(&apc("a=T,f=24,s=8,v=16,i=1,C=1,q=2", &rgb(8, 16)));
    r.feed("x");
    assert_eq!(r.row(0), "0123456789");
    assert_eq!(r.row(1), "x", "the wrap that was waiting still happens");
}
