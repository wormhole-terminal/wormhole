//! The key encoders against the bytes the specifications give: xterm's ctlseqs for the legacy
//! encoding, the kitty keyboard protocol's page for the rest. Mouse and paste have their tests
//! beside their code; modifyOtherKeys is in the module below.

mod other_keys;

use super::{
    InputModes, Key, KeyEvent, KeyKind, KeypadKey, KittyFlags, MediaKey, ModifierKey, Mods, encode_key,
};

const NONE: InputModes = modes(KittyFlags::empty());

const fn modes(kitty: KittyFlags) -> InputModes {
    // A literal, so the tests can have constants; `non_exhaustive` only binds other crates.
    InputModes {
        app_cursor: false,
        app_keypad: false,
        backarrow_bs: false,
        kitty,
        bracketed_paste: false,
        modify_other_keys: super::ModifyOtherKeys::Off,
        focus_events: false,
        mouse: super::MouseTracking::Off,
        mouse_encoding: super::MouseEncoding::X10,
    }
}

const DIS: KittyFlags = KittyFlags::DISAMBIGUATE;
const EVENTS: KittyFlags = KittyFlags::REPORT_EVENT_TYPES;
const ALTERNATES: KittyFlags = KittyFlags::REPORT_ALTERNATE_KEYS;
const ALL: KittyFlags = KittyFlags::REPORT_ALL_KEYS;
const TEXT: KittyFlags = KittyFlags::REPORT_ASSOCIATED_TEXT;

