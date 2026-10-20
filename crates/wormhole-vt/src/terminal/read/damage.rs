//! What changed on the screen since the host last looked: [`Damage`], filled by
//! [`Terminal::take_damage`].
//!
//! The grid marks a row dirty as it writes to it (one bit in the row's record, set on the way
//! that already touches the row), so telling the host which rows to draw again costs one pass over
//! the view's rows when it asks and nothing while a program prints. The cursor and the colours a
//! program changed are compared with what the last call saw: two small comparisons per call, none
//! per byte.

use crate::terminal::Terminal;
use crate::terminal::read::snapshot::Cursor;
use crate::types::Colors;

/// What changed on the screen since the last [`Terminal::take_damage`]: which rows of the view,
/// and whether the cursor or the colours a program set did.
///
/// Make one with [`Damage::new`] and pass the same value to every call: it keeps its buffer, so
/// once it has the size of the screen a call allocates nothing.
///
/// ```
/// use wormhole_vt::{Damage, Terminal};
///
/// let mut term = Terminal::new(80, 24);
/// let mut damage = Damage::new();
/// term.take_damage(&mut damage);
/// assert!(damage.is_full(), "the first look is of everything");
///
/// term.feed(b"\x1b[3;1Hhello");
/// term.take_damage(&mut damage);
/// assert_eq!(damage.rows().collect::<Vec<_>>(), [2]);
/// assert!(damage.cursor_changed());
///
/// term.take_damage(&mut damage);
/// assert!(damage.is_empty());
/// ```
#[derive(Clone, Debug, Default)]
#[non_exhaustive]
pub struct Damage {
    /// Everything changed: the view moved, the size changed, a clear, the first look.
    full: bool,
    /// Per row of the view, whether it changed; meaningful when `full` is not set.
    rows: Vec<bool>,
    cursor: bool,
    colors: bool,
}

impl Damage {
    /// A value that has seen nothing; the first [`Terminal::take_damage`] makes it [full](Self::is_full).
    #[must_use]
    pub fn new() -> Damage {
        Damage::default()
    }

    /// Whether everything changed (the view scrolled into the history, a resize, a clear, a
    /// picture appearing or going), or this value had not seen this screen's size before. Then
    /// [`rows`](Self::rows) lists every row.
    #[must_use]
    pub fn is_full(&self) -> bool {
        self.full
    }

    /// The rows of the view (0 is the top one) that changed, ascending; every row when the damage
    /// [is full](Self::is_full).
    pub fn rows(&self) -> impl Iterator<Item = usize> + '_ {
        let full = self.full;
        self.rows.iter().enumerate().filter_map(move |(row, &changed)| (full || changed).then_some(row))
    }

    /// Whether `row` of the view changed (also when the damage is full). A row the screen does not
    /// have did not.
    #[must_use]
    pub fn contains(&self, row: usize) -> bool {
        row < self.rows.len() && (self.full || self.rows[row])
    }

    /// Whether the cursor's place, visibility or look changed. In grid coordinates, as
    /// [`Terminal::cursor`] gives it, so scrolling the view does not count.
    #[must_use]
    pub fn cursor_changed(&self) -> bool {
        self.cursor
    }

    /// Whether the palette entries programs set (OSC 4, 10, 11, 12 and their resets) changed; see
    /// [`Terminal::colors`]. The colours the host sets itself are not counted.
    #[must_use]
    pub fn colors_changed(&self) -> bool {
        self.colors
    }

    /// Whether nothing changed: no row, not the cursor and not the colours.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        !self.full && !self.cursor && !self.colors && !self.rows.contains(&true)
    }
}

/// What the last [`Terminal::take_damage`] saw of the things it compares instead of marks.
#[derive(Debug)]
pub(in crate::terminal) struct Seen {
    /// `None` before the first call.
    cursor: Option<Cursor>,
    colors: Colors,
}

impl Seen {
    pub(in crate::terminal) fn new() -> Seen {
        Seen { cursor: None, colors: Colors::default() }
    }
}

impl Terminal {
    /// Say into `damage` what changed since the last call, and forget it: the rows of the view
    /// that were written to, whether everything did (the view moved, a resize, a clear), and
    /// whether the cursor or the colours a program set did. Pair it with
    /// [`copy_viewport`](Self::copy_viewport) or [`copy_row`](Self::copy_row) to copy only the
    /// rows listed.
    ///
    /// Only one reader should follow a terminal: the marks it clears are the terminal's, not
    /// `damage`'s. No allocation once `damage` has seen a screen of this size.
    pub fn take_damage(&mut self, damage: &mut Damage) {
        let resized = damage.rows.len() != self.rows;
        damage.rows.resize(self.rows, false);
        damage.full = self.grid.take_damage(&mut damage.rows) | resized;

        let cursor = self.cursor();
        damage.cursor = self.seen.cursor.replace(cursor) != Some(cursor);
        damage.colors = self.seen.colors != self.palette.set;
        if damage.colors {
            self.seen.colors = self.palette.set;
        }
    }
}
