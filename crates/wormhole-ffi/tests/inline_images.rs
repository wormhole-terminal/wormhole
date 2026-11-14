//! Inline pictures against the real engine and a real PTY: a script prints the escape sequences
//! (kitty with raw pixels, which needs no decoder, and a Sixel), the frame says where they are and
//! `inline_image` hands over the pixels.

use std::sync::Arc;
use std::time::{Duration, Instant};

use wormhole_ffi::{ContentsScope, Terminal, TerminalListener};

struct Quiet;

impl TerminalListener for Quiet {
    fn wake(&self) {}
}

const HEADER: usize = 48;
const RECORD: usize = 64;

fn start(script: &str) -> Arc<Terminal> {
    let args = vec!["-c".to_string(), format!("{script}; exec cat")];
    Terminal::start("/bin/sh".into(), args, 40, 10, 8.0, 16.0, Arc::new(Quiet)).expect("starts")
}

fn word(bytes: &[u8], at: usize) -> u32 {
    u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
}

/// Frames until one carries a placements section; its records as words.
fn placements(terminal: &Terminal) -> Vec<Vec<u32>> {
    let until = Instant::now() + Duration::from_secs(5);
    loop {
        let frame = terminal.frame();
        if frame.len() >= HEADER && word(&frame, 20) & 2 != 0 {
            let (rows, cols, count) =
                (word(&frame, 16) as usize, word(&frame, 8) as usize, word(&frame, 44) as usize);
            let start = HEADER + rows * (8 + cols * 16);
            return (0..count)
                .map(|n| (0..16).map(|w| word(&frame, start + n * RECORD + w * 4)).collect())
                .collect();
        }
        assert!(Instant::now() < until, "no placements section arrived");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn a_kitty_picture_arrives_as_a_placement_and_its_pixels_by_key() {
    // Two raw RGB pixels, red and green, over 3 x 2 cells, then text beside it.
    let terminal =
        start("printf '\\033_Ga=T,f=24,s=2,v=1,i=7,c=3,r=2,q=2;/wAAAP8A\\033\\\\'; printf 'after\\r\\n'");
    let [record] = placements(&terminal).try_into().expect("one placement");
    // key, z, line, column, cols, rows, source x y w h, offset x y, kind (fill: both cells given).
    assert_eq!(&record[1..6], &[0, 0, 0, 3, 2]);
    assert_eq!(&record[6..10], &[0, 0, 2, 1]);
    assert_eq!(record[12], 0);
    let image = terminal.inline_image(record[0]).expect("pixels");
    assert_eq!((image.width, image.height), (2, 1));
    assert_eq!(image.rgba, [255, 0, 0, 255, 0, 255, 0, 255]);
    assert!(terminal.inline_image(record[0] + 1000).is_none());
    // Nothing changed since: a still screen's second frame is none at all (`pack_frame`), a full frame repeats the section.
    terminal.frame();
    let again = terminal.frame();
    assert!(again.is_empty(), "a still screen sends no frame");
    let full = terminal.full_frame();
    assert_eq!(word(&full, 20) & 2, 2);
    assert_eq!(word(&full, 44), 1);
}

#[test]
fn a_sixel_is_a_placement_with_transparent_pixels_where_none_were_set() {
    let terminal = start("printf '\\033Pq#1;2;100;0;0~\\033\\\\'");
    let [record] = placements(&terminal).try_into().expect("one placement");
    let image = terminal.inline_image(record[0]).expect("pixels");
    // A sixel band is six pixels tall, and the engine pads it to the cell rows it covers.
    assert_eq!(image.width, 1);
    assert_eq!(image.rgba.len(), (image.width * image.height * 4) as usize);
    assert_eq!(&image.rgba[..4], &[255, 0, 0, 255]);
}

#[test]
fn text_pushing_the_picture_up_gives_it_a_negative_line_and_a_scroll_back_lists_it_again() {
    let terminal =
        start("printf '\\033_Ga=T,f=24,s=2,v=1,i=7,c=3,r=2,q=2;/wAAAP8A\\033\\\\'; printf 'after\\r\\n'");
    assert_eq!(placements(&terminal)[0][2] as i32, 0);
    // `cat` echoes each line twice (the tty's echo and its own output): the screen scrolls by one row, so the
    // picture's top row is in the history while its second row is still in view.
    terminal.write(b"1\n2\n3\n4\n".to_vec());
    let pushed = placements(&terminal);
    assert!((pushed[0][2] as i32) < 0, "{pushed:?}");
    // Pushed out of view altogether: the list is empty (and says so, once).
    terminal.write(b"5\n6\n7\n8\n9\n".to_vec());
    assert!(placements(&terminal).is_empty());
    // Scrolled back into the history it is listed again, with the same key.
    terminal.scroll(30);
    assert_eq!(placements(&terminal).len(), 1);
}

#[test]
fn contents_is_the_screen_or_everything_with_wrapped_rows_joined() {
    let terminal = start(
        "i=0; while [ $i -lt 30 ]; do printf \"line $i\\r\\n\"; i=$((i+1)); done; printf '%s' 0123456789012345678901234567890123456789ABCDE",
    );
    let until = Instant::now() + Duration::from_secs(5);
    while !terminal.screen_text().contains("ABCDE") {
        terminal.frame();
        assert!(Instant::now() < until);
        std::thread::sleep(Duration::from_millis(10));
    }
    let screen = terminal.contents(ContentsScope::Screen);
    assert!(!screen.contains("line 0\n"), "the first lines scrolled off the 10-row screen");
    let everything = terminal.contents(ContentsScope::Everything);
    assert!(everything.starts_with("line 0\nline 1\n"), "{everything:?}");
    assert!(
        everything.contains("0123456789012345678901234567890123456789ABCDE"),
        "the wrapped row is one line"
    );
    assert!(everything.len() > screen.len());
}
