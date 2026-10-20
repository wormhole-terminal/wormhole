//! The minimap (View → Show Minimap): the history and the screen drawn small along the terminal's
//! right edge, one block per run of text in its color, with the part on screen marked. Clicking
//! a line brings it to the middle of the screen; dragging the mark scrolls like a scrollbar.
//!
//! Lines are counted from the oldest line of the history (0) to the last line of the screen. When
//! they don't all fit, the minimap shows a window of them that moves with the view, as VS Code's
//! does: the top of the history while the view is there, the live screen while it's live.

use wormhole_vt::{Cell, Color, Colors, Flags, Line, NamedColor};

use super::screen::Screen;

/// Width of the minimap, in points.
pub const WIDTH: f32 = 64.0;
/// Height of one terminal line in the minimap, in points: the block and the gap under it.
pub const LINE_PITCH: f32 = 2.0;
/// Height of a block within [`LINE_PITCH`].
pub const BLOCK_H: f32 = 1.5;
/// Widest a column gets, so a narrow terminal's lines aren't stretched across the minimap.
pub const COLUMN_MAX: f32 = 1.0;

/// Columns `start..end` of a line drawn as one block, in the colors of `cell` (its first cell):
/// the text color for text, the background for colored blanks.
#[derive(Clone, Debug, PartialEq)]
pub struct MinimapRun {
    pub start: usize,
    pub end: usize,
    pub text: bool,
    pub cell: Cell,
}

/// How many lines there are and which of them the view shows; every position follows from it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MinimapGeometry {
    /// History and screen together.
    pub total: usize,
    pub screen: usize,
    /// Lines the minimap has room for.
    pub fits: usize,
    /// The first line on screen.
    pub view_top: usize,
}

impl MinimapGeometry {
    fn history(&self) -> usize {
        self.total.saturating_sub(self.screen)
    }

    /// The first line the minimap shows.
    pub fn first(&self) -> usize {
        let spare = self.total.saturating_sub(self.fits);
        let history = self.history();
        if spare == 0 || history == 0 {
            return 0;
        }
        (spare as f64 * self.view_top.min(history) as f64 / history as f64).round() as usize
    }

    /// The marked part, the lines on screen, as a top and a height in points.
    pub fn slider(&self) -> (f32, f32) {
        let top = self.view_top.saturating_sub(self.first()) as f32 * LINE_PITCH;
        (top, self.screen.min(self.total) as f32 * LINE_PITCH)
    }

    /// The line drawn at `y` points from the minimap's top, if there is one.
    pub fn line_at(&self, y: f32) -> Option<usize> {
        let line = self.first() + (y.max(0.0) / LINE_PITCH) as usize;
        (line < self.total).then_some(line)
    }

    /// The view's first line once the slider's top is dragged to `y`: the lines' own place while
    /// they all fit, else a scrollbar's, over the room the slider has to move in.
    pub fn view_top_for_slider(&self, y: f32) -> usize {
        let history = self.history();
        let track = self.fits.saturating_sub(self.screen) as f32 * LINE_PITCH;
        let top = if self.total <= self.fits || track <= 0.0 {
            (y / LINE_PITCH).round()
        } else {
            (y / track * history as f32).round()
        };
        (top.max(0.0) as usize).min(history)
    }

    /// The view's first line that puts `line` in the middle of the screen.
    pub fn view_top_centering(&self, line: usize) -> usize {
        line.saturating_sub(self.screen / 2).min(self.history())
    }

    /// How far into the history the view is scrolled with `view_top` as its first line.
    pub fn display_offset(&self, view_top: usize) -> usize {
        self.history() - view_top.min(self.history())
    }
}

