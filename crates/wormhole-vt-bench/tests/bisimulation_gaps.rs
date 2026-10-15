//! What the bisimulation cannot see would be a blind spot, so each gap a repaint reports has a
//! case here that shows the state it leaves out *is* different in the replica (the comparison
//! without the mask fails), that the repaint says so (the gap is reported), and that everything
//! else is still right (the comparison with the mask passes). The harness is also shown to catch
//! a repaint that is wrong without saying so.

use wormhole_vt::term::repaint::Gaps;
use wormhole_vt_bench::bisim::{Resize, Scenario, Setup, probe};

fn scenario(input: &[u8], cols: usize, rows: usize, resizes: &[(usize, usize, usize)]) -> Scenario {
    Scenario {
        name: "gap".into(),
        setup: Setup { cols, rows, scrollback: 100, allow_checksum: false, allow_window_ops: false },
        bytes: input.to_vec(),
        resizes: resizes.iter().map(|&(at, cols, rows)| Resize { at, cols, rows }).collect(),
    }
}

/// The repaint at the end of `s` reports `gap`; the field `field` differs without the mask and
/// nothing differs with it.
fn gap_is_visible(s: &Scenario, gap: Gaps, field: &str) {
    let seen = probe(s, s.bytes.len(), |_| {});
    assert!(seen.gaps.contains(gap), "expected {gap:?}, the repaint reported {:?}", seen.gaps);
    let unmasked = seen.unmasked.as_deref().unwrap_or("");
    assert!(unmasked.contains(field), "without the mask {field} should differ:\n{unmasked}");
    assert!(
        seen.masked.is_none(),
        "with the mask nothing else may differ:\n{}",
        seen.masked.unwrap_or_default()
    );
}

#[test]
fn a_repaint_that_is_wrong_is_caught() {
    let s = scenario(b"hello\r\nworld", 20, 4, &[]);
    let right = probe(&s, s.bytes.len(), |_| {});
    assert!(right.gaps.is_empty() && right.masked.is_none() && right.unmasked.is_none());
    // A character that is not there, put after the leading RIS.
    let wrong = probe(&s, s.bytes.len(), |bytes| bytes.splice(2..2, *b"Z").for_each(drop));
    assert!(wrong.masked.is_some(), "a stray character must show");
    // A mode the sender has not set.
    let wrong = probe(&s, s.bytes.len(), |bytes| bytes.extend_from_slice(b"\x1b[?2004h"));
    assert!(wrong.masked.as_deref().is_some_and(|d| d.contains("a/modes")), "{:?}", wrong.masked);
    // A wrap that is not there.
    let wrong = probe(&s, s.bytes.len(), |bytes| bytes.extend_from_slice(b"\x1b[1;1H\x1b[4h"));
    assert!(wrong.masked.is_some());
}

#[test]
fn pictures_are_not_repainted() {
    let image = b"\x1b_Ga=T,f=32,s=1,v=1;AAAAAA==\x1b\\x";
    let s = scenario(image, 20, 4, &[]);
    let seen = probe(&s, s.bytes.len(), |_| {});
    assert!(seen.gaps.contains(Gaps::PLACEMENTS | Gaps::IMAGES), "{:?}", seen.gaps);
    let unmasked = seen.unmasked.unwrap_or_default();
    assert!(unmasked.contains("placement["), "{unmasked}");
    assert!(unmasked.contains("a/graphics/tables"), "{unmasked}");
    assert!(seen.masked.is_none(), "{:?}", seen.masked);
}

#[test]
fn insert_mode_keeps_a_cluster_from_being_reopened() {
    gap_is_visible(&scenario(b"\x1b[4hX", 10, 3, &[]), Gaps::CLUSTER, "a/cluster");
}

#[test]
fn a_screen_filled_by_decaln_has_no_last_character() {
    gap_is_visible(&scenario(b"\x1b#8", 6, 3, &[]), Gaps::LAST_CHAR, "a/last_char");
}

#[test]
fn a_last_character_with_no_free_cell_and_none_on_screen() {
    // DECFRA fills the screen with '%' after '@' was the last character printed.
    gap_is_visible(&scenario(b"QRSTUVWX\r\nYZ6789!@\x1b[37$x", 8, 2, &[]), Gaps::TOUCH, "a/last_char");
}

#[test]
fn resizing_on_the_alternate_screen_leaves_it_a_history() {
    let input = b"main\x1b[?1049h1\r\n2\r\n3";
    let s = scenario(input, 10, 4, &[(input.len(), 8, 2)]);
    let seen = probe(&s, input.len(), |_| {});
    assert!(seen.gaps.contains(Gaps::ALT_HISTORY | Gaps::LIMIT), "{:?}", seen.gaps);
    let unmasked = seen.unmasked.unwrap_or_default();
    assert!(unmasked.contains("alternate/limit"), "{unmasked}");
    assert!(seen.masked.is_none(), "{:?}", seen.masked);
}

#[test]
fn a_conformance_level_below_vt300_has_no_cursor_information() {
    gap_is_visible(&scenario(b"\x1b[61;1\"p\x0e", 10, 3, &[]), Gaps::LOW_LEVEL, "a/charsets");
}

#[test]
fn a_wrapped_last_row_cannot_be_made() {
    let s = scenario(b"abc\x1b#8", 11, 4, &[(6, 5, 1)]);
    let seen = probe(&s, s.bytes.len(), |_| {});
    // A one-row screen whose row wraps into a row that is not there.
    assert!(seen.gaps.contains(Gaps::WRAP), "{:?}", seen.gaps);
    assert!(seen.masked.is_none(), "{:?}", seen.masked);
}

#[test]
fn ed3_after_which_the_history_is_not_empty() {
    let input = b"1\r\n2\r\n3\r\n4\r\n5\x1b[3J";
    let s = scenario(input, 17, 5, &[(input.len(), 15, 3)]);
    gap_is_visible(&s, Gaps::HISTORY_CLEARED, "a/history_just_cleared");
}
