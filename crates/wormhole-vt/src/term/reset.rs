//! Resets and the screen alignment pattern: RIS, DECSTR, DECALN.
//!
//! Spec: VT510 RIS ("returns the terminal to its initial state": clears page memory, homes the
//! cursor, normal rendition, default character sets, no margins, absolute origin, replace mode,
//! the main screen), DECSTR (Table 5-9: the modes and the rendition, not the screen's contents or
//! the cursor's place) and DECALN (fills the screen with E, homes the cursor, margins to the
//! extremes of the page).
//!
//! DECSTR leaves autowrap on. Table 5-9 resets it to the user's set-up default, which for
//! xterm-compatible programs is on (terminfo's `am`): ncurses sends DECSTR in `rs2` and expects
//! lines to still wrap afterwards.

use crate::grid::{Pen, Style};
use crate::types::CursorShape;

use super::conformance::Level;
use super::modes::Modes;
use super::tabs::TabStops;
use super::{Cursor, Event, Margins, Saved, Terminal};

impl Terminal {
    /// RIS.
    pub fn reset(&mut self) {
        // A screen DECCOLM made 132 columns wide goes back to 80 (xterm: a full reset checks that
        // the width can change and that it is changed).
        if self.modes.contains(Modes::COLUMNS_132) {
            self.resize(80, self.rows);
            self.push_event(Event::ColumnsRequested(80));
        }
        self.leave_alt_screen();
        self.grid.reset();
        self.other.reset();
        self.forget_all_images();
        self.cursor = Cursor::default();
        self.pen = Pen::default();
        // Mode 40 is a permission rather than a state of the screen (xterm keeps it in a resource):
        // a full reset leaves it as the program set it.
        self.modes = Modes::INITIAL | (self.modes & Modes::ALLOW_COLUMNS);
        self.recorded = 0;
        self.saved_modes.clear();
        self.margins = Margins::full(self.cols, self.rows);
        self.tabs = TabStops::new(self.cols);
        self.charsets = Default::default();
        self.locator = Default::default();
        self.preferred = Default::default();
        self.saved = [Saved::default(), Saved::default()];
        for keys in &mut self.keys {
            keys.reset();
        }
        self.sync_keyboard_flags();
        self.cursor_shape = None;
        self.cursor_blink = None;
        self.title_stack.clear();
        self.title_modes = Default::default();
        self.reset_colors();
        self.last_char = None;
        self.cluster.open = false;
        self.history_just_cleared = false;
        self.attr_extent = Default::default();
        self.guard_used = false;
        self.kept = Default::default();
        self.level = Level::Vt500;
        self.eight_bit = false;
        self.push_event(Event::FullReset);
    }

    /// DECSTR.
    pub(super) fn soft_reset(&mut self) {
        self.modes.remove(Modes::SOFT_RESET);
        self.modes.insert(Modes::SHOW_CURSOR | Modes::AUTOWRAP | Modes::GRAPHEME_CLUSTERS);
        self.margins = Margins::full(self.cols, self.rows);
        self.pen.style = Style::DEFAULT;
        self.pen.protected = false;
        self.pen.guarded = false;
        self.pen.link = None;
        self.charsets = Default::default();
        // Table 5-9: DECNRCM goes back to the multinational set and DECAUPSS to its default.
        self.preferred = Default::default();
        if let Some(bit) = super::modes::recorded_bit(42) {
            self.recorded &= !bit;
        }
        self.saved = [Saved::default(), Saved::default()];
        self.cursor_shape = None;
        self.cursor_blink = None;
        self.cluster.open = false;
    }

    /// DECALN: a screen of E's to line the picture up by.
    pub(super) fn screen_alignment(&mut self) {
        self.margins = Margins::full(self.cols, self.rows);
        self.modes.remove(Modes::ORIGIN | Modes::LEFT_RIGHT_MARGINS);
        let row = vec![b'E'; self.cols];
        let pen = Pen::default();
        for y in 0..self.rows {
            self.grid.write_ascii(y, 0, &row, &pen);
            self.grid.set_wrapped(y, false);
        }
        self.cursor = Cursor::default();
    }

    /// DECSCUSR: the shape and whether it blinks, `0` meaning the host's default.
    pub(super) fn set_cursor_style(&mut self, ps: u16) {
        let (shape, blink) = match ps {
            0 => (None, None),
            1 => (Some(CursorShape::Block), Some(true)),
            2 => (Some(CursorShape::Block), Some(false)),
            3 => (Some(CursorShape::Underline), Some(true)),
            4 => (Some(CursorShape::Underline), Some(false)),
            5 => (Some(CursorShape::Beam), Some(true)),
            6 => (Some(CursorShape::Beam), Some(false)),
            _ => return,
        };
        self.cursor_shape = shape;
        self.cursor_blink = blink;
    }
}
