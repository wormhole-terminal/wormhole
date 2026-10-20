//! The window: XTWINOPS (`CSI Ps ; Ps ; Ps t`), DECSLPP and DECSNLS. What a program may read of
//! the window, and what it may ask the window to do.
//!
//! Spec: xterm's ctlseqs, "Window manipulation" (reports 11, 13, 14, 15, 16, 18, 19, 20, 21;
//! operations 1 to 10 and the sizes of 24 lines and more, the VT510's DECSLPP) and the VT510
//! manual for DECSNLS.
//!
//! **Reports** of sizes, position and state are always answered: they say nothing a program
//! could not learn from the tty (the size) or that matters (the window's place on the screen).
//! The host tells the engine where its window is ([`Terminal::set_window`]); without it the text
//! area is taken to be the whole screen at the origin. **Operations** that change the window
//! (move, resize, iconify, raise, lower, maximise, full screen) and the reports of the window
//! title and icon name are for programs the user trusts with the window: they do nothing until the
//! host switches them on ([`Policy::window_ops`](crate::terminal::Policy::window_ops)), the same way DECRQCRA is off until
//! the host allows it. When on, the sizes in cells take effect in the engine at once (so that a
//! program that asks for the size right after it gets the new one, as on a real terminal) and
//! every operation is handed to the host as an [`Event::WindowOp`], which makes the window match
//! or sets the engine back to the size it has.

use crate::parser::Csi;

use crate::terminal::{Event, Terminal};

/// Where the host's window is and how big the things around it are, for the reports. Pixels are
/// the host's, with the origin of the screen at its top left.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct WindowGeometry {
    /// The window's top left corner on the screen.
    pub x: i32,
    /// See `x`.
    pub y: i32,
    /// The window as a whole (the text area, the sidebar and everything else).
    pub width: u32,
    /// See `width`.
    pub height: u32,
    /// The screen the window is on.
    pub screen_width: u32,
    /// See `screen_width`.
    pub screen_height: u32,
    /// The most columns the text area could have in this window if it were as large as can be
    /// (maximised, with everything around the text area as it is): XTWINOPS 19 reports it.
    pub max_cols: u16,
    /// See `max_cols`.
    pub max_rows: u16,
    /// The window is minimised to the Dock.
    pub iconified: bool,
}

/// How far a resize goes along one direction.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Extent {
    /// Left as it is (the parameter was left out).
    Keep,
    /// As large as the screen allows (the parameter was 0).
    Largest,
    /// This many pixels or cells.
    Size(u32),
}

impl Extent {
    fn of(param: Option<u16>) -> Extent {
        match param {
            None => Extent::Keep,
            Some(0) => Extent::Largest,
            Some(n) => Extent::Size(u32::from(n)),
        }
    }
}

/// Which way XTWINOPS 9 maximises.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Maximize {
    /// 0: back to the size before.
    Restore,
    /// 1: both ways.
    Both,
    /// 2: vertically.
    Vertical,
    /// 3: horizontally.
    Horizontal,
}

/// XTWINOPS 10.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fullscreen {
    /// 0.
    Exit,
    /// 1.
    Enter,
    /// 2.
    Toggle,
}

/// What a program asked the window to do (only when the host allowed it).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum WindowOp {
    /// 1.
    Deiconify,
    /// 2.
    Iconify,
    /// 3: the window's top left corner to `x`, `y` (left out: 0).
    Move {
        /// Pixels from the left of the screen.
        x: i32,
        /// Pixels from the top of the screen.
        y: i32,
    },
    /// 5.
    Raise,
    /// 6.
    Lower,
    /// 4 (in pixels, taken as whole cells), 8, DECSLPP and DECSNLS (rows only): the text area to
    /// this size in cells.
    ResizeCells {
        /// The rows.
        rows: Extent,
        /// The columns.
        cols: Extent,
    },
    /// 9.
    Maximize(Maximize),
    /// 10.
    Fullscreen(Fullscreen),
}

/// The most cells a size asked for with nothing known of the screen may be.
const UNKNOWN_SCREEN_MAX: usize = 500;

impl Terminal {
    /// A cell's size in pixels, as the host gave it.
    pub(in crate::terminal) fn cell_size(&self) -> (u32, u32) {
        (u32::from(self.cell_px.0), u32::from(self.cell_px.1))
    }

    /// Tell the engine where the window is, for the reports.
    pub fn set_window(&mut self, geometry: WindowGeometry) {
        self.window = Some(geometry);
    }

