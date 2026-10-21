//! The window title and the icon name: OSC 0, 1 and 2, the stack XTWINOPS 22 and 23 push them on,
//! the title modes, and the reports a host may switch on.

use super::Rig;
use crate::terminal::{Event, Policy};

#[test]
fn title_modes_come_back_to_the_plain_form() {
    // xterm ctlseqs: CSI > Ps T resets title modes 0 (set hex) .. 3 (query UTF-8); asking for UTF-8
    // (2, 3) is the plain form. Hex titles are decoded while mode 0 is set.
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b[>0t\x1b]2;616263\x07");
    assert_eq!(rig.term.title(), Some("abc"));
    rig.feed("\x1b[>2t\x1b]2;616263\x07");
    assert_eq!(rig.term.title(), Some("616263"), "UTF-8 is the plain text again");
    rig.feed("\x1b[>1t\x1b[>9t\x1b[>1;3t\x1b[>0T\x1b]2;x\x07");
    assert_eq!(rig.term.title(), Some("x"));
}

#[test]
fn a_title_stack_that_overflows_drops_the_oldest() {
    // xterm ctlseqs XTWINOPS 22/23: push and pop titles; the stack is bounded.
    let mut rig = Rig::new(20, 5);
    for n in 0..40 {
        rig.feed(&format!("\x1b]2;t{n}\x07\x1b[22;2t"));
    }
    rig.events();
    rig.feed("\x1b]2;now\x07");
    let mut popped = Vec::new();
    for _ in 0..40 {
        rig.feed("\x1b[23;2t");
        if let Some(title) = rig.term.title() {
            popped.push(title.to_string());
        }
    }
    assert_eq!(popped.first().map(String::as_str), Some("t39"), "the newest comes back first");
    assert!(popped.iter().any(|t| t == "t39") && !popped.iter().any(|t| t == "t0"), "t0 fell off the bottom");
}

#[test]
fn titles_are_set_reset_and_stacked() {
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b]0;one\x07");
    assert_eq!(rig.term.title(), Some("one"));
    assert_eq!(rig.events(), [Event::Title]);
    rig.feed("\x1b[22;0t\x1b]2;two\x1b\\\x1b[23;0t");
    assert_eq!(rig.term.title(), Some("one"));
    assert_eq!(rig.events(), [Event::Title]);
    rig.feed("\x1b]2;\x07");
    assert_eq!(rig.term.title(), None, "an empty title is no title");
    rig.feed("\x1b]1;icon\x07");
    assert_eq!(rig.events(), [Event::Title, Event::IconName("icon".into())]);
    // Popping with nothing on the stack does nothing.
    rig.feed("\x1b[23;0t");
    assert!(rig.events().is_empty());
}

#[test]
fn a_long_title_is_cut_at_a_character() {
    let mut rig = Rig::new(20, 5);
    rig.feed(&format!("\x1b]0;{}\x07", "é".repeat(3000)));
    let title = rig.term.title().map(str::len).unwrap_or(0);
    assert!(title <= 4096 && title > 4000, "{title} bytes");
}

#[test]
fn controls_are_dropped_from_titles_icon_names_and_folders_when_they_are_set() {
    let mut rig = Rig::new(20, 5);
    // C0 (the bell and the escape end a string, so the others), DEL, and C1 as the two bytes a
    // UTF-8 character takes.
    rig.feed("\x1b]2;a\x01b\x08c\x7fd\u{85}e\u{9b}f\u{80}g\u{9f}h\x0ci\x07");
    assert_eq!(rig.term.title(), Some("abcdefghi"));
    rig.events().clear();
    rig.feed("\x1b]1;x\u{9d}y\x03z\x07");
    assert_eq!(rig.events(), [Event::IconName("xyz".into())]);
    assert_eq!(rig.term.icon_name(), Some("xyz"));
    rig.feed("\x1b]7;file://h/a\u{85}b\x01/c\x7f\x07");
    assert_eq!(rig.term.cwd(), Some("file://h/ab/c"));
    assert_eq!(rig.term.cwd_reports(), ["file://h/ab/c"]);
    // What is not a control stays: format characters, marks, other scripts, a no-break space.
    rig.feed("\x1b]2;caf\u{e9} \u{a0}\u{200d}\u{65e5}\u{672c}\x07");
    assert_eq!(rig.term.title(), Some("caf\u{e9} \u{a0}\u{200d}\u{65e5}\u{672c}"));
}

#[test]
fn a_title_of_nothing_but_controls_is_no_title() {
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b]2;kept\x07\x1b]2;\x01\u{85}\x7f\x07");
    assert_eq!(rig.term.title(), None);
    // The stack holds clean text too: whatever a push saves is what a pop brings back.
    rig.feed("\x1b]2;a\x01b\x07\x1b[22;2t\x1b]2;other\x07\x1b[23;2t");
    assert_eq!(rig.term.title(), Some("ab"));
}

#[test]
fn hostile_titles_are_cleaned_in_hexadecimal_mode_too() {
    let mut rig = Rig::new(20, 5);
    // Title mode 0 reads hexadecimal: 61 is "a", 01 and 1b5b4a (an escape and "[J") are controls.
    rig.feed("\x1b[>0t\x1b]2;61011b5b4a62\x07");
    assert_eq!(rig.term.title(), Some("a[Jb"));
}

#[test]
fn title_reports_are_a_switch_of_their_own() {
    const ASK: &str = "\x1b]2;the title\x07\x1b]1;the icon\x07\x1b[21t\x1b[20t";
    // Neither off nor the window operations on is enough.
    let mut rig = Rig::new(20, 5);
    rig.feed(ASK);
    assert_eq!(rig.replies(), "");
    let mut rig = Rig::new(20, 5);
    rig.term.set_policy(Policy { window_ops: true, ..Policy::default() });
    rig.feed(ASK);
    assert_eq!(rig.replies(), "", "window_ops does not turn the title reports on");
    // On without the window operations: the reports are answered, the operations still are not.
    let mut rig = Rig::new(20, 5);
    rig.term.set_policy(Policy { title_report: true, ..Policy::default() });
    rig.feed(ASK);
    assert_eq!(rig.replies(), "\x1b]lthe title\x1b\\\x1b]Lthe icon\x1b\\");
    rig.feed("\x1b[2t\x1b[8;10;40t");
    assert!(!rig.events().iter().any(|event| matches!(event, Event::WindowOp(_))));
    assert_eq!((rig.term.cols(), rig.term.rows()), (20, 5));
}

#[test]
fn a_reported_title_never_carries_a_control() {
    let mut rig = Rig::new(20, 5);
    rig.term.set_policy(Policy { title_report: true, ..Policy::default() });
    rig.feed("\x1b]2;rm -rf\x08\x08\x08\u{9b}2J\u{85}\x7f\x0d\x0a\x07\x1b[21t");
    assert_eq!(rig.replies(), "\x1b]lrm -rf2J\x1b\\");
}
