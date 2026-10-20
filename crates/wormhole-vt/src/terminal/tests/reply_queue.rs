//! The queue of replies owed to the program (`reports::queue`): a reply is kept or dropped whole,
//! the cap holds also for replies held behind a picture's answer, and the one-byte controls of
//! S8C1T reach those too. The module's own tests cover the queue alone; these go through the
//! sequences that fill it.

use super::Rig;
use crate::terminal::{Event, Limits, Terminal};

/// A 1 by 1 PNG, base64: its answer waits for the host to decode it.
const PNG: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==";

fn with_cap(cap: usize) -> Rig {
    let limits = Limits { replies: cap, ..Limits::default() };
    Rig { term: Terminal::builder().size(80, 24).scrollback(10).limits(limits).build() }
}

/// Whether `replies` leave a terminal that reads them as program output in the ground state: a
/// string left open would swallow the text that follows it.
fn reads_back_to_ground(replies: &[u8]) -> bool {
    let mut term = Terminal::new(80, 24);
    term.feed(replies);
    term.feed(b"Zq");
    term.row_text(crate::types::Line(0), true).ends_with("Zq")
}

/// The handles of the pictures waiting for the host's decode.
fn waiting(rig: &mut Rig) -> Vec<crate::types::ImageHandle> {
    rig.events()
        .into_iter()
        .filter_map(|event| match event {
            Event::ImageData(upload) => Some(upload.image),
            _ => None,
        })
        .collect()
}

#[test]
fn a_reply_that_starts_under_the_cap_and_ends_over_it_is_not_cut() {
    // The audit's stream: the queue one reply short of 1 MiB, then a DECRQSS answer in pieces. The
    // piece that ended it (`ESC \`) started past the cap and was dropped alone, leaving a DCS
    // that the program never saw end.
    let mut term = Terminal::new(80, 24);
    while term.replies().len() < (1 << 20) - 4 {
        term.feed(b"\x1b[5n");
    }
    term.feed(b"\x1b[0;38;2;1;2;3m\x1bP$qm\x1b\\");
    let replies = term.take_replies();
    assert!(replies.len() <= 1 << 20, "{} bytes", replies.len());
    assert!(replies.ends_with(b"\x1b[0n"), "the answer that did not fit is not there, in part or whole");
    assert!(reads_back_to_ground(&replies));
}

#[test]
fn every_reply_is_whole_whatever_the_cap_cuts_through() {
    // Caps of every size around a few replies: DA1, a state report, a DECRQSS and a title-less
    // XTVERSION, each written in pieces, meet a queue with room for part of them.
    let asks =
        "\x1b[c\x1b[1$u\x1bP$qm\x1b\\\x1bP$q\"p\x1b\\\x1b[>q\x1b[?1$p\x1b[6n\x1b]4;1;?\x07\x1b[1$w\x1b[2$w";
    for cap in 20..140 {
        let mut rig = with_cap(cap);
        for _ in 0..20 {
            rig.feed(asks);
            let replies = rig.term.take_replies();
            assert!(reads_back_to_ground(&replies), "cap {cap}: {:?}", String::from_utf8_lossy(&replies));
        }
        // Without collecting: the queue holds what fitted and is whole.
        let mut rig = with_cap(cap);
        for _ in 0..20 {
            rig.feed(asks);
        }
        let held = rig.term.replies().to_vec();
        // What fitted, or the one reply bigger than the cap on its own.
        assert!(held.len() < cap + 256, "cap {cap}: {} bytes", held.len());
        assert!(reads_back_to_ground(&held), "cap {cap}: {:?}", String::from_utf8_lossy(&held));
    }
}

#[test]
fn an_answer_bigger_than_the_cap_still_reaches_an_empty_queue() {
    // OSC 52 read answers are as big as the clipboard; the host asks the terminal for one at a time.
    let mut term = Terminal::new(80, 24);
    let text = "x".repeat(2 << 20);
    term.clipboard_reply("c", &text, crate::terminal::Terminator::Bel);
    assert!(term.replies().len() > 2 << 20);
    term.feed(b"\x1b[5n");
    assert!(!term.replies().ends_with(b"\x1b[0n"), "and nothing joins it while it waits");
    term.clear_replies();
    term.feed(b"\x1b[5n");
    assert_eq!(term.replies(), b"\x1b[0n");
}

#[test]
fn replies_held_behind_a_picture_are_one_byte_controls_after_s8c1t() {
    let mut term = Terminal::new(80, 24);
    term.feed(b"\x1b G"); // S8C1T
    term.feed(b"\x1b[6n");
    assert!(term.take_replies().starts_with(b"\x9b"), "the direct reply is one byte");
    let mut rig = Rig { term };
    rig.feed(&format!("\x1b_Ga=T,f=100,i=1;{PNG}\x1b\\\x1b[6n"));
    let handles = waiting(&mut rig);
    assert_eq!(handles.len(), 1);
    assert!(rig.term.replies().is_empty(), "held behind the picture's answer");
    for handle in handles {
        rig.term.image_decoded(handle, Ok(()));
    }
    let released = rig.term.take_replies();
    assert!(!released.contains(&0x1b), "no 7-bit introducer left: {released:?}");
    assert!(released.starts_with(b"\x9fGi=1;OK\x9c\x9b"), "the answer, then the report: {released:?}");
}

#[test]
fn a_picture_answer_is_in_the_form_of_the_command_not_of_the_decode() {
    let mut rig = Rig::new(80, 24);
    rig.feed(&format!("\x1b_Ga=T,f=100,i=1;{PNG}\x1b\\"));
    let handles = waiting(&mut rig);
    rig.feed("\x1b G"); // S8C1T, after the command
    for handle in handles {
        rig.term.image_decoded(handle, Ok(()));
    }
    assert!(rig.term.replies().starts_with(b"\x1b_Gi=1;OK\x1b\\"), "{:?}", rig.term.replies());
}

#[test]
fn replies_released_after_a_decode_stay_under_the_cap() {
    // The audit's stream: a picture waiting, thousands of queries held behind it, the host never
    // collecting. Released into the queue they went past the cap each round without limit.
    let cap = 64 << 10;
    let mut rig = with_cap(cap);
    for round in 0..10 {
        rig.feed(&format!("\x1b_Ga=T,f=100,i=1;{PNG}\x1b\\"));
        let handles = waiting(&mut rig);
        for _ in 0..40 {
            rig.feed(&"\x1b[c".repeat(500));
        }
        for handle in handles {
            rig.term.image_decoded(handle, Ok(()));
        }
        let queued = rig.term.replies().len();
        // The answers to the pictures are kept whatever the cap says: a few dozen bytes a round.
        assert!(queued <= cap + (round + 1) * 64, "round {round}: {queued} bytes of {cap}");
    }
    let replies = rig.term.take_replies();
    assert!(reads_back_to_ground(&replies));
}

#[test]
fn replies_held_behind_a_picture_are_held_to_the_cap() {
    let cap = 4096;
    let mut rig = with_cap(cap);
    rig.feed(&format!("\x1b_Ga=T,f=100,i=1;{PNG}\x1b\\"));
    let handles = waiting(&mut rig);
    rig.feed(&"\x1b[c".repeat(5_000));
    for handle in handles {
        rig.term.image_decoded(handle, Err(crate::terminal::ImageError::BadData));
    }
    let released = rig.term.take_replies();
    assert!(released.starts_with(b"\x1b_Gi=1;ENODATA"), "{:?}", &released[..30.min(released.len())]);
    assert!(released.len() <= cap + 128, "{} bytes", released.len());
    assert!(reads_back_to_ground(&released));
}