    /// XTWINOPS (`CSI Ps ; Ps ; Ps t`) and DECSLPP (`CSI Ps t`, Ps of 24 and more).
    pub(in crate::terminal) fn window_operation(&mut self, csi: &Csi) {
        let ps = csi.param(0, 0);
        let (p1, p2) = (csi.get(1), csi.get(2));
        let allowed = self.policy.window_ops;
        match ps {
            11 => {
                let iconified = self.window.is_some_and(|w| w.iconified);
                self.reply(format_args!("\x1b[{}t", if iconified { 2 } else { 1 }));
            }
            13 => {
                let (x, y) = self.window.map_or((0, 0), |w| (w.x, w.y));
                self.reply(format_args!("\x1b[3;{x};{y}t"));
            }
            14 => {
                let (cols, rows) = (self.cols as u32, self.rows as u32);
                let (cell_w, cell_h) = self.cell_size();
                let (width, height) = match (p1, self.window) {
                    // The window with everything around the text area.
                    (Some(2), Some(w)) => (w.width, w.height),
                    _ => (cols * cell_w, rows * cell_h),
                };
                self.reply(format_args!("\x1b[4;{height};{width}t"));
            }
            15 => {
                let (width, height) = match self.window {
                    Some(w) => (w.screen_width, w.screen_height),
                    None => {
                        let (cell_w, cell_h) = self.cell_size();
                        (self.cols as u32 * cell_w, self.rows as u32 * cell_h)
                    }
                };
                self.reply(format_args!("\x1b[5;{height};{width}t"));
            }
            16 => {
                let (cell_w, cell_h) = self.cell_px;
                self.reply(format_args!("\x1b[6;{cell_h};{cell_w}t"));
            }
            18 => {
                let (cols, rows) = (self.cols, self.rows);
                self.reply(format_args!("\x1b[8;{rows};{cols}t"));
            }
            19 => {
                let (cols, rows) = self.window.map_or((self.cols, self.rows), |w| {
                    (usize::from(w.max_cols).max(1), usize::from(w.max_rows).max(1))
                });
                self.reply(format_args!("\x1b[9;{rows};{cols}t"));
            }
            20 | 21 if allowed => self.report_title(ps == 20),
            22 => self.push_title(csi.param(1, 0)),
            23 => self.pop_title(csi.param(1, 0)),
            1 if allowed => self.window_event(WindowOp::Deiconify),
            2 if allowed => self.window_event(WindowOp::Iconify),
            3 if allowed => {
                let at = |p: Option<u16>| i32::from(p.unwrap_or(0));
                self.window_event(WindowOp::Move { x: at(p1), y: at(p2) });
            }
            4 if allowed => {
                let (height, width) = (Extent::of(p1), Extent::of(p2));
                let (cell_w, cell_h) = self.cell_size();
                let cells = |extent: Extent, cell: u32| match extent {
                    Extent::Size(pixels) => Extent::Size((pixels / cell).max(1)),
                    other => other,
                };
                self.resize_window(cells(height, cell_h), cells(width, cell_w));
            }
            5 if allowed => self.window_event(WindowOp::Raise),
            6 if allowed => self.window_event(WindowOp::Lower),
            8 if allowed => self.resize_window(Extent::of(p1), Extent::of(p2)),
            9 if allowed => {
                let how = match p1 {
                    Some(1) => Maximize::Both,
                    Some(2) => Maximize::Vertical,
                    Some(3) => Maximize::Horizontal,
                    Some(0) => Maximize::Restore,
                    _ => return,
                };
                self.window_event(WindowOp::Maximize(how));
            }
            10 if allowed => {
                let how = match p1 {
                    Some(0) => Fullscreen::Exit,
                    Some(1) => Fullscreen::Enter,
                    Some(2) => Fullscreen::Toggle,
                    _ => return,
                };
                self.window_event(WindowOp::Fullscreen(how));
            }
            // DECSLPP: a number of lines of 24 or more.
            24.. if allowed => self.resize_window(Extent::Size(u32::from(ps)), Extent::Keep),
            _ => {}
        }
    }

    /// DECSNLS (`CSI Ps * |`): the number of lines per screen. Like DECSLPP it resizes the window.
    pub(in crate::terminal) fn set_lines_per_screen(&mut self, lines: u16) {
        if self.policy.window_ops && lines > 0 {
            self.resize_window(Extent::Size(u32::from(lines)), Extent::Keep);
        }
    }

    /// A resize in cells: made here at once when both sides are known, and handed to the host.
    fn resize_window(&mut self, rows: Extent, cols: Extent) {
        let (max_cols, max_rows) = self.window.map_or((UNKNOWN_SCREEN_MAX, UNKNOWN_SCREEN_MAX), |w| {
            (usize::from(w.max_cols), usize::from(w.max_rows))
        });
        // Asking for more than the screen holds only gets what it holds.
        let side = |extent: Extent, now: usize, max: usize| match extent {
            Extent::Keep => now,
            Extent::Largest => max.max(now),
            Extent::Size(n) => (n as usize).min(max.max(now)),
        };
        let (new_cols, new_rows) = (side(cols, self.cols, max_cols), side(rows, self.rows, max_rows));
        if rows != Extent::Largest
            && cols != Extent::Largest
            && (new_cols, new_rows) != (self.cols, self.rows)
        {
            // The window follows the text area by the difference, which the host tells later: a
            // program that asks for the window's size right after (it has asked for the resize,
            // and the window's size is what it wants to know) gets what the window will have.
            let grow = |new: usize, old: usize, cell: u16| (new as i64 - old as i64) * i64::from(cell);
            let (dx, dy) =
                (grow(new_cols, self.cols, self.cell_px.0), grow(new_rows, self.rows, self.cell_px.1));
            if let Some(window) = self.window.as_mut() {
                window.width = (i64::from(window.width) + dx).clamp(0, i64::from(u32::MAX)) as u32;
                window.height = (i64::from(window.height) + dy).clamp(0, i64::from(u32::MAX)) as u32;
            }
            self.resize(new_cols, new_rows);
        }
        self.window_event(WindowOp::ResizeCells { rows, cols });
    }

    fn window_event(&mut self, op: WindowOp) {
        self.push_event(Event::WindowOp(op));
    }
}
