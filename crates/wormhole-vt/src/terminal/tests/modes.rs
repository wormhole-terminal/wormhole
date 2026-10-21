//! Setting, resetting and asking for modes (SM, RM, DECSET, DECRST, DECRQM) and what a few of them
//! touch besides their own flag.

use super::Rig;
use crate::Damage;
use crate::types::Modes;

/// What DECRQM answers for ANSI mode `n` (`private` false) or DEC private mode `n`.
fn status(rig: &mut Rig, n: u16, private: bool) -> String {
    let marker = if private { "?" } else { "" };
    rig.feed(&format!("\x1b[{marker}{n}$p"));
    rig.replies()
}

#[test]
fn the_modes_that_are_only_a_flag_can_be_set_reset_and_asked_for() {
    let mut rig = Rig::new(20, 5);
    // Keyboard action, send/receive; scroll on output and on key, Alt as ESC, the bell as an urgency
    // hint, and Sixel's private palette.
    for (n, private) in
        [(2, false), (12, false), (1010, true), (1011, true), (1039, true), (1042, true), (1070, true)]
    {
        let marker = if private { "?" } else { "" };
        let reply = |pm: u8| format!("\x1b[{marker}{n};{pm}$y");
        rig.feed(&format!("\x1b[{marker}{n}h"));
        assert_eq!(status(&mut rig, n, private), reply(1), "{marker}{n} set");
        rig.feed(&format!("\x1b[{marker}{n}l"));
        assert_eq!(status(&mut rig, n, private), reply(2), "{marker}{n} reset");
    }
}

#[test]
fn blinking_cursor_mode_is_reset_until_a_program_sets_it() {
    let mut rig = Rig::new(20, 5);
    assert_eq!(status(&mut rig, 12, true), "\x1b[?12;2$y", "left to the host");
    rig.feed("\x1b[?12h");
    assert_eq!(status(&mut rig, 12, true), "\x1b[?12;1$y");
    rig.feed("\x1b[?12l");
    assert_eq!(status(&mut rig, 12, true), "\x1b[?12;2$y");
}

#[test]
fn only_turning_a_mouse_tracking_mode_on_takes_the_pointer_from_the_locator() {
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b[1'z");
    // Other modes, on or off, and a tracking mode going off, leave the locator as it is.
    rig.feed("\x1b[?25h\x1b[?1000l\x1b[?1l");
    assert!(rig.term.modes().contains(Modes::LOCATOR));
    for n in [9, 1000, 1002, 1003] {
        rig.feed("\x1b[1'z");
        assert!(rig.term.modes().contains(Modes::LOCATOR));
        rig.feed(&format!("\x1b[?{n}h"));
        assert!(!rig.term.modes().contains(Modes::LOCATOR), "?{n}h");
    }
}

#[test]
fn reverse_video_redraws_every_row() {
    let mut rig = Rig::new(10, 3);
    let mut damage = Damage::new();
    rig.term.take_damage(&mut damage);
    rig.term.take_damage(&mut damage);
    assert!(damage.is_empty());
    for (change, mode) in [("\x1b[?5h", true), ("\x1b[?5l", false)] {
        rig.feed(change);
        rig.term.take_damage(&mut damage);
        assert_eq!(damage.rows().collect::<Vec<_>>(), [0, 1, 2], "{change:?}");
        assert_eq!(rig.term.modes().contains(Modes::REVERSE_VIDEO), mode);
    }
}

#[test]
fn xtmodkeys_resets_only_with_no_parameters_at_all() {
    let mut rig = Rig::new(20, 5);
    let level = |rig: &mut Rig| {
        rig.feed("\x1b[?4m");
        rig.replies()
    };
    rig.feed("\x1b[>4;2m");
    assert_eq!(level(&mut rig), "\x1b[>4;2m");
    // Another resource, or an empty first parameter before another: not `CSI > m`.
    rig.feed("\x1b[>5m\x1b[>;1m\x1b[>;m");
    assert_eq!(level(&mut rig), "\x1b[>4;2m");
    rig.feed("\x1b[>m");
    assert_eq!(level(&mut rig), "\x1b[>4;0m");
}
