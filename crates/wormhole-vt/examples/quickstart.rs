//! The README's quick start as a program: feed a terminal some output, then read what it did.
//!
//! ```sh
//! cargo run -p wormhole-vt --example quickstart
//! ```
//!
//! A terminal is a state machine: `feed` takes the bytes a program printed, in any chunking, and
//! afterwards the host reads the screen (`cell`, `row_text`), collects the bytes the program is
//! waiting for (`take_replies`) and takes what the program asked of it (`events`). Nothing happens
//! between calls: there is no thread, no timer and no I/O.
#![allow(clippy::print_stdout)] // an example: what it prints is what it shows

use wormhole_vt::{Color, Column, Event, Flags, Line, NamedColor, Point, Terminal};

pub fn main() {
    let mut term = Terminal::new(80, 24);

    // Bytes from the program, in any chunking: a sequence may be cut anywhere.
    term.feed(b"\x1b[1;31mhello\x1b[0m\r\n");
    term.feed(b"\x1b]2;my ti"); // a title, cut in two ...
    term.feed(b"tle\x07"); // ... and finished
    term.feed(b"\x1b[6n"); // the program asks where the cursor is

    // What the program is waiting to read: write these bytes back to the PTY.
    let replies = term.take_replies();
    println!("write to the program: {:?}", String::from_utf8_lossy(&replies));
    assert_eq!(replies, b"\x1b[2;1R");

    // What happened besides drawing.
    for event in term.events() {
        println!("event: {event:?}");
        assert!(matches!(event, Event::Title), "only the title changed");
    }
    println!("title: {:?}", term.title());
    assert_eq!(term.title(), Some("my title"));

    // What is on screen.
    let cell = term.cell(Point::new(Line(0), Column(0))).expect("inside the screen");
    println!("first cell: {:?} {:?} {:?}", cell.c, cell.fg, cell.flags);
    assert_eq!(cell.c, 'h');
    assert!(cell.flags.contains(Flags::BOLD));
    assert_eq!(cell.fg, Color::Named(NamedColor::Red));
    println!("first row: {:?}", term.row_text(Line(0), true));
    assert_eq!(term.row_text(Line(0), true), "hello");

    // Everything in one place when you need more than the defaults.
    let big = Terminal::builder().size(120, 40).scrollback(10_000).build();
    println!("a bigger one: {} x {}", big.cols(), big.rows());
    assert_eq!((big.cols(), big.rows()), (120, 40));
}
