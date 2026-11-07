//! Folds: finished rows of the history that the view shows as one marker row ("124 lines hidden").
//! Display only. The rows stay in the grid, the scrollback, the host and `wh read`; a selection and
//! a copy are in grid lines, so they give the real text; only the snapshot's list of rows changes
//! (see `layout`).
//!
//! A fold is two pins (`wormhole_vt::Pin`) on its first and last row, which follow the text through
//! scrolling and reflow and die with the history. It needs nothing for "dissolves when a row is
//! overwritten": only rows of the history can be folded (a fold never reaches the live screen), and a
//! program cannot write to the history; what could bring a folded row back under the program's hands,
//! a resize that pulls history onto the screen, dissolves the fold in the same lock hold
//! ([`Screen::fold_validate`]). A program's bytes never ask for a fold: they are made by the host's
//! own call ([`Screen::fold_add`]), and only for rows that are finished.
//!
//! Never folded: the live screen, the live block, anything on the alternate screen, anything while a
//! program has mouse reporting (its coordinates are screen rows), and, while the terminal needs the
//! person (an input the host passes in, [`Screen::fold_set_needs_you`]), the history of the last two
//! screens, which shows expanded, and no new fold at all.

mod layout;

use wormhole_vt::input::MouseTracking;
use wormhole_vt::{Column, Line, Modes, Pin, Point, Scroll};

pub(crate) use layout::{Shown, Walk};

use super::screen::Screen;

/// A fold hides at least this many rows: a marker for fewer saves nothing.
pub const MIN_HIDDEN: usize = 3;
/// At most this many folds per terminal.
pub const MAX_FOLDS: usize = 256;
/// A fold hides at most this many rows.
pub const MAX_HIDDEN: usize = 100_000;

/// A fold as the snapshot and the view see it, in grid lines.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FoldSpan {
    pub id: u64,
    pub first: Line,
    pub last: Line,
    /// Shown open: the person opened it, or the terminal needs them and this is recent.
    pub expanded: bool,
    /// The block it hides ended with a non-zero exit code: its marker says so.
    pub failed: bool,
}

impl FoldSpan {
    /// How many grid rows it hides.
    pub fn hidden(&self) -> usize {
        (self.last.0 - self.first.0 + 1).max(0) as usize
    }
}

/// Why a fold was not made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Refused {
    /// A full-screen program has the screen, or one reports the mouse.
    NotNow,
    /// The terminal needs the person: no new fold.
    NeedsYou,
    /// What is left of the rows after the live screen and the rows near the cursor are taken off is
    /// shorter than [`MIN_HIDDEN`] or longer than [`MAX_HIDDEN`].
    Size,
    /// Part of the rows is folded already.
    Overlaps,
    TooMany,
    /// The block has not ended, or has no output on the grid.
    Unfinished,
}

/// What asks for a fold, besides which rows.
#[derive(Clone, Copy, Debug)]
pub struct Request {
    /// The rows belong to a block whose end mark was seen. Without one they must lie more than a
    /// screen above the cursor.
    pub finished: bool,
    pub failed: bool,
}

struct Fold {
    id: u64,
    first: Pin,
    last: Pin,
    expanded: bool,
    failed: bool,
}

/// A terminal's folds, in the order they were made.
#[derive(Default)]
pub(crate) struct FoldSet {
    folds: Vec<Fold>,
    next_id: u64,
    needs_you: bool,
}

impl FoldSet {
    pub(crate) fn is_empty(&self) -> bool {
        self.folds.is_empty()
    }
}

impl Screen {
    /// Whether any fold exists: the frame with none does no work for them.
    pub(crate) fn has_folds(&self) -> bool {
        !self.folds.is_empty()
    }

    /// Whether folds apply to what shows now.
    fn folding_now(&self) -> bool {
        let mode = self.mode();
        !mode.contains(Modes::ALT_SCREEN) && mode.mouse_tracking() == MouseTracking::Off
    }

    /// Where fold `fold` is, or `None` when it must dissolve: a pin whose row left the history, rows that
    /// are no longer all in the history, or no longer in order.
    fn fold_span(&self, fold: &Fold) -> Option<FoldSpan> {
        let (first, last) = (self.pin_point(fold.first)?.line, self.pin_point(fold.last)?.line);
        let live = first <= last && last < Line(0);
        live.then_some(FoldSpan { id: fold.id, first, last, expanded: fold.expanded, failed: fold.failed })
    }

    /// The last line a fold may reach: the history, and for rows without an end mark only what lies
    /// more than a screen above the cursor.
    fn fold_limit(&self, finished: bool) -> Line {
        if finished {
            return Line(-1);
        }
        Line(-1).min(self.grid_cursor().line - self.screen_lines() as i32 - 1)
    }

