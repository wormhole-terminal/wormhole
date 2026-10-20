//! XTPUSHCOLORS, XTPOPCOLORS and XTREPORTCOLORS: saving and restoring the colors a program set.
//!
//! Spec: xterm's ctlseqs. `CSI # P` pushes the dynamic and ANSI palette colors programs changed
//! (what [`Terminal::colors`] returns, and the special and extra dynamic colors OSC 5 and 13 to 19
//! set) onto a stack of 10; `CSI # Q` pops it and sets them back, so a program that changes the
//! theme can put it as it found it. A parameter 1 to 10 names a slot instead: `CSI n # P` stores
//! the palette in slot `n` without pushing, and `CSI n # Q` restores slot `n` without popping. Slots
//! are contiguous, so a store can overwrite one or add the next; a store or a restore of a slot
//! beyond that, a push on a full stack and a pop on an empty one do nothing. `CSI # R` reports as
//! `CSI ? current ; stored # Q`: the entry a pop would restore and how many are stored, which are the
//! same number here, the stack having no other cursor. RIS empties the stack.
//!
//! A pop changes the palette like OSC 4 does, and so the host hears [`Event::ColorsChanged`](
//! crate::Event::ColorsChanged) for it.

use crate::types::{Colors, Rgb};

use crate::terminal::Terminal;
use crate::terminal::osc::colors::{EXTRA_DYNAMIC, SPECIAL_COLORS};

/// How deep the stack goes (xterm's limit).
const DEPTH: usize = 10;

/// The colors programs had set when a palette was pushed.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(in crate::terminal) struct Snapshot {
    pub(in crate::terminal) set: Colors,
    pub(in crate::terminal) special: [Option<Rgb>; SPECIAL_COLORS],
    pub(in crate::terminal) extra: [Option<Rgb>; EXTRA_DYNAMIC],
}

impl std::fmt::Debug for Snapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?} {:?} {:?}", self.set, self.special, self.extra)
    }
}

/// XTPUSHCOLORS' stack.
#[derive(Clone, Debug, Default)]
pub(in crate::terminal) struct ColorStack {
    entries: Vec<Snapshot>,
}

impl ColorStack {
    /// The palettes, the oldest first.
    pub(in crate::terminal) fn entries(&self) -> &[Snapshot] {
        &self.entries
    }
}

impl Terminal {
    /// XTPUSHCOLORS (`CSI Pm # P`).
    pub(in crate::terminal) fn push_colors(&mut self, slot: u16) {
        let now =
            Snapshot { set: self.palette.set, special: self.palette.special, extra: self.palette.extra };
        let entries = &mut self.color_stack.entries;
        match usize::from(slot) {
            0 if entries.len() < DEPTH => entries.push(now),
            n @ 1..=DEPTH if n <= entries.len() => entries[n - 1] = now,
            n @ 1..=DEPTH if n == entries.len() + 1 => entries.push(now),
            _ => {}
        }
    }

    /// XTPOPCOLORS (`CSI Pm # Q`).
    pub(in crate::terminal) fn pop_colors(&mut self, slot: u16) {
        let entries = &mut self.color_stack.entries;
        let saved = match usize::from(slot) {
            0 => entries.pop(),
            n @ 1..=DEPTH => entries.get(n - 1).copied(),
            _ => None,
        };
        if let Some(saved) = saved {
            self.palette.set = saved.set;
            self.palette.special = saved.special;
            self.palette.extra = saved.extra;
            self.palette.touched = true;
        }
    }

    /// XTREPORTCOLORS (`CSI # R`).
    pub(in crate::terminal) fn report_color_stack(&mut self) {
        let stored = self.color_stack.entries.len();
        self.reply(format_args!("\x1b[?{stored};{stored}#Q"));
    }
}
