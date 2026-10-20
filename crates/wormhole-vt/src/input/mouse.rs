//! Pointer events as xterm mouse reports.
//!
//! Sources: xterm's "Mouse Tracking" in ctlseqs
//! (<https://invisible-island.net/xterm/ctlseqs/ctlseqs.html#h2-Mouse-Tracking>) for the X10, UTF-8
//! (1005), SGR (1006) and URXVT (1015) forms and the SGR-pixels extension (1016).

use super::key::Mods;
use super::modes::{InputModes, MouseEncoding, MouseTracking};
use super::{ESC, push_dec};

/// A pointer button.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MouseButton {
    /// The primary button.
    Left,
    /// The wheel button.
    Middle,
    /// The secondary button.
    Right,
    /// An extra button, 8 to 11 (8 and 9 are "back" and "forward" on most mice). Only the SGR
    /// forms can name them.
    Extra(u8),
    /// The wheel turned up.
    WheelUp,
    /// The wheel turned down.
    WheelDown,
    /// The wheel turned left.
    WheelLeft,
    /// The wheel turned right.
    WheelRight,
    /// No button: motion with nothing held.
    None,
}

/// What the pointer did.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum MouseKind {
    /// A button went down (a wheel step is a press).
    Press,
    /// A button came up.
    Release,
    /// The pointer moved, with `button` held or `MouseButton::None`.
    Motion,
}

/// One pointer event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct MouseEvent {
    /// The button pressed, released or held.
    pub button: MouseButton,
    /// What happened.
    pub kind: MouseKind,
    /// 0-based column of the cell under the pointer.
    pub col: u32,
    /// 0-based row of the cell under the pointer.
    pub row: u32,
    /// 0-based pixels from the left and top edge of the text area, for SGR-pixels (1016).
    pub pixel: Option<(u32, u32)>,
    /// The modifiers held (Shift, Alt/Meta and Control are reported).
    pub mods: Mods,
}

impl MouseEvent {
    /// An event at a cell, without modifiers or pixel position.
    pub const fn new(button: MouseButton, kind: MouseKind, col: u32, row: u32) -> MouseEvent {
        MouseEvent { button, kind, col, row, pixel: None, mods: Mods::empty() }
    }

    /// The same event with these modifiers.
    #[must_use]
    pub const fn with_mods(mut self, mods: Mods) -> Self {
        self.mods = mods;
        self
    }

    /// The same event with the pointer's pixel position inside the text area.
    #[must_use]
    pub const fn with_pixel(mut self, x: u32, y: u32) -> Self {
        self.pixel = Some((x, y));
        self
    }
}

