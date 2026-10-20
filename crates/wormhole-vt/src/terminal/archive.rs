//! Keeping the history that leaves the engine: the host's switch, and the events that carry the
//! lines out.
//!
//! The grid does the copying (`grid/archive.rs`) where it drops pages; this is the terminal's side:
//! turn it on, and hand what waited over as [`Event::HistoryDropped`] whenever the host takes
//! its events. Off, which is the default, nothing here runs but one test in
//! [`Terminal::events`].

use super::{Event, Terminal};
use crate::grid::Grid;

impl Terminal {
    /// Whether history lines that leave the engine are handed to the host; see
    /// [`set_archive_history`](Self::set_archive_history).
    pub fn archive_history(&self) -> bool {
        self.main_grid().archiving()
    }

    /// Hand the host the history lines the engine lets go of, as [`Event::HistoryDropped`]: lines
    /// scrolled out past the scrollback limit, lines cleared (ED 3, RIS,
    /// [`clear_history`](Self::clear_history)) and lines cut because the limit fell
    /// ([`set_scrollback_lines`](Self::set_scrollback_lines), a resize). The host can keep them
    /// on disk or in memory and show more history than the engine holds. Off by default, and free
    /// when off. Only lines that leave from now on are handed over; the alternate screen has none.
    ///
    /// The lines wait in the engine until the host calls [`events`](Self::events), at most
    /// [`Limits::archive_backlog`](crate::Limits::archive_backlog) bytes of them: drain the
    /// events regularly (also between large `feed`s), or the newest lines are only counted.
    /// Switching it off hands over what waits first.
    pub fn set_archive_history(&mut self, on: bool) {
        if on {
            let cap = self.limits.archive_backlog;
            self.main_grid_mut().set_archive(Some(cap));
        } else {
            self.flush_archive();
            self.main_grid_mut().set_archive(None);
        }
    }

    /// The main screen's grid: the one showing, or the one kept aside while the alternate screen
    /// shows. The history, and so the archive, is its.
    fn main_grid(&self) -> &Grid {
        if self.on_alt_screen() { &self.other } else { &self.grid }
    }

    pub(super) fn main_grid_mut(&mut self) -> &mut Grid {
        if self.on_alt_screen() { &mut self.other } else { &mut self.grid }
    }

    /// The backlog's size changed ([`Limits::archive_backlog`](crate::Limits::archive_backlog)).
    pub(super) fn set_archive_cap(&mut self, cap: usize) {
        if self.archive_history() {
            self.main_grid_mut().set_archive(Some(cap));
        }
    }

    /// Queue what the grid collected as events. The archive's batches are not subject to the
    /// events' cap: they are bounded by their own (a byte budget the host's draining resets).
    pub(super) fn flush_archive(&mut self) {
        let batches = self.main_grid_mut().take_archive();
        self.events.extend(batches.into_iter().map(Event::HistoryDropped));
    }
}
