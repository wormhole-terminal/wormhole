//! iTerm2's inline images: the "Inline Images Protocol" page.

use super::{CELL, freed, placed, png, rig, spots, uploads};
use crate::terminal::base64;
use crate::terminal::tests::Rig;
use crate::terminal::{Event, ImageFormat};
use crate::types::Extent;

fn file(args: &str, bytes: &[u8]) -> String {
    format!("\x1b]1337;File={args}:{}\x07", base64::encode(bytes))
}

#[test]
fn an_inline_file_is_handed_to_the_host_and_shown_at_its_size() {
    let mut r = rig(40, 10);
    let bytes = png(100, 40);
    r.feed("$ ");
    r.feed(&file("inline=1;name=Y2F0LnBuZw==", &bytes));
    let up = uploads(&mut r);
    assert_eq!(up.len(), 1);
    assert_eq!((up[0].format, up[0].width, up[0].height), (ImageFormat::File, 100, 40));
    assert_eq!(up[0].data, bytes);
    assert_eq!(spots(&r), [(0, 0, 2, 13, 3)]);
    assert_eq!(r.cursor(), (2, 15), "right of its last row, like kitty's");
    assert_eq!(r.replies(), "");
}

#[test]
fn a_file_that_is_not_inline_is_a_download_and_shows_nothing() {
    let mut r = rig(40, 10);
    r.feed(&file("name=YQ==", &png(10, 10)));
    r.feed(&file("inline=0", &png(10, 10)));
    assert!(r.events().is_empty());
    assert!(placed(&r).is_empty());
}

#[test]
fn what_is_not_a_picture_is_ignored() {
    let mut r = rig(40, 10);
    r.feed(&file("inline=1", b"hello, this is text"));
    r.feed("\x1b]1337;File=inline=1:!!!not base64\x07");
    r.feed(&file("inline=1", &png(0, 0)));
    r.feed(&file("inline=1", &png(20000, 20000)));
    assert!(r.events().is_empty());
    assert!(placed(&r).is_empty());
}

#[test]
fn sizes_in_cells_pixels_percent_and_auto() {
    let mut r = rig(40, 10);
    let bytes = png(100, 50);
    let cases = [
        ("inline=1;width=10;height=4", (10, 4), Extent::Fit),
        ("inline=1;width=10;height=4;preserveAspectRatio=0", (10, 4), Extent::Fill),
        ("inline=1;width=50%;height=auto", (20, 5), Extent::Pixels { width: 160, height: 80 }),
        ("inline=1;width=64px", (8, 2), Extent::Pixels { width: 64, height: 32 }),
        ("inline=1;height=3", (12, 3), Extent::Pixels { width: 96, height: 48 }),
    ];
    for (args, (cols, rows), extent) in cases {
        r.feed("\x1b[2J\x1b[1;1H");
        r.feed(&file(args, &bytes));
        let p = *placed(&r).last().unwrap_or_else(|| panic!("{args}"));
        assert_eq!((p.cols, p.rows, p.extent), (cols, rows, extent), "{args}");
    }
}

#[test]
fn a_picture_wider_than_the_terminal_is_scaled_down_to_it() {
    let mut r = rig(40, 20);
    r.feed(&file("inline=1", &png(640, 320)));
    let p = placed(&r)[0];
    assert_eq!((p.cols, p.rows), (40, 10));
    assert_eq!(p.extent, Extent::Pixels { width: 320, height: 160 });
    let _ = CELL;
}

#[test]
fn do_not_move_cursor_leaves_it() {
    let mut r = rig(40, 10);
    r.feed("ab");
    r.feed(&file("inline=1;doNotMoveCursor=1", &png(16, 16)));
    assert_eq!(r.cursor(), (0, 2));
    assert_eq!(spots(&r).len(), 1);
}

#[test]
fn the_multipart_form_is_put_together() {
    let mut r = rig(40, 10);
    let bytes = png(16, 32);
    let text = base64::encode(&bytes);
    let (a, b) = text.split_at(20);
    r.feed("\x1b]1337;MultipartFile=inline=1;width=2\x07");
    r.feed(&format!("\x1b]1337;FilePart={a}\x07\x1b]1337;FilePart={b}\x07"));
    assert!(r.events().is_empty(), "nothing before the end");
    r.feed("\x1b]1337;FileEnd\x07");
    let up = uploads(&mut r);
    assert_eq!(up[0].data, bytes);
    let p = placed(&r)[0];
    assert_eq!((p.cols, p.rows), (2, 2));
}

