//! Rectangular editing: DECCARA, DECRARA, DECCRA, DECFRA, DECERA, DECSERA and DECSACE.
//!
//! Spec: VT510 reference (DECCARA, DECRARA, DECCRA, DECFRA, DECERA, DECSERA, DECSACE) and xterm's
//! ctlseqs for the final bytes. All of them take a rectangle `Pt ; Pl ; Pb ; Pr` (top, left,
//! bottom, right; 1-based):
//!
//! - a missing or zero `Pt` and `Pl` mean 1, a missing or zero `Pb` and `Pr` the last line and
//!   column of the page;
//! - a rectangle whose top is below its bottom or whose left is right of its right edge (given
//!   so, before clipping) does nothing; one that reaches past the page is cut to it;
//! - with origin mode (DECOM) set the numbers count from the scrolling region's corner, and the
//!   defaults for the last line and column are the region's; without it, from the screen's;
//! - the top and bottom margins and the left and right ones do not limit the area (VT510 DECCARA:
//!   "ignores page margins"), and the cursor does not move;
//! - there is one page, so a page number other than 0 and 1 in DECCRA makes it do nothing.
//!
//! DECERA blanks every cell in the area, DECSERA only the unprotected ones (DECSCA), both with
//! the pen's background like the other erases. DECFRA fills with the pen's rendition. DECCARA and
//! DECRARA change the attributes bold (1), underline (4), blink (5), negative image (7) and,
//! as an xterm extension, hidden (8), over the cells DECSACE says.

use crate::grid::{Region, Style};
use crate::parser::Csi;
use crate::types::Flags;

use crate::terminal::Terminal;
use crate::terminal::modes::Modes;

/// Which cells DECCARA and DECRARA reach (DECSACE).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::terminal) enum AttrExtent {
    /// 0 and 1: the stream of cells from the first corner to the second, in reading order, each
    /// line running from the first column to the last (VT510 DECSACE).
    #[default]
    Stream,
    /// 2: every cell of the rectangle.
    Rectangle,
}

/// The rectangle a sequence names, in screen cells and before it is cut to the screen.
#[derive(Clone, Copy, Debug)]
pub(in crate::terminal) struct Corners {
    top: usize,
    left: usize,
    bottom: usize,
    right: usize,
}

/// What DECCARA or DECRARA does to the attributes of one cell. Applied in the order the
/// parameters came: a later one overrides an earlier one on the same bit.
#[derive(Clone, Copy, Debug, Default)]
struct AttrEdit {
    set: Flags,
    clear: Flags,
    toggle: Flags,
}

/// The attributes DECCARA and DECRARA 0 reset or reverse (VT510: bold, underline, blink, negative).
const BASE_ATTRS: Flags = Flags::BOLD.union(Flags::ALL_UNDERLINES).union(Flags::BLINK).union(Flags::INVERSE);

impl AttrEdit {
    /// The SGR number `ps` as DECCARA reads it; anything but 0, 1, 4, 5, 7, 8, 22, 24, 25, 27, 28
    /// is ignored.
    fn change(&mut self, ps: u16) {
        let (on, off) = match ps {
            0 => (Flags::empty(), BASE_ATTRS),
            1 => (Flags::BOLD, Flags::empty()),
            4 => (Flags::UNDERLINE, Flags::ALL_UNDERLINES),
            5 => (Flags::BLINK, Flags::empty()),
            7 => (Flags::INVERSE, Flags::empty()),
            8 => (Flags::HIDDEN, Flags::empty()),
            22 => (Flags::empty(), Flags::BOLD),
            24 => (Flags::empty(), Flags::ALL_UNDERLINES),
            25 => (Flags::empty(), Flags::BLINK),
            27 => (Flags::empty(), Flags::INVERSE),
            28 => (Flags::empty(), Flags::HIDDEN),
            _ => return,
        };
        self.set = (self.set - off) | on;
        self.clear = (self.clear - on) | off;
    }

    /// The SGR number `ps` as DECRARA reads it: the attribute is reversed, and 0 reverses all four
    /// of the VT510's.
    fn reverse(&mut self, ps: u16) {
        self.toggle ^= match ps {
            0 => BASE_ATTRS & !Flags::ALL_UNDERLINES | Flags::UNDERLINE,
            1 => Flags::BOLD,
            4 => Flags::UNDERLINE,
            5 => Flags::BLINK,
            7 => Flags::INVERSE,
            8 => Flags::HIDDEN,
            _ => return,
        };
    }

    fn apply(self, style: &Style) -> Style {
        let mut attrs = (style.attrs - self.clear) | self.set;
        for bit in self.toggle {
            if bit == Flags::UNDERLINE {
                // The underline styles exclude each other: reversing is on for none, off for any.
                if attrs.intersects(Flags::ALL_UNDERLINES) {
                    attrs -= Flags::ALL_UNDERLINES;
                } else {
                    attrs |= Flags::UNDERLINE;
                }
            } else {
                attrs ^= bit;
            }
        }
        Style { attrs, ..*style }
    }
}

impl Corners {
    /// Both orders right: top not below bottom, left not right of right.
    pub(in crate::terminal) fn region(self, cols: usize, rows: usize) -> Option<Region> {
        (self.top <= self.bottom && self.left <= self.right).then(|| Region {
            top: self.top.min(rows - 1),
            bottom: self.bottom.min(rows - 1),
            left: self.left.min(cols - 1),
            right: self.right.min(cols - 1),
        })
    }
}

