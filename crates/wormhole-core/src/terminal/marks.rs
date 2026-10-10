//! Where command blocks are, read back from the grid.
//!
//! `pty_filter::osc_tap` wraps each part of a block a shell with integration announces (OSC 133:
//! the prompt `A` … `B`, the command line `B` … `C`, the output `C` … `D`) in a hyperlink with one
//! of the [`uri`]s and an id per prompt, which alacritty stores on the cells. The cells carry it
//! wherever the text goes (scrolling, the history filling up and dropping lines, `clear`, a
//! resize's reflow) and lose it when the row is overwritten, so what this module finds is right
//! wherever the text still is, with no bookkeeping that could fall out of step. A prompt of
//! several lines (a secondary or right prompt) shares one id; its first line is where the block
//! starts. Blank rows carry no cells, so a row without a mark belongs to the block above it
//! (`blocks` decides that; here a row says only what it carries itself).
//!
//! A scan visits every cell of the lines it passes (most have no `extra`, a cheap check, and the
//! marked ones share one link, a pointer compare), so a scan of the whole grid is for a key press,
//! not for every frame; the snapshot marks only the rows it copies.

use alacritty_terminal::event::EventListener;
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::Line;
use alacritty_terminal::term::cell::{Cell, Hyperlink};
use alacritty_terminal::term::{Term, TermMode};

/// The URIs of the tap's marks. Not links anyone can open: a renderer that draws hyperlinks
/// must skip them ([`is_mark`]).
pub(super) const PROMPT_URI: &str = "wormhole:prompt";
pub(super) const COMMAND_URI: &str = "wormhole:command";
pub(super) const OUTPUT_URI: &str = "wormhole:output";
const ID_PREFIX: &str = "wormhole-prompt-";

/// The hyperlink id of the `seq`-th prompt of a terminal (its block's id).
pub(super) fn prompt_id(seq: u64) -> String {
    format!("{ID_PREFIX}{seq}")
}

/// Whether a hyperlink's URI is one of the tap's marks rather than a program's link.
pub fn is_mark(uri: &str) -> bool {
    uri.starts_with("wormhole:")
}

/// The three parts of a command block, as the shell's marks divide it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// The shell's prompt, a secondary or right prompt included.
    Prompt,
    /// The command line as the shell echoed it while it was typed.
    Command,
    /// What the command printed.
    Output,
}

pub(super) fn uri(part: Part) -> &'static str {
    match part {
        Part::Prompt => PROMPT_URI,
        Part::Command => COMMAND_URI,
        Part::Output => OUTPUT_URI,
    }
}

/// Which way to look for a prompt from a line.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Direction {
    /// The nearest prompt start above (towards the history).
    Up,
    /// The nearest prompt start below.
    Down,
}

/// What a row carries of a block: the block's id and which of its parts are on the row. A prompt
/// row carries the prompt and, once something is typed, the command; a long command's wrapped
/// row the command alone; the rest the output. Where two blocks meet on one row (a program that
/// ended without a newline, the prompt right after it), the row is the later block's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowMark {
    pub id: u64,
    pub prompt: bool,
    pub command: bool,
    pub output: bool,
}

impl RowMark {
    /// The row shows the prompt or the command line, so the output starts after it.
    pub fn is_command_line(&self) -> bool {
        self.prompt || self.command
    }
}

/// The block mark on one cell, if its hyperlink is one.
pub(super) fn cell_mark(cell: &Cell) -> Option<(u64, Part)> {
    link_mark(&cell.hyperlink()?)
}

fn link_mark(link: &Hyperlink) -> Option<(u64, Part)> {
    let part = match link.uri() {
        PROMPT_URI => Part::Prompt,
        COMMAND_URI => Part::Command,
        OUTPUT_URI => Part::Output,
        _ => return None,
    };
    let id = link.id().strip_prefix(ID_PREFIX)?.parse().ok()?;
    Some((id, part))
}

/// What the cells of one row carry (see [`RowMark`]).
pub fn row_mark<'a>(cells: impl IntoIterator<Item = &'a Cell>) -> Option<RowMark> {
    let mut found: Option<RowMark> = None;
    // Cells printed under one mark share one link: a pointer compare tells them apart.
    let mut last: Option<(Hyperlink, Option<(u64, Part)>)> = None;
    for cell in cells {
        if cell.extra.is_none() {
            continue;
        }
        let Some(link) = cell.hyperlink() else { continue };
        let mark = match &last {
            Some((seen, mark)) if *seen == link => *mark,
            _ => {
                let mark = link_mark(&link);
                last = Some((link, mark));
                mark
            }
        };
        let Some((id, part)) = mark else { continue };
        let row = match &mut found {
            Some(row) if row.id == id => row,
            Some(row) if row.id > id => continue,
            _ => found.insert(RowMark { id, prompt: false, command: false, output: false }),
        };
        match part {
            Part::Prompt => row.prompt = true,
            Part::Command => row.command = true,
            Part::Output => row.output = true,
        }
    }
    found
}

