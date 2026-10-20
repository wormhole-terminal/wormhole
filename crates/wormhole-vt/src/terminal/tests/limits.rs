//! The host lowering the caps: `Limits`. The pictures' caps are tested with the pictures
//! (`graphics/tests/limits.rs`).

use super::Rig;
use crate::terminal::{Event, Limits, Terminal};
use crate::types::Line;

fn rig_with(limits: Limits) -> Rig {
    Rig { term: Terminal::builder().size(40, 5).scrollback(100).limits(limits).build() }
}

fn lowered(change: impl FnOnce(&mut Limits)) -> Limits {
    let mut limits = Limits::default();
    change(&mut limits);
    limits
}

#[test]
fn the_default_is_the_ceiling_and_a_value_above_it_is_cut() {
    let ceiling = Limits::default();
    assert_eq!(Terminal::new(80, 24).limits(), ceiling);
    let high = lowered(|l| {
        l.image_bytes = u64::MAX;
        l.image_count = usize::MAX;
        l.image_uploads = usize::MAX;
        l.image_payload = usize::MAX;
        l.sixel = usize::MAX;
        l.osc = usize::MAX;
        l.clipboard = usize::MAX;
        l.osc_image = usize::MAX;
        l.osc_iterm = usize::MAX;
        l.title = usize::MAX;
        l.cwd = usize::MAX;
        l.link_uri = usize::MAX;
        l.events = usize::MAX;
        l.replies = usize::MAX;
    });
    assert_eq!(high.clamped(), ceiling);
    assert_eq!(Terminal::builder().limits(high).build().limits(), ceiling);
    let mut term = Terminal::new(80, 24);
    term.set_limits(high);
    assert_eq!(term.limits(), ceiling);
    // A lower value stays, next to one that was too high.
    let mixed = lowered(|l| {
        l.title = 10;
        l.clipboard = usize::MAX;
    });
    term.set_limits(mixed);
    assert_eq!((term.limits().title, term.limits().clipboard), (10, ceiling.clipboard));
}

#[test]
fn the_ceilings_are_what_the_documentation_says() {
    let c = Limits::default();
    assert_eq!((c.image_bytes, c.image_count), (128 << 20, 1024));
    assert_eq!((c.image_uploads, c.image_payload, c.sixel), (256 << 20, 32 << 20, 4 << 20));
    assert_eq!((c.osc, c.clipboard, c.osc_image, c.osc_iterm), (8 << 10, 8 << 20, 64 << 20, 64 << 10));
    assert_eq!((c.title, c.cwd, c.link_uri), (4096, 4096, 8192));
    assert_eq!((c.events, c.replies), (16_384, 1 << 20));
}

#[test]
fn a_title_and_a_folder_are_cut_at_the_hosts_length() {
    let mut r = rig_with(lowered(|l| {
        l.title = 10;
        l.cwd = 12;
    }));
    // 'é' is two bytes: a cut in the middle of one backs up.
    r.feed("\x1b]0;éééééééééé\x07\x1b]1;éééééééééé\x07\x1b]7;file://héééé/é\x07");
    assert_eq!(r.term.title(), Some("ééééé"));
    assert_eq!(r.term.icon_name(), Some("ééééé"));
    assert_eq!(r.term.cwd(), Some("file://h\u{e9}\u{e9}"));
    // A host cannot ask for more than the built-in cap.
    r.term.set_limits(lowered(|l| l.title = usize::MAX));
    r.feed(&format!("\x1b]2;{}\x07", "a".repeat(5000)));
    assert_eq!(r.term.title().map(str::len), Some(4096));
}

#[test]
fn a_string_over_the_osc_cap_is_dropped_whole() {
    let mut r = rig_with(lowered(|l| l.osc = 40));
    r.feed("\x1b]2;short\x07");
    assert_eq!(r.term.title(), Some("short"));
    r.feed(&format!("\x1b]2;{}\x07", "x".repeat(100)));
    assert_eq!(r.term.title(), Some("short"), "dropped, not cut");
    r.term.set_limits(Limits::default());
    r.feed(&format!("\x1b]2;{}\x07", "x".repeat(100)));
    assert_eq!(r.term.title().map(str::len), Some(100));
}