/// The lines the minimap shows, copied as runs under the terminal's lock; kept between frames
/// and read again only when the output or the view changed.
#[derive(Default)]
pub struct MinimapSnapshot {
    pub geometry: MinimapGeometry,
    pub cols: usize,
    /// The runs of each line shown, from [`MinimapGeometry::first`] on.
    pub lines: Vec<Vec<MinimapRun>>,
    /// Colors programs set at runtime (OSC 4/10/11), which the runs' cells refer to.
    pub colors: Colors,
    /// What the lines were read for: the output's revision, the geometry and the columns.
    read_for: Option<(u64, MinimapGeometry, usize)>,
}

impl MinimapSnapshot {
    /// Bring the copy up to date with `term` for a minimap `fits` lines tall. `revision` changes
    /// whenever the program printed. Returns whether anything was read again.
    pub(super) fn update(&mut self, screen: &Screen, fits: usize, revision: u64) -> bool {
        let (history, lines, cols) = (screen.history_size(), screen.screen_lines(), screen.columns());
        let geometry = MinimapGeometry {
            total: history + lines,
            screen: lines,
            fits,
            view_top: history - screen.display_offset().min(history),
        };
        self.geometry = geometry;
        if self.read_for == Some((revision, geometry, cols)) {
            return false;
        }
        self.read_for = Some((revision, geometry, cols));
        self.cols = cols;
        self.colors = screen.colors();
        let first = geometry.first();
        let count = fits.min(geometry.total - first);
        self.lines.resize_with(count, Vec::new);
        let mut cells = Vec::new();
        for (k, runs) in self.lines.iter_mut().enumerate() {
            screen.copy_row(Line((first + k) as i32 - history as i32), &mut cells, None);
            line_runs(cells.iter(), runs);
        }
        true
    }
}

/// What a cell adds to the minimap: text, a colored blank, or nothing.
fn ink(cell: &Cell) -> Option<bool> {
    let blank = matches!(cell.c, ' ' | '\0' | '\t') && !cell.flags.contains(Flags::WIDE_CHAR_SPACER);
    if !blank && !cell.flags.contains(Flags::HIDDEN) {
        return Some(true);
    }
    let background = cell.flags.contains(Flags::INVERSE) || cell.bg != Color::Named(NamedColor::Background);
    background.then_some(false)
}

/// Whether two cells of the same kind are drawn in the same color.
fn same_look(a: &Cell, b: &Cell, text: bool) -> bool {
    let looks = Flags::INVERSE | Flags::DIM | Flags::HIDDEN;
    a.flags.intersection(looks) == b.flags.intersection(looks) && a.bg == b.bg && (!text || a.fg == b.fg)
}

/// A line's cells as runs of one color (`out` is reused); a blank breaks a run of text, so
/// words stay apart.
fn line_runs<'a>(cells: impl Iterator<Item = &'a Cell>, out: &mut Vec<MinimapRun>) {
    out.clear();
    for (col, cell) in cells.enumerate() {
        let Some(text) = ink(cell) else { continue };
        if let Some(last) = out.last_mut()
            && last.end == col
            && last.text == text
            && same_look(&last.cell, cell, text)
        {
            last.end = col + 1;
            continue;
        }
        out.push(MinimapRun { start: col, end: col + 1, text, cell: *cell });
    }
}

#[cfg(test)]
mod tests {
    use wormhole_vt::Scroll;

    use super::{LINE_PITCH, MinimapGeometry, MinimapSnapshot};
    use crate::testing::TestTerm;

    fn term(rows: usize) -> TestTerm {
        TestTerm::new(20, rows as u16, 1000)
    }

    fn feed(term: &mut TestTerm, bytes: &[u8]) {
        term.feed(bytes);
    }

    fn spans(snapshot: &MinimapSnapshot, line: usize) -> Vec<(usize, usize, bool)> {
        snapshot.lines[line].iter().map(|run| (run.start, run.end, run.text)).collect()
    }

