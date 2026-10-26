//! `mouse`: clicks, drags and the wheel for a program that asked for the mouse.
//!
//! Delivered only to a program that turned mouse reporting on (or asked for the DEC locator): the
//! API does not emulate selection or the app's own clicks. The reports are made with the modes as
//! they are when they are sent (`wormhole_term::input::mouse`).

use serde::{Deserialize, Serialize};
use wormhole_term::input::mouse::{self as input, MouseAction, MouseButton as Button, MouseInput};
use wormhole_term::types::Modes;

use super::Target;

/// The most lines one wheel event may scroll.
pub const SCROLL_LINES_MAX: u16 = 100;
/// The most presses one click may make (a triple click).
pub const CLICK_COUNT_MAX: u8 = 3;

/// `mouse`'s request: what the pointer does in a terminal.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MouseSpec {
    pub target: Target,
    pub op: MouseOp,
}

/// What the pointer does.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum MouseOp {
    /// A press and a release, `count` times.
    Click(Pointer),
    Press(Pointer),
    Release(Pointer),
    /// The pointer moves, no button down.
    Move(Pointer),
    Scroll(Wheel),
}

/// Where, with what, and the keys held.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pointer {
    /// 0-based row of the screen.
    pub row: u32,
    /// 0-based column.
    pub col: u32,
    #[serde(default)]
    pub button: ButtonName,
    /// How many clicks (a double click is 2).
    #[serde(default = "one")]
    pub count: u8,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub shift: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub alt: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ctrl: bool,
}

/// A turn of the wheel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wheel {
    pub row: u32,
    pub col: u32,
    pub direction: Direction,
    #[serde(default = "one_line")]
    pub lines: u16,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub shift: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub alt: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub ctrl: bool,
}

fn one() -> u8 {
    1
}

