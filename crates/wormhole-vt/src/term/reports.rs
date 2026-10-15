//! What the terminal answers: device status, attributes, mode and version reports, window
//! reports. The bytes go to [`Terminal::replies`](super::Terminal::replies) in the order the
//! requests came.
//!
//! Spec: xterm's ctlseqs for every reply's form (DSR `CSI 5 n` → `CSI 0 n`, CPR `CSI 6 n` →
//! `CSI r ; c R`, DA1/DA2/DA3, DECRQM `CSI Ps $ p` → `CSI Ps ; Pm $ y`, XTVERSION
//! `CSI > q` → `DCS > | text ST`, XTWINOPS 14/16/18); the VT510 manual for the status of the
//! printer, the user-defined keys and the keyboard (DSR `?15`, `?25`, `?26`).
//!
//! Window reports that need pixels use the cell size the host gave ([`Terminal::set_cell_size`]).
//! Reports that would let a program read the window's title (XTWINOPS 20 and 21) are not
//! answered: a title is attacker-controlled text, and answering would put it in the input of
//! whatever program is running (xterm leaves them off by default for the same reason).

use std::io::Write;

use crate::parser::Csi;

use super::Terminal;
use super::conformance::Level;
use super::modes::{Modes, Status};

/// The most reply bytes kept for a host that does not collect them.
const REPLIES_MAX: usize = 1 << 20;

/// The version in XTVERSION: Wormhole's, as the crate says it.
const VERSION: &str = env!("CARGO_PKG_VERSION");

/// The version a secondary DA reply carries: an xterm patch level, not Wormhole's version.
/// Programs that ask (vim, esctest) compare it with xterm's patch numbers, which run from about
/// 300 to 999; Wormhole's own version is XTVERSION's answer. It stays fixed so a release never
/// changes what programs conclude about the terminal (1.0.0 would have sent 10000).
const DA2_VERSION: u32 = 777;

/// What DA1 lists, with the conformance level a family needs to have it. Each is a feature that is
/// there: 132 columns (DECCOLM, with mode 40), sixel graphics, selective erase, national
/// replacement character sets (with DECNRCM), the DEC Technical set, a locator port (the pointer,
/// as DECELR and its friends use it), terminal state interrogation (DECRQSS, DECRQPSR, DECRQTSR),
/// user windows (one page, the window's size: DECRQDE and the window extension controls),
/// horizontal scrolling (SL, SR, DECSLRM, DECBI, DECFI, DECIC, DECDC), ANSI color, rectangular
/// editing, the ANSI text locator (DEC locator mode) and xterm's clipboard access. Not listed:
/// 2, a printer port (Media Copy has nowhere to send a page), 3 ReGIS, 7 soft characters (DRCS),
/// 8 user-defined keys and the VT510's others.
const FEATURES: [(u16, Level); 13] = [
    (1, Level::Vt200),
    (4, Level::Vt200),
    (6, Level::Vt200),
    (9, Level::Vt200),
    (15, Level::Vt300),
    (16, Level::Vt300),
    (17, Level::Vt400),
    (18, Level::Vt300),
    (21, Level::Vt400),
    (22, Level::Vt200),
    (28, Level::Vt400),
    (29, Level::Vt300),
    (52, Level::Vt200),
];

impl Terminal {
    /// Append a reply. Writing to a `Vec` cannot fail. A host that does not write the replies out
    /// (the program is not reading them) must not grow the buffer without bound: past the cap
    /// new ones are dropped.
    pub(super) fn reply(&mut self, args: std::fmt::Arguments<'_>) {
        // An answer about a picture is waiting for the host: this one goes out after it.
        if self.graphics.hold_reply(args) {
        } else if self.replies.len() < REPLIES_MAX {
            let from = self.replies.len();
            let _ = self.replies.write_fmt(args);
            if self.eight_bit {
                super::conformance::eight_bit_in_place(&mut self.replies, from);
            }
        }
    }

    /// DSR (`CSI Ps n`).
    pub(super) fn device_status(&mut self, ps: u16) {
        match ps {
            5 => self.reply(format_args!("\x1b[0n")),
            6 => {
                let (row, col) = self.report_position();
                self.reply(format_args!("\x1b[{row};{col}R"));
            }
            _ => {}
        }
    }

    /// The cursor as CPR reports it: 1-based, relative to the margins in origin mode. A pending
    /// wrap reports the last column.
    fn report_position(&self) -> (usize, usize) {
        if self.modes.contains(Modes::ORIGIN) {
            (
                self.cursor.y.saturating_sub(self.margins.top) + 1,
                self.cursor.x.saturating_sub(self.margins.left) + 1,
            )
        } else {
            (self.cursor.y + 1, self.cursor.x + 1)
        }
    }

