//! The DEC locator: a pointing device that reports where it is when asked, when a button goes
//! down or up, or when it leaves a rectangle.
//!
//! Spec: xterm's ctlseqs (the VT220 and VT420 functions it documents) for DECELR (`CSI Ps ; Pu ' z`:
//! off, on, or on for one report; the unit of the coordinates), DECSLE (`CSI Pm ' {`: which button
//! transitions are reported), DECRQLP (`CSI Ps ' |`: one report now), DECEFR (`CSI Pt ; Pl ; Pb ; Pr
//! ' w`: report when the locator leaves this rectangle, once) and the report, DECLRP
//! (`CSI Pe ; Pb ; Pr ; Pc ; Pp & w`). The device status reports 53 and 55 say a locator is there,
//! 56 that it is a mouse (`CSI ? 57 ; 1 n`).
//!
//! The terminal has no pointer of its own: the host tells it where the pointer is and when the
//! buttons change, through [`Terminal::locator_moved`] and [`Terminal::locator_button`], once
//! [`Terminal::locator_enabled`] says a program asked. The locator and the xterm mouse tracking
//! modes are alternatives: switching on one switches the other off.

use crate::parser::Csi;
use crate::types::TermMode;

use crate::terminal::Terminal;
use crate::terminal::modes::Modes;
use crate::terminal::modes::conformance::Level;

/// A button the locator reports. The mouse of a Mac has three.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LocatorButton {
    /// The primary button.
    Left,
    /// The middle button.
    Middle,
    /// The secondary button.
    Right,
}

impl LocatorButton {
    /// DECLRP's `Pb` bit for the button down (right 1, middle 2, left 4).
    fn bit(self) -> u8 {
        match self {
            LocatorButton::Right => 1,
            LocatorButton::Middle => 2,
            LocatorButton::Left => 4,
        }
    }

    /// DECLRP's `Pe` for the button going down; the next number is for it going up.
    fn event(self) -> u8 {
        match self {
            LocatorButton::Left => 2,
            LocatorButton::Middle => 4,
            LocatorButton::Right => 6,
        }
    }
}

/// Where the pointer is: the cell, and the pixel inside the text area (0-based), for programs that
/// asked for pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Pointer {
    /// The column, from 0.
    pub col: usize,
    /// The row, from 0.
    pub row: usize,
    /// Pixels from the text area's left edge.
    pub x: u32,
    /// Pixels from the text area's top edge.
    pub y: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
enum Reporting {
    #[default]
    Off,
    On,
    /// Until the next report.
    Once,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Units {
    #[default]
    Cells,
    Pixels,
}

/// A rectangle the locator is reported leaving, in the unit in force when it was made, counted
/// from 1 and edges included.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Filter {
    top: u32,
    left: u32,
    bottom: u32,
    right: u32,
}

/// What the sequences set of the locator, without the pointer the host moves.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::terminal) struct Settings {
    /// DECELR's first parameter: 0 off, 1 on, 2 once.
    pub(in crate::terminal) reporting: u8,
    pub(in crate::terminal) pixels: bool,
    pub(in crate::terminal) on_down: bool,
    pub(in crate::terminal) on_up: bool,
    /// DECEFR: top, left, bottom and right in the unit in force.
    pub(in crate::terminal) filter: Option<[u32; 4]>,
}

/// The locator's settings and what is known of the pointer.
#[derive(Clone, Debug, Default)]
pub(in crate::terminal) struct Locator {
    reporting: Reporting,
    units: Units,
    /// DECSLE 1: button transitions down are reported.
    on_down: bool,
    /// DECSLE 3: button transitions up are reported.
    on_up: bool,
    filter: Option<Filter>,
    pointer: Pointer,
    /// The buttons that are down, as `Pb` has them.
    buttons: u8,
}

impl Locator {
    /// What the sequences set, without the pointer the host moves: for the state dump.
    pub(in crate::terminal) fn settings(&self) -> Settings {
        Settings {
            reporting: self.reporting as u8,
            pixels: self.units == Units::Pixels,
            on_down: self.on_down,
            on_up: self.on_up,
            filter: self.filter.map(|f| [f.top, f.left, f.bottom, f.right]),
        }
    }

    /// Whether a program asked for reports.
    pub(in crate::terminal) fn enabled(&self) -> bool {
        self.reporting != Reporting::Off
    }