    #[test]
    fn words_are_runs_and_colored_blanks_count() {
        let mut term = term(3);
        // "ls -la", then a red word, then three blanks on a blue background.
        feed(&mut term, b"ls -la \x1b[31mred\x1b[0m \x1b[44m   \x1b[0m");
        let mut snapshot = MinimapSnapshot::default();
        assert!(snapshot.update(term.screen(), 10, 1));
        assert_eq!(spans(&snapshot, 0), [(0, 2, true), (3, 6, true), (7, 10, true), (11, 14, false)]);
        assert!(snapshot.lines[1].is_empty() && snapshot.lines[2].is_empty());
    }

    #[test]
    fn a_frame_without_output_reads_nothing_again() {
        let mut term = term(3);
        feed(&mut term, b"hello");
        let mut snapshot = MinimapSnapshot::default();
        assert!(snapshot.update(term.screen(), 10, 1));
        assert!(!snapshot.update(term.screen(), 10, 1));
        feed(&mut term, b" world");
        assert!(snapshot.update(term.screen(), 10, 2), "the program printed");
        assert_eq!(spans(&snapshot, 0), [(0, 5, true), (6, 11, true)]);
        assert!(snapshot.update(term.screen(), 12, 2), "the minimap grew");
    }

    #[test]
    fn the_shown_lines_follow_the_view_through_a_long_history() {
        let mut term = term(4);
        for n in 0..40 {
            feed(&mut term, format!("line {n}\r\n").as_bytes());
        }
        let mut snapshot = MinimapSnapshot::default();
        // 41 lines (37 of history), room for 10: live, the minimap shows the last 10.
        snapshot.update(term.screen(), 10, 1);
        let geometry = snapshot.geometry;
        assert_eq!((geometry.total, geometry.view_top, geometry.first()), (41, 37, 31));
        assert_eq!(snapshot.lines.len(), 10);
        assert_eq!(spans(&snapshot, 0), [(0, 4, true), (5, 7, true)], "line 31");
        // Scrolled to the top: the first 10.
        term.scroll_display(Scroll::Top);
        snapshot.update(term.screen(), 10, 1);
        assert_eq!((snapshot.geometry.view_top, snapshot.geometry.first()), (0, 0));
        assert_eq!(spans(&snapshot, 0), [(0, 4, true), (5, 6, true)], "line 0");
    }

    #[test]
    fn a_click_centers_its_line_and_the_slider_scrolls_like_a_scrollbar() {
        // Everything fits: lines stay where they are, the slider sits on the screen's lines.
        let short = MinimapGeometry { total: 30, screen: 10, fits: 100, view_top: 20 };
        assert_eq!(short.first(), 0);
        assert_eq!(short.slider(), (20.0 * LINE_PITCH, 10.0 * LINE_PITCH));
        assert_eq!(short.line_at(5.0 * LINE_PITCH + 0.5), Some(5));
        assert_eq!(short.line_at(30.0 * LINE_PITCH), None, "below the last line");
        assert_eq!(short.view_top_centering(5), 0);
        assert_eq!(short.view_top_centering(12), 7);
        assert_eq!(short.view_top_centering(29), 20, "as far as the history goes");
        assert_eq!(short.view_top_for_slider(4.0 * LINE_PITCH), 4);
        assert_eq!(short.display_offset(4), 16);

        // A long history: the slider's room (100 − 10 lines) spans the whole history.
        let long = MinimapGeometry { total: 1010, screen: 10, fits: 100, view_top: 500 };
        assert_eq!(long.first(), 455);
        let (top, height) = long.slider();
        assert_eq!((top, height), (45.0 * LINE_PITCH, 10.0 * LINE_PITCH));
        assert_eq!(long.view_top_for_slider(top), 500, "the slider stays under the pointer");
        assert_eq!(long.view_top_for_slider(0.0), 0);
        assert_eq!(long.view_top_for_slider(90.0 * LINE_PITCH), 1000);
        assert_eq!(long.view_top_for_slider(500.0), 1000, "past the end");
        assert_eq!(long.line_at(0.0), Some(455));
    }
}