    /// DSR with the `?` marker (DEC-specific reports, VT510 DECDSR). `pid` is the second
    /// parameter (the checksum request's id).
    pub(super) fn device_status_private(&mut self, ps: u16, pid: u16) {
        match ps {
            // DECXCPR: the VT400 family adds the page (always 1) to xterm's form.
            6 => {
                let (row, col) = self.report_position();
                if self.level >= Level::Vt400 {
                    self.reply(format_args!("\x1b[?{row};{col};1R"));
                } else {
                    self.reply(format_args!("\x1b[?{row};{col}R"));
                }
            }
            // Printer: no printer (13).
            15 => self.reply(format_args!("\x1b[?13n")),
            // User-defined keys: unlocked (20).
            25 => self.reply(format_args!("\x1b[?20n")),
            // Keyboard: North American (27;1), ready (0) and an LK201 (0) for the families that
            // report the status and the type.
            26 => match self.level {
                Level::Vt100 | Level::Vt200 => self.reply(format_args!("\x1b[?27;1n")),
                Level::Vt300 => self.reply(format_args!("\x1b[?27;1;0n")),
                Level::Vt400 | Level::Vt500 => self.reply(format_args!("\x1b[?27;1;0;0n")),
            },
            // Locator: available (53), which is the pointer; it is a mouse (57;1).
            53 | 55 => self.reply(format_args!("\x1b[?53n")),
            56 => self.reply(format_args!("\x1b[?57;1n")),
            // Macro space: none (DECMSR, `CSI Pn * {`). Macro memory checksum: nothing in it
            // (DECCKSR, `DCS Pid ! ~ 0000 ST`).
            62 => self.reply(format_args!("\x1b[0*{{")),
            63 => self.reply(format_args!("\x1bP{pid}!~0000\x1b\\")),
            // Data integrity: ready, no malfunction.
            75 => self.reply(format_args!("\x1b[?70n")),
            // Multiple sessions: not configured for them.
            85 => self.reply(format_args!("\x1b[?83n")),
            _ => {}
        }
    }

    /// DA1: the family the conformance level names, and its features: what the VT510 manual and
    /// xterm's ctlseqs number, and only what the engine does at that level (`FEATURES`).
    pub(super) fn primary_attributes(&mut self) {
        if self.level == Level::Vt100 {
            self.reply(format_args!("\x1b[?1;2c"));
            return;
        }
        // In pieces, not in a string: a reply allocates nothing.
        self.reply(format_args!("\x1b[?{}", self.level.param()));
        for (feature, since) in FEATURES {
            if self.level >= since {
                self.reply(format_args!(";{feature}"));
            }
        }
        self.reply(format_args!("c"));
    }

    /// DA2: the terminal type of the conformance level's family, the version, and no ROM
    /// cartridge (0).
    pub(super) fn secondary_attributes(&mut self) {
        let kind = self.level.terminal_type();
        self.reply(format_args!("\x1b[>{kind};{DA2_VERSION};0c"));
    }

    /// DA3: the unit id, eight hex digits (zero: a terminal that is not a physical unit).
    pub(super) fn tertiary_attributes(&mut self) {
        self.reply(format_args!("\x1bP!|00000000\x1b\\"));
    }

    /// XTVERSION.
    pub(super) fn report_version(&mut self) {
        self.reply(format_args!("\x1bP>|Wormhole {VERSION}\x1b\\"));
    }

    /// DECRQM for an ANSI mode.
    pub(super) fn report_ansi_mode(&mut self, n: u16) {
        let status = self.ansi_status(n) as u8;
        self.reply(format_args!("\x1b[{n};{status}$y"));
    }

    /// DECRQM for a DEC private mode.
    pub(super) fn report_private_mode(&mut self, n: u16) {
        let status: Status = self.private_status(n);
        self.reply(format_args!("\x1b[?{n};{}$y", status as u8));
    }

    /// The kitty keyboard protocol's query: the flags in force.
    pub(super) fn report_keyboard_flags(&mut self) {
        let flags = self.keys[usize::from(self.on_alt_screen())].current();
        self.reply(format_args!("\x1b[?{flags}u"));
    }

    /// XTSMGRAPHICS (`CSI ? Pi ; Pa ; Pv S`): what a sixel program asks before drawing. Item 1 is
    /// the number of color registers, item 2 the largest picture, item 3 ReGIS (not supported).
    pub(super) fn graphics_attributes(&mut self, csi: &Csi) {
        let item = csi.param(0, 0);
        let action = csi.param(1, 0);
        // Reading (1) or reading the maximum (4) is all we answer; setting is refused (3).
        let ok = action == 1 || action == 4;
        match item {
            1 if ok => self.reply(format_args!("\x1b[?1;0;256S")),
            2 if ok => {
                let w = self.cols * usize::from(self.cell_px.0);
                let h = self.rows * usize::from(self.cell_px.1);
                self.reply(format_args!("\x1b[?2;0;{w};{h}S"));
            }
            1..=3 => self.reply(format_args!("\x1b[?{item};3;0S")),
            _ => {}
        }
    }
}
