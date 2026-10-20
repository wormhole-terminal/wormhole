//! The input encoders never allocate: a key, a pointer event, a paste or a focus report is written
//! into a buffer the caller already has, in every encoding.
//!
//! Its own test binary, because the allocator is global and a test elsewhere must not count here.

use wormhole_vt::input::{
    InputModes, Key, KeyEvent, KeyKind, KittyFlags, Mods, MouseButton, MouseEncoding, MouseEvent, MouseKind,
    MouseTracking, encode_key, encode_mouse, focus, paste,
};

#[path = "common/mod.rs"]
mod common;
use common::alloc::{Counting, allocations};

#[global_allocator]
static ALLOCATOR: Counting = Counting;

// Outside the crate `InputModes` can only be filled in field by field.
#[allow(clippy::field_reassign_with_default)]
#[test]
fn encoding_into_a_buffer_with_room_allocates_nothing() {
    let mut out = Vec::with_capacity(64 * 1024);
    let mut bracketed = InputModes::default();
    bracketed.bracketed_paste = true;
    bracketed.focus_events = true;
    let mut kitty = InputModes::default();
    kitty.kitty = KittyFlags::all();
    let mut mouse = InputModes::default();
    mouse.mouse = MouseTracking::Motion;

    let text = "echo hi\r\n\x1b[201~rm -rf ~\u{9b}\n".repeat(50);
    let keys = [
        KeyEvent::new(Key::Char('a')).with_text("a"),
        KeyEvent::new(Key::Char('c')).with_mods(Mods::CTRL),
        KeyEvent::new(Key::Up).with_mods(Mods::SHIFT | Mods::ALT),
        KeyEvent::new(Key::F(5)),
        KeyEvent::new(Key::F(20)).with_kind(KeyKind::Release),
        KeyEvent::new(Key::Char('ф')).with_mods(Mods::CTRL | Mods::SHIFT).with_base_layout('a'),
        KeyEvent::new(Key::Char('e')).with_text("e\u{301}"),
    ];

    let before = allocations();
    for round in 0..100 {
        out.clear();
        for key in &keys {
            encode_key(key, &InputModes::default(), &mut out);
            encode_key(key, &kitty, &mut out);
        }
        for encoding in [
            MouseEncoding::X10,
            MouseEncoding::Utf8,
            MouseEncoding::Urxvt,
            MouseEncoding::Sgr,
            MouseEncoding::SgrPixels,
        ] {
            mouse.mouse_encoding = encoding;
            let event =
                MouseEvent::new(MouseButton::Left, MouseKind::Press, round, 3 * round).with_pixel(9, 9);
            encode_mouse(&event, &mouse, &mut out);
        }
        paste(&text, &bracketed, &mut out);
        paste(&text, &InputModes::default(), &mut out);
        focus(true, &bracketed, &mut out);
    }
    assert_eq!(allocations() - before, 0, "the encoders allocated");
    assert!(!out.is_empty());
}
