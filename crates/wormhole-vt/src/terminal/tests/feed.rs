//! Input cut between two calls of `feed`: a sequence or a character split anywhere reads the same.

use super::Rig;

#[test]
fn with_8_bit_controls_a_title_keeps_its_utf8_and_still_ends_at_st() {
    // S8C1T, then a title whose characters have bytes of 0x80..=0x9F inside them (— is E2 80 94,
    // œ is C5 9C), ended by the one-byte ST, then text: cut anywhere, it reads the same.
    let mut bytes = b"\x1b G\x1b]2;a \xe2\x80\x94 \xc5\x93 \xe2\x80\x9c\x9c".to_vec();
    bytes.extend_from_slice(b"X");
    for cut in 0..=bytes.len() {
        let mut rig = Rig::new(20, 3);
        rig.feed_bytes(&bytes[..cut]).feed_bytes(&bytes[cut..]);
        assert_eq!(rig.term.title(), Some("a \u{2014} \u{153} \u{201c}"), "cut at {cut}");
        assert_eq!(rig.row(0), "X", "cut at {cut}");
    }
}

#[test]
fn a_sequence_cut_between_two_feeds_is_joined_by_the_terminals_own_parser() {
    let mut rig = Rig::new(20, 3);
    rig.feed("ab\x1b[2").feed("C\x1b]2;ti").feed("tle\x07");
    assert_eq!(rig.cursor(), (0, 4), "CSI 2 C moved two columns on");
    assert_eq!(rig.term.title(), Some("title"));
}