fn bytes(ev: KeyEvent, modes: InputModes) -> Option<Vec<u8>> {
    let mut out = Vec::new();
    encode_key(&ev, &modes, &mut out).then_some(out)
}
fn enc(ev: KeyEvent, modes: InputModes) -> Option<String> {
    bytes(ev, modes).map(|b| String::from_utf8(b).expect("the tests only expect UTF-8"))
}
fn s(x: &str) -> Option<String> {
    Some(x.to_string())
}
fn key(key: Key) -> KeyEvent<'static> {
    KeyEvent::new(key)
}
/// A character key that typed `text`.
fn typed<'a>(c: char, text: &'a str) -> KeyEvent<'a> {
    KeyEvent::new(Key::Char(c)).with_text(text)
}
fn with(ev: KeyEvent<'static>, mods: Mods) -> KeyEvent<'static> {
    ev.with_mods(mods)
}

const SHIFT: Mods = Mods::SHIFT;
const ALT: Mods = Mods::ALT;
const CTRL: Mods = Mods::CTRL;

// ---------------------------------------------------------------------------
// Legacy xterm encoding
// ---------------------------------------------------------------------------

#[test]
fn legacy_enter_backspace_tab_escape() {
    assert_eq!(enc(key(Key::Enter), NONE), s("\r"));
    // xterm: Alt sends ESC before the key; Shift+Enter is a plain CR (hosts that want more map it).
    assert_eq!(enc(with(key(Key::Enter), ALT), NONE), s("\x1b\r"));
    assert_eq!(enc(with(key(Key::Enter), SHIFT), NONE), s("\r"));
    assert_eq!(enc(key(Key::Backspace), NONE), s("\x7f"));
    assert_eq!(enc(with(key(Key::Backspace), ALT), NONE), s("\x1b\x7f"));
    assert_eq!(enc(with(key(Key::Backspace), CTRL), NONE), s("\x08"));
    assert_eq!(enc(key(Key::Tab), NONE), s("\t"));
    // kcbt, back-tab.
    assert_eq!(enc(with(key(Key::Tab), SHIFT), NONE), s("\x1b[Z"));
    assert_eq!(enc(key(Key::Escape), NONE), s("\x1b"));
}

#[test]
fn legacy_cursor_keys_follow_decckm_and_modifiers() {
    let app = InputModes { app_cursor: true, ..NONE };
    assert_eq!(enc(key(Key::Up), NONE), s("\x1b[A"));
    assert_eq!(enc(key(Key::Down), app), s("\x1bOB"));
    assert_eq!(enc(key(Key::Home), app), s("\x1bOH"));
    assert_eq!(enc(key(Key::End), NONE), s("\x1b[F"));
    // ctlseqs: modifier parameter 2 shift, 3 alt, 4 shift+alt, 5 ctrl, 9 meta; modified keys ignore DECCKM.
    assert_eq!(enc(with(key(Key::Up), SHIFT), NONE), s("\x1b[1;2A"));
    assert_eq!(enc(with(key(Key::Right), CTRL), app), s("\x1b[1;5C"));
    assert_eq!(enc(with(key(Key::Left), SHIFT | ALT), NONE), s("\x1b[1;4D"));
    assert_eq!(enc(with(key(Key::Left), CTRL | ALT), NONE), s("\x1b[1;7D"));
    assert_eq!(enc(with(key(Key::Up), Mods::META), NONE), s("\x1b[1;9A"));
}

#[test]
fn legacy_editing_and_function_keys() {
    assert_eq!(enc(key(Key::Insert), NONE), s("\x1b[2~"));
    assert_eq!(enc(key(Key::Delete), NONE), s("\x1b[3~"));
    assert_eq!(enc(with(key(Key::Delete), SHIFT), NONE), s("\x1b[3;2~"));
    assert_eq!(enc(key(Key::PageUp), NONE), s("\x1b[5~"));
    assert_eq!(enc(key(Key::PageDown), NONE), s("\x1b[6~"));
    assert_eq!(enc(key(Key::F(1)), NONE), s("\x1bOP"));
    assert_eq!(enc(key(Key::F(3)), NONE), s("\x1bOR"));
    assert_eq!(enc(with(key(Key::F(4)), CTRL), NONE), s("\x1b[1;5S"));
    assert_eq!(enc(key(Key::F(5)), NONE), s("\x1b[15~"));
    assert_eq!(enc(key(Key::F(6)), NONE), s("\x1b[17~"));
    assert_eq!(enc(key(Key::F(10)), NONE), s("\x1b[21~"));
    assert_eq!(enc(key(Key::F(11)), NONE), s("\x1b[23~"));
    assert_eq!(enc(with(key(Key::F(12)), ALT), NONE), s("\x1b[24;3~"));
    // No form every program agrees on.
    assert_eq!(enc(key(Key::F(13)), NONE), None);
    assert_eq!(enc(key(Key::F(0)), NONE), None);
}

#[test]
fn legacy_control_combinations() {
    let ctrl = |c| with(key(Key::Char(c)), CTRL);
    assert_eq!(bytes(ctrl('a'), NONE), Some(vec![1]));
    assert_eq!(bytes(ctrl('C'), NONE), Some(vec![3]));
    assert_eq!(bytes(ctrl(' '), NONE), Some(vec![0]));
    assert_eq!(bytes(ctrl('2'), NONE), Some(vec![0]));
    assert_eq!(bytes(ctrl('['), NONE), Some(vec![27]));
    assert_eq!(bytes(ctrl('\\'), NONE), Some(vec![28]));
    assert_eq!(bytes(ctrl(']'), NONE), Some(vec![29]));
    assert_eq!(bytes(ctrl('6'), NONE), Some(vec![30]));
    assert_eq!(bytes(ctrl('-'), NONE), Some(vec![31]));
    assert_eq!(bytes(ctrl('/'), NONE), Some(vec![31]));
    assert_eq!(bytes(ctrl('?'), NONE), Some(vec![127]));
    assert_eq!(bytes(ctrl('~'), NONE), Some(vec![30]));
    assert_eq!(bytes(ctrl('1'), NONE), Some(b"1".to_vec()));
    assert_eq!(bytes(with(key(Key::Char('x')), CTRL | ALT), NONE), Some(vec![0x1b, 24]));
    // No C0 code: the text the host says it typed, or nothing.
    assert_eq!(enc(with(typed('.', "."), CTRL), NONE), s("."));
    assert_eq!(enc(ctrl('.'), NONE), None);
}

#[test]
fn legacy_text_and_alt() {
    assert_eq!(enc(typed('a', "a"), NONE), s("a"));
    assert_eq!(enc(with(typed('a', "A"), SHIFT), NONE), s("A"));
    assert_eq!(enc(typed(' ', " "), NONE), s(" "));
    // The layout composed a character with Option: that is the text (German Option+L is `@`).
    assert_eq!(enc(with(typed('l', "@"), ALT), NONE), s("@"));
    // Option as Meta: Alt without text is ESC and the key.
    assert_eq!(enc(with(key(Key::Char('b')), ALT), NONE), s("\x1bb"));
    assert_eq!(enc(with(key(Key::Char('b')), ALT | SHIFT), NONE), s("\x1bB"));
    assert_eq!(enc(with(key(Key::Char(' ')), ALT), NONE), s("\x1b "));
    assert_eq!(enc(with(key(Key::Char('b')), Mods::META), NONE), s("\x1bb"));
    // A key that typed nothing and has no modifiers types itself.
    assert_eq!(enc(key(Key::Char('a')), NONE), s("a"));
    assert_eq!(enc(with(key(Key::Char('é')), SHIFT), NONE), s("É"));
}

#[test]
fn legacy_backspace_follows_decbkm() {
    // VT520 DECBKM: set, the backarrow key sends BS; reset (the start), DEL. xterm swaps the two
    // for Control.
    let bs = InputModes { backarrow_bs: true, ..NONE };
    assert_eq!(enc(key(Key::Backspace), NONE), s("\x7f"));
    assert_eq!(enc(key(Key::Backspace), bs), s("\x08"));
    assert_eq!(enc(with(key(Key::Backspace), CTRL), bs), s("\x7f"));
    assert_eq!(enc(with(key(Key::Backspace), ALT), bs), s("\x1b\x08"));
}

#[test]
fn legacy_keypad() {
    let kp = |k| key(Key::Keypad(k));
    let app = InputModes { app_keypad: true, ..NONE };
    assert_eq!(enc(kp(KeypadKey::Digit(7)), NONE), s("7"));
    assert_eq!(enc(kp(KeypadKey::Add), NONE), s("+"));
    assert_eq!(enc(kp(KeypadKey::Enter), NONE), s("\r"));
    // ctlseqs, "Numeric Keypad": ESC O p to y for the digits, j k l m n o for * + , - . /, M enter, X equals.
    assert_eq!(enc(kp(KeypadKey::Digit(0)), app), s("\x1bOp"));
    assert_eq!(enc(kp(KeypadKey::Digit(9)), app), s("\x1bOy"));
    assert_eq!(enc(kp(KeypadKey::Multiply), app), s("\x1bOj"));
    assert_eq!(enc(kp(KeypadKey::Add), app), s("\x1bOk"));
    assert_eq!(enc(kp(KeypadKey::Subtract), app), s("\x1bOm"));
    assert_eq!(enc(kp(KeypadKey::Decimal), app), s("\x1bOn"));
    assert_eq!(enc(kp(KeypadKey::Divide), app), s("\x1bOo"));
    assert_eq!(enc(kp(KeypadKey::Enter), app), s("\x1bOM"));
    assert_eq!(enc(kp(KeypadKey::Equal), app), s("\x1bOX"));
    // The navigation keys of the keypad are the cursor block's.
    assert_eq!(enc(kp(KeypadKey::Left), NONE), s("\x1b[D"));
    assert_eq!(enc(kp(KeypadKey::PageDown), NONE), s("\x1b[6~"));
    assert_eq!(enc(kp(KeypadKey::Digit(10)), NONE), None);
}

#[test]
fn legacy_has_no_releases_and_no_keys_without_bytes() {
    assert_eq!(enc(key(Key::Up).with_kind(KeyKind::Release), NONE), None);
    assert_eq!(enc(key(Key::Up).with_kind(KeyKind::Repeat), NONE), s("\x1b[A"));
    assert_eq!(enc(key(Key::Media(MediaKey::Stop)), NONE), None);
    assert_eq!(enc(key(Key::Modifier(ModifierKey::LeftShift)), NONE), None);
    assert_eq!(enc(key(Key::CapsLock), NONE), None);
    // Lock states are not a modifier in this encoding.
    assert_eq!(enc(with(key(Key::Up), Mods::CAPS_LOCK | Mods::NUM_LOCK), NONE), s("\x1b[A"));
}

// ---------------------------------------------------------------------------
// The kitty keyboard protocol
// ---------------------------------------------------------------------------

#[test]
fn kitty_disambiguate_escape_and_modified_keys() {
    let m = modes(DIS);
    assert_eq!(enc(key(Key::Escape), m), s("\x1b[27u"));
    assert_eq!(enc(with(key(Key::Escape), ALT), m), s("\x1b[27;3u"));
    // Enter, Tab and Backspace stay legacy when unmodified.
    assert_eq!(enc(key(Key::Enter), m), s("\r"));
    assert_eq!(enc(key(Key::Tab), m), s("\t"));
    assert_eq!(enc(key(Key::Backspace), m), s("\x7f"));
    assert_eq!(enc(with(key(Key::Enter), SHIFT), m), s("\x1b[13;2u"));
    assert_eq!(enc(with(key(Key::Enter), ALT), m), s("\x1b[13;3u"));
    assert_eq!(enc(with(key(Key::Enter), CTRL), m), s("\x1b[13;5u"));
    assert_eq!(enc(with(key(Key::Tab), SHIFT), m), s("\x1b[9;2u"));
    assert_eq!(enc(with(key(Key::Backspace), CTRL), m), s("\x1b[127;5u"));
}

#[test]
fn kitty_disambiguate_text_keys() {
    let m = modes(DIS);
    assert_eq!(enc(typed('a', "a"), m), s("a"));
    assert_eq!(enc(with(typed('a', "A"), SHIFT), m), s("A"));
    assert_eq!(enc(typed(' ', " "), m), s(" "));
    let ctrl = |c| with(key(Key::Char(c)), CTRL);
    assert_eq!(enc(ctrl('a'), m), s("\x1b[97;5u"));
    // The key code is the unshifted, lower-case key.
    assert_eq!(enc(ctrl('A'), m), s("\x1b[97;5u"));
    assert_eq!(enc(with(key(Key::Char('a')), CTRL | SHIFT), m), s("\x1b[97;6u"));
    assert_eq!(enc(ctrl(' '), m), s("\x1b[32;5u"));
    assert_eq!(enc(ctrl('['), m), s("\x1b[91;5u"));
    assert_eq!(enc(with(key(Key::Char('x')), CTRL | ALT), m), s("\x1b[120;7u"));
    assert_eq!(enc(with(key(Key::Char('b')), ALT), m), s("\x1b[98;3u"));
    assert_eq!(enc(with(key(Key::Char('a')), Mods::SUPER), m), s("\x1b[97;9u"));
    assert_eq!(enc(with(key(Key::Char('a')), Mods::HYPER | Mods::META), m), s("\x1b[97;49u"));
    // Shift alone without text is reported.
    assert_eq!(enc(with(key(Key::Char('a')), SHIFT), m), s("\x1b[97;2u"));
}

#[test]
fn kitty_functional_keys() {
    let m = modes(DIS);
    // Unmodified and unambiguous: legacy bytes, DECCKM honoured.
    assert_eq!(enc(key(Key::Up), m), s("\x1b[A"));
    assert_eq!(enc(key(Key::Up), InputModes { app_cursor: true, ..m }), s("\x1bOA"));
    assert_eq!(enc(key(Key::F(1)), m), s("\x1bOP"));
    assert_eq!(enc(key(Key::Delete), m), s("\x1b[3~"));
    // Modified: the protocol's own forms, no Option word jump.
    assert_eq!(enc(with(key(Key::Left), ALT), m), s("\x1b[1;3D"));
    assert_eq!(enc(with(key(Key::Home), SHIFT), m), s("\x1b[1;2H"));
    assert_eq!(enc(with(key(Key::PageUp), CTRL), m), s("\x1b[5;5~"));
    assert_eq!(enc(with(key(Key::F(3)), CTRL), m), s("\x1b[13;5~"));
    assert_eq!(enc(with(key(Key::F(1)), SHIFT), m), s("\x1b[1;2P"));
    assert_eq!(enc(with(key(Key::F(12)), SHIFT), m), s("\x1b[24;2~"));
    // F13 to F35 are in the private use area; no legacy form.
    assert_eq!(enc(key(Key::F(13)), m), s("\x1b[57376u"));
    assert_eq!(enc(with(key(Key::F(20)), SHIFT), m), s("\x1b[57383;2u"));
    assert_eq!(enc(key(Key::F(35)), m), s("\x1b[57398u"));
    assert_eq!(enc(key(Key::F(36)), m), None);
}

#[test]
fn kitty_keys_without_a_legacy_form() {
    let m = modes(DIS);
    assert_eq!(enc(key(Key::CapsLock), m), s("\x1b[57358u"));
    assert_eq!(enc(key(Key::ScrollLock), m), s("\x1b[57359u"));
    assert_eq!(enc(key(Key::NumLock), m), s("\x1b[57360u"));
    assert_eq!(enc(key(Key::PrintScreen), m), s("\x1b[57361u"));
    assert_eq!(enc(key(Key::Pause), m), s("\x1b[57362u"));
    assert_eq!(enc(key(Key::Menu), m), s("\x1b[57363u"));
    assert_eq!(enc(key(Key::Media(MediaKey::Play)), m), s("\x1b[57428u"));
    assert_eq!(enc(key(Key::Media(MediaKey::PlayPause)), m), s("\x1b[57430u"));
    assert_eq!(enc(key(Key::Media(MediaKey::MuteVolume)), m), s("\x1b[57440u"));
}

#[test]
fn kitty_keypad() {
    let m = modes(DIS);
    let kp = |k| key(Key::Keypad(k));
    // A keypad key that typed text is that text, like any text key ...
    assert_eq!(enc(kp(KeypadKey::Digit(5)).with_text("5"), m), s("5"));
    // ... and its own code when it did not, or with a modifier, or when all keys are reported.
    assert_eq!(enc(kp(KeypadKey::Digit(5)), m), s("\x1b[57404u"));
    assert_eq!(enc(with(kp(KeypadKey::Digit(5)).with_text("5"), CTRL), m), s("\x1b[57404;5u"));
    assert_eq!(enc(kp(KeypadKey::Digit(0)).with_text("0"), modes(DIS | ALL)), s("\x1b[57399u"));
    assert_eq!(enc(kp(KeypadKey::Digit(0)).with_text("0"), modes(DIS | ALL | TEXT)), s("\x1b[57399;1;48u"));
    assert_eq!(enc(kp(KeypadKey::Enter).with_text("\r"), m), s("\x1b[57414u"));
    assert_eq!(enc(kp(KeypadKey::Left), m), s("\x1b[57417u"));
    assert_eq!(enc(kp(KeypadKey::Begin), m), s("\x1b[E"));
    assert_eq!(enc(kp(KeypadKey::Delete), m), s("\x1b[57426u"));
}

#[test]
fn kitty_report_all_keys() {
    let m = modes(DIS | ALL);
    assert_eq!(enc(typed('a', "a"), m), s("\x1b[97u"));
    assert_eq!(enc(with(typed('a', "A"), SHIFT), m), s("\x1b[97;2u"));
    assert_eq!(enc(key(Key::Enter).with_text("\r"), m), s("\x1b[13u"));
    assert_eq!(enc(key(Key::Tab), m), s("\x1b[9u"));
    assert_eq!(enc(key(Key::Backspace), m), s("\x1b[127u"));
    assert_eq!(enc(key(Key::Escape), m), s("\x1b[27u"));
    // Cursor keys lose DECCKM: the escape code is the same in both states.
    assert_eq!(enc(key(Key::Up), InputModes { app_cursor: true, ..m }), s("\x1b[A"));
    assert_eq!(enc(key(Key::F(1)), m), s("\x1b[P"));
    assert_eq!(enc(key(Key::Insert), m), s("\x1b[2~"));
    assert_eq!(enc(typed(' ', " "), m), s("\x1b[32u"));
    // Flag 8 works without flag 1.
    assert_eq!(enc(typed('z', "z"), modes(ALL)), s("\x1b[122u"));
}

#[test]
fn kitty_modifier_keys_need_report_all() {
    let shift = key(Key::Modifier(ModifierKey::LeftShift)).with_mods(SHIFT);
    assert_eq!(enc(shift, modes(DIS)), None);
    assert_eq!(enc(shift, modes(DIS | ALL)), s("\x1b[57441;2u"));
    let ctrl = key(Key::Modifier(ModifierKey::RightControl)).with_mods(CTRL);
    assert_eq!(enc(ctrl, modes(DIS | ALL)), s("\x1b[57448;5u"));
    let release = shift.with_kind(KeyKind::Release);
    assert_eq!(enc(release, modes(DIS | ALL | EVENTS)), s("\x1b[57441;2:3u"));
}

#[test]
fn kitty_alternate_keys() {
    let m = modes(DIS | ALTERNATES);
    // Shift+text without flag 8 is plain text.
    assert_eq!(enc(with(typed('a', "A"), SHIFT), m), s("A"));
    assert_eq!(enc(with(key(Key::Char('a')), CTRL | SHIFT), m), s("\x1b[97:65;6u"));
    let all = modes(DIS | ALTERNATES | ALL);
    assert_eq!(enc(with(typed('a', "A"), SHIFT), all), s("\x1b[97:65;2u"));
    // The host can say what Shift types where the key's upper case is not it: Shift+1 is `!`.
    assert_eq!(enc(with(key(Key::Char('1')), SHIFT).with_shifted('!'), all), s("\x1b[49:33;2u"));
    // A host that gives an already shifted key reports no alternate.
    assert_eq!(enc(typed('!', "!"), all), s("\x1b[33u"));
    assert_eq!(enc(typed('a', "a"), all), s("\x1b[97u"));
    // Shift on a key with no shifted form: no alternate.
    assert_eq!(enc(with(typed(' ', " "), SHIFT), all), s("\x1b[32;2u"));
    // Without flag 4 the alternates stay out.
    assert_eq!(enc(with(typed('a', "A"), SHIFT), modes(DIS | ALL)), s("\x1b[97;2u"));
}

#[test]
fn kitty_base_layout_key() {
    // The protocol's example: ctrl+ф on a Russian layout is ctrl+a on the US one.
    let m = modes(DIS | ALTERNATES);
    let cyrillic = with(key(Key::Char('ф')), CTRL).with_base_layout('a');
    assert_eq!(enc(cyrillic, m), s("\x1b[1092::97;5u"));
    assert_eq!(enc(with(cyrillic, CTRL | SHIFT), m), s("\x1b[1092:1060:97;6u"));
    assert_eq!(enc(cyrillic, modes(DIS)), s("\x1b[1092;5u"));
    // The same key on both layouts says nothing extra.
    assert_eq!(enc(with(key(Key::Char('a')), CTRL).with_base_layout('a'), m), s("\x1b[97;5u"));
}

#[test]
fn kitty_associated_text() {
    let m = modes(DIS | ALL | TEXT);
    assert_eq!(enc(typed('a', "a"), m), s("\x1b[97;1;97u"));
    assert_eq!(enc(with(typed('a', "A"), SHIFT), m), s("\x1b[97;2;65u"));
    // The layout composed a character: the key is not Alt (the host leaves it out) and the text is there.
    assert_eq!(enc(typed('l', "@"), m), s("\x1b[108;1;64u"));
    // Several code points are separated by colons.
    assert_eq!(enc(typed('e', "e\u{301}"), m), s("\x1b[101;1;101:769u"));
    // Control combinations, functional keys and control characters carry no text.
    assert_eq!(enc(with(typed('a', "\x01"), CTRL), m), s("\x1b[97;5u"));
    assert_eq!(enc(key(Key::Enter).with_text("\r"), m), s("\x1b[13u"));
    assert_eq!(enc(typed('a', "\x07"), m), s("\x1b[97u"));
    assert_eq!(
        enc(with(typed('a', "A"), SHIFT), modes(DIS | ALL | TEXT | ALTERNATES)),
        s("\x1b[97:65;2;65u")
    );
    // Without flag 8 there is nothing to attach it to.
    assert_eq!(enc(typed('a', "a"), modes(DIS | TEXT)), s("a"));
}

#[test]
fn kitty_event_types() {
    // Press is the default type and carries no `:1`.
    let flag_only = modes(EVENTS);
    assert_eq!(enc(key(Key::Escape), flag_only), s("\x1b[27u"));
    assert_eq!(enc(with(key(Key::Enter), SHIFT), flag_only), s("\x1b[13;2u"));
    assert_eq!(enc(typed('a', "a"), flag_only), s("a"));

    let m = modes(DIS | EVENTS);
    let release = |ev: KeyEvent<'static>| ev.with_kind(KeyKind::Release);
    let repeat = |ev: KeyEvent<'static>| ev.with_kind(KeyKind::Repeat);
    // 2 is a repeat, 3 a release, after the modifiers.
    assert_eq!(enc(release(key(Key::Escape)), m), s("\x1b[27;1:3u"));
    assert_eq!(enc(repeat(key(Key::Escape)), m), s("\x1b[27;1:2u"));
    assert_eq!(enc(release(with(key(Key::Char('a')), CTRL)), m), s("\x1b[97;5:3u"));
    // Keys that have a legacy form need the escape code to carry the type.
    assert_eq!(enc(release(key(Key::Up)), m), s("\x1b[1;1:3A"));
    assert_eq!(enc(repeat(key(Key::Up)), m), s("\x1b[1;1:2A"));
    assert_eq!(enc(release(key(Key::Delete)), m), s("\x1b[3;1:3~"));
    // Text keys produce text, which has no events: a repeat types again, a release is silent.
    assert_eq!(enc(repeat(typed('a', "a")), m), s("a"));
    assert_eq!(enc(release(typed('a', "a")), m), None);
    // Enter, Tab and Backspace have no release unless all keys are reported.
    assert_eq!(enc(release(key(Key::Enter)), m), None);
    assert_eq!(enc(repeat(key(Key::Enter)), m), s("\r"));
    assert_eq!(enc(release(key(Key::Enter)), modes(DIS | EVENTS | ALL)), s("\x1b[13;1:3u"));
    // With all keys reported, text keys have events too, and a release has no text.
    let all = modes(DIS | EVENTS | ALL | TEXT);
    assert_eq!(enc(repeat(typed('a', "a")), all), s("\x1b[97;1:2;97u"));
    assert_eq!(enc(release(typed('a', "a")), all), s("\x1b[97;1:3u"));
    assert_eq!(enc(release(with(key(Key::Char('a')), SHIFT)), all), s("\x1b[97;2:3u"));
    // Without flag 2 there are only presses.
    assert_eq!(enc(release(key(Key::Escape)), modes(DIS)), None);
    assert_eq!(enc(repeat(key(Key::Escape)), modes(DIS)), s("\x1b[27u"));
}

#[test]
fn kitty_lock_modifiers() {
    let m = modes(DIS);
    let locks = Mods::CAPS_LOCK | Mods::NUM_LOCK;
    // 1 + ctrl 4 + caps_lock 64 = 69, 1 + 4 + 128 = 133.
    assert_eq!(enc(with(key(Key::Char('a')), CTRL | Mods::CAPS_LOCK), m), s("\x1b[97;69u"));
    assert_eq!(enc(with(key(Key::Char('a')), CTRL | Mods::NUM_LOCK), m), s("\x1b[97;133u"));
    // "The Lock modifiers are not reported for text producing keys" and do not make a plain
    // cursor key an escape code.
    assert_eq!(enc(with(typed('a', "A"), locks), m), s("A"));
    assert_eq!(enc(with(key(Key::Up), locks), m), s("\x1b[A"));
    assert_eq!(enc(with(key(Key::Up), locks | SHIFT), m), s("\x1b[1;194A"));
    assert_eq!(enc(with(key(Key::Escape), Mods::NUM_LOCK), m), s("\x1b[27;129u"));
}
