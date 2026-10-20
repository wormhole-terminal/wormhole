//! modifyOtherKeys: xterm's ctlseqs ("Alt and Meta Keys") and its table of modified keys
//! (invisible-island.net/xterm/modified-keys-us-pc105.html): `CSI 27 ; modifiers ; code ~`.

use super::{ALT, CTRL, DIS, NONE, SHIFT, enc, key, s, typed, with};
use crate::input::{InputModes, Key, KeypadKey, ModifyOtherKeys, Mods};

const L1: ModifyOtherKeys = ModifyOtherKeys::Level1;
const L2: ModifyOtherKeys = ModifyOtherKeys::Level2;

fn other(level: ModifyOtherKeys) -> InputModes {
    InputModes { modify_other_keys: level, ..NONE }
}

#[test]
fn off_changes_nothing() {
    let m = other(ModifyOtherKeys::Off);
    assert_eq!(enc(with(key(Key::Char('a')), ALT), m), s("\x1ba"));
    assert_eq!(enc(with(key(Key::Tab), ALT), m), s("\t"));
}

#[test]
fn level_1_sends_alt_and_meta_keys_as_sequences() {
    let m = other(L1);
    // ctlseqs: "alt-Tab sends CSI 27 ; 3 ; 9 ~ (the second parameter is 3 for alt, and the third
    // parameter is the ASCII value of tab, 9)".
    assert_eq!(enc(with(key(Key::Tab), ALT), m), s("\x1b[27;3;9~"));
    assert_eq!(enc(with(key(Key::Char('a')), ALT), m), s("\x1b[27;3;97~"));
    assert_eq!(enc(with(key(Key::Char('a')), ALT | SHIFT), m), s("\x1b[27;4;65~"));
    assert_eq!(enc(with(key(Key::Char('a')), ALT | CTRL), m), s("\x1b[27;7;97~"));
    assert_eq!(enc(with(key(Key::Char('a')), Mods::META), m), s("\x1b[27;9;97~"));
    assert_eq!(enc(with(key(Key::Char('1')), ALT), m), s("\x1b[27;3;49~"));
    assert_eq!(enc(with(key(Key::Char(' ')), ALT), m), s("\x1b[27;3;32~"));
    assert_eq!(enc(with(key(Key::Escape), ALT), m), s("\x1b[27;3;27~"));
    // Alt with a character the layout composed (Option+L is `@` on a German layout) was used up.
    assert_eq!(enc(with(typed('l', "@"), ALT), m), s("@"));
}

#[test]
fn level_1_leaves_shift_and_the_control_codes_as_they_were() {
    let m = other(L1);
    assert_eq!(enc(with(typed('a', "A"), SHIFT), m), s("A"));
    assert_eq!(enc(with(key(Key::Char('a')), SHIFT), m), s("A"));
    assert_eq!(enc(with(key(Key::Char('a')), CTRL), m), s("\u{1}"));
    assert_eq!(enc(with(key(Key::Char('a')), CTRL | SHIFT), m), s("\u{1}"));
    assert_eq!(enc(with(key(Key::Char(' ')), CTRL), m), s("\0"));
    assert_eq!(enc(with(key(Key::Char('[')), CTRL), m), s("\u{1b}"));
    assert_eq!(enc(with(key(Key::Char('/')), CTRL), m), s("\u{1f}"));
    assert_eq!(enc(with(key(Key::Char('2')), CTRL), m), s("\0"));
    assert_eq!(enc(with(typed('2', "@"), CTRL | SHIFT), m), s("\0"), "ctrl-@");
    assert_eq!(enc(with(typed('6', "^"), CTRL | SHIFT), m), s("\u{1e}"), "ctrl-^");
    assert_eq!(enc(with(key(Key::Escape), SHIFT), m), s("\x1b"));
    assert_eq!(enc(with(key(Key::Backspace), ALT), m), s("\x1b\x7f"));
    assert_eq!(enc(with(key(Key::Tab), SHIFT), m), s("\x1b[Z"));
    // No modifier, no sequence.
    assert_eq!(enc(key(Key::Char('a')), m), s("a"));
    assert_eq!(enc(key(Key::Tab), m), s("\t"));
}

#[test]
fn level_1_sends_control_on_the_other_keys_as_sequences() {
    let m = other(L1);
    // The digits and punctuation that make no control code (xterm's table: `! ? - 0 1 9` among them).
    assert_eq!(enc(with(key(Key::Char('1')), CTRL), m), s("\x1b[27;5;49~"));
    assert_eq!(enc(with(key(Key::Char('0')), CTRL), m), s("\x1b[27;5;48~"));
    assert_eq!(enc(with(key(Key::Char('9')), CTRL), m), s("\x1b[27;5;57~"));
    assert_eq!(enc(with(key(Key::Char('-')), CTRL), m), s("\x1b[27;5;45~"));
    assert_eq!(enc(with(key(Key::Char(',')), CTRL), m), s("\x1b[27;5;44~"));
    // Shift+1 is `!`: Control on it has no code.
    assert_eq!(enc(with(typed('1', "!"), CTRL | SHIFT), m), s("\x1b[27;6;33~"));
    // Control and Shift on `/` is `?`, which is no control code either (xterm's table).
    assert_eq!(enc(with(typed('/', "?"), CTRL | SHIFT), m), s("\x1b[27;6;63~"));
}

