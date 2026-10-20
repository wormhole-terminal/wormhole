//! The engine on real PTYs and on test terminals, where it meets the host: answering color
//! queries itself, pictures placed in the grid, the clipboard, DECCOLM, a synchronized update
//! holding the UI back, and copies keeping tabs. What a terminal does as a whole is in
//! `terminal::tests`.

use std::time::{Duration, Instant};

use wormhole_vt::{COLOR_COUNT, Column, Line, Point, Rgb, SelectionType, Side};

use crate::testing::TestTerm;
use crate::tests::{drain, wait_until};
use crate::{GridSize, Pty, PtyEvent, ScreenSnapshot, StartOptions};

const SIZE: GridSize = GridSize { cols: 80, rows: 24, cell_width: 8.0, cell_height: 16.0 };

fn options() -> StartOptions<'static> {
    StartOptions { scrollback_lines: 1000, ..StartOptions::default() }
}

fn spawn(script: &str) -> (Pty, crate::PtyEvents) {
    Pty::spawn_program(SIZE, &options(), "/bin/sh", &["-c", script]).expect("spawn")
}

/// A 1×1 PNG, base64.
const PNG: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

#[test]
fn the_engine_answers_color_queries_with_the_themes_colors_unless_the_program_set_them() {
    // Background (OSC 11), then the cursor position, then the foreground after setting it (OSC 10).
    // The program waits a moment, so that the palette is in before it asks.
    let script = r#"sleep 0.3; stty raw -echo; printf '\033]11;?\007\033[6n'; r=$(dd bs=1 count=30 2>/dev/null); stty sane; printf '%s' "$r" | tr '\033\007' EB; sleep 5"#;
    let (pty, _events) = spawn(script);
    let mut palette = [Rgb { r: 0, g: 0, b: 0 }; COLOR_COUNT];
    palette[257] = Rgb { r: 1, g: 2, b: 3 };
    pty.set_default_colors(&palette);
    wait_until("both replies, in order", || pty.screen_text().contains("E]11;rgb:0101/0202/0303BE[1;1R"));
}

#[test]
fn the_engine_answers_a_screen_checksum_request_once_the_host_allows_it() {
    // DECRQCRA of the one cell holding "A", read back raw (the reply is 11 bytes).
    let script = r#"sleep 0.3; stty raw -echo; printf 'A\033[5;0;1;1;1;1*y'; r=$(dd bs=1 count=11 2>/dev/null); stty sane; printf '%s' "$r" | tr '\033' E; sleep 5"#;
    let allowed = StartOptions { allow_screen_checksum: true, ..options() };
    let (pty, _events) = Pty::spawn_program(SIZE, &allowed, "/bin/sh", &["-c", script]).expect("spawn");
    wait_until("the checksum", || pty.screen_text().contains("EP5!~0041E\\"));
}

/// The window operations the program in `script` asked for, as the UI hears them, once its output
/// has shown `done`.
fn window_ops(options: &StartOptions, script: &str) -> (Pty, Vec<crate::WindowOp>) {
    let (pty, events) = Pty::spawn_program(SIZE, options, "/bin/sh", &["-c", script]).expect("spawn");
    wait_until("the output", || pty.screen_text().contains("done"));
    let mut ops = Vec::new();
    while let Some(event) = events.try_recv() {
        if let PtyEvent::WindowOp(op) = event {
            ops.push(op);
        }
    }
    (pty, ops)
}

#[test]
fn window_operations_reach_the_ui_only_when_the_host_allows_them() {
    use crate::{Extent, WindowOp};

    let script = "printf '\\033[8;30;100t\\033[2t'; printf done; sleep 5";
    let (_pty, ops) = window_ops(&options(), script);
    assert!(ops.is_empty(), "off by default: {ops:?}");
    let allowed = StartOptions { allow_window_ops: true, ..options() };
    let (pty, ops) = window_ops(&allowed, script);
    assert_eq!(
        ops,
        [WindowOp::ResizeCells { rows: Extent::Size(30), cols: Extent::Size(100) }, WindowOp::Iconify]
    );
    // The engine took the size at once; the UI brings it back to the pane's when the window is done.
    pty.sync_size();
}

#[test]
fn deccolm_asks_the_ui_for_the_width_when_window_operations_are_allowed() {
    use crate::{Extent, WindowOp};

    let allowed = StartOptions { allow_window_ops: true, ..options() };
    let (_pty, ops) = window_ops(&allowed, "printf '\\033[?40h\\033[?3h'; printf done; sleep 5");
    assert_eq!(ops, [WindowOp::ResizeCells { rows: Extent::Keep, cols: Extent::Size(132) }]);
}

