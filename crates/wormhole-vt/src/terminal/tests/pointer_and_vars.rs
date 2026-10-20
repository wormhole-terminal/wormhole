//! OSC 22 (a program's pointer shape) and iTerm2's `SetUserVar`, as events.

use super::Rig;
use crate::terminal::Event;
use crate::terminal::osc::base64;

fn shape(name: &str) -> Event {
    Event::PointerShape(name.to_string())
}

#[test]
fn a_program_sets_the_pointer_and_gives_it_back() {
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b]22;pointer\x07");
    assert_eq!(rig.events(), [shape("pointer")]);
    rig.feed("\x1b]22;=text\x1b\\");
    assert_eq!(rig.events(), [shape("text")], "kitty's explicit form, ended by ST");
    rig.feed("\x1b]22;\x07");
    assert_eq!(rig.events(), [shape("")], "empty: the host's own pointer");
    assert_eq!(rig.replies(), "", "nothing is ever answered");
}

#[test]
fn every_css_shape_and_the_x11_names_programs_use_arrive_as_a_css_keyword() {
    let mut rig = Rig::new(20, 5);
    for name in [
        "default",
        "pointer",
        "text",
        "crosshair",
        "wait",
        "progress",
        "help",
        "move",
        "not-allowed",
        "grab",
        "grabbing",
        "ew-resize",
        "ns-resize",
        "nwse-resize",
        "zoom-in",
        "vertical-text",
    ] {
        rig.feed(&format!("\x1b]22;{name}\x07"));
        assert_eq!(rig.events(), [shape(name)], "{name}");
    }
    for (x11, css) in [("left_ptr", "default"), ("xterm", "text"), ("hand2", "pointer"), ("watch", "wait")] {
        rig.feed(&format!("\x1b]22;{x11}\x07"));
        assert_eq!(rig.events(), [shape(css)], "{x11}");
    }
}

#[test]
fn a_list_gives_the_first_shape_the_host_can_draw() {
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b]22;no-such-shape,Text,crosshair,pointer\x07");
    assert_eq!(rig.events(), [shape("crosshair")]);
}

#[test]
fn names_that_are_not_shapes_are_dropped() {
    let mut rig = Rig::new(20, 5);
    for bad in [
        "auto",
        "none",
        "url(http://example.com/x.png)",
        "POINTER",
        "../../etc/passwd",
        "point er",
        "\u{e9}",
        &"x".repeat(5000),
    ] {
        rig.feed(&format!("\x1b]22;{bad}\x07"));
        assert_eq!(rig.events(), [], "{bad:?}");
    }
    // The stack and the queries of kitty's protocol are not read.
    rig.feed("\x1b]22;>pointer\x07\x1b]22;<\x07\x1b]22;?pointer,text\x07\x1b]22;?__current__\x07");
    assert_eq!(rig.events(), []);
    assert_eq!(rig.replies(), "");
}

#[test]
fn the_same_shape_again_is_one_event() {
    let mut rig = Rig::new(20, 5);
    rig.feed(&"\x1b]22;pointer\x07".repeat(1000));
    assert_eq!(rig.events(), [shape("pointer")]);
}

fn var(name: &str, value: &[u8]) -> String {
    format!("\x1b]1337;SetUserVar={name}={}\x07", base64::encode(value))
}

fn user_var(name: &str, value: &[u8]) -> Event {
    Event::UserVar { name: name.to_string(), value: value.to_vec() }
}

#[test]
fn a_user_variable_arrives_decoded_and_not_as_the_raw_event() {
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b]1337;SetUserVar=user=Zm9v\x07");
    assert_eq!(rig.events(), [user_var("user", b"foo")]);
    rig.feed(&var("blob", &[0, 255, 1, 2]));
    assert_eq!(rig.events(), [user_var("blob", &[0, 255, 1, 2])]);
    rig.feed("\x1b]1337;SetUserVar=empty=\x1b\\");
    assert_eq!(rig.events(), [user_var("empty", b"")]);
}

#[test]
fn the_rest_of_osc_1337_stays_the_raw_event() {
    let mut rig = Rig::new(20, 5);
    rig.feed("\x1b]1337;SetMark\x07\x1b]1337;SetUserVar=a=Yg==\x07\x1b]1337;CurrentDir=/tmp\x07");
    assert_eq!(
        rig.events(),
        [Event::Iterm(b"SetMark".to_vec()), user_var("a", b"b"), Event::Iterm(b"CurrentDir=/tmp".to_vec())]
    );
}

#[test]
fn a_user_variable_cut_into_pieces_is_the_same() {
    let mut rig = Rig::new(20, 5);
    let text = var("name", b"some value");
    for piece in text.as_bytes().chunks(3) {
        rig.feed_bytes(piece);
    }
    assert_eq!(rig.events(), [user_var("name", b"some value")]);
}

#[test]
fn bad_names_and_values_are_dropped_not_handed_on() {
    let mut rig = Rig::new(20, 5);
    for bad in [
        "\x1b]1337;SetUserVar=\x07".to_string(),
        "\x1b]1337;SetUserVar==Yg==\x07".to_string(),
        "\x1b]1337;SetUserVar=novalue\x07".to_string(),
        "\x1b]1337;SetUserVar=a=not base64!\x07".to_string(),
        "\x1b]1337;SetUserVar=a=Y\x07".to_string(),
        var(&"n".repeat(257), b"x"),
    ] {
        rig.feed(&bad);
        assert_eq!(rig.events(), [], "{bad:?}");
    }
    rig.feed(&var(&"n".repeat(256), b"x"));
    assert_eq!(rig.events().len(), 1, "256 bytes is the longest name");
}

#[test]
fn a_value_is_at_most_64_kib() {
    let mut rig = Rig::new(20, 5);
    rig.feed(&var("big", &vec![b'x'; 64 * 1024]));
    assert!(matches!(&rig.events()[..], [Event::UserVar { value, .. }] if value.len() == 64 * 1024));
    rig.feed(&var("big", &vec![b'x'; 64 * 1024 + 1]));
    assert_eq!(rig.events(), []);
    // However it arrives.
    for piece in var("big", &vec![b'x'; 64 * 1024 + 1]).as_bytes().chunks(7) {
        rig.feed_bytes(piece);
    }
    assert_eq!(rig.events(), []);
}

#[test]
fn values_nobody_takes_do_not_pile_up_past_a_megabyte() {
    let mut rig = Rig::new(20, 5);
    let text = var("v", &vec![b'x'; 64 * 1024]);
    for _ in 0..40 {
        rig.feed(&text);
    }
    assert_eq!(rig.events().len(), 16, "1 MiB of 64 KiB values");
    rig.feed(&text);
    assert_eq!(rig.events().len(), 1, "the host took them: room again");
}
