//! The color commands: OSC 4 and 5 (palette and special colors), OSC 10 to 19 (the dynamic
//! colors), OSC 104, 105 and 110 to 119 (their resets).
//!
//! Spec: xterm's ctlseqs, "Operating System Commands". OSC 4 `c ; spec ; …` sets palette entry
//! `c` or, with `?` for the spec, asks for it; the numbers 256 to 260 name the special colors
//! (bold, underline, blink, reverse, italic) that OSC 5 `n ; spec` addresses as 0 to 4. OSC 10
//! sets the foreground, 11 the background, 12 the text cursor, 13 to 19 the colors of the
//! pointer, the Tek 4014 window and the highlight; a list of specs sets the following ones too.
//! The replies end as the request did. OSC 104 resets the palette entries it lists (all of them
//! without a list), 105 the special colors, 110 to 119 the dynamic colors 10 to 19 (xterm's
//! numbers are the dynamic ones plus 100).
//!
//! What a program sets is kept here over the host's palette, per terminal, until it is reset
//! (and RIS resets it): the host draws with it ([`Terminal::colors`]), and a query answers the
//! color in force. The special colors and the dynamic colors past the cursor's are kept and
//! answered but not drawn: xterm needs a resource to draw with them, and nothing here asks for it.

use crate::types::{COLOR_COUNT, Colors, NamedColor, Rgb};

use crate::terminal::Terminal;
use crate::terminal::event::Terminator;
use crate::terminal::osc::spec::{ColorSpec, parse_color};

/// How many special colors there are: bold, underline, blink, reverse, italic.
pub(in crate::terminal) const SPECIAL_COLORS: usize = 5;
/// The dynamic colors OSC 13 to 19 set, past the foreground, background and cursor.
pub(in crate::terminal) const EXTRA_DYNAMIC: usize = 7;
/// The palette number of the first special color in OSC 4.
const SPECIAL_BASE: usize = 256;

/// The colors programs set over the host's palette, and that palette.
pub(in crate::terminal) struct Palette {
    /// What programs set with OSC 4, 10, 11 and 12: what the host draws with.
    pub(in crate::terminal) set: Colors,
    /// The special colors (OSC 4 from 256, OSC 5) and the dynamic colors OSC 13 to 19 set.
    pub(in crate::terminal) special: [Option<Rgb>; SPECIAL_COLORS],
    pub(in crate::terminal) extra: [Option<Rgb>; EXTRA_DYNAMIC],
    /// The host's palette, which color queries answer with when a program has not changed an
    /// entry.
    pub(in crate::terminal) defaults: Box<[Rgb; COLOR_COUNT]>,
    /// A command may have changed `set` since the feed began: the end of the feed compares it with
    /// what the host was told (`watch`), and only then.
    pub(in crate::terminal) touched: bool,
}

impl Palette {
    /// Nothing set, over xterm's palette until the host gives its own.
    pub(in crate::terminal) fn new() -> Palette {
        Palette {
            set: Colors::default(),
            special: [None; SPECIAL_COLORS],
            extra: [None; EXTRA_DYNAMIC],
            defaults: Box::new(crate::terminal::osc::spec::xterm_defaults()),
            touched: false,
        }
    }
}

/// The color a query for an extra dynamic color (OSC 13 to 19) answers when none was set:
/// the pointer and Tek colors follow the text colors, the highlight is the text on its background.
const EXTRA_DEFAULTS: [NamedColor; EXTRA_DYNAMIC] = [
    NamedColor::Foreground, // 13: pointer foreground
    NamedColor::Background, // 14: pointer background
    NamedColor::Foreground, // 15: Tek foreground
    NamedColor::Background, // 16: Tek background
    NamedColor::Foreground, // 17: highlight background
    NamedColor::Cursor,     // 18: Tek cursor
    NamedColor::Background, // 19: highlight foreground
];

/// A color the commands name.
#[derive(Clone, Copy)]
enum Slot {
    /// A palette entry, 0 to 255.
    Palette(usize),
    /// A special color, 0 to 4.
    Special(usize),
    /// The foreground, background or cursor.
    Named(NamedColor),
    /// A dynamic color from OSC 13 to 19, 0 to 6.
    Extra(usize),
}

/// The number in `text`, if it is one.
fn number(text: &[u8]) -> Option<usize> {
    std::str::from_utf8(text).ok()?.parse::<u16>().ok().map(usize::from)
}

impl Terminal {
    /// The color in force for `slot`: the program's, else the host's.
    fn color_of(&self, slot: Slot) -> Rgb {
        match slot {
            Slot::Palette(i) => self.palette.set[i].unwrap_or(self.palette.defaults[i]),
            Slot::Named(named) => self.palette.set[named].unwrap_or(self.palette.defaults[named as usize]),
            // Nothing in the host's palette stands for these: the foreground does.
            Slot::Special(i) => {
                self.palette.special[i].unwrap_or(self.palette.defaults[NamedColor::Foreground as usize])
            }
            Slot::Extra(i) => {
                self.palette.extra[i].unwrap_or(self.palette.defaults[EXTRA_DEFAULTS[i] as usize])
            }
        }
    }