impl Terminal {
    /// The corners `Pt ; Pl ; Pb ; Pr` that start at parameter `first`.
    pub(in crate::terminal) fn corners(&self, csi: &Csi, first: usize) -> Corners {
        let (row0, col0, last_row, last_col) = if self.modes.contains(Modes::ORIGIN) {
            (self.margins.top, self.margins.left, self.margins.bottom, self.margins.right)
        } else {
            (0, 0, self.rows - 1, self.cols - 1)
        };
        let from = |i: usize, base: usize, default: usize| match csi.param(i, 0) {
            0 => default,
            n => base + usize::from(n) - 1,
        };
        Corners {
            top: from(first, row0, row0),
            left: from(first + 1, col0, col0),
            bottom: from(first + 2, row0, last_row),
            right: from(first + 3, col0, last_col),
        }
    }

    /// The sequences with `$` as intermediate that edit rectangles.
    pub(in crate::terminal) fn csi_rectangle(&mut self, csi: &Csi) {
        match csi.final_byte() {
            b'r' => self.change_attributes(csi, false),
            b't' => self.change_attributes(csi, true),
            b'v' => self.copy_rectangle(csi),
            b'x' => self.fill_rectangle(csi),
            b'z' => self.erase_rectangle(csi, false),
            b'{' => self.erase_rectangle(csi, true),
            _ => {}
        }
    }

    /// DECSACE (`CSI Ps * x`).
    pub(in crate::terminal) fn select_attribute_extent(&mut self, ps: u16) {
        match ps {
            0 | 1 => self.attr_extent = AttrExtent::Stream,
            2 => self.attr_extent = AttrExtent::Rectangle,
            _ => return,
        }
        self.kept.attribute_extent = ps;
    }

    /// DECCARA (`CSI Pt ; Pl ; Pb ; Pr ; Ps… $ r`) and, with `reverse`, DECRARA (`… $ t`).
    fn change_attributes(&mut self, csi: &Csi, reverse: bool) {
        let mut edit = AttrEdit::default();
        // No attribute at all is attribute 0.
        let attrs = (4..csi.len().max(5)).map(|i| csi.param(i, 0));
        for ps in attrs {
            if reverse {
                edit.reverse(ps);
            } else {
                edit.change(ps);
            }
        }
        let corners = self.corners(csi, 0);
        let change = |style: &Style| edit.apply(style);
        match self.attr_extent {
            AttrExtent::Rectangle => {
                if let Some(area) = corners.region(self.cols, self.rows) {
                    for y in area.top..=area.bottom {
                        self.grid.restyle(y, area.left..area.right + 1, change);
                    }
                }
            }
            AttrExtent::Stream => {
                // The stream needs the corners in reading order, not left of right.
                if corners.top > corners.bottom || corners.top >= self.rows {
                    return;
                }
                let last = corners.bottom.min(self.rows - 1);
                for y in corners.top..=last {
                    let start = if y == corners.top { corners.left } else { 0 };
                    let end = if y == corners.bottom { corners.right.saturating_add(1) } else { self.cols };
                    self.grid.restyle(y, start.min(self.cols)..end.min(self.cols), change);
                }
            }
        }
    }

    /// DECCRA (`CSI Pts ; Pls ; Pbs ; Prs ; Pps ; Ptd ; Pld ; Ppd $ v`).
    fn copy_rectangle(&mut self, csi: &Csi) {
        if csi.param(4, 0) > 1 || csi.param(7, 0) > 1 {
            return;
        }
        let Some(from) = self.corners(csi, 0).region(self.cols, self.rows) else { return };
        let (row0, col0) =
            if self.modes.contains(Modes::ORIGIN) { (self.margins.top, self.margins.left) } else { (0, 0) };
        let to_y = row0 + usize::from(csi.param_nonzero(5, 1)) - 1;
        let to_x = col0 + usize::from(csi.param_nonzero(6, 1)) - 1;
        self.grid.copy_rect(from, to_y, to_x);
    }

    /// DECFRA (`CSI Pc ; Pt ; Pl ; Pb ; Pr $ x`): `Pc` is the character's code, of the printable
    /// ones (32 to 126 and 160 to 255 as the VT510 has it, and any other character one cell
    /// wide); anything else does nothing.
    fn fill_rectangle(&mut self, csi: &Csi) {
        let Some(c) = char::from_u32(u32::from(csi.param(0, 0))) else { return };
        let printable = matches!(c, ' '..='~' | '\u{a0}'..='\u{ff}')
            || (!c.is_control() && c > '\u{ff}' && crate::unicode::width(c) == 1);
        if !printable {
            return;
        }
        if let Some(area) = self.corners(csi, 1).region(self.cols, self.rows) {
            self.grid.fill_rect(area, c, &self.pen);
        }
    }

    /// DECERA (`CSI Pt ; Pl ; Pb ; Pr $ z`) and, `selective`, DECSERA (`… $ {`).
    fn erase_rectangle(&mut self, csi: &Csi, selective: bool) {
        if let Some(area) = self.corners(csi, 0).region(self.cols, self.rows) {
            let bg = self.erase_color();
            self.grid.erase_rect(area, bg, selective);
        }
    }
}