fn one_line() -> u16 {
    1
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ButtonName {
    #[default]
    Left,
    Middle,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Direction {
    Up,
    Down,
    Left,
    Right,
}

impl MouseOp {
    /// Where the pointer goes: `(row, col)`.
    pub fn at(&self) -> (u32, u32) {
        match self {
            MouseOp::Click(p) | MouseOp::Press(p) | MouseOp::Release(p) | MouseOp::Move(p) => (p.row, p.col),
            MouseOp::Scroll(w) => (w.row, w.col),
        }
    }

    /// What is wrong with the request itself (not with where it points: the screen's size is the
    /// app's to check).
    pub fn problem(&self) -> Option<String> {
        match self {
            MouseOp::Click(p) if p.count == 0 || p.count > CLICK_COUNT_MAX => {
                Some(format!("a click takes a count of 1 to {CLICK_COUNT_MAX}"))
            }
            MouseOp::Scroll(w) if w.lines == 0 || w.lines > SCROLL_LINES_MAX => {
                Some(format!("scroll takes 1 to {SCROLL_LINES_MAX} lines"))
            }
            _ => None,
        }
    }

    /// Whether a program that asked for the mouse in `modes` gets this as reports (a wheel turn is
    /// not a locator event).
    pub fn is_wheel(&self) -> bool {
        matches!(self, MouseOp::Scroll(_))
    }

    /// The reports for a program with `modes`, with the pointer in the middle of the cell when it
    /// wants pixels (`cell` is a cell's size in pixels). `None` when its tracking level reports
    /// none of them.
    pub fn encode(&self, modes: Modes, cell: (f32, f32)) -> Option<Vec<u8>> {
        let mut out = Vec::new();
        let mut send = |input: MouseInput| out.extend(input::encode(&input, modes).unwrap_or_default());
        let base = |row: u32, col: u32, shift, alt, control| MouseInput {
            button: Button::None,
            action: MouseAction::Motion,
            col: col as usize,
            row: row as usize,
            shift,
            alt,
            control,
            pixel: input::wants_pixels(modes).then(|| {
                let centre = |n: u32, size: f32| (n as f32 * size + size / 2.0) as u32;
                (centre(col, cell.0), centre(row, cell.1))
            }),
        };
        let button = |name: ButtonName| match name {
            ButtonName::Left => Button::Left,
            ButtonName::Middle => Button::Middle,
            ButtonName::Right => Button::Right,
        };
        match self {
            MouseOp::Click(p) => {
                for _ in 0..p.count.max(1) {
                    let at = base(p.row, p.col, p.shift, p.alt, p.ctrl);
                    send(MouseInput { button: button(p.button), action: MouseAction::Press, ..at });
                    send(MouseInput { button: button(p.button), action: MouseAction::Release, ..at });
                }
            }
            MouseOp::Press(p) => {
                let at = base(p.row, p.col, p.shift, p.alt, p.ctrl);
                send(MouseInput { button: button(p.button), action: MouseAction::Press, ..at });
            }
            MouseOp::Release(p) => {
                let at = base(p.row, p.col, p.shift, p.alt, p.ctrl);
                send(MouseInput { button: button(p.button), action: MouseAction::Release, ..at });
            }
            MouseOp::Move(p) => send(base(p.row, p.col, p.shift, p.alt, p.ctrl)),
            MouseOp::Scroll(w) => {
                let wheel = match w.direction {
                    Direction::Up => Button::WheelUp,
                    Direction::Down => Button::WheelDown,
                    Direction::Left => Button::WheelLeft,
                    Direction::Right => Button::WheelRight,
                };
                let at = base(w.row, w.col, w.shift, w.alt, w.ctrl);
                for _ in 0..w.lines {
                    send(MouseInput { button: wheel, action: MouseAction::Press, ..at });
                }
            }
        }
        (!out.is_empty()).then_some(out)
    }
}

#[cfg(test)]
mod tests {
    use wormhole_term::types::Modes;

    use super::{ButtonName, Direction, MouseOp, Pointer, Wheel};

    fn pointer(row: u32, col: u32) -> Pointer {
        Pointer { row, col, button: ButtonName::Left, count: 1, shift: false, alt: false, ctrl: false }
    }

    const CELL: (f32, f32) = (8.0, 16.0);

    #[test]
    fn a_click_is_a_press_and_a_release_in_the_encoding_the_program_asked_for() {
        let click = MouseOp::Click(pointer(4, 10));
        let sgr = Modes::MOUSE_CLICK | Modes::MOUSE_SGR;
        assert_eq!(click.encode(sgr, CELL), Some(b"\x1b[<0;11;5M\x1b[<0;11;5m".to_vec()));
        assert_eq!(click.encode(Modes::MOUSE_CLICK, CELL), Some(b"\x1b[M +%\x1b[M#+%".to_vec()));
        assert_eq!(click.encode(Modes::empty(), CELL), None, "a program that did not ask gets nothing");
        let double = MouseOp::Click(Pointer { count: 2, ..pointer(0, 0) });
        let twice = double.encode(sgr, CELL).expect("sent");
        assert_eq!(twice, b"\x1b[<0;1;1M\x1b[<0;1;1m\x1b[<0;1;1M\x1b[<0;1;1m");
        let right = MouseOp::Click(Pointer { button: ButtonName::Right, ctrl: true, ..pointer(0, 0) });
        assert_eq!(right.encode(sgr, CELL), Some(b"\x1b[<18;1;1M\x1b[<18;1;1m".to_vec()));
    }

    #[test]
    fn press_release_move_and_the_wheel() {
        let sgr = Modes::MOUSE_CLICK | Modes::MOUSE_MOTION | Modes::MOUSE_SGR;
        assert_eq!(MouseOp::Press(pointer(1, 2)).encode(sgr, CELL), Some(b"\x1b[<0;3;2M".to_vec()));
        assert_eq!(MouseOp::Release(pointer(1, 2)).encode(sgr, CELL), Some(b"\x1b[<0;3;2m".to_vec()));
        assert_eq!(MouseOp::Move(pointer(1, 2)).encode(sgr, CELL), Some(b"\x1b[<35;3;2M".to_vec()));
        let wheel = |direction, lines| {
            MouseOp::Scroll(Wheel { row: 0, col: 0, direction, lines, shift: false, alt: false, ctrl: false })
        };
        assert_eq!(wheel(Direction::Up, 2).encode(sgr, CELL), Some(b"\x1b[<64;1;1M\x1b[<64;1;1M".to_vec()));
        assert_eq!(wheel(Direction::Down, 1).encode(sgr, CELL), Some(b"\x1b[<65;1;1M".to_vec()));
        // A program that tracks clicks only is not told of motion.
        assert_eq!(MouseOp::Move(pointer(0, 0)).encode(Modes::MOUSE_CLICK | Modes::MOUSE_SGR, CELL), None);
    }

    #[test]
    fn a_program_that_wants_pixels_gets_the_middle_of_the_cell() {
        let pixels = Modes::MOUSE_CLICK | Modes::MOUSE_SGR | Modes::MOUSE_PIXELS;
        let press = MouseOp::Press(pointer(4, 10)).encode(pixels, CELL).expect("sent");
        assert_eq!(press, b"\x1b[<0;85;73M", "10 * 8 + 4 and 4 * 16 + 8, counted from 1");
    }

    #[test]
    fn the_request_is_checked_for_its_own_sake() {
        assert!(MouseOp::Click(Pointer { count: 0, ..pointer(0, 0) }).problem().is_some());
        assert!(MouseOp::Click(Pointer { count: 4, ..pointer(0, 0) }).problem().is_some());
        assert!(MouseOp::Click(Pointer { count: 3, ..pointer(0, 0) }).problem().is_none());
        let wheel = |lines| {
            MouseOp::Scroll(Wheel {
                row: 0,
                col: 0,
                direction: Direction::Up,
                lines,
                shift: false,
                alt: false,
                ctrl: false,
            })
        };
        assert!(
            wheel(0).problem().is_some() && wheel(101).problem().is_some() && wheel(100).problem().is_none()
        );
        assert_eq!(MouseOp::Move(pointer(3, 9)).at(), (3, 9));
    }

    #[test]
    fn the_wire_shapes_are_the_documented_ones() {
        let click: MouseOp =
            serde_json::from_str(r#"{"kind":"click","row":4,"col":10,"button":"left","count":1}"#)
                .expect("parses");
        assert_eq!(click, MouseOp::Click(pointer(4, 10)));
        assert_eq!(
            serde_json::to_string(&click).expect("serializes"),
            r#"{"kind":"click","row":4,"col":10,"button":"left","count":1}"#
        );
        let scroll: MouseOp =
            serde_json::from_str(r#"{"kind":"scroll","row":4,"col":10,"direction":"down","lines":3}"#)
                .expect("parses");
        assert!(scroll.is_wheel());
        let bare: MouseOp = serde_json::from_str(r#"{"kind":"press","row":0,"col":0}"#).expect("parses");
        assert_eq!(bare, MouseOp::Press(pointer(0, 0)), "left, once, by default");
    }
}
