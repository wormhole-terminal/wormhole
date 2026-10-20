//! CSI dispatch: which function a sequence is, from its marker, intermediates and final byte.
//!
//! Spec: ECMA-48 8.3 for the functions with a final byte alone, xterm's ctlseqs for the rest
//! (private markers `? > < =`, intermediates). A sequence the terminal does not know is ignored
//! whole, its parameters included. A sequence with too many parameters or intermediates
//! ([`Csi::overflow`]) is dropped: the parser kept only part of it.

use crate::parser::Csi;

use crate::terminal::Terminal;
use crate::terminal::modes::conformance::Level;
use crate::terminal::modes::keyboard::SetMode;
use crate::terminal::modes::{ModeSet, Switch};
use crate::terminal::screen::cursor::Landing;
use crate::terminal::screen::edit::{Erase, EraseExtent, Shift};

/// A parameter as a count: 0 or none is the default.
fn count(csi: &Csi, i: usize) -> usize {
    usize::from(csi.param_nonzero(i, 1))
}

impl Terminal {
    pub(in crate::terminal) fn dispatch_csi(&mut self, csi: &Csi) {
        if csi.overflow() {
            return;
        }
        let keeps_clear = matches!(csi.final_byte(), b'H' | b'f' | b'J' | b'K') && csi.marker().is_none();
        if !keeps_clear {
            self.history_just_cleared = false;
        }
        match (csi.marker(), csi.intermediates()) {
            (None, []) => self.csi_plain(csi),
            (Some(b'?'), []) => self.csi_private(csi),
            (None, [b' ']) => match csi.final_byte() {
                b'q' => self.set_cursor_style(csi.param(0, 0)),
                b'@' if self.level >= Level::Vt400 => self.shift_columns(Shift::Left, count(csi, 0)),
                b'A' if self.level >= Level::Vt400 => self.shift_columns(Shift::Right, count(csi, 0)),
                _ => {}
            },
            (None, [b'&']) if csi.final_byte() == b'u' => self.report_preferred_set(),
            (None, [b'!']) if csi.final_byte() == b'p' => self.soft_reset(),
            (None, [b'"']) if csi.final_byte() == b'p' => self.set_conformance_level(csi),
            (None, [b'"']) if csi.final_byte() == b'v' && self.level >= Level::Vt300 => {
                self.report_displayed_extent();
            }
            (None, [b'"']) if csi.final_byte() == b'q' => {
                self.pen.protected = csi.param(0, 0) == 1;
            }
            // DECRQM is a VT300 feature: a terminal taken down to VT100 or VT200 does not answer.
            (None, [b'$']) if csi.final_byte() == b'p' && self.level >= Level::Vt300 => {
                self.report_ansi_mode(csi.param(0, 0));
            }
            (Some(b'?'), [b'$']) if csi.final_byte() == b'p' && self.level >= Level::Vt300 => {
                self.report_private_mode(csi.param(0, 0));
            }
            (None, [b'$']) if csi.final_byte() == b'w' && self.level >= Level::Vt300 => {
                self.presentation_report(csi.param(0, 0));
            }
            (None, [b'$']) if csi.final_byte() == b'u' && self.level >= Level::Vt400 => {
                self.state_report(csi.param(0, 0));
            }
            (None, [b'+' | b'*' | b'$']) if self.keep_setting(csi) => {}
            (None, [b'$']) => self.csi_rectangle(csi),
            (None, [b'*']) if csi.final_byte() == b'|' => self.set_lines_per_screen(csi.param(0, 0)),
            (None, [b'*']) => match csi.final_byte() {
                b'x' => self.select_attribute_extent(csi.param(0, 0)),
                b'y' => self.checksum_request(csi),
                _ => {}
            },
            (None, [b'\'']) => match csi.final_byte() {
                b'}' => self.insert_columns(count(csi, 0)),
                b'~' => self.delete_columns(count(csi, 0)),
                b'z' => self.enable_locator(csi),
                b'{' => self.select_locator_events(csi),
                b'|' => self.request_locator_position(csi.param(0, 0)),
                b'w' => self.enable_filter_rectangle(csi),
                _ => {}
            },
            (Some(b'>'), []) => match csi.final_byte() {
                b'c' => self.secondary_attributes(),
                b'q' => self.report_version(),
                b't' => self.set_title_modes(csi.values(), true),
                b'T' => self.set_title_modes(csi.values(), false),
                b'u' => {
                    self.keys[self.screen_slot()].push(csi.param(0, 0) as u8);
                    self.sync_keyboard_flags();
                }
                _ => {}
            },
            (Some(b'<'), []) if csi.final_byte() == b'u' => {
                self.keys[self.screen_slot()].pop(count(csi, 0));
                self.sync_keyboard_flags();
            }
            (Some(b'='), []) => match csi.final_byte() {
                b'c' => self.tertiary_attributes(),
                b'u' => {
                    let mode = SetMode::from_param(csi.param_nonzero(1, 1));
                    if let Some(mode) = mode {
                        self.keys[self.screen_slot()].set(csi.param(0, 0) as u8, mode);
                        self.sync_keyboard_flags();
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    /// Sequences without a marker or intermediates.
    fn csi_plain(&mut self, csi: &Csi) {
        match csi.final_byte() {
            b'm' => self.sgr(csi),
            b'H' | b'f' => self.position(count(csi, 0) - 1, count(csi, 1) - 1),
            b'A' => self.cursor_up(count(csi, 0), Landing::Stay),
            b'B' => self.cursor_down(count(csi, 0), Landing::Stay),
            b'C' | b'a' => self.cursor_forward(count(csi, 0)),
            b'D' => self.cursor_back(count(csi, 0)),
            b'E' => self.cursor_down(count(csi, 0), Landing::LineStart),
            b'F' => self.cursor_up(count(csi, 0), Landing::LineStart),
            b'G' | b'`' => self.set_column(count(csi, 0) - 1),
            b'd' => self.set_row(count(csi, 0) - 1),
            b'e' => self.cursor_down(count(csi, 0), Landing::Stay),
            b'J' => {
                if let Some(extent) = EraseExtent::from_param(csi.param(0, 0)) {
                    self.erase_display(extent, Erase::Everything);
                }
            }
            b'K' => {
                if let Some(extent) = EraseExtent::from_param(csi.param(0, 0)) {
                    self.erase_line(extent, Erase::Everything);
                }
            }
            b'X' => self.erase_chars(count(csi, 0)),
            b'@' => self.insert_chars(count(csi, 0)),
            b'P' => self.delete_chars(count(csi, 0)),
            b'L' => self.insert_lines(count(csi, 0)),
            b'M' => self.delete_lines(count(csi, 0)),
            b'S' => self.scroll_up(count(csi, 0)),
            // With more parameters this is xterm's mouse tracking initiation, which is ignored.
            b'T' if csi.len() <= 1 => self.scroll_down(count(csi, 0)),
            b'b' => self.repeat_last(count(csi, 0)),
            b'I' => self.tab_forward(count(csi, 0)),
            b'Z' => self.tab_back(count(csi, 0)),
            b'g' => match csi.param(0, 0) {
                0 => self.tabs.clear(self.cursor.x),
                3 => self.tabs.clear_all(),
                _ => {}
            },
            b'h' => self.each_mode(csi, Switch::Set, ModeSet::Ansi),
            b'l' => self.each_mode(csi, Switch::Reset, ModeSet::Ansi),
            b'n' => self.device_status(csi.param(0, 0)),
            b'c' if csi.param(0, 0) == 0 => self.primary_attributes(),
            b'r' => self.set_vertical_margins(usize::from(csi.param(0, 0)), usize::from(csi.param(1, 0))),
            // DECSLRM while DECLRMM is set, SCOSC otherwise.
            b's' => {
                if self.modes.contains(crate::terminal::modes::Modes::LEFT_RIGHT_MARGINS) {
                    self.set_horizontal_margins(usize::from(csi.param(0, 0)), usize::from(csi.param(1, 0)));
                } else if csi.len() <= 1 && csi.get(0).is_none() {
                    self.save_cursor();
                }
            }
            b'u' if csi.get(0).is_none() => self.restore_cursor(),
            b't' => self.window_operation(csi),
            _ => {}
        }
    }

    /// Sequences with the `?` marker.
    fn csi_private(&mut self, csi: &Csi) {
        match csi.final_byte() {
            b'h' => self.each_mode(csi, Switch::Set, ModeSet::Private),
            b'l' => self.each_mode(csi, Switch::Reset, ModeSet::Private),
            b'J' => {
                if let Some(extent) = EraseExtent::from_param(csi.param(0, 0)) {
                    self.erase_display(extent, Erase::Unprotected);
                }
            }
            b'K' => {
                if let Some(extent) = EraseExtent::from_param(csi.param(0, 0)) {
                    self.erase_line(extent, Erase::Unprotected);
                }
            }
            b'n' => self.device_status_private(csi.param(0, 0), csi.param(1, 0)),
            b'W' if csi.param(0, 0) == 5 => self.tabs.reset(self.cols),
            b'S' => self.graphics_attributes(csi),
            b'u' => self.report_keyboard_flags(),
            b's' => self.save_private_modes(csi),
            b'r' => self.restore_private_modes(csi),
            _ => {}
        }
    }

    /// SM/RM and DECSET/DECRST: every mode in the list.
    fn each_mode(&mut self, csi: &Csi, switch: Switch, set: ModeSet) {
        for i in 0..csi.len() {
            let Some(n) = csi.get(i) else { continue };
            match set {
                ModeSet::Private => self.set_private_mode(n, switch),
                ModeSet::Ansi => self.set_ansi_mode(n, switch),
            }
        }
    }
}