    /// Switch it off (a mouse tracking mode took the pointer).
    pub(in crate::terminal) fn disable(&mut self) {
        self.reporting = Reporting::Off;
        self.filter = None;
    }

    /// The position in the unit in force, row first.
    fn position(&self, at: Pointer) -> (u32, u32) {
        match self.units {
            Units::Cells => (at.row as u32 + 1, at.col as u32 + 1),
            Units::Pixels => (at.y + 1, at.x + 1),
        }
    }
}

impl Terminal {
    /// Whether a program asked for locator reports: the host should deliver the pointer.
    pub fn locator_enabled(&self) -> bool {
        self.locator.enabled()
    }

    /// The mode set the host reads, with [`TermMode::LOCATOR`] when a program asked for the locator.
    pub(in crate::terminal) fn locator_mode(&self) -> TermMode {
        if self.locator.enabled() { TermMode::LOCATOR } else { TermMode::empty() }
    }

    /// The pointer moved. Reported only when it left the filter rectangle a program set.
    pub fn locator_moved(&mut self, at: Pointer) {
        self.locator.pointer = at;
        let Some(filter) = self.locator.filter else { return };
        if !self.locator.enabled() {
            return;
        }
        let (row, col) = self.locator.position(at);
        if row < filter.top || row > filter.bottom || col < filter.left || col > filter.right {
            self.locator.filter = None;
            self.locator_report(10);
        }
    }

    /// A button went down (`pressed`) or up at `at`. Reported when DECSLE asked for that transition.
    pub fn locator_button(&mut self, button: LocatorButton, pressed: bool, at: Pointer) {
        self.locator.pointer = at;
        if pressed {
            self.locator.buttons |= button.bit();
        } else {
            self.locator.buttons &= !button.bit();
        }
        let wanted = if pressed { self.locator.on_down } else { self.locator.on_up };
        if self.locator.enabled() && wanted {
            self.locator_report(button.event() + u8::from(!pressed));
        }
    }

    /// DECLRP for event `event`: where the pointer is and which buttons are down. A report that
    /// was asked to be the only one ends the reporting.
    fn locator_report(&mut self, event: u8) {
        let (row, col) = self.locator.position(self.locator.pointer);
        let buttons = self.locator.buttons;
        self.reply(format_args!("\x1b[{event};{buttons};{row};{col};1&w"));
        if self.locator.reporting == Reporting::Once {
            self.locator.disable();
        }
    }

    /// DECELR.
    pub(in crate::terminal) fn enable_locator(&mut self, csi: &Csi) {
        if self.level < Level::Vt300 {
            return;
        }
        let reporting = match csi.param(0, 0) {
            0 => Reporting::Off,
            1 => Reporting::On,
            2 => Reporting::Once,
            _ => return,
        };
        match csi.param(1, 0) {
            0 | 2 => self.locator.units = Units::Cells,
            1 => self.locator.units = Units::Pixels,
            _ => return,
        }
        self.locator.reporting = reporting;
        self.locator.filter = None;
        if self.locator.enabled() {
            // One protocol at a time: the pointer is the locator's now.
            self.modes
                .remove(Modes::MOUSE_CLICK | Modes::MOUSE_DRAG | Modes::MOUSE_MOTION | Modes::MOUSE_X10);
        }
    }

    /// DECSLE.
    pub(in crate::terminal) fn select_locator_events(&mut self, csi: &Csi) {
        if self.level < Level::Vt300 {
            return;
        }
        for i in 0..csi.len().max(1) {
            match csi.param(i, 0) {
                0 => {
                    self.locator.on_down = false;
                    self.locator.on_up = false;
                    self.locator.filter = None;
                }
                1 => self.locator.on_down = true,
                2 => self.locator.on_down = false,
                3 => self.locator.on_up = true,
                4 => self.locator.on_up = false,
                _ => {}
            }
        }
    }

    /// DECRQLP: one report now, or `CSI 0 & w` when the locator is not enabled.
    pub(in crate::terminal) fn request_locator_position(&mut self, ps: u16) {
        if self.level < Level::Vt300 || ps > 1 {
            return;
        }
        if self.locator.enabled() {
            self.locator_report(1);
        } else {
            self.reply(format_args!("\x1b[0&w"));
        }
    }