/// The mark on grid `line`, if its cells carry one.
pub(super) fn mark_at<T: EventListener>(term: &Term<T>, line: Line) -> Option<RowMark> {
    if line < term.topmost_line() || line > term.bottommost_line() {
        return None;
    }
    row_mark(&term.grid()[line])
}

/// Whether `line` is the first line of a prompt: it shows a prompt whose block the line above
/// has nothing of (a secondary prompt's line above has the block's first line or its wrapped
/// command).
fn starts_prompt<T: EventListener>(term: &Term<T>, line: Line) -> bool {
    match mark_at(term, line) {
        Some(mark) if mark.prompt => mark_at(term, line - 1).is_none_or(|above| above.id != mark.id),
        _ => false,
    }
}

/// Every line a prompt starts on, oldest first; none while a full-screen program has the screen
/// (the alternate screen has no history and no prompts).
pub(super) fn prompt_lines<T: EventListener>(term: &Term<T>) -> Vec<Line> {
    if term.mode().contains(TermMode::ALT_SCREEN) {
        return Vec::new();
    }
    (term.topmost_line().0..=term.bottommost_line().0)
        .map(Line)
        .filter(|&line| starts_prompt(term, line))
        .collect()
}

/// The nearest line a prompt starts on, strictly above or below `from`.
pub(super) fn prompt_line<T: EventListener>(
    term: &Term<T>,
    from: Line,
    direction: Direction,
) -> Option<Line> {
    if term.mode().contains(TermMode::ALT_SCREEN) {
        return None;
    }
    let (top, bottom) = (term.topmost_line(), term.bottommost_line());
    match direction {
        Direction::Up => {
            let mut line = Line(from.0 - 1).min(bottom);
            while line >= top {
                if let Some(mark) = mark_at(term, line).filter(|mark| mark.prompt) {
                    // Up to the block's first line.
                    while mark_at(term, line - 1).is_some_and(|above| above.id == mark.id) {
                        line -= 1;
                    }
                    return Some(line);
                }
                line -= 1;
            }
            None
        }
        Direction::Down => {
            ((from.0 + 1).max(top.0)..=bottom.0).map(Line).find(|&line| starts_prompt(term, line))
        }
    }
}

#[cfg(test)]
mod tests {
    use alacritty_terminal::term::cell::{Cell, Hyperlink};

    use super::{Part, RowMark, is_mark, prompt_id, row_mark, uri};

    fn marked(text: &str, id: u64, part: Part) -> Vec<Cell> {
        let link = Hyperlink::new(Some(prompt_id(id)), uri(part).to_string());
        text.chars()
            .map(|c| {
                let mut cell = Cell { c, ..Cell::default() };
                cell.set_hyperlink(Some(link.clone()));
                cell
            })
            .collect()
    }

    #[test]
    fn a_row_says_which_block_and_which_parts_it_carries() {
        let mut row = marked("$ ", 3, Part::Prompt);
        row.extend(marked("ls", 3, Part::Command));
        row.push(Cell::default());
        assert_eq!(row_mark(&row), Some(RowMark { id: 3, prompt: true, command: true, output: false }));
        assert_eq!(row_mark(&marked("out", 3, Part::Output)).map(|m| m.output), Some(true));
        assert_eq!(row_mark(&vec![Cell::default(); 4]), None, "a blank row carries nothing");
        // A program's own link isn't a mark.
        let link = Hyperlink::new(Some("x".to_string()), "https://example.com".to_string());
        let mut cell = Cell::default();
        cell.set_hyperlink(Some(link));
        assert_eq!(row_mark(&[cell]), None);
        assert!(is_mark("wormhole:output") && !is_mark("https://example.com"));
    }

    #[test]
    fn where_two_blocks_meet_the_row_is_the_later_ones() {
        // `printf foo` ended without a newline; the next prompt starts on its row.
        let mut row = marked("foo", 1, Part::Output);
        row.extend(marked("$ ", 2, Part::Prompt));
        assert_eq!(row_mark(&row), Some(RowMark { id: 2, prompt: true, command: false, output: false }));
        let mut reversed = marked("$ ", 2, Part::Prompt);
        reversed.extend(marked("foo", 1, Part::Output));
        assert_eq!(row_mark(&reversed).map(|m| m.id), Some(2), "whatever the order of the cells");
    }
}
