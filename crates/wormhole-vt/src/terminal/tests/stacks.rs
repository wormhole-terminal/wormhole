//! xterm's stacks (ctlseqs): video attributes (XTPUSHSGR `CSI Pm # {`, XTPOPSGR `CSI # }`,
//! XTREPORTSGR `CSI Pt ; Pl ; Pb ; Pr # |`) and colors (XTPUSHCOLORS `CSI # P`, XTPOPCOLORS
//! `CSI # Q`, XTREPORTCOLORS `CSI # R`).

use super::Rig;
use crate::terminal::Event;
use crate::types::{NamedColor, Rgb};

/// The pen as DECRQSS reports it.
fn pen(rig: &mut Rig) -> String {
    rig.feed("\x1bP$qm\x1b\\");
    let reply = rig.replies();
    reply.trim_start_matches("\x1bP1$r").trim_end_matches("\x1b\\").to_string()
}

#[test]
fn a_push_without_parameters_saves_every_attribute_and_a_pop_brings_them_back() {
    let mut rig = Rig::new(20, 4);
    rig.feed("\x1b[1;31m\x1b[#{\x1b[0;3;4;44m");
    assert_eq!(pen(&mut rig), "0;3;4;44m");
    rig.feed("\x1b[#}");
    assert_eq!(pen(&mut rig), "0;1;31m");
}

#[test]
fn a_push_with_parameters_saves_and_restores_only_those() {
    let mut rig = Rig::new(20, 4);
    // Bold and the foreground (30), saved; italic is not.
    rig.feed("\x1b[1;3;31m\x1b[1;30#{");
    rig.feed("\x1b[0;4;32m");
    rig.feed("\x1b[#}");
    // Bold and the red are back; the underline stays, italic was never saved and was reset.
    assert_eq!(pen(&mut rig), "0;1;4;31m");
}

#[test]
fn the_alias_forms_push_and_pop_too() {
    let mut rig = Rig::new(20, 4);
    rig.feed("\x1b[1m\x1b[#p\x1b[0m");
    assert_eq!(pen(&mut rig), "0m");
    rig.feed("\x1b[#q");
    assert_eq!(pen(&mut rig), "0;1m");
}

#[test]
fn the_stack_is_ten_deep_and_a_pop_on_an_empty_one_does_nothing() {
    let mut rig = Rig::new(20, 4);
    rig.feed("\x1b[#}");
    assert_eq!(pen(&mut rig), "0m", "nothing to pop");
    // Eleven pushes, each of a different foreground: the eleventh is dropped.
    for n in 1..=11 {
        rig.feed(&format!("\x1b[38;5;{n}m\x1b[#{{"));
    }
    rig.feed("\x1b[0m");
    for _ in 0..10 {
        rig.feed("\x1b[#}");
    }
    assert_eq!(pen(&mut rig), "0;38;5;1m", "ten pops reached the first push, the eleventh was never kept");
    rig.feed("\x1b[#}");
    assert_eq!(pen(&mut rig), "0;38;5;1m");
}

#[test]
fn a_full_reset_empties_the_stack() {
    let mut rig = Rig::new(20, 4);
    rig.feed("\x1b[1m\x1b[#{\x1bc\x1b[#}");
    assert_eq!(pen(&mut rig), "0m");
}

#[test]
fn a_push_naming_nothing_it_knows_pushes_nothing() {
    let mut rig = Rig::new(20, 4);
    rig.feed("\x1b[1m\x1b[99#{\x1b[0m\x1b[#}");
    assert_eq!(pen(&mut rig), "0m");
}

#[test]
fn the_underline_stays_one_style() {
    let mut rig = Rig::new(20, 4);
    // Save the single underline, change to the double one, restore: the double one is gone.
    rig.feed("\x1b[4m\x1b[4#{\x1b[21m\x1b[#}");
    assert_eq!(pen(&mut rig), "0;4m");
}

#[test]
fn the_report_holds_what_every_cell_of_the_rectangle_has_in_common() {
    let mut rig = Rig::new(10, 3);
    rig.feed("\x1b[1;3;31mab\x1b[0;1;4;31mcd\x1b[0m");
    // Bold and the red are in every cell of the first four; italic and the underline are not.
    rig.feed("\x1b[1;1;1;4#|");
    assert_eq!(rig.replies(), "\x1b[0;1;31m");
    // One cell: all of it.
    rig.feed("\x1b[1;1;1;1#|");
    assert_eq!(rig.replies(), "\x1b[0;1;3;31m");
    // A different color makes the report say nothing of it.
    rig.feed("\x1b[1;1;1;5#|");
    assert_eq!(rig.replies(), "\x1b[0m");
    // The whole screen without parameters.
    rig.feed("\x1b[#|");
    assert_eq!(rig.replies(), "\x1b[0m");
}