#[test]
fn level_1_sends_enter_and_tab_with_any_modifier_as_sequences() {
    let m = other(L1);
    assert_eq!(enc(with(key(Key::Enter), SHIFT), m), s("\x1b[27;2;13~"));
    assert_eq!(enc(with(key(Key::Enter), CTRL), m), s("\x1b[27;5;13~"));
    assert_eq!(enc(with(key(Key::Enter), ALT), m), s("\x1b[27;3;13~"));
    assert_eq!(enc(key(Key::Enter), m), s("\r"));
    assert_eq!(enc(with(key(Key::Tab), CTRL), m), s("\x1b[27;5;9~"));
    assert_eq!(enc(with(key(Key::Tab), CTRL | SHIFT), m), s("\x1b[27;6;9~"));
}

#[test]
fn level_2_sends_every_modified_key_as_a_sequence() {
    let m = other(L2);
    // ctlseqs: "shift-Tab sends CSI 27 ; 2 ; 9 ~ rather than CSI Z".
    assert_eq!(enc(with(key(Key::Tab), SHIFT), m), s("\x1b[27;2;9~"));
    assert_eq!(enc(with(key(Key::Char('a')), SHIFT), m), s("\x1b[27;2;65~"));
    assert_eq!(enc(with(typed('[', "{"), SHIFT), m), s("\x1b[27;2;123~"));
    assert_eq!(enc(with(key(Key::Char(' ')), SHIFT), m), s("\x1b[27;2;32~"));
    assert_eq!(enc(with(key(Key::Char('a')), CTRL), m), s("\x1b[27;5;97~"));
    assert_eq!(enc(with(key(Key::Char('a')), CTRL | SHIFT), m), s("\x1b[27;6;65~"));
    assert_eq!(enc(with(key(Key::Char(' ')), CTRL), m), s("\x1b[27;5;32~"));
    assert_eq!(enc(with(key(Key::Char('[')), CTRL), m), s("\x1b[27;5;91~"));
    assert_eq!(enc(with(key(Key::Char('2')), CTRL), m), s("\x1b[27;5;50~"));
    assert_eq!(enc(with(key(Key::Char('a')), ALT), m), s("\x1b[27;3;97~"));
    assert_eq!(enc(with(key(Key::Escape), CTRL), m), s("\x1b[27;5;27~"));
    assert_eq!(enc(with(key(Key::Escape), SHIFT), m), s("\x1b[27;2;27~"));
    assert_eq!(enc(with(key(Key::Backspace), SHIFT), m), s("\x1b[27;2;127~"));
    assert_eq!(enc(with(key(Key::Backspace), ALT), m), s("\x1b[27;3;127~"));
    // The modifier is 1 + shift + 2 alt + 4 ctrl + 8 meta.
    assert_eq!(enc(with(key(Key::Char('a')), SHIFT | ALT | CTRL | Mods::META), m), s("\x1b[27;16;65~"));
}

#[test]
fn level_2_keeps_what_xterms_table_keeps() {
    let m = other(L2);
    // The symbol Shift types on a digit stays a symbol.
    assert_eq!(enc(with(typed('1', "!"), SHIFT), m), s("!"));
    // Control+Backspace is BS at every level.
    assert_eq!(enc(with(key(Key::Backspace), CTRL), m), s("\x08"));
    // Unmodified keys are as ever, and the other kinds of key are not "other" keys.
    assert_eq!(enc(typed('a', "a"), m), s("a"));
    assert_eq!(enc(key(Key::Enter), m), s("\r"));
    assert_eq!(enc(with(key(Key::Up), CTRL), m), s("\x1b[1;5A"));
    assert_eq!(enc(with(key(Key::F(5)), SHIFT), m), s("\x1b[15;2~"));
    assert_eq!(enc(with(key(Key::Delete), ALT), m), s("\x1b[3;3~"));
    assert_eq!(enc(with(key(Key::Keypad(KeypadKey::Up)), CTRL), m), s("\x1b[1;5A"));
}

#[test]
fn the_backarrow_mode_decides_the_code_of_backspace() {
    let m = InputModes { modify_other_keys: L2, backarrow_bs: true, ..NONE };
    assert_eq!(enc(with(key(Key::Backspace), SHIFT), m), s("\x1b[27;2;8~"));
}

#[test]
fn the_kitty_flags_take_precedence() {
    let m = InputModes { modify_other_keys: L2, kitty: DIS, ..NONE };
    assert_eq!(enc(with(key(Key::Char('a')), CTRL), m), s("\x1b[97;5u"));
}
