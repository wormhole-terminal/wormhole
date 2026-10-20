//! XTMODKEYS (`CSI > Pp ; Pv m`) and XTQMODKEYS (`CSI ? Pp m`) for modifyOtherKeys, against xterm's
//! ctlseqs, and how the level reaches the host (`Modes::modify_other_keys`).

use super::Rig;
use crate::types::{Modes, ModifyOtherKeys};

fn level(rig: &Rig) -> ModifyOtherKeys {
    rig.term.modes().modify_other_keys()
}

fn ask(rig: &mut Rig) -> String {
    rig.feed("\x1b[?4m");
    rig.replies()
}

#[test]
fn the_level_is_set_asked_and_reset() {
    let mut rig = Rig::new(10, 3);
    assert_eq!((level(&rig), ask(&mut rig)), (ModifyOtherKeys::Off, "\x1b[>4;0m".into()));
    rig.feed("\x1b[>4;1m");
    assert_eq!((level(&rig), ask(&mut rig)), (ModifyOtherKeys::Level1, "\x1b[>4;1m".into()));
    rig.feed("\x1b[>4;2m");
    assert_eq!((level(&rig), ask(&mut rig)), (ModifyOtherKeys::Level2, "\x1b[>4;2m".into()));
    // The answer is a sequence that sets the value back.
    rig.feed("\x1b[>4;0m");
    assert_eq!(level(&rig), ModifyOtherKeys::Off);
    rig.feed("\x1b[>4;2m\x1b[>4m");
    assert_eq!(level(&rig), ModifyOtherKeys::Off, "the value left out is the initial one");
    rig.feed("\x1b[>4;2m\x1b[>4;m");
    assert_eq!(level(&rig), ModifyOtherKeys::Off);
}

#[test]
fn with_no_parameters_every_resource_is_reset() {
    let mut rig = Rig::new(10, 3);
    rig.feed("\x1b[>4;2m\x1b[>m");
    assert_eq!(level(&rig), ModifyOtherKeys::Off);
}

#[test]
fn other_resources_and_values_are_left_alone() {
    let mut rig = Rig::new(10, 3);
    rig.feed("\x1b[>4;1m");
    // modifyCursorKeys and friends are not kept; a level past 2 and the colon form are not read.
    rig.feed("\x1b[>1;3m\x1b[>2;0m\x1b[>4;3m\x1b[>4:1m\x1b[>4;9m");
    assert_eq!(level(&rig), ModifyOtherKeys::Level1);
    rig.feed("\x1b[?1m\x1b[?0m\x1b[?7m");
    assert_eq!(rig.replies(), "");
}

#[test]
fn the_host_hears_of_a_change() {
    let mut rig = Rig::new(10, 3);
    let _ = rig.events();
    rig.feed("\x1b[>4;2m");
    assert!(rig.events().contains(&crate::terminal::Event::ModesChanged));
    let modes = rig.term.modes();
    assert!(modes.contains(Modes::MODIFY_OTHER_KEYS | Modes::MODIFY_OTHER_KEYS_ALL));
}

#[test]
fn a_soft_and_a_full_reset_put_it_back() {
    let mut rig = Rig::new(10, 3);
    rig.feed("\x1b[>4;2m\x1b[!p");
    assert_eq!(level(&rig), ModifyOtherKeys::Off, "DECSTR resets the key modifier modes, as xterm's");
    rig.feed("\x1b[>4;1m\x1bc");
    assert_eq!(level(&rig), ModifyOtherKeys::Off);
}

#[test]
fn a_repaint_keeps_the_level() {
    for level in ["1", "2"] {
        let mut a = Rig::new(10, 3);
        a.feed(&format!("\x1b[>4;{level}m"));
        let mut bytes = Vec::new();
        let repainted = a.term.repaint(&mut bytes).expect("ready");
        let mut b = Rig::new(10, 3);
        b.term.seed(&repainted.seeds);
        b.feed_bytes(&bytes);
        assert_eq!(a.term.dump_state().diff(&b.term.dump_state(), 4), "");
        assert_eq!(b.term.modes().modify_other_keys(), a.term.modes().modify_other_keys());
    }
}
