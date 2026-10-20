//! Mouse events -> xterm mouse reporting sequences.
//!
//! Supports the tracking modes 1000 (`MOUSE_REPORT_CLICK`), 1002
//! (`MOUSE_DRAG`) and 1003 (`MOUSE_MOTION`), and the encodings X10 (default),
//! UTF-8 (1005, `UTF8_MOUSE`) and SGR (1006, `SGR_MOUSE`).

use wormhole_vt::TermMode;

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
pub fn wants_locator(mode: TermMode) -> bool {
    mode.contains(TermMode::LOCATOR)
}

/// Whether the application has requested any kind of mouse reporting.
pub fn wants_mouse(mode: TermMode) -> bool {
    mode.intersects(TermMode::MOUSE_MODE)
}

/// Encode a mouse event, or `None` when the current mode does not report it.
pub fn encode(input: &MouseInput, mode: TermMode) -> Option<Vec<u8>> {
    if !wants_mouse(mode) {
        return None;
    }
    let drag = mode.intersects(TermMode::MOUSE_DRAG | TermMode::MOUSE_MOTION);
    let any_motion = mode.contains(TermMode::MOUSE_MOTION);

    let base: u32 = match input.button {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
        MouseButton::WheelUp => 64,
        MouseButton::WheelDown => 65,
        MouseButton::WheelLeft => 66,
        MouseButton::WheelRight => 67,
        MouseButton::None => 3,
    };
    let is_wheel = base >= 64;

    let code = match input.action {
        MouseAction::Press => {
            if input.button == MouseButton::None {
                return None;
            }
            base
        }
        MouseAction::Release => {
            // Wheel "buttons" have no release; release of nothing is meaningless.
            if is_wheel || input.button == MouseButton::None {
                return None;
            }
            base
        }
        MouseAction::Motion => {
            if is_wheel {
                return None;
            }
            let allowed = if input.button == MouseButton::None { any_motion } else { drag };
            if !allowed {
                return None;
            }
            base + 32
        }
    };
    let code = code + 4 * input.shift as u32 + 8 * input.alt as u32 + 16 * input.control as u32;

    // 1-based coordinates.
    let x = input.col as u32 + 1;
    let y = input.row as u32 + 1;

    if mode.contains(TermMode::SGR_MOUSE) {
        let fin = if input.action == MouseAction::Release { 'm' } else { 'M' };
        return Some(format!("\x1b[<{code};{x};{y}{fin}").into_bytes());
    }

    // Normal/UTF-8 encodings can't express which button was released.
    let code = if input.action == MouseAction::Release { 3 + (code & !3) } else { code };

    let mut out = b"\x1b[M".to_vec();
    if mode.contains(TermMode::UTF8_MOUSE) {
        for v in [code, x, y] {
            // Values are offset by 32 and encoded as UTF-8 up to U+07FF.
            let c = char::from_u32(v + 32).filter(|c| (*c as u32) < 0x800)?;
            let mut buf = [0; 4];
            out.extend_from_slice(c.encode_utf8(&mut buf).as_bytes());
        }
    } else {
        // X10: one byte each; coordinates cap at 223 (255 - 32). Events outside
        // that range can't be represented and are dropped.
        if x > 223 || y > 223 {
            return None;
        }
        out.extend([(code + 32) as u8, (x + 32) as u8, (y + 32) as u8]);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLICK: TermMode = TermMode::MOUSE_REPORT_CLICK;
    const DRAG: TermMode = TermMode::MOUSE_DRAG;
    const MOTION: TermMode = TermMode::MOUSE_MOTION;
    const SGR: TermMode = TermMode::SGR_MOUSE;
    const UTF8: TermMode = TermMode::UTF8_MOUSE;

    fn ev(button: MouseButton, action: MouseAction, col: usize, row: usize) -> MouseInput {
        MouseInput { button, action, col, row, shift: false, alt: false, control: false }
    }
    use MouseAction::*;
    use MouseButton as B;

    #[test]
    fn wants_mouse_modes() {
        assert!(!wants_mouse(TermMode::empty()));
        assert!(!wants_mouse(SGR | UTF8));
        assert!(wants_mouse(CLICK));
        assert!(wants_mouse(DRAG));
        assert!(wants_mouse(MOTION));
    }

    #[test]
    fn nothing_without_tracking() {
        assert_eq!(encode(&ev(B::Left, Press, 0, 0), SGR), None);
        assert_eq!(encode(&ev(B::Left, Press, 0, 0), TermMode::empty()), None);
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
