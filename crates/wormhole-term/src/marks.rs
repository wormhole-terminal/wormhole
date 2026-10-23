//! Where command blocks are, read back from the grid.
//!
//! The engine numbers the blocks a shell with integration announces (OSC 133: the prompt `A` …
//! `B`, the command line `B` … `C`, the output `C` … `D`) and keeps the block and the part on the
//! rows' cells. The cells carry it wherever the text goes (scrolling, the history filling up and
//! dropping lines, `clear`, a resize's reflow) and lose it when the row is overwritten, so what
//! this module finds is right wherever the text still is, with no bookkeeping that could fall
//! out of step. A prompt of several lines (a secondary or right prompt) shares one id; its first
//! line is where the block starts. Blank rows carry no cells, so a row without a mark belongs to
//! the block above it (`blocks` decides that; here a row says only what it carries itself).
//!
//! Reading a row's mark is the engine's (`Screen::row_mark`; it visits the row's cells, so a scan
//! of the whole grid is for a key press, not for every frame; the snapshot marks only the rows it
//! copies). This module says what a mark is and what the rows' marks mean together.

use wormhole_vt::{Line, Modes};

use super::row_mark::RowMark;
use super::screen::Screen;

/// Which way to look for a prompt from a line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// The nearest prompt start above (towards the history).
    Up,
    /// The nearest prompt start below.
    Down,
}

/// The mark on grid `line`, if its cells carry one.
pub(super) fn mark_at(screen: &Screen, line: Line) -> Option<RowMark> {
    if line < screen.topmost_line() || line > screen.bottommost_line() {
        return None;
    }
    screen.row_mark(line)
}

/// Whether `line` is the first line of a prompt: it shows a prompt whose block the line above
/// has nothing of (a secondary prompt's line above has the block's first line or its wrapped
/// command).
fn starts_prompt(screen: &Screen, line: Line) -> bool {
    match mark_at(screen, line) {
        Some(mark) if mark.prompt => mark_at(screen, line - 1).is_none_or(|above| above.id != mark.id),
        _ => false,
    }
}

/// Every line a prompt starts on, oldest first; none while a full-screen program has the screen
/// (the alternate screen has no history and no prompts).
pub(super) fn prompt_lines(screen: &Screen) -> Vec<Line> {
    if screen.mode().contains(Modes::ALT_SCREEN) {
        return Vec::new();
    }
    (screen.topmost_line().0..=screen.bottommost_line().0)
        .map(Line)
        .filter(|&line| starts_prompt(screen, line))
        .collect()
}

/// The nearest line a prompt starts on, strictly above or below `from`.
pub(super) fn prompt_line(screen: &Screen, from: Line, direction: Direction) -> Option<Line> {
    if screen.mode().contains(Modes::ALT_SCREEN) {
        return None;
    }
    let (top, bottom) = (screen.topmost_line(), screen.bottommost_line());
    match direction {
        Direction::Up => {
            let mut line = Line(from.0 - 1).min(bottom);
            while line >= top {
                if let Some(mark) = mark_at(screen, line).filter(|mark| mark.prompt) {
                    // Up to the block's first line.
                    while mark_at(screen, line - 1).is_some_and(|above| above.id == mark.id) {
                        line -= 1;
                    }
                    return Some(line);
                }
                line -= 1;
            }
            None
        }
        Direction::Down => {
            ((from.0 + 1).max(top.0)..=bottom.0).map(Line).find(|&line| starts_prompt(screen, line))
        }
    }
}