#[test]
fn a_string_being_read_keeps_the_cap_it_started_with() {
    let mut r = Rig::new(40, 5);
    r.feed("\x1b]2;start");
    r.term.set_limits(lowered(|l| l.osc = 8));
    r.feed("-and-more\x07");
    assert_eq!(r.term.title(), Some("start-and-more"));
    r.feed("\x1b]2;start-and-more\x07");
    assert_eq!(r.term.title(), Some("start-and-more"), "unchanged: the next string is held to 8");
}

#[test]
fn the_clipboard_has_a_cap_of_its_own() {
    let mut r = rig_with(lowered(|l| l.clipboard = 64));
    let store =
        |bytes: usize| format!("\x1b]52;c;{}\x07", crate::terminal::osc::base64::encode(&vec![b'a'; bytes]));
    r.feed(&store(20));
    assert!(matches!(&r.events()[..], [Event::ClipboardStore { text, .. }] if text.len() == 20));
    r.feed(&store(100));
    assert!(r.events().is_empty());
    // A read request is short.
    r.feed("\x1b]52;c;?\x07");
    assert!(matches!(&r.events()[..], [Event::ClipboardLoad { .. }]));
    // The cap does not touch the small commands.
    r.feed("\x1b]2;title\x07");
    assert_eq!(r.term.title(), Some("title"));
}

#[test]
fn the_rest_of_osc_1337_has_a_cap_of_its_own() {
    let mut r = rig_with(lowered(|l| l.osc_iterm = 64));
    let var = |bytes: usize| format!("\x1b]1337;SetUserVar=name={}\x07", "v".repeat(bytes));
    r.feed(&var(10));
    assert_eq!(r.events().len(), 1);
    r.feed(&var(100));
    assert!(r.events().is_empty());
}

#[test]
fn a_link_target_over_the_cap_is_ignored_like_one_with_a_control_character() {
    let mut r = rig_with(lowered(|l| l.link_uri = 30));
    r.feed("\x1b]8;;https://a.example/ok\x07ok\x1b]8;;\x07 ");
    r.feed(&format!("\x1b]8;;https://a.example/{}\x07long\x1b]8;;\x07", "p".repeat(40)));
    let link = |x: usize| r.term.grid().link_at(crate::types::Point::new(Line(0), crate::types::Column(x)));
    assert_eq!(link(0).map(|l| l.uri.to_string()).as_deref(), Some("https://a.example/ok"));
    assert!(link(3).is_none(), "the long link did not take");
}

#[test]
fn undrained_events_are_held_to_the_hosts_count() {
    let mut r = rig_with(lowered(|l| l.events = 5));
    for i in 0..50 {
        r.feed(&format!("\x1b]9;note {i}\x07"));
    }
    let events = r.events();
    assert_eq!(events.len(), 5);
    assert_eq!(events[0], Event::Notification { title: String::new(), body: "note 0".into() });
    // Resets are never dropped, whatever the cap.
    r.feed("\x1b]9;x\x07\x1b]9;x\x07\x1bc");
    assert!(r.events().contains(&Event::FullReset));
}

#[test]
fn unread_replies_are_held_to_the_hosts_size() {
    let mut r = rig_with(lowered(|l| l.replies = 40));
    r.feed(&"\x1b[5n".repeat(100));
    let held = r.term.replies().len();
    // Each answer is `ESC [ 0 n`; one that starts past the cap is dropped whole.
    assert!((40..=44).contains(&held) && held.is_multiple_of(4), "{held}");
    assert!(r.term.replies().starts_with(b"\x1b[0n"));
    r.term.clear_replies();
    r.feed("\x1b[5n");
    assert_eq!(r.term.replies(), b"\x1b[0n", "the host collected: answers flow again");
}

#[test]
fn set_limits_applies_from_the_next_sequence_and_changes_nothing_kept() {
    let mut r = Rig::new(40, 5);
    r.feed("\x1b]2;a fairly long title\x07");
    r.term.set_limits(lowered(|l| l.title = 4));
    assert_eq!(r.term.title(), Some("a fairly long title"));
    r.feed("\x1b]2;a fairly long title\x07");
    assert_eq!(r.term.title(), Some("a fa"));
}
