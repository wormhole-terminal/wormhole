//! `Event::ModesChanged` and `Event::ColorsChanged`: once per look of the host, only for what it
//! acts on.

use super::Rig;
use crate::terminal::Event;
use crate::types::Modes;

fn changes(rig: &mut Rig) -> Vec<Event> {
    rig.events().into_iter().filter(|e| matches!(e, Event::ModesChanged | Event::ColorsChanged)).collect()
}

#[test]
fn a_host_relevant_mode_flipping_is_announced() {
    let mut rig = Rig::new(20, 5);
    assert!(changes(&mut rig).is_empty(), "a new terminal has nothing to announce");
    for (set, reset) in [
        ("\x1b[?1000h", "\x1b[?1000l"),
        ("\x1b[?1006h", "\x1b[?1006l"),
        ("\x1b[?1049h", "\x1b[?1049l"),
        ("\x1b[?2004h", "\x1b[?2004l"),
        ("\x1b[?1004h", "\x1b[?1004l"),
        ("\x1b[?1h", "\x1b[?1l"),
        ("\x1b=", "\x1b>"),
        ("\x1b[?67h", "\x1b[?67l"),
        ("\x1b[=1;1u", "\x1b[=0;1u"),
        ("\x1b[?25l", "\x1b[?25h"),
        ("\x1b[?2026h", "\x1b[?2026l"),
        ("\x1b[?5h", "\x1b[?5l"),
        ("\x1b[?2031h", "\x1b[?2031l"),
        ("\x1b[?1007l", "\x1b[?1007h"),
        ("\x1b[1'z", "\x1b[0'z"),
    ] {
        rig.feed(set);
        assert_eq!(changes(&mut rig), [Event::ModesChanged], "{set:?}");
        rig.feed(set);
        assert!(changes(&mut rig).is_empty(), "{set:?} again changes nothing");
        rig.feed(reset);
        assert_eq!(changes(&mut rig), [Event::ModesChanged], "{reset:?}");
    }
}

