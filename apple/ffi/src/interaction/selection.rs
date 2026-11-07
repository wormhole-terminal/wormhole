//! Selecting text with the pointer; the engine holds the selection, the view only draws it.

use wormhole_term::types::{Column, Line, Point, SelectionType, Side};

use crate::terminal::Terminal;

/// What a drag extends by: a click is cells, a double click words, a triple click lines; Option held
/// makes a rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum SelectionKind {
    Cell,
    Word,
    Line,
    Block,
}

/// Which half of a cell the pointer is over: a selection includes a cell from the half the drag started in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Enum)]
pub enum CellSide {
    Left,
    Right,
}

/// The selected columns `first...=last` of a viewport row.
#[derive(Debug, Clone, Copy, PartialEq, Eq, uniffi::Record)]
pub struct RowSpan {
    pub row: u32,
    pub first: u32,
    pub last: u32,
}

fn side_of(side: CellSide) -> Side {
    match side {
        CellSide::Left => Side::Left,
        CellSide::Right => Side::Right,
    }
}

#[uniffi::export]
impl Terminal {
    /// Start a selection at a viewport cell.
    pub fn selection_start(&self, kind: SelectionKind, row: u32, col: u32, side: CellSide) {
        let kind = match kind {
            SelectionKind::Cell => SelectionType::Simple,
            SelectionKind::Word => SelectionType::Semantic,
            SelectionKind::Line => SelectionType::Lines,
            SelectionKind::Block => SelectionType::Block,
        };
        let point = self.grid_point(row, col);
        self.with_pty(|pty| pty.start_selection(kind, point, side_of(side)));
    }

    /// Extend the selection being dragged to a viewport cell.
    pub fn selection_update(&self, row: u32, col: u32, side: CellSide) {
        let point = self.grid_point(row, col);
        self.with_pty(|pty| pty.update_selection(point, side_of(side)));
    }

    /// The button went up: a click without a drag selects nothing.
    pub fn selection_finish(&self) {
        self.with_pty(|pty| pty.finish_selection());
    }

    /// Select the history and the screen.
    pub fn select_all(&self) {
        self.with_pty(|pty| pty.select_all());
    }

    /// The selected text, if anything is selected (images' placeholders are not text).
    pub fn selection_text(&self) -> Option<String> {
        self.with_pty(|pty| pty.selection_text()).flatten()
    }

    /// The selection on the rows the last frame showed, one span per row it touches (empty: none).
    /// Read the frame first: the spans are as of it.
    pub fn selection_spans(&self) -> Vec<RowSpan> {
        self.with_snapshot(|snapshot| {
            (0..snapshot.rows().len())
                .filter_map(|row| {
                    let (first, last) = snapshot.selected_columns(row)?;
                    Some(RowSpan { row: row as u32, first: first as u32, last: last as u32 })
                })
                .collect()
        })
    }
}

impl Terminal {
    /// The grid line a viewport cell is, with the engine's display offset now: a drag that scrolled
    /// the history follows it before the next frame is taken.
    pub(super) fn grid_point(&self, row: u32, col: u32) -> Point {
        let offset = self.with_pty(|pty| pty.display_offset()).unwrap_or(0) as i32;
        Point::new(Line(row as i32 - offset), Column(col as usize))
    }
}
