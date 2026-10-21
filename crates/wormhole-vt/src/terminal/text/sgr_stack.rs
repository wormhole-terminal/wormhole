//! XTPUSHSGR, XTPOPSGR and XTREPORTSGR: saving and restoring the pen's rendition, and reading the
//! rendition a rectangle of cells has in common.
//!
//! Spec: xterm's ctlseqs. `CSI Pm # {` (or `CSI Pm # p`) pushes the video attributes the numbers
//! select onto a stack, "limited to 10 levels"; with no numbers "all of the video attributes are
//! saved". The selectors are the SGR numbers of bold (1), faint (2), italic (3), underline (4),
//! blink (5), inverse (7), invisible (8), crossed-out (9), double underline (21), and 30 and 31 for
//! the foreground and background colors. `CSI # }` (or `CSI # q`) pops: the attributes that were
//! pushed go back to the pen and the rest of the pen stays as it is. A push on a full stack and a
//! pop on an empty one do nothing, and RIS empties the stack. Underline (4) saves and restores any
//! underline style but double (the single, curly, dotted and dashed ones), and the pen keeps one
//! style at a time.
//!
//! `CSI Pt ; Pl ; Pb ; Pr # |` answers with an SGR sequence that holds what every cell of the
//! rectangle (as DECCARA reads it) has in common: the attributes all have, the foreground and the
//! background when they are the same everywhere, and nothing for what differs. The form is the one
//! DECRQSS gives for SGR (`CSI 0 ; 1 ; … m`). A rectangle that is upside down is not answered.

use crate::grid::Style;
use crate::parser::Csi;
use crate::types::{Color, Flags, Line};

use crate::terminal::Terminal;
use crate::terminal::reports::rqss::SgrParams;

/// How deep the stack goes (xterm's limit).
const DEPTH: usize = 10;

/// The attributes of the pen a push selects, one bit each.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::terminal) struct Mask(u16);

impl Mask {
    const BOLD: u16 = 1;
    const FAINT: u16 = 1 << 1;
    const ITALIC: u16 = 1 << 2;
    const UNDERLINE: u16 = 1 << 3;
    const BLINK: u16 = 1 << 4;
    const INVERSE: u16 = 1 << 5;
    const INVISIBLE: u16 = 1 << 6;
    const CROSSED_OUT: u16 = 1 << 7;
    const DOUBLE_UNDERLINE: u16 = 1 << 8;
    const FOREGROUND: u16 = 1 << 9;
    const BACKGROUND: u16 = 1 << 10;
    const ALL: u16 = (1 << 11) - 1;

    /// The selector numbers, in the order a push lists them, with their bits.
    pub(in crate::terminal) const SELECTORS: [(u16, u16); 11] = [
        (1, Mask::BOLD),
        (2, Mask::FAINT),
        (3, Mask::ITALIC),
        (4, Mask::UNDERLINE),
        (5, Mask::BLINK),
        (7, Mask::INVERSE),
        (8, Mask::INVISIBLE),
        (9, Mask::CROSSED_OUT),
        (21, Mask::DOUBLE_UNDERLINE),
        (30, Mask::FOREGROUND),
        (31, Mask::BACKGROUND),
    ];

    /// The attribute flags a mask covers, by selector.
    const FLAGS: [(u16, Flags); 9] = [
        (Mask::BOLD, Flags::BOLD),
        (Mask::FAINT, Flags::DIM),
        (Mask::ITALIC, Flags::ITALIC),
        (
            Mask::UNDERLINE,
            Flags::UNDERLINE
                .union(Flags::UNDERCURL)
                .union(Flags::DOTTED_UNDERLINE)
                .union(Flags::DASHED_UNDERLINE),
        ),
        (Mask::BLINK, Flags::BLINK),
        (Mask::INVERSE, Flags::INVERSE),
        (Mask::INVISIBLE, Flags::HIDDEN),
        (Mask::CROSSED_OUT, Flags::STRIKEOUT),
        (Mask::DOUBLE_UNDERLINE, Flags::DOUBLE_UNDERLINE),
    ];

    /// The selectors a push named: all of them without parameters, and only the known ones with.
    fn of(csi: &Csi) -> Mask {
        // A default (none, or 0) is all of them.
        if csi.len() <= 1 && csi.param(0, 0) == 0 {
            return Mask(Mask::ALL);
        }
        let mut mask = 0;
        for i in 0..csi.len() {
            if let Some(n) = csi.get(i)
                && let Some(&(_, bit)) = Mask::SELECTORS.iter().find(|(number, _)| *number == n)
            {
                mask |= bit;
            }
        }
        Mask(mask)
    }

    /// The selector numbers this mask has, for writing a push.
    pub(in crate::terminal) fn numbers(self) -> impl Iterator<Item = u16> {
        Mask::SELECTORS.into_iter().filter(move |(_, bit)| self.0 & bit != 0).map(|(number, _)| number)
    }

    fn has(self, bit: u16) -> bool {
        self.0 & bit != 0
    }
}

/// One push: what was selected, and the pen's value of just those attributes (the rest is the
/// default, so two entries that saved the same compare equal).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::terminal) struct Entry {
    pub(in crate::terminal) mask: Mask,
    pub(in crate::terminal) style: Style,
}