#[test]
fn the_cursors_look_is_announced_too() {
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b[5 q");
    assert_eq!(changes(&mut rig), [Event::ModesChanged]);
    rig.feed("\x1b[5 q");
    assert!(changes(&mut rig).is_empty());
    rig.feed("\x1b[?12l");
    assert_eq!(changes(&mut rig), [Event::ModesChanged], "blink");
    rig.feed("\x1b[0 q");
    assert_eq!(changes(&mut rig), [Event::ModesChanged], "back to the host's own");
}

#[test]
fn many_modes_flipping_are_one_announcement() {
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b[?1000h\x1b[?1006h\x1b[?2004h\x1b[?1049h\x1b[?25l");
    assert_eq!(changes(&mut rig), [Event::ModesChanged]);
    let modes = rig.term.modes();
    assert!(modes.contains(Modes::ALT_SCREEN | Modes::BRACKETED_PASTE | Modes::MOUSE_SGR));
}

#[test]
fn a_mode_set_and_reset_before_the_host_looks_is_no_change() {
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b[?2004h\x1b[?2004l\x1b[?1000h\x1b[?1000l");
    assert!(changes(&mut rig).is_empty());
    // Across two feeds that the host drains between, it is two changes; with no look between, none.
    rig.feed("\x1b[?2004h");
    assert_eq!(changes(&mut rig), [Event::ModesChanged]);
    rig.feed("\x1b[?2004l");
    assert_eq!(changes(&mut rig), [Event::ModesChanged]);
    rig.feed("\x1b[?2004h");
    rig.feed("\x1b[?2004l");
    assert!(changes(&mut rig).is_empty());
}

#[test]
fn modes_only_the_engine_acts_on_are_not_announced() {
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b[4h\x1b[20h\x1b[?7l\x1b[?6h\x1b[?69h\x1b[?45h\x1b[?1010h\x1b[?18h\x1b[?2027l");
    assert!(changes(&mut rig).is_empty());
}

#[test]
fn a_reset_announces_the_modes_it_changed_and_the_input_is_split_anywhere() {
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b[?1000h");
    changes(&mut rig);
    rig.feed("\x1bc");
    assert_eq!(changes(&mut rig), [Event::ModesChanged]);
    // The sequence cut across feeds is announced by the feed that completes it.
    for piece in "\x1b[?10".as_bytes().chunks(2) {
        rig.feed_bytes(piece);
    }
    assert!(changes(&mut rig).is_empty());
    rig.feed("04h");
    assert_eq!(changes(&mut rig), [Event::ModesChanged]);
}

#[test]
fn colors_programs_set_are_announced_once() {
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b]4;1;rgb:ff/00/00\x07\x1b]10;rgb:11/22/33\x07\x1b]11;#000000\x07\x1b]12;#ffffff\x07");
    assert_eq!(changes(&mut rig), [Event::ColorsChanged]);
    assert!(rig.term.colors()[1].is_some());
    // The same color again, or a query, is no change.
    rig.feed("\x1b]4;1;rgb:ff/00/00\x07\x1b]4;1;?\x07\x1b]10;?\x07");
    assert!(changes(&mut rig).is_empty());
    // A set and a reset in one feed leave things as they were.
    rig.feed("\x1b]4;2;rgb:00/ff/00\x07\x1b]104;2\x07");
    assert!(changes(&mut rig).is_empty());
}

#[test]
fn every_color_command_that_changes_what_colors_returns_announces() {
    for (set, reset) in [
        ("\x1b]4;7;#010203\x07", "\x1b]104;7\x07"),
        ("\x1b]4;7;#010203\x07", "\x1b]104\x07"),
        ("\x1b]10;#010203\x07", "\x1b]110\x07"),
        ("\x1b]11;#010203\x07", "\x1b]111\x07"),
        ("\x1b]12;#010203\x07", "\x1b]112\x07"),
        ("\x1b]10;#010203\x07", "\x1bc"),
    ] {
        let mut rig = Rig::new(20, 5);
        rig.feed(set);
        assert_eq!(changes(&mut rig), [Event::ColorsChanged], "{set:?}");
        rig.feed(reset);
        assert!(changes(&mut rig).contains(&Event::ColorsChanged), "{reset:?}");
    }
    // The special colors and OSC 13 to 19 are not in `colors()`: nothing for a host to redraw.
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b]5;0;#010203\x07\x1b]13;#010203\x07");
    assert!(changes(&mut rig).is_empty());
}

#[test]
fn the_hosts_own_palette_is_not_a_change() {
    let mut rig = Rig::new(20, 5);
    rig.term.set_default_colors(&[crate::types::Rgb { r: 1, g: 2, b: 3 }; crate::types::COLOR_COUNT]);
    rig.feed("text\r\n");
    assert!(changes(&mut rig).is_empty());
}

#[test]
fn several_feeds_before_the_host_looks_are_one_announcement_after_the_other_events() {
    let mut rig = Rig::new(20, 5);
    for _ in 0..50 {
        rig.feed("\x1b]2;t\x07\x1b[?2004h\x1b]4;1;#000001\x07");
        rig.feed("\x1b[?1000h");
        rig.feed("\x1b[?2004l\x1b[?1000l\x1b]104\x07");
    }
    let events = rig.events();
    assert_eq!(events, [Event::Title], "the host never saw a mode or a color come and go");
    rig.feed("\x1b[?2004h\x1b]4;1;#000001\x07\x1b]2;x\x07");
    rig.feed("\x1b[?1000h");
    assert_eq!(rig.events(), [Event::Title, Event::ModesChanged, Event::ColorsChanged]);
}

#[test]
fn the_announcements_are_not_lost_when_the_queue_is_full() {
    let mut rig = Rig::new(20, 5);
    rig.feed(&"\x07\x1b]2;a\x07\x1b]7;file://h/a\x07\x1b]9;n\x07".repeat(10_000));
    rig.feed("\x1b[?2004h\x1b]4;1;#000001\x07");
    let events = rig.events();
    assert!(events.len() >= 16_384, "the queue was full");
    assert_eq!(&events[events.len() - 2..], [Event::ModesChanged, Event::ColorsChanged]);
}