#[test]
fn a_kitty_picture_is_decoded_kept_placed_and_freed() {
    let script = format!(
        "printf '\\033_Ga=T,f=100,i=7,c=3,r=2,q=2;{PNG}\\033\\\\'; printf 'done\\n'; read go; \
         printf '\\033_Ga=d,d=I,i=7\\033\\\\'; printf 'freed'; sleep 5"
    );
    let (pty, _events) = spawn(&script);
    wait_until("the output", || pty.screen_text().contains("done"));
    let mut snapshot = ScreenSnapshot::default();
    wait_until("the reservation", || pty.images().len() == 1);
    pty.snapshot(&mut snapshot, 0);
    let [placement] = snapshot.placements() else { panic!("one placement: {:?}", snapshot.placements()) };
    assert_eq!((placement.cols, placement.rows, placement.line, placement.column), (3, 2, Line(0), 0));
    let key = crate::engine::image_key(placement.image);
    assert_eq!(pty.images().dimensions(key), Some((1, 1)));
    // The store counts the picture from its reservation; the pixels come from the decoder thread.
    wait_until("the pixels", || pty.images().get(key).is_some());
    assert_eq!(pty.images().get(key).map(|image| image.rgba.len()), Some(4));
    // The picture's cells hold no text, so a copy of everything has none of it.
    pty.select_all();
    assert!(!pty.selection_text().unwrap_or_default().contains('\u{10EEEE}'));
    // The program waits for this line, so no machine load can free the picture before it was checked.
    pty.write(b"\n".to_vec());
    wait_until("the delete", || pty.screen_text().contains("freed"));
    wait_until("the pixels to go", || pty.images().is_empty());
    pty.snapshot(&mut snapshot, 0);
    assert!(snapshot.placements().is_empty());
}

#[test]
fn raw_pixels_and_sixel_are_kept_without_a_decoder_and_a_bad_picture_is_refused() {
    // A 2×1 RGB picture, a Sixel, and a PNG that is not one (the program asked for the answer).
    let script = "printf '\\033_Ga=T,f=24,s=2,v=1,i=7,q=2;AAAAAAAA\\033\\\\'; printf '\\033Pq#1;2;100;0;0~\\033\\\\'; \
                  printf '\\033_Ga=T,f=100,i=9;bm90IGEgcG5n\\033\\\\'; \
                  sleep 5";
    let (pty, _events) = spawn(script);
    wait_until("the pictures", || pty.images().len() == 2);
    let mut snapshot = ScreenSnapshot::default();
    pty.snapshot(&mut snapshot, 0);
    assert_eq!(snapshot.placements().len(), 2);
    // The refusal reaches the program (kitty's `ENODATA`), after the decode failed.
    // (The terminal echoes the reply, as it does what a program did not read.)
    wait_until("the error reply", || pty.screen_text().contains("^[_Gi=9;ENODATA"));
}

#[test]
fn placeholder_cells_find_their_picture_through_the_virtual_placement() {
    // A virtual placement of image 5 (4×2 cells), then the first tile as a placeholder cell:
    // U+10EEEE with the id in the foreground color and row and column 0 as diacritics.
    let script = format!(
        "printf '\\033_Ga=T,f=100,i=5,U=1,c=4,r=2,q=2;{PNG}\\033\\\\'; printf '\\033[38;5;5m\\xf4\\x8e\\xbb\\xae\\xcc\\x85\\xcc\\x85\\033[0m done'; sleep 5"
    );
    let (pty, _events) = spawn(&script);
    wait_until("the output", || pty.screen_text().contains("done"));
    wait_until("the reservation", || pty.images().len() == 1);
    let tiles = pty.virtual_placement(5, 0).expect("a virtual placement");
    assert_eq!((tiles.cols, tiles.rows), (4, 2));
    assert!(pty.images().contains(tiles.key));
    assert_eq!(pty.virtual_placement(6, 0), None);
    let mut snapshot = ScreenSnapshot::default();
    pty.snapshot(&mut snapshot, 0);
    assert!(snapshot.placements().is_empty(), "virtual placements are drawn from their cells");
    let first = &snapshot.rows()[0];
    let cell = first.cells()[0];
    assert!(crate::images::placeholder_at(&cell, first.extras().cell(0)).is_some());
}

