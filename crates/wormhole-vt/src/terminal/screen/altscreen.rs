//! The alternate screen: modes 47, 1047, 1048 and 1049.
//!
//! Spec: xterm's ctlseqs. 47 only switches buffers; 1047 is the same and clears the alternate
//! screen when it is left; 1048 saves and restores the cursor as DECSC and DECRC do; 1049 is
//! 1048 and 1047 together (save the cursor, switch, clear on entry; switch back, restore). The
//! alternate screen has no history, and the cursor, pen and margins are the terminal's, not the
//! screen's, so a switch moves none of them.

use crate::grid::BlockPart;
use crate::terminal::Terminal;
use crate::terminal::modes::{Modes, Switch};

/// Whether showing the alternate screen blanks it first.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::terminal) enum Blank {
    Yes,
    No,
}

impl Terminal {
    /// Whether the alternate screen is showing.
    pub(in crate::terminal) fn on_alt_screen(&self) -> bool {
        self.modes.contains(Modes::ALT_SCREEN)
    }

    /// The index of what is kept per screen (DECSC's slot, the keyboard flags, the pictures) for
    /// the screen showing: 0 for the main screen, 1 for the alternate one.
    pub(in crate::terminal) fn screen_slot(&self) -> usize {
        usize::from(self.on_alt_screen())
    }

    /// Show the alternate screen, blank when `blank` says so and it was not showing already
    /// (xterm clears when it switches; asking again for a screen that shows changes nothing).
    pub(in crate::terminal) fn enter_alt_screen(&mut self, blank: Blank) {
        if self.on_alt_screen() {
            return;
        }
        // The selection's anchors are rows of the screen that is going away.
        self.clear_selection();
        std::mem::swap(&mut self.grid, &mut self.other);
        self.modes.insert(Modes::ALT_SCREEN);
        // Whatever the program prints while it owns the screen is not part of a command block.
        self.stashed_content = std::mem::replace(&mut self.pen.content, BlockPart::None);
        self.sync_keyboard_flags();
        self.grid.damage_all();
        self.cluster.open = false;
        // The cursor is shared and the rows are not: this screen's row may show fewer columns.
        self.normalize_cursor();
        if blank == Blank::Yes {
            self.clear_alt_screen();
        }
    }

    /// Blank the screen showing, with the pen's background as ED 2 does (`bce`).
    fn clear_alt_screen(&mut self) {
        let bg = self.erase_color();
        self.grid.erase_rows(0..self.rows, bg, false);
    }

    /// Show the main screen again.
    pub(in crate::terminal) fn leave_alt_screen(&mut self) {
        if self.on_alt_screen() {
            self.clear_selection();
            std::mem::swap(&mut self.grid, &mut self.other);
            self.modes.remove(Modes::ALT_SCREEN);
            // The pictures of a screen are its own, and this one is gone from view.
            self.forget_screen_images(1);
            self.pen.content = std::mem::replace(&mut self.stashed_content, BlockPart::None);
            self.sync_keyboard_flags();
            self.grid.damage_all();
            self.cluster.open = false;
            self.normalize_cursor();
        }
    }

    /// Mode 47, 1047 or 1049 set or reset.
    pub(in crate::terminal) fn set_alt_screen_mode(&mut self, n: u16, switch: Switch) {
        match (n, switch) {
            (47 | 1047, Switch::Set) => self.enter_alt_screen(Blank::No),
            (1049, Switch::Set) => {
                self.save_cursor();
                self.enter_alt_screen(Blank::Yes);
            }
            (47, Switch::Reset) => self.leave_alt_screen(),
            (1047, Switch::Reset) => {
                if self.on_alt_screen() {
                    self.clear_alt_screen();
                }
                self.leave_alt_screen();
            }
            (1049, Switch::Reset) if self.on_alt_screen() => {
                self.leave_alt_screen();
                self.restore_cursor();
            }
            _ => {}
        }
    }

    /// The kitty flags of the screen that shows, as modes.
    pub(in crate::terminal) fn sync_keyboard_flags(&mut self) {
        let flags = self.keys[self.screen_slot()].current();
        self.modes.remove(Modes::KEYS);
        self.modes.insert(Modes::from_bits_retain(u64::from(flags) << 12));
    }
}
