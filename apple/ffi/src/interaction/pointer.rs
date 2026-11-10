//! The pointer, the wheel, paste and focus, for a program that asked for them and for the view.

use wormhole_term::input::{self, mouse};
use wormhole_term::types::{Modes, Scroll};

use crate::terminal::Terminal;

/// Wheel steps reported to a program per event, at most (a fast flick must not flood it).
const MAX_WHEEL_REPORTS: u32 = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum PointerButton {
    Left,
    Middle,
    Right,
    /// Motion with no button held.
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum PointerAction {
    Press,
    Release,
    Motion,
}

/// A button or motion over the grid, in viewport cells and (for a program that asked for pixels,
/// mode 1016) points from the text area's corner.
#[derive(Debug, Clone, Copy, uniffi::Record)]
pub struct PointerEvent {
    pub button: PointerButton,
    pub action: PointerAction,
    pub col: u32,
    pub row: u32,
    pub x: u32,
    pub y: u32,
    pub shift: bool,
    pub alt: bool,
    pub control: bool,
}

/// A wheel or trackpad movement in cells: `lines` positive scrolls up, `columns` positive scrolls left.
/// Fractions are kept between calls, so a trackpad's small steps add up to whole lines.
#[derive(Debug, Clone, Copy, uniffi::Record)]
pub struct WheelStep {
    pub lines: f32,
    pub columns: f32,
    pub col: u32,
    pub row: u32,
    pub x: u32,
    pub y: u32,
    pub shift: bool,
    pub alt: bool,
    pub control: bool,
}

/// What is carried from one pointer call to the next.
#[derive(Default)]
pub(crate) struct Memory {
    /// The cell of the last motion reported (a program hears only a new cell, unless it wants pixels).
    last_cell: Option<(u32, u32)>,
    remainder_lines: f32,
    remainder_columns: f32,
}

fn button_of(button: PointerButton) -> mouse::MouseButton {
    match button {
        PointerButton::Left => mouse::MouseButton::Left,
        PointerButton::Middle => mouse::MouseButton::Middle,
        PointerButton::Right => mouse::MouseButton::Right,
        PointerButton::None => mouse::MouseButton::None,
    }
}

fn input_of(
    event: &PointerEvent,
    button: mouse::MouseButton,
    action: mouse::MouseAction,
    mode: Modes,
) -> mouse::MouseInput {
    mouse::MouseInput {
        button,
        action,
        col: event.col as usize,
        row: event.row as usize,
        shift: event.shift,
        alt: event.alt,
        control: event.control,
        pixel: mouse::wants_pixels(mode).then_some((event.x, event.y)),
    }
}

#[uniffi::export]
impl Terminal {
    /// Give a pointer event to the program if it asked for the mouse (reporting modes 9, 1000 to 1006,
    /// 1016, or the DEC locator) and Shift is not held (Shift takes the pointer back for selecting).
    /// Returns whether the program took it: when `false` the view handles the event itself.
    pub fn report_pointer(&self, event: PointerEvent) -> bool {
        self.with_pty(|pty| {
            let mode = pty.mode();
            if mouse::wants_locator(mode) && !event.shift {
                return locator(pty, &event);
            }
            if !mouse::wants_mouse(mode) || event.shift {
                return false;
            }
            let action = match event.action {
                PointerAction::Press => mouse::MouseAction::Press,
                PointerAction::Release => mouse::MouseAction::Release,
                PointerAction::Motion => mouse::MouseAction::Motion,
            };
            if action == mouse::MouseAction::Motion && !mouse::wants_pixels(mode) {
                let mut memory = self.interaction.pointer();
                if memory.last_cell == Some((event.col, event.row)) {
                    return true;
                }
                memory.last_cell = Some((event.col, event.row));
            }
            let input = input_of(&event, button_of(event.button), action, mode);
            if let Some(bytes) = mouse::encode(&input, mode) {
                pty.write(bytes);
            }
            if event.button == PointerButton::Left {
                let mut selected = self.interaction.app_selection();
                match action {
                    mouse::MouseAction::Press => selected.pressed(event.row as usize),
                    mouse::MouseAction::Release => {
                        selected.released(event.row as usize, std::time::Instant::now())
                    }
                    mouse::MouseAction::Motion => {}
                }
            }
            true
        })
        .unwrap_or(false)
    }

    /// Whether a program asked for the mouse, so the view knows whether a click selects.
    pub fn takes_pointer(&self) -> bool {
        self.with_pty(|pty| mouse::wants_mouse(pty.mode()) || mouse::wants_locator(pty.mode()))
            .unwrap_or(false)
    }

    /// The wheel. A program that asked for the mouse hears wheel buttons; a full-screen program on the
    /// alternate screen with alternate-scroll gets arrow keys; otherwise the view scrolls the history.
    /// Shift sends it to the history whatever the program asked for.
    pub fn scroll_wheel(&self, step: WheelStep) {
        let (lines, columns) = {
            let mut memory = self.interaction.pointer();
            let lines = step.lines + memory.remainder_lines;
            let columns = step.columns + memory.remainder_columns;
            memory.remainder_lines = lines - lines.trunc();
            memory.remainder_columns = columns - columns.trunc();
            (lines.trunc() as i32, columns.trunc() as i32)
        };
        self.with_pty(|pty| {
            let mode = pty.mode();
            let event = PointerEvent {
                button: PointerButton::None,
                action: PointerAction::Press,
                col: step.col,
                row: step.row,
                x: step.x,
                y: step.y,
                shift: step.shift,
                alt: step.alt,
                control: step.control,
            };
            if mouse::wants_mouse(mode) && !step.shift {
                let report = |button, count: i32| {
                    for _ in 0..count.unsigned_abs().min(MAX_WHEEL_REPORTS) {
                        let input = input_of(&event, button, mouse::MouseAction::Press, mode);
                        if let Some(bytes) = mouse::encode(&input, mode) {
                            pty.write(bytes);
                        }
                    }
                };
                report(
                    if columns > 0 { mouse::MouseButton::WheelLeft } else { mouse::MouseButton::WheelRight },
                    columns,
                );
                report(
                    if lines > 0 { mouse::MouseButton::WheelUp } else { mouse::MouseButton::WheelDown },
                    lines,
                );
            } else if lines != 0 && mode.contains(Modes::ALT_SCREEN) && mode.contains(Modes::ALTERNATE_SCROLL)
            {
                let key: &[u8] = match (lines > 0, mode.contains(Modes::APP_CURSOR)) {
                    (true, true) => b"\x1bOA",
                    (true, false) => b"\x1b[A",
                    (false, true) => b"\x1bOB",
                    (false, false) => b"\x1b[B",
                };
                pty.write(key.repeat(lines.unsigned_abs() as usize));
            } else if lines != 0 {
                pty.scroll(Scroll::Delta(lines));
            }
        });
    }

    /// Pasted text, wrapped in bracketed-paste markers when the program asked for them (control
    /// characters removed so the text cannot end the bracket), line breaks as Return. The view goes
    /// back to the live screen.
    pub fn paste(&self, text: String) {
        self.with_pty(|pty| pty.write_input(input::paste(&text, pty.mode())));
    }

    /// The window or the view gained or lost the keyboard: a program that asked for focus events
    /// (DECSET 1004) is told.
    pub fn focus_changed(&self, focused: bool) {
        self.with_pty(|pty| {
            if let Some(bytes) = input::keys::focus_report(focused, pty.mode()) {
                pty.write(bytes);
            }
        });
    }

    /// The shell-escaped text to insert for dropped files (each path followed by a space).
    pub fn dropped_text(&self, paths: Vec<String>) -> String {
        input::file_drop::dropped_text(&paths)
    }

    /// Erase the history; the screen stays.
    pub fn clear_history(&self) {
        self.with_pty(|pty| pty.clear_history());
    }
}

/// The pointer for a program that asked for the DEC locator.
fn locator(pty: &wormhole_term::Pty, event: &PointerEvent) -> bool {
    let button = match (event.action, event.button) {
        (PointerAction::Motion, _) => None,
        (
            PointerAction::Press,
            button @ (PointerButton::Left | PointerButton::Middle | PointerButton::Right),
        ) => Some((button_of(button), true)),
        (
            PointerAction::Release,
            button @ (PointerButton::Left | PointerButton::Middle | PointerButton::Right),
        ) => Some((button_of(button), false)),
        _ => return true,
    };
    let at =
        mouse::LocatorPointer { col: event.col as usize, row: event.row as usize, x: event.x, y: event.y };
    pty.locator(button, at);
    true
}
