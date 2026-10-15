//! CSI sequences: parameters, sub-parameters, markers, intermediates, overflow and aborts.

use super::{Event, check, run, show};
use crate::parser::{Csi, MAX_PARAMS};

/// The one CSI in `input`.
#[track_caller]
fn csi(input: &[u8]) -> Csi {
    match run(input).as_slice() {
        [Event::Csi(csi)] => csi.clone(),
        other => panic!("expected one CSI for {:?}, got {other:?}", String::from_utf8_lossy(input)),
    }
}

#[test]
fn plain_sequences() {
    check(b"\x1b[H", &["CSI H"]);
    check(b"\x1b[2J", &["CSI 2 J"]);
    check(b"\x1b[1;2H", &["CSI 1;2 H"]);
    check(b"\x1b[38;2;255;128;0m", &["CSI 38;2;255;128;0 m"]);
    check(b"a\x1b[1mb\x1b[0mc", &["\"a\"", "CSI 1 m", "\"b\"", "CSI 0 m", "\"c\""]);
}

#[test]
fn every_final_byte() {
    for f in 0x40u8..=0x7E {
        let c = csi(&[0x1B, b'[', b'1', f]);
        assert_eq!(c.final_byte(), f);
        assert_eq!(c.values(), [1]);
    }
}

#[test]
fn empty_parameters_are_defaults() {
    let c = csi(b"\x1b[m");
    assert_eq!((c.len(), c.get(0), c.param(0, 7), c.values()), (1, None, 7, &[0][..]));
    let c = csi(b"\x1b[;5H");
    assert_eq!((c.len(), c.get(0), c.get(1)), (2, None, Some(5)));
    let c = csi(b"\x1b[1;H");
    assert_eq!((c.len(), c.get(0), c.get(1)), (2, Some(1), None));
    let c = csi(b"\x1b[;;m");
    assert_eq!((c.len(), c.values()), (3, &[0, 0, 0][..]));
    check(b"\x1b[;5H", &["CSI ;5 H"]);
}

#[test]
fn a_reported_sequence_has_at_least_one_parameter() {
    let many = format!("\x1b[{}m", "1;".repeat(MAX_PARAMS + 5));
    for input in [&b"\x1b[m"[..], b"\x1b[;m", b"\x1b[:m", b"\x1b[?h", b"\x1b[1:2:3m", many.as_bytes()] {
        let c = csi(input);
        assert_ne!(c.len(), 0, "{:?}", String::from_utf8_lossy(input));
        assert!(!c.is_empty());
    }
}

#[test]
fn param_accessors() {
    let c = csi(b"\x1b[0;;7H");
    assert_eq!(c.param(0, 1), 0, "an explicit 0 stays 0");
    assert_eq!(c.param_nonzero(0, 1), 1, "0 is the default for a movement");
    assert_eq!(c.param(1, 9), 9);
    assert_eq!(c.param_nonzero(1, 9), 9);
    assert_eq!(c.param(2, 9), 7);
    assert_eq!(c.param(3, 9), 9, "absent");
    assert_eq!(c.get(3), None);
    assert_eq!(c.param(usize::MAX, 4), 4);
}

#[test]
fn private_markers() {
    for m in b"<=>?" {
        let c = csi(&[0x1B, b'[', *m, b'1', b'h']);
        assert_eq!(c.marker(), Some(*m));
        assert_eq!(c.values(), [1]);
    }
    assert_eq!(csi(b"\x1b[1h").marker(), None);
    check(b"\x1b[?1049h", &["CSI ?1049 h"]);
    check(b"\x1b[>c", &["CSI > c"]);
    check(b"\x1b[?25;2026$p", &["CSI ?25;2026 $ p"]);
}

#[test]
fn a_marker_after_the_start_makes_the_sequence_ignored() {
    check(b"\x1b[1?hX", &["\"X\""]);
    check(b"\x1b[;?hX", &["\"X\""]);
    check(b"\x1b[??hX", &["\"X\""]);
    check(b"\x1b[?1;>hX", &["\"X\""]);
}

