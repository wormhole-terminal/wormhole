//! Strings the host asked for: they arrive whole, in order with the other events, with the place
//! in the replies a host answers at; the engine's own strings cannot be asked for.

use super::Rig;
use crate::terminal::passthrough::{MAX_BYTES, Passthrough, PassthroughError};
use crate::terminal::{Event, Terminal, Terminator};

fn rig(passthrough: Passthrough) -> Rig {
    let Ok(builder) = Terminal::builder().size(20, 5).scrollback(10).passthrough(passthrough) else {
        panic!("the claims are fine")
    };
    Rig { term: builder.build() }
}

fn osc(number: u32, payload: &str, terminator: Terminator, replies_before: usize) -> Event {
    Event::Osc { number, payload: payload.as_bytes().to_vec(), terminator, replies_before }
}

#[test]
fn a_claimed_osc_arrives_whole_with_how_it_ended() {
    let mut r = rig(Passthrough::new().osc(51234, 100));
    r.feed("\x1b]51234;hello\x07\x1b]51234\x07\x1b]51234;a;b\x1b\\\x1b]51235;other\x07");
    assert_eq!(
        r.events(),
        [
            osc(51234, "hello", Terminator::Bel, 0),
            osc(51234, "", Terminator::Bel, 0),
            osc(51234, "a;b", Terminator::St, 0),
        ],
        "51235 was not asked for"
    );
}

#[test]
fn a_string_is_whole_however_the_input_is_cut() {
    let mut r = rig(Passthrough::new().osc(51234, 100).dcs(b"!", b'z', 100).apc(b'Q', 100));
    let stream = b"\x1b]51234;hello world\x07\x1bP1;2!zdcs data\x1b\\\x1b_Qapc data\x1b\\";
    let whole = {
        let mut whole = rig(Passthrough::new().osc(51234, 100).dcs(b"!", b'z', 100).apc(b'Q', 100));
        whole.feed_bytes(stream);
        whole.events()
    };
    assert_eq!(whole.len(), 3);
    for byte in stream {
        r.feed_bytes(&[*byte]);
    }
    assert_eq!(r.events(), whole);
}

#[test]
fn a_payload_over_the_limit_is_dropped_whole_and_the_next_string_is_read() {
    let mut r = rig(Passthrough::new().osc(51234, 4).dcs(b"!", b'z', 4).apc(b'Q', 4));
    r.feed("\x1b]51234;abcd\x07\x1b]51234;abcde\x07\x1b]51234;abcdefghijklmnop\x07\x1b]51234;ok\x07");
    assert_eq!(r.events(), [osc(51234, "abcd", Terminator::Bel, 0), osc(51234, "ok", Terminator::Bel, 0)]);
    r.feed("\x1bP!zabcd\x1b\\\x1bP!zabcde\x1b\\\x1bP!zok\x1b\\");
    let payloads: Vec<Vec<u8>> = r
        .events()
        .into_iter()
        .filter_map(|event| match event {
            Event::Dcs { payload, .. } => Some(payload),
            _ => None,
        })
        .collect();
    assert_eq!(payloads, [b"abcd".to_vec(), b"ok".to_vec()]);
    // The APC limit counts the whole string, first byte included.
    r.feed("\x1b_Qabc\x1b\\\x1b_Qabcd\x1b\\");
    assert_eq!(r.events(), [Event::Apc { payload: b"Qabc".to_vec(), replies_before: 0 }]);
}

#[test]
fn a_claim_is_capped_at_a_megabyte() {
    let mut r = rig(Passthrough::new().osc(51234, usize::MAX));
    r.feed(&format!("\x1b]51234;{}\x07", "x".repeat(MAX_BYTES)));
    r.feed(&format!("\x1b]51234;{}\x07", "x".repeat(MAX_BYTES + 1)));
    assert!(matches!(&r.events()[..], [Event::Osc { payload, .. }] if payload.len() == MAX_BYTES));
}

#[test]
fn a_dcs_arrives_with_its_header_and_a_marker_is_not_matched() {
    let mut r = rig(Passthrough::new().dcs(b"!", b'z', 64).dcs(b"", b'y', 64));
    r.feed("\x1bP1;2!zdata\x1b\\\x1bPy\x1b\\\x1bP!ydata\x1b\\\x1bP?!zdata\x1b\\\x1bP$!zdata\x1b\\");
    assert_eq!(
        r.events(),
        [
            Event::Dcs {
                params: vec![1, 2],
                intermediates: b"!".to_vec(),
                final_byte: b'z',
                payload: b"data".to_vec(),
                replies_before: 0
            },
            Event::Dcs {
                params: vec![0],
                intermediates: vec![],
                final_byte: b'y',
                payload: vec![],
                replies_before: 0
            },
        ]
    );
}