#[test]
fn the_clipboard_is_written_and_read_through_osc_52() {
    let script = r#"printf '\033]52;c;aGVsbG8=\007'; printf 'asked'; printf '\033]52;c;?\033\\'; sleep 5"#;
    let (pty, events) = spawn(script);
    wait_until("the output", || pty.screen_text().contains("asked"));
    let mut stored = None;
    let mut load = None;
    let deadline = Instant::now() + Duration::from_secs(10);
    while load.is_none() {
        assert!(Instant::now() < deadline, "the read never reached the UI");
        match events.try_recv() {
            Some(PtyEvent::ClipboardStore(text)) => stored = Some(text),
            Some(PtyEvent::ClipboardLoad(format, _reply)) => load = Some(format),
            _ => std::thread::sleep(Duration::from_millis(5)),
        }
    }
    assert_eq!(stored.as_deref(), Some("hello"));
    assert_eq!(load.map(|format| format("hi")), Some("\x1b]52;c;aGk=\x1b\\".to_string()));
    drop(drain(&events));
}

#[test]
fn deccolm_clears_the_screen_and_the_grid_keeps_the_ptys_width() {
    let script = r#"printf 'before'; printf '\033[?40h\033[?3h'; printf 'after'; sleep 5"#;
    let (pty, _events) = spawn(script);
    wait_until("the output", || pty.screen_text().contains("after"));
    let mut snapshot = ScreenSnapshot::default();
    pty.snapshot(&mut snapshot, 0);
    assert_eq!(snapshot.columns(), 80, "the PTY is 80 columns wide, and so is the grid");
    assert!(!pty.screen_text().contains("before"), "132 columns clears the screen");
}

#[test]
fn a_synchronized_update_keeps_the_last_frame_until_it_ends() {
    let mut term = TestTerm::new(20, 3, 10);
    let mut snapshot = ScreenSnapshot::default();
    term.feed(b"first");
    term.snapshot(&mut snapshot, 0);
    assert_eq!(snapshot.rows()[0].cells()[0].c, 'f');
    term.feed(b"\x1b[?2026h\x1b[H\x1b[2Jsecond");
    term.snapshot(&mut snapshot, 0);
    assert_eq!(snapshot.rows()[0].cells()[0].c, 'f', "held back");
    assert!(!snapshot.rows()[0].changed());
    term.feed(b"\x1b[?2026l");
    term.snapshot(&mut snapshot, 0);
    assert_eq!(snapshot.rows()[0].cells()[0].c, 's');
    assert!(snapshot.rows()[0].changed());
}

#[test]
fn a_copy_keeps_the_tabs_a_program_printed() {
    let mut term = TestTerm::new(30, 3, 10);
    term.feed(b"name\tvalue\r\nx\t\ty");
    term.start_selection(SelectionType::Simple, Point::new(Line(0), Column(0)), Side::Left);
    term.update_selection(Point::new(Line(1), Column(29)), Side::Right);
    assert_eq!(term.screen().selection_text().as_deref(), Some("name\tvalue\nx\t\ty"));
}

/// Ten lines on a five-row screen: five of them in the history.
fn full_term() -> TestTerm {
    let mut term = TestTerm::new(20, 5, 100);
    for i in 0..10 {
        term.feed(format!("line {i}\r\n").as_bytes());
    }
    assert!(term.history_size() >= 5);
    term
}

#[test]
fn clear_empties_the_screen_and_the_scrollback() {
    // What macOS `/usr/bin/clear` writes: erase the history (ED 3), home, erase the screen (ED 2).
    let mut term = full_term();
    term.feed(b"\x1b[3J\x1b[H\x1b[2J");
    assert_eq!(term.history_size(), 0);
    assert_eq!(term.screen_text().trim(), "");
    // And ncurses' order, history last.
    let mut term = full_term();
    term.feed(b"\x1b[H\x1b[2J\x1b[3J");
    assert_eq!((term.history_size(), term.screen_text().trim()), (0, ""));
}

#[test]
fn a_screen_erase_alone_keeps_the_old_screen_in_the_scrollback() {
    // zsh's Ctrl-L and `tput clear` without ED 3: the screen is blank and what was on it can be
    // scrolled back to.
    let mut term = full_term();
    let history = term.history_size();
    term.feed(b"\x1b[H\x1b[2J");
    assert_eq!(term.screen_text().trim(), "");
    assert!(term.history_size() > history, "{} lines before, {} after", history, term.history_size());
}

