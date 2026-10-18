//! The invariants of a terminal, checked: the grids' (both screens) and the state around them.

use super::Terminal;
use super::graphics::{IMAGE_BUDGET_BYTES, MAX_IMAGES};

impl Terminal {
    /// Check everything a terminal promises, for tests and fuzzing; the first violation, described,
    /// as the error. It visits every cell of both screens, so nothing calls it outside a test.
    ///
    /// - Both grids are valid ([`Grid::check_invariants`](crate::grid::Grid::check_invariants)) and
    ///   have the terminal's size.
    /// - The cursor, the saved cursors and the margins are inside the screen, the margins in order.
    /// - A wrap is only pending on the cursor's last column.
    /// - There is one tab stop slot per column.
    /// - The pictures of a screen are within their count and byte budgets.
    pub fn check_invariants(&self) -> Result<(), String> {
        self.grid.check_invariants().map_err(|e| format!("screen: {e}"))?;
        self.other.check_invariants().map_err(|e| format!("other screen: {e}"))?;
        for (name, grid) in [("screen", &self.grid), ("other screen", &self.other)] {
            if (grid.cols(), grid.rows()) != (self.cols, self.rows) {
                return Err(format!(
                    "{name} is {}x{}, the terminal {}x{}",
                    grid.cols(),
                    grid.rows(),
                    self.cols,
                    self.rows
                ));
            }
        }
        if self.cursor.x >= self.cols || self.cursor.y >= self.rows {
            return Err(format!(
                "cursor at {},{} outside {}x{}",
                self.cursor.x, self.cursor.y, self.cols, self.rows
            ));
        }
        if self.cursor.pending && self.cursor.x != self.right_limit() {
            return Err(format!("a wrap pending at column {}, not the last", self.cursor.x));
        }
        for (slot, saved) in self.saved.iter().enumerate() {
            if saved.cursor.x >= self.cols || saved.cursor.y >= self.rows {
                return Err(format!(
                    "saved cursor {slot} at {},{} outside the screen",
                    saved.cursor.x, saved.cursor.y
                ));
            }
        }
        let m = self.margins;
        if m.top > m.bottom || m.bottom >= self.rows || m.left > m.right || m.right >= self.cols {
            return Err(format!("margins {m:?} outside {}x{} or out of order", self.cols, self.rows));
        }
        if self.tabs.len() != self.cols {
            return Err(format!("{} tab stop slots for {} columns", self.tabs.len(), self.cols));
        }
        self.graphics.check(MAX_IMAGES, IMAGE_BUDGET_BYTES)
    }
}