impl Entry {
    fn save(mask: Mask, pen: &Style) -> Entry {
        let mut style = Style::DEFAULT;
        for (bit, flags) in Mask::FLAGS {
            if mask.has(bit) {
                style.attrs |= pen.attrs & flags;
            }
        }
        if mask.has(Mask::FOREGROUND) {
            style.fg = pen.fg;
        }
        if mask.has(Mask::BACKGROUND) {
            style.bg = pen.bg;
        }
        Entry { mask, style }
    }

    /// Put the saved attributes back into `pen`.
    fn restore(&self, pen: &mut Style) {
        for (bit, flags) in Mask::FLAGS {
            if self.mask.has(bit) {
                pen.attrs = (pen.attrs - flags) | (self.style.attrs & flags);
            }
        }
        // One underline style at a time: the one that came back wins over the other kind.
        if self.mask.has(Mask::UNDERLINE) && self.style.attrs.intersects(Flags::ALL_UNDERLINES) {
            pen.attrs -= Flags::DOUBLE_UNDERLINE;
        } else if self.mask.has(Mask::DOUBLE_UNDERLINE) && self.style.attrs.contains(Flags::DOUBLE_UNDERLINE)
        {
            pen.attrs -= Flags::ALL_UNDERLINES - Flags::DOUBLE_UNDERLINE;
        }
        if self.mask.has(Mask::FOREGROUND) {
            pen.fg = self.style.fg;
        }
        if self.mask.has(Mask::BACKGROUND) {
            pen.bg = self.style.bg;
        }
    }
}

/// XTPUSHSGR's stack.
#[derive(Clone, Debug, Default)]
pub(in crate::terminal) struct SgrStack {
    entries: Vec<Entry>,
}

impl SgrStack {
    /// The entries, the oldest first.
    pub(in crate::terminal) fn entries(&self) -> &[Entry] {
        &self.entries
    }
}

impl Terminal {
    /// XTPUSHSGR (`CSI Pm # {`, `CSI Pm # p`).
    pub(in crate::terminal) fn push_sgr(&mut self, csi: &Csi) {
        let mask = Mask::of(csi);
        if mask.0 == 0 || self.sgr_stack.entries.len() >= DEPTH {
            return;
        }
        self.sgr_stack.entries.push(Entry::save(mask, &self.pen.style));
    }

    /// XTPOPSGR (`CSI # }`, `CSI # q`).
    pub(in crate::terminal) fn pop_sgr(&mut self) {
        if let Some(entry) = self.sgr_stack.entries.pop() {
            entry.restore(&mut self.pen.style);
        }
    }

    /// XTREPORTSGR (`CSI Pt ; Pl ; Pb ; Pr # |`).
    pub(in crate::terminal) fn report_sgr(&mut self, csi: &Csi) {
        let Some(area) = self.corners(csi, 0).region(self.cols, self.rows) else { return };
        let mut common: Option<Style> = None;
        for y in area.top..=area.bottom {
            let Some(view) = self.grid.row_view(Line(y as i32)) else { continue };
            for cell in &view.cells()[area.left..=area.right] {
                let style = *view.style(cell.style_id());
                common = Some(match common {
                    None => Style { underline_color: style.underline_color, ..style },
                    Some(so_far) => Style {
                        attrs: so_far.attrs & style.attrs,
                        fg: if so_far.fg == style.fg {
                            so_far.fg
                        } else {
                            Color::Named(crate::types::NamedColor::Foreground)
                        },
                        bg: if so_far.bg == style.bg {
                            so_far.bg
                        } else {
                            Color::Named(crate::types::NamedColor::Background)
                        },
                        underline_color: if so_far.underline_color == style.underline_color {
                            so_far.underline_color
                        } else {
                            None
                        },
                    },
                });
            }
        }
        if let Some(style) = common {
            self.reply(format_args!("\x1b[{}", SgrParams(&style)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_push_saves_only_what_it_names_and_a_pop_puts_back_only_that() {
        let mut pen = Style::DEFAULT;
        pen.attrs = Flags::BOLD | Flags::ITALIC | Flags::UNDERCURL;
        let bold_and_underline = Entry::save(Mask(Mask::BOLD | Mask::UNDERLINE), &pen);
        assert_eq!(bold_and_underline.style.attrs, Flags::BOLD | Flags::UNDERCURL);
        // The pen changed since: italic is on still, bold is gone, a double underline came.
        pen.attrs = Flags::ITALIC | Flags::DOUBLE_UNDERLINE;
        bold_and_underline.restore(&mut pen);
        assert_eq!(pen.attrs, Flags::ITALIC | Flags::BOLD | Flags::UNDERCURL);
    }

    #[test]
    fn a_double_underline_comes_back_alone_and_one_that_was_off_takes_nothing_with_it() {
        // Saved on: it replaces the other kinds of underline.
        let mut pen = Style::DEFAULT;
        pen.attrs = Flags::DOUBLE_UNDERLINE;
        let on = Entry::save(Mask(Mask::DOUBLE_UNDERLINE), &pen);
        pen.attrs = Flags::UNDERCURL;
        on.restore(&mut pen);
        assert_eq!(pen.attrs, Flags::DOUBLE_UNDERLINE);
        // Saved off: the underline the pen has now is not the push's business.
        let mut pen = Style::DEFAULT;
        let off = Entry::save(Mask(Mask::DOUBLE_UNDERLINE), &pen);
        pen.attrs = Flags::UNDERCURL | Flags::BOLD;
        off.restore(&mut pen);
        assert_eq!(pen.attrs, Flags::UNDERCURL | Flags::BOLD);
    }
}