#[test]
fn a_program_that_asks_at_once_gets_the_themes_colors_before_any_paint() {
    // The terminal is never drawn and no palette is given afterwards: the colors come with the
    // spawn. The program asks for the background (OSC 11) and the foreground (OSC 10) in its
    // first bytes.
    let script = r#"stty raw -echo; printf '\033]11;?\007\033]10;?\007'; r=$(dd bs=1 count=48 2>/dev/null); stty sane; printf '%s' "$r" | tr '\033\007' EB; sleep 5"#;
    let mut colors = Box::new([Rgb { r: 0, g: 0, b: 0 }; COLOR_COUNT]);
    colors[256] = Rgb { r: 0x10, g: 0x20, b: 0x30 };
    colors[257] = Rgb { r: 0xfa, g: 0xfb, b: 0xfc };
    let options = StartOptions { colors: Some(&*colors), ..options() };
    let (pty, _events) = Pty::spawn_program(SIZE, &options, "/bin/sh", &["-c", script]).expect("spawn");
    wait_until("both replies", || {
        pty.screen_text().contains("E]11;rgb:fafa/fbfb/fcfcBE]10;rgb:1010/2020/3030B")
    });
}

#[test]
fn a_program_asking_whether_the_theme_is_dark_is_told_and_hears_of_a_change() {
    use wormhole_vt::ColorScheme;
    // It asks at once (996), then asks to be told of changes (2031), says so, and waits for the
    // report a change makes: `CSI ? 997 ; 1 n` for the dark theme it was started with, then
    // `CSI ? 997 ; 2 n` once the theme turns light.
    let script = r#"stty raw -echo; printf '\033[?996n'; r=$(dd bs=1 count=9 2>/dev/null); printf '\033[?2031h'; sleep 0.3; printf READY; r2=$(dd bs=1 count=9 2>/dev/null); stty sane; printf '%s%s' "$r" "$r2" | tr '\033' E; sleep 5"#;
    let options = StartOptions { color_scheme: Some(ColorScheme::Dark), ..options() };
    let (pty, _events) = Pty::spawn_program(SIZE, &options, "/bin/sh", &["-c", script]).expect("spawn");
    wait_until("the program to have asked", || pty.screen_text().contains("READY"));
    pty.set_color_scheme(ColorScheme::Light);
    wait_until("the report of the change", || pty.screen_text().contains("E[?997;1nE[?997;2n"));
}

#[test]
fn a_decoder_that_panics_refuses_the_picture_and_releases_the_replies_behind_it() {
    use crate::events::EventProxy;
    use crate::images::DECODES_MAX;
    use crate::pty::FairLock;
    use crate::screen::Screen;
    use std::sync::{Arc, Mutex};
    use wormhole_vt::Event;

    let (proxy, _events) = EventProxy::new();
    let sent = Arc::new(Mutex::new(Vec::new()));
    let sink = sent.clone();
    proxy.shared().set_writer(Box::new(move |bytes| sink.lock().expect("sink").extend(bytes)));
    // A picture that needs a decoder, then a cursor report asked after it: held behind the picture.
    let mut screen = Screen::new(SIZE, 100);
    screen.term.feed(format!("\x1b_Ga=T,f=100,i=7;{PNG}\x1b\\\x1b[6n").as_bytes());
    let handle = screen
        .term
        .events()
        .find_map(|event| match event {
            Event::ImageData(upload) => Some(upload.image),
            _ => None,
        })
        .expect("the picture reached the host");
    assert!(screen.term.replies().is_empty(), "the cursor report waits for the picture");
    let screen = FairLock::new(screen);
    let shared = proxy.shared();
    assert!(shared.images().reserve(super::images::key(handle), 1, 1));
    assert!(shared.begin_decode());

    super::images::settle_decode(&proxy, &screen, handle, || panic!("hostile data"));

    let sent = String::from_utf8_lossy(&sent.lock().expect("sink")).into_owned();
    assert!(sent.contains("i=7;ENODATA"), "the refusal: {sent:?}");
    // (The picture moved the cursor: column 2.)
    assert!(sent.ends_with("\x1b\\\x1b[1;2R"), "the cursor report after it: {sent:?}");
    assert!(shared.images().is_empty(), "no pixels are kept");
    // The decode's place is free again.
    assert!((0..DECODES_MAX).all(|_| shared.begin_decode()));
}