    /// DECEFR: report once when the pointer leaves the rectangle. What is left out is where the
    /// pointer is now.
    pub(in crate::terminal) fn enable_filter_rectangle(&mut self, csi: &Csi) {
        if self.level < Level::Vt300 || !self.locator.enabled() {
            return;
        }
        let (row, col) = self.locator.position(self.locator.pointer);
        let edge = |i: usize, now: u32| csi.get(i).filter(|&n| n > 0).map_or(now, u32::from);
        self.locator.filter =
            Some(Filter { top: edge(0, row), left: edge(1, col), bottom: edge(2, row), right: edge(3, col) });
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::Parser;
    use crate::terminal::{LocatorButton, Pointer, Terminal};

    fn at(col: usize, row: usize) -> Pointer {
        Pointer { col, row, x: col as u32 * 8 + 3, y: row as u32 * 16 + 5 }
    }

    fn run(term: &mut Terminal, bytes: &str) -> String {
        let mut parser = Parser::new();
        term.advance(&mut parser, bytes.as_bytes());
        String::from_utf8_lossy(&term.take_replies()).into_owned()
    }

    #[test]
    fn a_disabled_locator_is_unavailable() {
        let mut term = Terminal::new(20, 5, 0);
        assert_eq!(run(&mut term, "\x1b[1'|"), "\x1b[0&w");
    }

    #[test]
    fn a_request_reports_the_pointer_in_cells() {
        let mut term = Terminal::new(20, 5, 0);
        run(&mut term, "\x1b[1;2'z");
        term.locator_moved(at(4, 2));
        assert_eq!(run(&mut term, "\x1b[0'|"), "\x1b[1;0;3;5;1&w");
    }

    #[test]
    fn pixels_count_from_one() {
        let mut term = Terminal::new(20, 5, 0);
        run(&mut term, "\x1b[1;1'z");
        term.locator_moved(at(4, 2));
        assert_eq!(run(&mut term, "\x1b['|"), "\x1b[1;0;38;36;1&w");
    }

    #[test]
    fn buttons_are_reported_when_selected() {
        let mut term = Terminal::new(20, 5, 0);
        run(&mut term, "\x1b[1'z");
        term.locator_button(LocatorButton::Left, true, at(1, 1));
        assert_eq!(run(&mut term, ""), "", "nothing is selected yet");
        run(&mut term, "\x1b[1;3'{");
        term.locator_button(LocatorButton::Right, true, at(1, 1));
        term.locator_button(LocatorButton::Right, false, at(2, 1));
        assert_eq!(run(&mut term, ""), "\x1b[6;5;2;2;1&w\x1b[7;4;2;3;1&w");
        run(&mut term, "\x1b[2;4'{");
        term.locator_button(LocatorButton::Left, false, at(2, 1));
        assert_eq!(run(&mut term, ""), "");
    }

    #[test]
    fn once_ends_after_one_report() {
        let mut term = Terminal::new(20, 5, 0);
        run(&mut term, "\x1b[2'z");
        assert!(term.locator_enabled());
        assert_eq!(run(&mut term, "\x1b['|"), "\x1b[1;0;1;1;1&w");
        assert!(!term.locator_enabled());
        assert_eq!(run(&mut term, "\x1b['|"), "\x1b[0&w");
    }

    #[test]
    fn leaving_a_filter_rectangle_is_reported_once() {
        let mut term = Terminal::new(20, 5, 0);
        run(&mut term, "\x1b[1'z");
        term.locator_moved(at(5, 2));
        run(&mut term, "\x1b[2;4;4;8'w");
        term.locator_moved(at(6, 2));
        assert_eq!(run(&mut term, ""), "");
        term.locator_moved(at(9, 2));
        assert_eq!(run(&mut term, ""), "\x1b[10;0;3;10;1&w");
        term.locator_moved(at(15, 4));
        assert_eq!(run(&mut term, ""), "", "a rectangle is used once");
    }

    #[test]
    fn mouse_tracking_and_the_locator_exclude_each_other() {
        let mut term = Terminal::new(20, 5, 0);
        run(&mut term, "\x1b[?1000h\x1b[1'z");
        assert!(term.locator_enabled());
        assert!(!term.mode().intersects(crate::types::TermMode::MOUSE_MODE));
        assert!(term.mode().contains(crate::types::TermMode::LOCATOR));
        run(&mut term, "\x1b[?1000h");
        assert!(!term.locator_enabled());
    }
}