    /// Fold lines `first..=last` of the grid. The rows are cut to what may be folded
    /// ([`fold_limit`](Self::fold_limit)); the result is the fold's id.
    pub(crate) fn fold_add(&mut self, first: Line, last: Line, request: Request) -> Result<u64, Refused> {
        if !self.folding_now() {
            return Err(Refused::NotNow);
        }
        if self.folds.needs_you {
            return Err(Refused::NeedsYou);
        }
        let (first, last) = (first.max(self.topmost_line()), last.min(self.fold_limit(request.finished)));
        let hidden = (last.0 - first.0 + 1).max(0) as usize;
        if !(MIN_HIDDEN..=MAX_HIDDEN).contains(&hidden) {
            return Err(Refused::Size);
        }
        if self.folds.folds.len() >= MAX_FOLDS {
            return Err(Refused::TooMany);
        }
        let overlaps = self
            .folds
            .folds
            .iter()
            .filter_map(|fold| self.fold_span(fold))
            .any(|span| span.first <= last && first <= span.last);
        if overlaps {
            return Err(Refused::Overlaps);
        }
        // The last pin on the row's last column: a reflow that splits the row puts it on the last piece.
        let end = Point::new(last, Column(self.last_column().0));
        let (first_pin, last_pin) = (self.pin(Point::new(first, Column(0))), self.pin(end));
        self.folds.next_id += 1;
        let id = self.folds.next_id;
        self.folds.folds.push(Fold {
            id,
            first: first_pin,
            last: last_pin,
            expanded: false,
            failed: request.failed,
        });
        Ok(id)
    }

    /// Drop the folds that cannot stay (see [`fold_span`](Self::fold_span)).
    pub(crate) fn fold_validate(&mut self) {
        if self.folds.is_empty() || self.on_alt_screen() {
            return;
        }
        let dead: Vec<usize> = (0..self.folds.folds.len())
            .filter(|&at| self.fold_span(&self.folds.folds[at]).is_none())
            .collect();
        for at in dead.into_iter().rev() {
            let fold = self.folds.folds.remove(at);
            self.release_pin(fold.first);
            self.release_pin(fold.last);
        }
    }

    /// The folds in view into `out`, by first line, with the rules above applied: nothing on the
    /// alternate screen or under mouse reporting (the folds wait, their pins are not read), recent
    /// ones open while the terminal needs the person.
    pub(crate) fn fold_spans(&mut self, out: &mut Vec<FoldSpan>) {
        out.clear();
        if self.folds.is_empty() || !self.folding_now() {
            return;
        }
        self.fold_validate();
        let recent = Line(-(self.screen_lines() as i32));
        out.extend(self.folds.folds.iter().filter_map(|fold| self.fold_span(fold)).map(|mut span| {
            span.expanded |= self.folds.needs_you && span.last >= recent;
            span
        }));
        out.sort_unstable_by_key(|span| span.first);
    }

    /// Open or close fold `id`; whether it exists.
    pub(crate) fn fold_toggle(&mut self, id: u64) -> bool {
        let Some(fold) = self.folds.folds.iter_mut().find(|fold| fold.id == id) else { return false };
        fold.expanded = !fold.expanded;
        true
    }

    /// Open the fold that hides `line` (find stepping to a match in it); whether one was closed.
    pub(crate) fn fold_expand_at(&mut self, line: Line) -> bool {
        if !self.folding_now() {
            return false;
        }
        let found = self.folds.folds.iter().position(|fold| {
            self.fold_span(fold).is_some_and(|span| !fold.expanded && span.first <= line && line <= span.last)
        });
        let Some(at) = found else { return false };
        self.folds.folds[at].expanded = true;
        true
    }

    /// Tell the folds whether the terminal needs the person (an agent asks for an answer, a bell).
    /// An input, not a dependency: the structural rules above hold whatever it says.
    pub(crate) fn fold_set_needs_you(&mut self, needs_you: bool) {
        self.folds.needs_you = needs_you;
    }

    /// Scroll the view by `rows` shown rows (up when positive, as `Scroll::Delta`): a fold is one step.
    pub(crate) fn scroll_shown_rows(&mut self, rows: i32) {
        let target = self.walk(|walk, bottom| walk.step(bottom, rows.unsigned_abs() as usize, rows > 0));
        self.scroll_to_bottom_line(target);
    }

    /// Scroll so that grid `line` is in view, in the middle when it was not (find, a block's output).
    pub(crate) fn reveal_shown(&mut self, line: Line) {
        let rows = self.screen_lines();
        let target = self.walk(|walk, bottom| {
            let mut view = Vec::new();
            walk.view(bottom, rows, &mut view);
            let wanted = walk.item_at(line);
            (!view.contains(&wanted)).then(|| walk.step(line, rows / 2, false))
        });
        if let Some(target) = target {
            self.scroll_to_bottom_line(target);
        }
    }

    /// Scroll so that grid `line` is the view's top row, as far as the history allows.
    pub(crate) fn scroll_line_to_top_shown(&mut self, line: Line) {
        let rows = self.screen_lines();
        let target = self.walk(|walk, _| walk.step(line, rows - 1, false));
        self.scroll_to_bottom_line(target);
    }

    /// `f` with a walk over the folds and the line the view is anchored at.
    fn walk<R>(&mut self, f: impl FnOnce(&Walk, Line) -> R) -> R {
        let mut spans = Vec::new();
        self.fold_spans(&mut spans);
        let bottom = Line(self.screen_lines() as i32 - 1 - self.display_offset() as i32);
        f(&Walk::new(&spans, self.topmost_line(), self.bottommost_line()), bottom)
    }

    /// Scroll the view so that its bottom row shows `bottom`, within the history.
    fn scroll_to_bottom_line(&mut self, bottom: Line) {
        let offset = (self.screen_lines() as i32 - 1 - bottom.0).clamp(0, self.history_size() as i32);
        self.scroll_display(Scroll::Delta(offset - self.display_offset() as i32));
    }
}

#[cfg(test)]
mod tests;