#[test]
fn an_apc_arrives_with_its_first_byte_and_kitty_graphics_is_left_alone() {
    let mut r = rig(Passthrough::new().apc(b'Q', 64));
    r.feed("\x1b_Qhello\x1b\\\x1b_Rnot asked for\x1b\\");
    assert_eq!(r.events(), [Event::Apc { payload: b"Qhello".to_vec(), replies_before: 0 }]);
    r.feed("\x1b_Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA\x1b\\");
    assert!(r.replies().contains("i=31"), "the engine still answers the graphics protocol");
}

#[test]
fn nothing_is_passed_through_unless_asked_and_the_engines_own_still_work() {
    let mut r = Rig::new(20, 5);
    r.feed("\x1b]51234;x\x07\x1bP!zx\x1b\\\x1b_Qx\x1b\\ok");
    assert!(r.events().is_empty());
    assert_eq!(r.row(0), "ok");
    // The program status protocol is the engine's.
    r.feed("\x1b]7501;state=working\x1b\\");
    assert!(matches!(&r.events()[..], [Event::ProgramStatus { .. }]));
    let mut r = rig(Passthrough::new().osc(51234, 64));
    r.feed("\x1b]2;a title\x07\x1b]7501;state=working\x1b\\");
    assert_eq!(r.term.title(), Some("a title"));
    assert!(matches!(&r.events()[..], [Event::Title, Event::ProgramStatus { .. }]));
}

#[test]
fn replies_before_says_where_the_hosts_answer_goes() {
    let mut r = rig(Passthrough::new().osc(51234, 64).dcs(b"!", b'z', 64).apc(b'Q', 64));
    // A device status report is answered before each sequence the host will answer, and one after.
    r.feed("\x1b[5n\x1b]51234;a\x07\x1b[5n\x1bP!zb\x1b\\\x1b[5n\x1b_Qc\x1b\\\x1b[5n");
    let at: Vec<usize> = r
        .events()
        .into_iter()
        .map(|event| match event {
            Event::Osc { replies_before, .. }
            | Event::Dcs { replies_before, .. }
            | Event::Apc { replies_before, .. } => replies_before,
            other => panic!("{other:?}"),
        })
        .collect();
    let dsr = "\x1b[0n".len();
    assert_eq!(at, [dsr, 2 * dsr, 3 * dsr]);
    assert_eq!(r.term.replies().len(), 4 * dsr);
    // A host that answers the first at that place gets the program's answers in the order it asked.
    let mut replies = r.term.take_replies();
    let ack = b"\x1b]51234;ack\x07";
    replies.splice(at[0]..at[0], ack.iter().copied());
    assert_eq!(String::from_utf8_lossy(&replies[..dsr + ack.len()]), "\x1b[0n\x1b]51234;ack\x07");
}

#[test]
fn passed_through_strings_keep_the_order_of_the_other_events() {
    let mut r = rig(Passthrough::new().osc(51234, 64));
    r.feed("\x07\x1b]51234;one\x07\x07\x1b]2;title\x07\x1b]51234;two\x07");
    let kinds: Vec<String> = r
        .events()
        .into_iter()
        .map(|event| match event {
            Event::Osc { payload, .. } => String::from_utf8_lossy(&payload).into_owned(),
            other => format!("{other:?}"),
        })
        .collect();
    assert_eq!(kinds, ["Bell", "one", "Bell", "Title", "two"]);
}

#[test]
fn events_nobody_reads_are_capped_like_the_engines_own() {
    let mut r = rig(Passthrough::new().osc(51234, 16));
    for _ in 0..20_000 {
        r.feed("\x1b]51234;x\x07");
    }
    assert_eq!(r.events().len(), 16_384);
}

#[test]
fn the_engines_own_strings_cannot_be_asked_for() {
    for number in [0, 1, 2, 4, 5, 7, 8, 9, 10, 11, 19, 52, 99, 104, 105, 110, 119, 133, 777, 1337, 7501] {
        assert_eq!(
            Terminal::builder().passthrough(Passthrough::new().osc(number, 10)).err(),
            Some(PassthroughError::ReservedOsc(number)),
            "OSC {number}"
        );
    }
    for (claim, name) in [
        (Passthrough::new().dcs(b"", b'q', 10), "Sixel"),
        (Passthrough::new().dcs(b"$", b'q', 10), "DECRQSS"),
        (Passthrough::new().dcs(b"+", b'q', 10), "XTGETTCAP"),
        (Passthrough::new().dcs(b"!", b'u', 10), "DECAUPSS"),
        (Passthrough::new().dcs(b"$", b't', 10), "DECRSPS"),
        (Passthrough::new().dcs(b"$", b'p', 10), "DECRSTS"),
        (Passthrough::new().apc(b'G', 10), "kitty"),
    ] {
        let error = Terminal::builder().passthrough(claim).err();
        assert!(
            matches!(error, Some(PassthroughError::Reserved(n)) if n.contains(name)),
            "{name}: {error:?}"
        );
    }
}

