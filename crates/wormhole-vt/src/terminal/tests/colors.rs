//! The colors a program changes: the palette and the special colors (OSC 4, 5, 104, 105) and
//! what the host's own colors answer until it does.

use super::Rig;

#[test]
fn special_colors_go_back_to_the_hosts_one_by_one_or_all_at_once() {
    // xterm ctlseqs OSC 4 (256..: the special colors), OSC 5, OSC 104 (reset entries, also 256..),
    // OSC 105 (reset special colors).
    let mut rig = Rig::new(20, 5);
    let ask = |rig: &mut Rig| {
        rig.feed("\x1b]4;256;?\x07\x1b]4;257;?\x07\x1b]4;258;?\x07");
        rig.replies()
    };
    let before = ask(&mut rig);
    rig.feed("\x1b]4;256;rgb:11/22/33;257;rgb:44/55/66\x07\x1b]5;2;rgb:77/88/99\x07");
    let set = ask(&mut rig);
    assert_ne!(set, before);
    rig.feed("\x1b]104;256\x07");
    let one = ask(&mut rig);
    assert!(one.contains("rgb:4444/5555/6666") && one.contains("rgb:7777/8888/9999"));
    assert!(!one.contains("rgb:1111/2222/3333"), "{one:?}");
    rig.feed("\x1b]105;1\x07");
    assert!(!ask(&mut rig).contains("rgb:4444"));
    rig.feed("\x1b]104;999;x\x07\x1b]105;99\x07");
    assert!(ask(&mut rig).contains("rgb:7777"), "numbers it does not have change nothing");
    rig.feed("\x1b]105\x07");
    assert_eq!(ask(&mut rig), before);
    rig.feed("\x1b]4;256;rgb:11/22/33\x07\x1b]104\x07");
    assert_eq!(ask(&mut rig), before, "OSC 104 without a list resets the special colors too");
}

#[test]
fn colors_are_the_hosts_until_a_program_changes_them() {
    use crate::types::{COLOR_COUNT, NamedColor, Rgb};
    let mut rig = Rig::new(10, 4);
    let mut palette = [Rgb::default(); COLOR_COUNT];
    palette[1] = Rgb { r: 0x11, g: 0x22, b: 0x33 };
    palette[NamedColor::Foreground as usize] = Rgb { r: 0xaa, g: 0xbb, b: 0xcc };
    palette[NamedColor::Cursor as usize] = Rgb { r: 1, g: 2, b: 3 };
    rig.term.set_default_colors(&palette);
    rig.feed("\x1b]4;1;?\x07\x1b]10;?\x1b\\\x1b]12;?\x07");
    assert_eq!(
        rig.replies(),
        "\x1b]4;1;rgb:1111/2222/3333\x07\x1b]10;rgb:aaaa/bbbb/cccc\x1b\\\x1b]12;rgb:0101/0202/0303\x07"
    );
    // Set two at once with 10, ask for the second; set a palette entry with a short form.
    rig.feed("\x1b]10;#ff0000;rgb:0/f/0\x07\x1b]11;?\x07\x1b]4;1;#00f;2;?\x07");
    assert_eq!(rig.replies(), "\x1b]11;rgb:0000/ffff/0000\x07\x1b]4;2;rgb:0000/0000/0000\x07");
    assert_eq!(rig.term.colors()[NamedColor::Foreground], Some(Rgb { r: 255, g: 0, b: 0 }));
    assert_eq!(rig.term.colors()[1], Some(Rgb { r: 0, g: 0, b: 0xf0 }), "#00f is the top bits of the byte");
    // Resets.
    rig.feed("\x1b]104;1\x07\x1b]110\x07\x1b]111\x07");
    assert_eq!(rig.term.colors()[1], None);
    assert_eq!(rig.term.colors()[NamedColor::Foreground], None);
    assert_eq!(rig.term.colors()[NamedColor::Background], None);
    rig.feed("\x1b]4;5;rgb:1/2/3;6;rgb:4/5/6\x07\x1b]104\x07");
    assert_eq!(rig.term.colors()[5], None);
    assert_eq!(rig.term.colors()[6], None);
}