    fn set_color(&mut self, slot: Slot, color: Option<Rgb>) {
        self.palette.touched = true;
        match slot {
            Slot::Palette(i) => self.palette.set[i] = color,
            Slot::Named(named) => self.palette.set[named] = color,
            Slot::Special(i) => self.palette.special[i] = color,
            Slot::Extra(i) => self.palette.extra[i] = color,
        }
    }

    /// OSC 4 `c ; spec ; c ; spec …` with palette numbers (0 to 255) and special colors (256 to
    /// 260), and OSC 5 `n ; spec …` with special colors alone (`command` says which).
    pub(in crate::terminal) fn osc_palette(&mut self, command: u32, rest: &[u8], end: Terminator) {
        let mut parts = rest.split(|&b| b == b';');
        while let (Some(index), Some(spec)) = (parts.next(), parts.next()) {
            let Some(n) = number(index) else { continue };
            let slot = match (command, n) {
                (4, 0..=255) => Slot::Palette(n),
                (4, SPECIAL_BASE..) if n < SPECIAL_BASE + SPECIAL_COLORS => Slot::Special(n - SPECIAL_BASE),
                (5, 0..=4) => Slot::Special(n),
                _ => continue,
            };
            if spec == b"?" {
                let color = ColorSpec(self.color_of(slot));
                let end = end.as_str();
                self.reply(format_args!("\x1b]{command};{n};{color}{end}"));
            } else if let Some(color) = parse_color(spec) {
                self.set_color(slot, Some(color));
            }
        }
    }

    /// OSC 10 to 19 `spec ; spec …`: this dynamic color and the ones after it.
    pub(in crate::terminal) fn osc_dynamic_colors(&mut self, number: u32, rest: &[u8], end: Terminator) {
        for (offset, spec) in rest.split(|&b| b == b';').enumerate() {
            let command = number as usize + offset;
            let Some(slot) = dynamic_slot(command) else { break };
            if spec == b"?" {
                let color = ColorSpec(self.color_of(slot));
                let end = end.as_str();
                self.reply(format_args!("\x1b]{command};{color}{end}"));
            } else if let Some(color) = parse_color(spec) {
                self.set_color(slot, Some(color));
            }
        }
    }

    /// OSC 104: the palette entries listed go back to the host's colors, and all of them
    /// (the special colors too) without a list.
    pub(in crate::terminal) fn osc_reset_palette(&mut self, rest: &[u8]) {
        self.palette.touched = true;
        if rest.is_empty() {
            for slot in 0..SPECIAL_BASE {
                self.palette.set[slot] = None;
            }
            self.palette.special = [None; SPECIAL_COLORS];
            return;
        }
        for index in rest.split(|&b| b == b';') {
            match number(index) {
                Some(n @ 0..=255) => self.palette.set[n] = None,
                Some(n) if (SPECIAL_BASE..SPECIAL_BASE + SPECIAL_COLORS).contains(&n) => {
                    self.palette.special[n - SPECIAL_BASE] = None;
                }
                _ => {}
            }
        }
    }

    /// OSC 105: the special colors listed (0 to 4), or all, go back to the host's.
    pub(in crate::terminal) fn osc_reset_special(&mut self, rest: &[u8]) {
        if rest.is_empty() {
            self.palette.special = [None; SPECIAL_COLORS];
            return;
        }
        for index in rest.split(|&b| b == b';') {
            if let Some(n @ 0..=4) = number(index) {
                self.palette.special[n] = None;
            }
        }
    }

    /// OSC 110 to 119: dynamic color 10 to 19 back to the host's.
    pub(in crate::terminal) fn osc_reset_dynamic(&mut self, command: u32) {
        if let Some(slot) = dynamic_slot(command as usize - 100) {
            self.set_color(slot, None);
        }
    }

    /// RIS: everything a program set.
    pub(in crate::terminal) fn reset_colors(&mut self) {
        self.palette.touched = true;
        self.palette.set = Default::default();
        self.palette.special = [None; SPECIAL_COLORS];
        self.palette.extra = [None; EXTRA_DYNAMIC];
    }
}

/// The dynamic color OSC `command` (10 to 19) stands for.
fn dynamic_slot(command: usize) -> Option<Slot> {
    Some(match command {
        10 => Slot::Named(NamedColor::Foreground),
        11 => Slot::Named(NamedColor::Background),
        12 => Slot::Named(NamedColor::Cursor),
        13..=19 => Slot::Extra(command - 13),
        _ => return None,
    })
}