#[test]
fn a_part_without_its_start_or_a_new_file_in_between_is_dropped() {
    let mut r = rig(40, 10);
    r.feed("\x1b]1337;FilePart=AAAA\x07\x1b]1337;FileEnd\x07");
    assert!(r.events().is_empty());
    r.feed("\x1b]1337;MultipartFile=inline=1\x07\x1b]1337;FilePart=AAAA\x07");
    r.feed(&file("inline=1", &png(8, 16)));
    r.feed("\x1b]1337;FileEnd\x07");
    assert_eq!(uploads(&mut r).len(), 1, "only the whole file");
}

#[test]
fn the_rest_of_the_protocol_is_the_hosts() {
    let mut r = rig(40, 10);
    r.feed("\x1b]1337;SetMark\x07\x1b]1337;CurrentDir=/tmp\x07");
    assert_eq!(r.events(), [Event::Iterm(b"SetMark".to_vec()), Event::Iterm(b"CurrentDir=/tmp".to_vec())]);
    let _ = freed(&mut r);
}

#[test]
fn what_is_not_a_picture_is_cut_off_at_64_kib_however_it_arrives() {
    let mut r = rig(40, 10);
    let var = |kib: usize| format!("\x1b]1337;Custom=name={}\x07", "v".repeat(kib * 1024));
    r.feed(&var(60));
    assert!(matches!(&r.events()[..], [Event::Iterm(data)] if data.len() > 60 * 1024));
    r.feed(&var(70));
    assert!(r.events().is_empty(), "past 64 KiB it is dropped whole, not handed to the host");
    // Cut into small pieces, so the kind is not known when the first bytes arrive.
    for piece in var(70).as_bytes().chunks(5) {
        r.feed_bytes(piece);
    }
    assert!(r.events().is_empty());
    for piece in var(10).as_bytes().chunks(5) {
        r.feed_bytes(piece);
    }
    assert_eq!(r.events().len(), 1, "and the next string is read as usual");
}

#[test]
fn a_picture_keeps_its_own_limit_past_64_kib() {
    let mut r = rig(40, 10);
    let mut bytes = png(100, 40);
    bytes.resize(200 * 1024, 7);
    r.feed(&file("inline=1", &bytes));
    assert_eq!(uploads(&mut r).len(), 1);
    // The multipart form too: its chunks are pictures' as well.
    r.feed("\x1b]1337;MultipartFile=inline=1\x07");
    r.feed(&format!("\x1b]1337;FilePart={}\x07", "A".repeat(100 * 1024)));
    r.feed("\x1b]1337;FileEnd\x07");
    assert!(r.events().is_empty(), "a part that is no picture shows nothing, but is no host event");
}

fn _unused(_: &Rig) {}

#[test]
fn an_inline_file_with_only_a_height_keeps_its_shape_unless_told_not_to() {
    // iTerm2 inline images: width and height are N cells, Npx pixels, N% or auto;
    // preserveAspectRatio=0 stretches to the given size.
    let mut r = rig(40, 20);
    let file = |args: &str| {
        format!("\x1b]1337;File=inline=1;{args}:{}\x07", crate::terminal::base64::encode(&png(100, 40)))
    };
    r.feed(&file("height=5"));
    r.feed("\r\n");
    r.feed(&file("height=5;preserveAspectRatio=0"));
    let (keep, stretch) = (spots(&r)[0], spots(&r)[1]);
    assert_eq!(keep.4, 5, "five rows");
    assert!(keep.3 > stretch.3, "kept shape: {keep:?} against stretched {stretch:?}");
    assert_eq!(placed(&r).len(), 2);
}

#[test]
fn a_multipart_file_that_is_started_again_or_never_ended_leaves_nothing_behind() {
    // iTerm2: MultipartFile=args, then FilePart=base64 pieces, then FileEnd.
    let mut r = rig(40, 20);
    let bytes = png(40, 40);
    let b64 = crate::terminal::base64::encode(&bytes);
    let (a, b) = b64.split_at(b64.len() / 2);
    r.feed(&format!("\x1b]1337;MultipartFile=inline=1\x07\x1b]1337;FilePart={a}\x07"));
    // A second start throws the first away; a part with no start and an end with no start are ignored.
    r.feed(&format!("\x1b]1337;MultipartFile=inline=1\x07\x1b]1337;FilePart={a}\x07\x1b]1337;FilePart={b}\x07\x1b]1337;FileEnd\x07"));
    r.feed(&format!("\x1b]1337;FilePart={a}\x07\x1b]1337;FileEnd\x07"));
    assert_eq!(uploads(&mut r).len(), 1);
    assert_eq!(placed(&r).len(), 1);
}