#[test]
fn the_report_gives_an_underline_color_only_when_every_cell_has_it() {
    let mut rig = Rig::new(10, 3);
    rig.feed("\x1b[58;5;1mab\x1b[58;5;2mc\x1b[0m");
    rig.feed("\x1b[1;1;1;2#|");
    assert_eq!(rig.replies(), "\x1b[0;58;5;1m");
    rig.feed("\x1b[1;1;1;3#|");
    assert_eq!(rig.replies(), "\x1b[0m");
}

#[test]
fn an_upside_down_rectangle_is_not_answered() {
    let mut rig = Rig::new(10, 3);
    rig.feed("\x1b[3;1;1;5#|");
    assert_eq!(rig.replies(), "");
}

const RED: Rgb = Rgb { r: 255, g: 0, b: 0 };

#[test]
fn colors_pushed_come_back_with_a_pop_and_the_host_hears_of_it() {
    let mut rig = Rig::new(20, 4);
    rig.feed("\x1b]4;1;rgb:ff/00/00\x07\x1b]10;rgb:11/22/33\x07\x1b[#P");
    rig.feed("\x1b]4;1;rgb:00/ff/00\x07\x1b]104;2\x07");
    let _ = rig.events();
    rig.feed("\x1b[#Q");
    assert_eq!(rig.term.colors()[1], Some(RED));
    assert_eq!(rig.term.colors()[NamedColor::Foreground], Some(Rgb { r: 0x11, g: 0x22, b: 0x33 }));
    assert!(rig.events().contains(&Event::ColorsChanged));
    rig.feed("\x1b[#Q");
    assert_eq!(rig.term.colors()[1], Some(RED), "a pop on an empty stack does nothing");
}

#[test]
fn a_pop_resets_what_the_push_found_unset() {
    let mut rig = Rig::new(20, 4);
    rig.feed("\x1b[#P\x1b]4;5;#ffffff\x07\x1b]11;#101010\x07");
    rig.feed("\x1b[#Q");
    assert_eq!(rig.term.colors()[5], None);
    assert_eq!(rig.term.colors()[NamedColor::Background], None);
}

#[test]
fn slots_store_and_restore_without_pushing_or_popping() {
    let mut rig = Rig::new(20, 4);
    rig.feed("\x1b]4;1;rgb:ff/00/00\x07\x1b[1#P");
    rig.feed("\x1b]4;1;rgb:00/ff/00\x07\x1b[#R");
    assert_eq!(rig.replies(), "\x1b[?1;1#Q", "the slot made one entry");
    rig.feed("\x1b[1#Q");
    assert_eq!(rig.term.colors()[1], Some(RED));
    rig.feed("\x1b[#R");
    assert_eq!(rig.replies(), "\x1b[?1;1#Q", "a restore does not pop");
    // A slot past the next one is not made; the next one is.
    rig.feed("\x1b[5#P\x1b[#R");
    assert_eq!(rig.replies(), "\x1b[?1;1#Q");
    rig.feed("\x1b[2#P\x1b[#R");
    assert_eq!(rig.replies(), "\x1b[?2;2#Q");
}

#[test]
fn the_color_stack_is_ten_deep_and_the_report_counts_it() {
    let mut rig = Rig::new(20, 4);
    rig.feed("\x1b[#R");
    assert_eq!(rig.replies(), "\x1b[?0;0#Q");
    for _ in 0..12 {
        rig.feed("\x1b[#P");
    }
    rig.feed("\x1b[#R");
    assert_eq!(rig.replies(), "\x1b[?10;10#Q");
    rig.feed("\x1b[#Q\x1b[#R");
    assert_eq!(rig.replies(), "\x1b[?9;9#Q");
    rig.feed("\x1bc\x1b[#R");
    assert_eq!(rig.replies(), "\x1b[?0;0#Q");
}

#[test]
fn a_repaint_keeps_both_stacks() {
    let mut a = Rig::new(20, 4);
    a.feed("\x1b[1;31m\x1b[#{\x1b[0;4;44m\x1b[4#{\x1b[1;38;5;9m\x1b[#{");
    a.feed("\x1b]4;1;rgb:ff/00/00\x07\x1b[#P\x1b]4;1;#00ff00\x07\x1b]11;#102030\x07\x1b[#P\x1b]104\x07\x1b]4;2;#0000ff\x07");
    a.feed("\x1b[3;1Hhello\x1b[44m");
    let mut bytes = Vec::new();
    let repainted = a.term.repaint(&mut bytes).expect("ready");
    let mut b = Rig::new(20, 4);
    b.term.seed(&repainted.seeds);
    b.feed_bytes(&bytes);
    assert_eq!(a.term.dump_state().diff(&b.term.dump_state(), 6), "");
    // And a pop on each does the same.
    let tail = b"\x1b[#}\x1b[#Q\x1b[#}";
    a.feed_bytes(tail);
    b.feed_bytes(tail);
    assert_eq!(a.term.dump_state().diff(&b.term.dump_state(), 6), "");
}