#[test]
fn claims_that_match_nothing_are_refused() {
    let refused = |claim: Passthrough| Terminal::builder().passthrough(claim).err();
    assert_eq!(refused(Passthrough::new().osc(100_000, 1)), Some(PassthroughError::OscNumber(100_000)));
    assert_eq!(refused(Passthrough::new().dcs(b"A", b'z', 1)), Some(PassthroughError::Dcs));
    assert_eq!(refused(Passthrough::new().dcs(b"", b'\x1b', 1)), Some(PassthroughError::Dcs));
    assert_eq!(refused(Passthrough::new().dcs(b"!!!!!", b'z', 1)), Some(PassthroughError::Dcs));
    assert_eq!(refused(Passthrough::new().apc(0x1b, 1)), Some(PassthroughError::Apc(0x1b)));
    // The first mistake is the one reported, and a good claim beside it does not save the rest.
    assert_eq!(
        refused(Passthrough::new().osc(1, 1).osc(51234, 1).osc(100_000, 1)),
        Some(PassthroughError::ReservedOsc(1))
    );
    let mut many = Passthrough::new();
    for number in 20_000..20_300 {
        many = many.osc(number, 1);
    }
    assert_eq!(refused(many), Some(PassthroughError::TooMany));
    assert!(Passthrough::new().is_empty());
    let message = PassthroughError::Reserved("Sixel (DCS q)").to_string();
    assert!(message.contains("Sixel"), "{message}");
}

#[test]
fn osc_dcs_and_apc_claims_share_the_limit_of_256() {
    let refused = |claim: Passthrough| Terminal::builder().passthrough(claim).err();
    let mut claims = Passthrough::new();
    for number in 20_000..20_100 {
        claims = claims.osc(number, 1);
    }
    for intermediate in 0x20u8..=0x2F {
        for final_byte in *b"rsvwxyz" {
            claims = claims.dcs(&[intermediate], final_byte, 1);
        }
    }
    for first in *b"abcdefghijklmnopqrstuvwxyzABCDEF0123456789!\"" {
        claims = claims.apc(first, 1);
    }
    assert_eq!(refused(claims.clone()), None, "100 and 112 and 44 are the 256 allowed");
    assert_eq!(refused(claims.osc(20_100, 1)), Some(PassthroughError::TooMany), "and one more is too many");
}

#[test]
fn dcs_claims_that_share_a_final_byte_or_the_intermediates_are_still_separate_claims() {
    let mut r = rig(Passthrough::new().dcs(b"", b'z', 64).dcs(b"!", b'z', 64).dcs(b"!", b'y', 64));
    r.feed("\x1bPzone\x1b\\\x1bP!ztwo\x1b\\\x1bP!ythree\x1b\\");
    let payloads: Vec<Vec<u8>> = r
        .events()
        .into_iter()
        .filter_map(|event| match event {
            Event::Dcs { payload, .. } => Some(payload),
            _ => None,
        })
        .collect();
    assert_eq!(payloads, [b"one".to_vec(), b"two".to_vec(), b"three".to_vec()]);
}

#[test]
fn the_choice_can_change_while_the_terminal_runs() {
    let mut r = Rig::new(20, 5);
    r.feed("\x1b]51234;before\x07");
    assert!(r.events().is_empty());
    assert!(r.term.set_passthrough(Passthrough::new().osc(51234, 64)).is_ok());
    r.feed("\x1b]51234;now\x07");
    assert_eq!(r.events(), [osc(51234, "now", Terminator::Bel, 0)]);
    // A refused choice changes nothing.
    assert_eq!(r.term.set_passthrough(Passthrough::new().osc(2, 64)), Err(PassthroughError::ReservedOsc(2)));
    r.feed("\x1b]51234;still\x07");
    assert_eq!(r.events().len(), 1);
    assert!(r.term.set_passthrough(Passthrough::new()).is_ok());
    r.feed("\x1b]51234;gone\x07");
    assert!(r.events().is_empty());
}