/// Appends the report for `ev` and says whether there was one (a program that did not ask for the
/// event, or an event the chosen encoding cannot write, gets nothing).
pub(super) fn encode(ev: &MouseEvent, modes: &InputModes, out: &mut Vec<u8>) -> bool {
    let tracking = modes.mouse;
    if tracking == MouseTracking::Off {
        return false;
    }
    let base: u32 = match ev.button {
        MouseButton::Left => 0,
        MouseButton::Middle => 1,
        MouseButton::Right => 2,
        MouseButton::None => 3,
        MouseButton::WheelUp => 64,
        MouseButton::WheelDown => 65,
        MouseButton::WheelLeft => 66,
        MouseButton::WheelRight => 67,
        // 128 and up: bit 7 of the button code, SGR only.
        MouseButton::Extra(n @ 8..=11) => 128 + u32::from(n - 8),
        MouseButton::Extra(_) => return false,
    };
    let is_wheel = (64..=67).contains(&base);
    let sgr = matches!(modes.mouse_encoding, MouseEncoding::Sgr | MouseEncoding::SgrPixels);
    if base >= 128 && !sgr {
        return false;
    }

    let mut code = match ev.kind {
        // A press of "no button" is meaningless.
        MouseKind::Press if ev.button == MouseButton::None => return false,
        MouseKind::Press => base,
        // A wheel step has no release.
        MouseKind::Release if is_wheel || ev.button == MouseButton::None => return false,
        MouseKind::Release => base,
        MouseKind::Motion if is_wheel => return false,
        MouseKind::Motion => {
            let wanted = if ev.button == MouseButton::None {
                tracking == MouseTracking::Motion
            } else {
                matches!(tracking, MouseTracking::Drag | MouseTracking::Motion)
            };
            if !wanted {
                return false;
            }
            base + 32
        }
    };
    if tracking == MouseTracking::X10 {
        // X10 compatibility mode (9) is presses of the three buttons, without modifiers.
        if ev.kind != MouseKind::Press || base > 2 {
            return false;
        }
    } else {
        let m = ev.mods;
        code += 4 * u32::from(m.contains(Mods::SHIFT))
            + 8 * u32::from(m.intersects(Mods::ALT | Mods::META))
            + 16 * u32::from(m.contains(Mods::CTRL));
    }

    // The pixel position is 1-based like every number of the SGR family: xterm writes the
    // offset into the text area plus one (`EmitMousePosition` in button.c adds 1 to the column,
    // the row and the pixels alike), so the first pixel is 1. kitty writes the 0-based pixel;
    // the two differ by one pixel, which only shows at a cell's edge.
    let (x, y) = match (modes.mouse_encoding, ev.pixel) {
        (MouseEncoding::SgrPixels, Some((x, y))) => (x.saturating_add(1), y.saturating_add(1)),
        _ => (ev.col.saturating_add(1), ev.row.saturating_add(1)),
    };

    if sgr {
        out.extend_from_slice(&[ESC, b'[', b'<']);
        push_dec(out, code);
        out.push(b';');
        push_dec(out, x);
        out.push(b';');
        push_dec(out, y);
        out.push(if ev.kind == MouseKind::Release { b'm' } else { b'M' });
        return true;
    }

    // The older forms cannot say which button was released: it is button 3, the modifiers stay.
    if ev.kind == MouseKind::Release {
        code = 3 + (code & !3);
    }
    match modes.mouse_encoding {
        MouseEncoding::Urxvt => {
            out.extend_from_slice(&[ESC, b'[']);
            push_dec(out, code + 32);
            out.push(b';');
            push_dec(out, x);
            out.push(b';');
            push_dec(out, y);
            out.push(b'M');
        }
        MouseEncoding::Utf8 => {
            let mut report = [0u8; 12];
            let mut len = 0;
            for v in [code, x, y] {
                // Each value is offset by 32 and written as UTF-8, which stops at U+07FF.
                let Some(c) = char::from_u32(v.saturating_add(32)).filter(|c| (*c as u32) < 0x800) else {
                    return false;
                };
                len += c.encode_utf8(&mut report[len..]).len();
            }
            out.extend_from_slice(&[ESC, b'[', b'M']);
            out.extend_from_slice(&report[..len]);
        }
        _ => {
            // X10: one byte each, so a position stops at 223 (255 - 32).
            if x > 223 || y > 223 || code + 32 > 255 {
                return false;
            }
            out.extend_from_slice(&[ESC, b'[', b'M', (code + 32) as u8, (x + 32) as u8, (y + 32) as u8]);
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    const CLICK: MouseTracking = MouseTracking::Click;

    fn modes(tracking: MouseTracking, encoding: MouseEncoding) -> InputModes {
        InputModes { mouse: tracking, mouse_encoding: encoding, ..InputModes::default() }
    }
    fn report(ev: MouseEvent, modes: InputModes) -> Option<Vec<u8>> {
        let mut out = Vec::new();
        super::encode(&ev, &modes, &mut out).then_some(out)
    }
    fn ev(button: MouseButton, kind: MouseKind, col: u32, row: u32) -> MouseEvent {
        MouseEvent::new(button, kind, col, row)
    }
    use MouseButton as B;
    use MouseEncoding::{Sgr, SgrPixels, Urxvt, Utf8, X10};
    use MouseKind::{Motion, Press, Release};

    #[test]
    fn nothing_without_tracking() {
        let off = modes(MouseTracking::Off, Sgr);
        assert_eq!(report(ev(B::Left, Press, 0, 0), off), None);
    }

    #[test]
    fn x10_press_release() {
        let m = modes(CLICK, X10);
        assert_eq!(report(ev(B::Left, Press, 0, 0), m), Some(b"\x1b[M !!".to_vec()));
        assert_eq!(report(ev(B::Right, Press, 9, 4), m), Some(b"\x1b[M\"*%".to_vec()));
        assert_eq!(report(ev(B::Middle, Release, 0, 0), m), Some(b"\x1b[M#!!".to_vec()));
    }

    #[test]
    fn x10_modifiers_and_the_release_button() {
        let m = modes(CLICK, X10);
        let e = ev(B::Left, Press, 0, 0).with_mods(Mods::SHIFT | Mods::ALT | Mods::CTRL);
        assert_eq!(report(e, m), Some(vec![0x1b, b'[', b'M', 32 + 28, 33, 33]));
        let released = MouseEvent { kind: Release, ..e };
        assert_eq!(report(released, m), Some(vec![0x1b, b'[', b'M', 32 + 31, 33, 33]));
    }

    #[test]
    fn x10_positions_stop_at_223() {
        let m = modes(CLICK, X10);
        assert_eq!(report(ev(B::Left, Press, 222, 222), m), Some(vec![0x1b, b'[', b'M', 32, 255, 255]));
        assert_eq!(report(ev(B::Left, Press, 223, 0), m), None);
        assert_eq!(report(ev(B::Left, Press, 0, 223), m), None);
    }

    #[test]
    fn the_x10_mode_reports_presses_only_and_no_modifiers() {
        let m = modes(MouseTracking::X10, X10);
        let e = ev(B::Left, Press, 1, 1).with_mods(Mods::CTRL);
        assert_eq!(report(e, m), Some(b"\x1b[M \"\"".to_vec()));
        assert_eq!(report(ev(B::Left, Release, 1, 1), m), None);
        assert_eq!(report(ev(B::WheelUp, Press, 1, 1), m), None);
        assert_eq!(report(ev(B::Left, Motion, 1, 1), m), None);
    }

    #[test]
    fn wheel() {
        assert_eq!(
            report(ev(B::WheelUp, Press, 0, 0), modes(CLICK, X10)),
            Some(vec![0x1b, b'[', b'M', 96, 33, 33])
        );
        let sgr = modes(CLICK, Sgr);
        assert_eq!(report(ev(B::WheelDown, Press, 1, 2), sgr), Some(b"\x1b[<65;2;3M".to_vec()));
        assert_eq!(report(ev(B::WheelLeft, Press, 0, 0), sgr), Some(b"\x1b[<66;1;1M".to_vec()));
        assert_eq!(report(ev(B::WheelRight, Press, 0, 0), sgr), Some(b"\x1b[<67;1;1M".to_vec()));
        assert_eq!(report(ev(B::WheelUp, Release, 0, 0), sgr), None);
        assert_eq!(report(ev(B::WheelUp, Motion, 0, 0), modes(MouseTracking::Motion, Sgr)), None);
    }

    #[test]
    fn sgr() {
        let m = modes(CLICK, Sgr);
        assert_eq!(report(ev(B::Left, Press, 0, 0), m), Some(b"\x1b[<0;1;1M".to_vec()));
        assert_eq!(report(ev(B::Right, Release, 299, 99), m), Some(b"\x1b[<2;300;100m".to_vec()));
        // No cap on positions.
        assert_eq!(report(ev(B::Left, Press, 1000, 500), m), Some(b"\x1b[<0;1001;501M".to_vec()));
        let e = ev(B::Middle, Press, 4, 5).with_mods(Mods::CTRL | Mods::SHIFT);
        assert_eq!(report(e, m), Some(b"\x1b[<21;5;6M".to_vec()));
    }

    #[test]
    fn extra_buttons_are_sgr_only() {
        let sgr = modes(CLICK, Sgr);
        assert_eq!(report(ev(B::Extra(8), Press, 0, 0), sgr), Some(b"\x1b[<128;1;1M".to_vec()));
        assert_eq!(report(ev(B::Extra(11), Release, 0, 0), sgr), Some(b"\x1b[<131;1;1m".to_vec()));
        assert_eq!(report(ev(B::Extra(8), Press, 0, 0), modes(CLICK, X10)), None);
        assert_eq!(report(ev(B::Extra(12), Press, 0, 0), sgr), None);
    }

    #[test]
    fn motion_is_filtered_by_the_tracking_level() {
        let drag = ev(B::Left, Motion, 2, 3);
        let hover = ev(B::None, Motion, 2, 3);
        let tracking = |t| modes(t, Sgr);
        assert_eq!(report(drag, tracking(CLICK)), None);
        assert_eq!(report(hover, tracking(CLICK)), None);
        assert_eq!(report(drag, tracking(MouseTracking::Drag)), Some(b"\x1b[<32;3;4M".to_vec()));
        assert_eq!(report(hover, tracking(MouseTracking::Drag)), None);
        assert_eq!(report(drag, tracking(MouseTracking::Motion)), Some(b"\x1b[<32;3;4M".to_vec()));
        assert_eq!(report(hover, tracking(MouseTracking::Motion)), Some(b"\x1b[<35;3;4M".to_vec()));
        let right = ev(B::Right, Motion, 0, 0);
        assert_eq!(
            report(right, modes(MouseTracking::Drag, X10)),
            Some(vec![0x1b, b'[', b'M', 32 + 34, 33, 33])
        );
        // Clicks are still reported at the wider levels.
        assert_eq!(
            report(ev(B::Left, Press, 0, 0), tracking(MouseTracking::Motion)),
            Some(b"\x1b[<0;1;1M".to_vec())
        );
    }

    #[test]
    fn a_press_or_release_of_nothing_is_not_reported() {
        let m = modes(MouseTracking::Motion, Sgr);
        assert_eq!(report(ev(B::None, Press, 0, 0), m), None);
        assert_eq!(report(ev(B::None, Release, 0, 0), m), None);
    }

    #[test]
    fn utf8() {
        let m = modes(CLICK, Utf8);
        assert_eq!(report(ev(B::Left, Press, 0, 0), m), Some(b"\x1b[M !!".to_vec()));
        // Column 300 is 301 + 32 = 333 = U+014D = C5 8D.
        assert_eq!(report(ev(B::Left, Press, 300, 0), m), Some(vec![0x1b, b'[', b'M', 32, 0xC5, 0x8D, 33]));
        assert_eq!(report(ev(B::Left, Release, 0, 0), m), Some(b"\x1b[M#!!".to_vec()));
        // The last value that fits: 2047 - 32 = 2015, so column 2014.
        assert!(report(ev(B::Left, Press, 2014, 0), m).is_some());
        assert_eq!(report(ev(B::Left, Press, 2015, 0), m), None);
    }

    #[test]
    fn urxvt() {
        let m = modes(CLICK, Urxvt);
        assert_eq!(report(ev(B::Left, Press, 0, 0), m), Some(b"\x1b[32;1;1M".to_vec()));
        assert_eq!(report(ev(B::Left, Press, 299, 99), m), Some(b"\x1b[32;300;100M".to_vec()));
        // The release names no button.
        assert_eq!(report(ev(B::Right, Release, 4, 4), m), Some(b"\x1b[35;5;5M".to_vec()));
        assert_eq!(report(ev(B::WheelUp, Press, 0, 0), m), Some(b"\x1b[96;1;1M".to_vec()));
    }

    #[test]
    fn sgr_pixels_use_the_pixel_when_there_is_one() {
        let m = modes(CLICK, SgrPixels);
        let e = ev(B::Left, Press, 2, 3);
        // 1-based, as xterm writes them (button.c adds 1 to the pixel offset): the pixel 37 of
        // the text area, counting from 0, is 38.
        assert_eq!(report(e.with_pixel(37, 58), m), Some(b"\x1b[<0;38;59M".to_vec()));
        assert_eq!(
            report(MouseEvent { kind: Release, ..e }.with_pixel(37, 58), m),
            Some(b"\x1b[<0;38;59m".to_vec())
        );
        assert_eq!(report(e.with_pixel(0, 0), m), Some(b"\x1b[<0;1;1M".to_vec()));
        // No pixel: the cell, as plain SGR.
        assert_eq!(report(e, m), Some(b"\x1b[<0;3;4M".to_vec()));
        // The pixel does not change the other forms.
        assert_eq!(report(e.with_pixel(37, 58), modes(CLICK, Sgr)), Some(b"\x1b[<0;3;4M".to_vec()));
    }
}