#[test]
fn intermediates() {
    let c = csi(b"\x1b[2 q");
    assert_eq!((c.intermediates(), c.values(), c.final_byte()), (&b" "[..], &[2][..], b'q'));
    check(b"\x1b[ q", &["CSI SP q"]);
    let c = csi(b"\x1b[!p");
    assert_eq!((c.intermediates(), c.len(), c.get(0)), (&b"!"[..], 1, None));
    let c = csi(b"\x1b[1$ x");
    assert_eq!(c.intermediates(), b"$ ");
    assert!(!c.overflow());
}

#[test]
fn too_many_intermediates_overflow() {
    let c = csi(b"\x1b[1 !\"#x");
    assert!(!c.overflow());
    assert_eq!(c.intermediates(), b" !\"#");
    let c = csi(b"\x1b[1 !\"#$x");
    assert!(c.overflow());
    assert_eq!(c.intermediates(), b" !\"#");
}

#[test]
fn parameter_bytes_after_an_intermediate_make_the_sequence_ignored() {
    check(b"\x1b[ 1qX", &["\"X\""]);
    check(b"\x1b[ ;qX", &["\"X\""]);
    check(b"\x1b[ ?qX", &["\"X\""]);
}

#[test]
fn sub_parameters() {
    let c = csi(b"\x1b[38:2::255:128:0;1m");
    assert_eq!(c.values(), [38, 2, 0, 255, 128, 0, 1]);
    assert!(c.has_subparams());
    assert_eq!(c.len(), 2);
    let groups: Vec<&[u16]> = c.groups().collect();
    assert_eq!(groups, [&[38, 2, 0, 255, 128, 0][..], &[1][..]]);
    assert_eq!(c.param(0, 9), 38);
    assert_eq!(c.param(1, 9), 1, "parameters are addressed by group");
    assert_eq!(c.get(2), None);
    check(b"\x1b[38:2::255:128:0;1m", &["CSI 38:2::255:128:0;1 m"]);

    let c = csi(b"\x1b[4:3m");
    assert_eq!(c.groups().collect::<Vec<_>>(), [&[4, 3][..]]);
    // A colon can start a sequence, or follow a semicolon.
    let c = csi(b"\x1b[:5m");
    assert_eq!(c.groups().collect::<Vec<_>>(), [&[0, 5][..]]);
    let c = csi(b"\x1b[1;:2;3:4:5m");
    assert_eq!(c.groups().collect::<Vec<_>>(), [&[1][..], &[0, 2][..], &[3, 4, 5][..]]);
    assert_eq!(c.param(2, 0), 3);
    assert_eq!(c.len(), 3);
    // Without colons, groups are single values.
    let c = csi(b"\x1b[1;2;3m");
    assert!(!c.has_subparams());
    assert_eq!(c.groups().collect::<Vec<_>>(), [&[1][..], &[2][..], &[3][..]]);
}

#[test]
fn values_saturate_like_xterm() {
    assert_eq!(csi(b"\x1b[65535H").values(), [65535]);
    assert_eq!(csi(b"\x1b[65536H").values(), [65535]);
    assert_eq!(csi(b"\x1b[99999999999999999999999999H").values(), [65535]);
    assert_eq!(csi(b"\x1b[0000000000000000000012H").values(), [12]);
}

