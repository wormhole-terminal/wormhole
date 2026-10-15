//! Sixel: DEC VT330/VT340 Programmer Reference, chapter 14; xterm's ctlseqs for modes 80 and 8452.

use super::{placed, rig, spots, uploads};
use crate::term::tests::Rig;
use crate::term::{Compression, ImageFormat};
use crate::types::{Extent, SourceRect};

/// A sixel of `w` columns and `bands` bands, every pixel set, in 1:1 pixels.
fn sixel(w: usize, bands: usize) -> String {
    let mut data = String::from("\x1bP9;1q\"1;1\n#1;2;100;0;0");
    for band in 0..bands {
        data.push_str(&format!("#1!{w}~"));
        if band + 1 < bands {
            data.push('-');
        }
    }
    data.push_str("\x1b\\");
    data
}

fn start(r: &mut Rig, w: usize, bands: usize) {
    r.feed(&sixel(w, bands));
}

#[test]
fn a_sixel_becomes_rgba_for_the_host_and_a_placement_at_the_cursor() {
    let mut r = rig(40, 10);
    r.feed("ab");
    start(&mut r, 20, 3);
    let up = uploads(&mut r);
    assert_eq!(up.len(), 1);
    assert_eq!((up[0].format, up[0].compression), (ImageFormat::Rgba, Compression::None));
    assert_eq!((up[0].width, up[0].height), (20, 18));
    assert_eq!(&up[0].data[..4], &[255, 0, 0, 255]);
    assert_eq!(up[0].data.len(), 20 * 18 * 4);
    // 20x18 pixels in 8x16 cells: 3 columns, 2 rows, from the cursor.
    assert_eq!(spots(&r), [(0, 0, 2, 3, 2)]);
    let p = placed(&r)[0];
    assert_eq!(
        (p.source, p.extent),
        (SourceRect { x: 0, y: 0, width: 20, height: 18 }, Extent::Pixels { width: 20, height: 18 })
    );
    assert_eq!(r.replies(), "", "a Sixel is not answered");
}

#[test]
fn with_scrolling_the_cursor_goes_to_the_row_under_the_picture_at_its_left_edge() {
    // DEC STD 070 / VT340: sixel scrolling on (DECSDM reset), the default.
    let mut r = rig(40, 10);
    r.feed("ab");
    start(&mut r, 20, 3);
    assert_eq!(r.cursor(), (2, 2));
    r.feed("x");
    assert_eq!(r.row(2), "  x");
}

#[test]
fn the_screen_scrolls_when_the_picture_does_not_fit_below_the_cursor() {
    let mut r = rig(40, 5);
    r.feed("\x1b[5;1Hlast");
    r.feed("\x1b[5;1H");
    start(&mut r, 8, 6);
    // 36 pixels are 3 rows; the cursor ends on the row below, the bottom one.
    assert_eq!(spots(&r), [(0, 1, 0, 1, 3)]);
    assert_eq!(r.cursor(), (4, 0));
    assert_eq!(r.term.history_size(), 3);
}

#[test]
fn mode_8452_leaves_the_cursor_right_of_the_picture() {
    // xterm: "8452: Sixel scrolling leaves cursor to right of graphic".
    let mut r = rig(40, 10);
    r.feed("\x1b[?8452h");
    start(&mut r, 20, 3);
    assert_eq!(r.cursor(), (1, 3));
}

#[test]
fn with_display_scrolling_off_the_picture_is_at_the_top_left_and_the_cursor_stays() {
    // DECSDM set: "sixel display mode": no scrolling, the picture at the screen's upper left.
    let mut r = rig(40, 4);
    r.feed("\x1b[?80h\x1b[3;5H");
    start(&mut r, 16, 20);
    let p = placed(&r)[0];
    assert_eq!((p.line.0, p.column, p.cols), (0, 0, 2));
    assert_eq!(p.rows, 4, "cut at the bottom of the screen");
    assert_eq!(p.source.height, 4 * 120 / 8, "and the picture with it");
    assert_eq!(r.cursor(), (2, 4));
    assert_eq!(r.term.history_size(), 0);
}

#[test]
fn a_picture_wider_than_the_line_is_cut() {
    let mut r = rig(10, 10);
    r.feed("\x1b[1;9H");
    start(&mut r, 40, 1);
    let p = placed(&r)[0];
    assert_eq!((p.column, p.cols), (8, 2));
    assert_eq!(p.source.width, 16);
}

#[test]
fn the_aspect_ratio_stretches_the_pixels() {
    // DCS P1: 0 and 1 mean 2:1, so each row of sixels is two rows of pixels.
    let mut r = rig(40, 10);
    r.feed("\x1bP0;1q#1;2;100;0;0#1!8~\x1b\\");
    let up = uploads(&mut r);
    assert_eq!((up[0].width, up[0].height), (8, 12));
    r.feed("\x1bP;1q#1;2;100;0;0#1!8~\x1b\\");
    assert_eq!(uploads(&mut r)[0].height, 12, "an omitted P1 is 0");
    r.feed("\x1bP7;1q#1;2;100;0;0#1!8~\x1b\\");
    assert_eq!(uploads(&mut r)[0].height, 6);
}

#[test]
fn a_sixel_that_paints_nothing_or_too_much_draws_nothing() {
    let mut r = rig(40, 10);
    r.feed("\x1bPq\x1b\\\x1bPq??????\x1b\\\x1bPq!99999~\x1b\\");
    assert!(uploads(&mut r).is_empty());
    assert!(placed(&r).is_empty());
    // And the terminal goes on as it was.
    r.feed("ok");
    assert_eq!(r.row(0), "ok");
}

#[test]
fn a_sixel_over_its_cap_is_dropped_whole_and_the_stream_goes_on() {
    let mut r = rig(40, 10);
    let big = format!("\x1bPq{}\x1b\\ok", "~".repeat(5 * 1024 * 1024));
    r.feed(&big);
    assert!(uploads(&mut r).is_empty());
    assert_eq!(r.row(0), "ok");
}

#[test]
fn a_sixel_cut_into_pieces_is_the_same() {
    let whole = sixel(30, 2);
    let mut a = rig(40, 10);
    a.feed(&whole);
    let mut b = rig(40, 10);
    for piece in whole.as_bytes().chunks(7) {
        b.feed_bytes(piece);
    }
    assert_eq!(
        uploads(&mut a),
        uploads(&mut b)
            .into_iter()
            .map(|mut u| {
                u.image = crate::types::ImageHandle(1);
                u
            })
            .collect::<Vec<_>>()
    );
    assert_eq!(spots(&a), spots(&b));
}
