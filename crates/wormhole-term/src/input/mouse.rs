//! Mouse events -> xterm mouse reporting sequences.
//!
//! The reports are `wormhole_vt::input::encode_mouse` (tracking modes 9, 1000, 1002 and 1003; the
//! encodings X10, UTF-8, URXVT, SGR and SGR-pixels); this maps the app's pointer structs onto the
//! engine's.

use wormhole_vt::Modes;
use wormhole_vt::input::MouseEncoding;
use wormhole_vt::input::{self as vt, InputModes, Mods, encode_mouse};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    WheelUp,
    WheelDown,
    WheelLeft,
    WheelRight,
    /// Motion without any button held.
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseAction {
    Press,
    Release,
    Motion,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MouseInput {
    pub button: MouseButton,
    pub action: MouseAction,
    /// 0-based viewport column.
    pub col: usize,
    /// 0-based viewport row.
    pub row: usize,
    pub shift: bool,
    pub alt: bool,
    pub control: bool,
    /// 0-based pixels from the text area's corner, for a program that asked for SGR-pixels (1016,
    /// see [`wants_pixels`]); without it a report falls back to the cell.
    pub pixel: Option<(u32, u32)>,
}

/// Where the pointer is for the DEC locator (DECELR): the cell, and the pixel inside the text area.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LocatorPointer {
    /// 0-based viewport column.
    pub col: usize,
    /// 0-based viewport row.
    pub row: usize,
    /// 0-based pixels from the text area's left edge.
    pub x: u32,
    /// 0-based pixels from the text area's top edge.
    pub y: u32,
}

/// Whether a program asked for the pointer as a DEC locator (it then gets
/// [`Pty::locator`](crate::Pty::locator) events instead of mouse reports).
pub fn wants_locator(mode: Modes) -> bool {
    mode.contains(Modes::LOCATOR)
}

/// Whether the application has requested any kind of mouse reporting.
pub fn wants_mouse(mode: Modes) -> bool {
    InputModes::from(mode).reports_mouse()
}

/// Whether the program asked for SGR reports in pixels (1016): events then need their pixel.
pub fn wants_pixels(mode: Modes) -> bool {
    mode.mouse_encoding() == MouseEncoding::SgrPixels
}