#[test]
fn thirty_two_values_fit_and_more_overflow() {
    let params = |n: usize| (1..=n).map(|i| i.to_string()).collect::<Vec<_>>().join(";");
    let input = |n: usize| format!("\x1b[{}m", params(n)).into_bytes();
    let c = csi(&input(MAX_PARAMS));
    assert!(!c.overflow());
    assert_eq!(c.values().len(), MAX_PARAMS);
    assert_eq!(c.values()[MAX_PARAMS - 1], MAX_PARAMS as u16);
    let c = csi(&input(MAX_PARAMS + 1));
    assert!(c.overflow());
    assert_eq!(c.values().len(), MAX_PARAMS);
    assert_eq!(c.values()[MAX_PARAMS - 1], MAX_PARAMS as u16, "the first 32 are kept");
    let c = csi(&input(500));
    assert!(c.overflow());
    // Sub-parameters count against the same limit.
    let sub = format!("\x1b[{}m", vec!["1"; 40].join(":")).into_bytes();
    let c = csi(&sub);
    assert!(c.overflow());
    assert_eq!(c.values().len(), MAX_PARAMS);
    assert_eq!(c.groups().count(), 1);
    // The next sequence starts clean.
    let events = run(&[sub.as_slice(), b"\x1b[1m"].concat());
    let Event::Csi(second) = &events[1] else { panic!("{events:?}") };
    assert!(!second.overflow());
    assert_eq!(second.values(), [1]);
}

#[test]
fn a_control_in_a_sequence_runs_and_the_sequence_goes_on() {
    check(b"\x1b[1\n2H", &["exec 0a", "CSI 12 H"]);
    check(b"\x1b[\r1;\x082m", &["exec 0d", "exec 08", "CSI 1;2 m"]);
    check(b"\x1b[ \x07q", &["exec 07", "CSI SP q"]);
    check(b"\x1b[1;\x7f2m", &["CSI 1;2 m"]);
}

#[test]
fn escape_starts_over() {
    check(b"\x1b[1;2\x1b[3m", &["CSI 3 m"]);
    check(b"\x1b[1;2\x1b7", &["ESC 7"]);
    check(b"\x1b[?1\x1b[H", &["CSI H"]);
    check(b"\x1b[ \x1b[H", &["CSI H"]);
    // The intermediates and marker of the aborted sequence do not leak.
    let c = csi(b"\x1b[?1 \x1b[m");
    assert_eq!((c.marker(), c.intermediates().len()), (None, 0));
}

#[test]
fn can_and_sub_cancel() {
    check(b"\x1b[1;2\x18X", &["exec 18", "\"X\""]);
    check(b"\x1b[1;2\x1aX", &["exec 1a", "\"X\""]);
    check(b"\x1b[ \x18X", &["exec 18", "\"X\""]);
    check(b"\x1b[?1 \x1b[;\x1aX", &["exec 1a", "\"X\""]);
}

#[test]
fn high_bytes_in_a_sequence_are_ignored() {
    check(b"\x1b[1\xc3\xa92m", &["CSI 12 m"]);
    check(b"\x1b[1\xff2m", &["CSI 12 m"]);
}

#[test]
fn ignored_sequences_end_at_a_final_byte() {
    check(b"\x1b[1?2;3 4mX", &["\"X\""]);
    // Controls still run inside one, ESC and CAN end it.
    check(b"\x1b[1?\n2;3mX", &["exec 0a", "\"X\""]);
    check(b"\x1b[1?2\x1b[mX", &["CSI m", "\"X\""]);
    check(b"\x1b[1?2\x18X", &["exec 18", "\"X\""]);
}

#[test]
fn csi_is_open_until_the_final_byte() {
    use crate::parser::Parser;
    let mut parser = Parser::new();
    let mut rec = super::Recorder::default();
    parser.advance(&mut rec, b"\x1b[1;2");
    assert!(!parser.is_ground());
    parser.advance(&mut rec, b"H");
    assert!(parser.is_ground());
    assert_eq!(show(&rec.finish()), ["CSI 1;2 H"]);
}

#[test]
fn sgr_churn() {
    let input = b"\x1b[0;1;38;2;10;20;30;48:2::1:2:3;4:3;58:5:99m";
    let c = csi(input);
    assert_eq!(c.groups().count(), 10);
    assert_eq!(c.param(9, 0), 58);
}
