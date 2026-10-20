//! In-band resize notifications: DECSET 2048.
//!
//! Spec: <https://gist.github.com/rockorager/e695fb2924d36b2bcf1fff4a3704bd83> ("Private Mode for
//! In-Band Window Resize Notifications"). A program that sets `CSI ? 2048 h` is told the size of
//! the text area as `CSI 48 ; rows ; cols ; height_px ; width_px t` at once (also when the mode was
//! set already), and again after every resize, once the screen has its new size. DECRQM answers the
//! mode like any other (`CSI ? 2048 $ p`).
//!
//! The pixels are the text area: the columns and rows times the cell the host gave
//! ([`Terminal::set_cell_size`](crate::Terminal::set_cell_size)), which is the one thing the
//! engine knows; a host that has padding around the cells reports it away by giving the cell size
//! of the grid alone. A report is made when the size or the cell changed since the last one (a host
//! that sets the cell size and then resizes gets one report, not two).

use crate::terminal::Terminal;
use crate::terminal::modes::Modes;

/// The size last reported in band: rows, columns and the cell in pixels. Nothing before a report
/// was made, and after the mode is reset.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(in crate::terminal) struct Reported(Option<[usize; 4]>);

impl Terminal {
    /// Tell the program the size now, the way DECSET 2048 does.
    pub(in crate::terminal) fn report_inband_size(&mut self) {
        let (cell_w, cell_h) = (usize::from(self.cell_px.0), usize::from(self.cell_px.1));
        let (rows, cols) = (self.rows, self.cols);
        self.inband = Reported(Some([rows, cols, cell_w, cell_h]));
        self.reply(format_args!("\x1b[48;{rows};{cols};{};{}t", rows * cell_h, cols * cell_w));
    }

    /// After a resize: tell the program when the mode is set and the size is not what it was last
    /// told.
    pub(in crate::terminal) fn report_inband_change(&mut self) {
        if !self.modes.contains(Modes::INBAND_RESIZE) {
            return;
        }
        let now = [self.rows, self.cols, usize::from(self.cell_px.0), usize::from(self.cell_px.1)];
        if self.inband.0 != Some(now) {
            self.report_inband_size();
        }
    }

    /// DECRST 2048.
    pub(in crate::terminal) fn forget_inband_size(&mut self) {
        self.inband = Reported::default();
    }
}