/// Encode a mouse event, or `None` when the current mode does not report it.
pub fn encode(input: &MouseInput, mode: Modes) -> Option<Vec<u8>> {
    let button = match input.button {
        MouseButton::Left => vt::MouseButton::Left,
        MouseButton::Middle => vt::MouseButton::Middle,
        MouseButton::Right => vt::MouseButton::Right,
        MouseButton::WheelUp => vt::MouseButton::WheelUp,
        MouseButton::WheelDown => vt::MouseButton::WheelDown,
        MouseButton::WheelLeft => vt::MouseButton::WheelLeft,
        MouseButton::WheelRight => vt::MouseButton::WheelRight,
        MouseButton::None => vt::MouseButton::None,
    };
    let kind = match input.action {
        MouseAction::Press => vt::MouseKind::Press,
        MouseAction::Release => vt::MouseKind::Release,
        MouseAction::Motion => vt::MouseKind::Motion,
    };
    let mut mods = Mods::empty();
    mods.set(Mods::SHIFT, input.shift);
    mods.set(Mods::ALT, input.alt);
    mods.set(Mods::CTRL, input.control);
    // A position past u32 is past every encoding; the engine refuses it where the form has a limit.
    let cell = |n: usize| u32::try_from(n).unwrap_or(u32::MAX);
    let mut event = vt::MouseEvent::new(button, kind, cell(input.col), cell(input.row)).with_mods(mods);
    if let Some((x, y)) = input.pixel {
        event = event.with_pixel(x, y);
    }
    let mut out = Vec::new();
    encode_mouse(&event, &InputModes::from(mode), &mut out).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLICK: Modes = Modes::MOUSE_CLICK;
    const DRAG: Modes = Modes::MOUSE_DRAG;
    const MOTION: Modes = Modes::MOUSE_MOTION;
    const SGR: Modes = Modes::MOUSE_SGR;
    const UTF8: Modes = Modes::MOUSE_UTF8;

    fn ev(button: MouseButton, action: MouseAction, col: usize, row: usize) -> MouseInput {
        MouseInput { button, action, col, row, shift: false, alt: false, control: false, pixel: None }
    }
    use MouseAction::*;
    use MouseButton as B;

    #[test]
    fn wants_mouse_modes() {
        assert!(!wants_mouse(Modes::empty()));
        assert!(!wants_mouse(SGR | UTF8));
        assert!(wants_mouse(CLICK));
        assert!(wants_mouse(DRAG));
        assert!(wants_mouse(MOTION));
    }

    #[test]
    fn nothing_without_tracking() {
        assert_eq!(encode(&ev(B::Left, Press, 0, 0), SGR), None);
        assert_eq!(encode(&ev(B::Left, Press, 0, 0), Modes::empty()), None);
    }

    #[test]
    fn x10_press_release() {
        assert_eq!(encode(&ev(B::Left, Press, 0, 0), CLICK), Some(b"\x1b[M !!".to_vec()));
        assert_eq!(encode(&ev(B::Right, Press, 9, 4), CLICK), Some(b"\x1b[M\"*%".to_vec()));
        assert_eq!(encode(&ev(B::Middle, Release, 0, 0), CLICK), Some(b"\x1b[M#!!".to_vec()));
    }

    #[test]
    fn x10_modifiers() {
        let mut e = ev(B::Left, Press, 0, 0);
        e.shift = true;
        e.alt = true;
        e.control = true;
        assert_eq!(encode(&e, CLICK), Some(vec![0x1b, b'[', b'M', 32 + 28, 33, 33]));
        e.action = Release;
        assert_eq!(encode(&e, CLICK), Some(vec![0x1b, b'[', b'M', 32 + 31, 33, 33]));
    }

    #[test]
    fn x10_coordinate_cap() {
        assert_eq!(encode(&ev(B::Left, Press, 222, 222), CLICK), Some(vec![0x1b, b'[', b'M', 32, 255, 255]));
        assert_eq!(encode(&ev(B::Left, Press, 223, 0), CLICK), None);
        assert_eq!(encode(&ev(B::Left, Press, 0, 223), CLICK), None);
    }

    #[test]
    fn wheel() {
        assert_eq!(encode(&ev(B::WheelUp, Press, 0, 0), CLICK), Some(vec![0x1b, b'[', b'M', 96, 33, 33]));
        assert_eq!(encode(&ev(B::WheelDown, Press, 1, 2), SGR | CLICK), Some(b"\x1b[<65;2;3M".to_vec()));
        assert_eq!(encode(&ev(B::WheelLeft, Press, 0, 0), SGR | CLICK), Some(b"\x1b[<66;1;1M".to_vec()));
        assert_eq!(encode(&ev(B::WheelRight, Press, 0, 0), SGR | CLICK), Some(b"\x1b[<67;1;1M".to_vec()));
        assert_eq!(encode(&ev(B::WheelUp, Release, 0, 0), SGR | CLICK), None);
        assert_eq!(encode(&ev(B::WheelUp, Motion, 0, 0), SGR | MOTION), None);
    }

    #[test]
    fn sgr_encoding() {
        let m = SGR | CLICK;
        assert_eq!(encode(&ev(B::Left, Press, 0, 0), m), Some(b"\x1b[<0;1;1M".to_vec()));
        assert_eq!(encode(&ev(B::Right, Release, 299, 99), m), Some(b"\x1b[<2;300;100m".to_vec()));
        // No coordinate cap in SGR.
        assert_eq!(encode(&ev(B::Left, Press, 1000, 500), m), Some(b"\x1b[<0;1001;501M".to_vec()));
        let mut e = ev(B::Middle, Press, 4, 5);
        e.control = true;
        e.shift = true;
        assert_eq!(encode(&e, m), Some(b"\x1b[<21;5;6M".to_vec()));
    }

    #[test]
    fn motion_filtering() {
        let drag = ev(B::Left, Motion, 2, 3);
        let hover = ev(B::None, Motion, 2, 3);
        // Click-only: no motion at all.
        assert_eq!(encode(&drag, SGR | CLICK), None);
        assert_eq!(encode(&hover, SGR | CLICK), None);
        // Button-event tracking: drags only.
        assert_eq!(encode(&drag, SGR | DRAG), Some(b"\x1b[<32;3;4M".to_vec()));
        assert_eq!(encode(&hover, SGR | DRAG), None);
        // Any-event tracking: both.
        assert_eq!(encode(&drag, SGR | MOTION), Some(b"\x1b[<32;3;4M".to_vec()));
        assert_eq!(encode(&hover, SGR | MOTION), Some(b"\x1b[<35;3;4M".to_vec()));
        assert_eq!(encode(&ev(B::Right, Motion, 0, 0), DRAG), Some(vec![0x1b, b'[', b'M', 32 + 34, 33, 33]));
        // Clicks are still reported in drag/motion modes.
        assert_eq!(encode(&ev(B::Left, Press, 0, 0), SGR | MOTION), Some(b"\x1b[<0;1;1M".to_vec()));
    }

    #[test]
    fn the_older_and_the_newer_forms_the_engine_reads() {
        // X10 compatibility (9): presses only, no modifiers.
        let x10 = Modes::MOUSE_X10;
        assert!(wants_mouse(x10));
        assert_eq!(encode(&ev(B::Left, Press, 0, 0), x10), Some(b"\x1b[M !!".to_vec()));
        assert_eq!(encode(&ev(B::Left, Release, 0, 0), x10), None);
        // URXVT (1015): decimal numbers, the button offset by 32.
        let urxvt = Modes::MOUSE_URXVT | CLICK;
        assert_eq!(encode(&ev(B::Left, Press, 299, 99), urxvt), Some(b"\x1b[32;300;100M".to_vec()));
    }

    #[test]
    fn sgr_pixels_report_the_pixel_and_fall_back_to_the_cell() {
        let m = Modes::MOUSE_PIXELS | CLICK;
        assert!(wants_pixels(m) && !wants_pixels(SGR | CLICK));
        let mut e = ev(B::Left, Press, 2, 3);
        assert_eq!(encode(&e, m), Some(b"\x1b[<0;3;4M".to_vec()), "no pixel known: the cell");
        e.pixel = Some((25, 61));
        assert_eq!(encode(&e, m), Some(b"\x1b[<0;25;61M".to_vec()), "the pixel, as the engine writes it");
        e.action = Release;
        assert_eq!(encode(&e, m), Some(b"\x1b[<0;25;61m".to_vec()));
    }

    #[test]
    fn invalid_combinations() {
        assert_eq!(encode(&ev(B::None, Press, 0, 0), SGR | MOTION), None);
        assert_eq!(encode(&ev(B::None, Release, 0, 0), SGR | MOTION), None);
    }

    #[test]
    fn utf8_encoding() {
        let m = UTF8 | CLICK;
        // Small values are plain bytes.
        assert_eq!(encode(&ev(B::Left, Press, 0, 0), m), Some(b"\x1b[M !!".to_vec()));
        // Column 300 -> 301 + 32 = 333 = U+014D -> C5 8D.
        assert_eq!(encode(&ev(B::Left, Press, 300, 0), m), Some(vec![0x1b, b'[', b'M', 32, 0xC5, 0x8D, 33]));
        // Release is button 3.
        assert_eq!(encode(&ev(B::Left, Release, 0, 0), m), Some(b"\x1b[M#!!".to_vec()));
        // Max representable: 2047 - 32 = 2015 (1-based) -> col 2014.
        assert!(encode(&ev(B::Left, Press, 2014, 0), m).is_some());
        assert_eq!(encode(&ev(B::Left, Press, 2015, 0), m), None);
    }

    #[test]
    fn sgr_wins_over_utf8() {
        assert_eq!(encode(&ev(B::Left, Press, 0, 0), SGR | UTF8 | CLICK), Some(b"\x1b[<0;1;1M".to_vec()));
    }
}
