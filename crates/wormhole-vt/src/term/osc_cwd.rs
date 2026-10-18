//! OSC 7: the folder the shell says it is in (and ConEmu's `9;9`, which says the same).
//!
//! The host hears of it as [`Event::Cwd`] and reads [`Terminal::cwd`] or, when a prompt says it
//! several times between two looks, every report in [`Terminal::cwd_reports`].

use super::event::Event;
use super::oscs::fill;
use super::{CWD_MAX, CWD_REPORTS_MAX, Terminal};

impl Terminal {
    /// OSC 7.
    pub(super) fn set_cwd(&mut self, text: &[u8]) {
        fill(&mut self.cwd, text, CWD_MAX);
        // The report is kept in a buffer that is reused, so a prompt that says it every time
        // allocates nothing once the host has taken the first.
        if self.cwd_reported < self.cwd_reports.len() {
            self.cwd_reports[self.cwd_reported].clone_from(&self.cwd);
        } else {
            self.cwd_reports.push(self.cwd.clone());
        }
        self.cwd_reported = (self.cwd_reported + 1).min(CWD_REPORTS_MAX);
        self.push_event(Event::Cwd);
    }

    /// The folders reported (OSC 7) since [`clear_cwd_reports`](Self::clear_cwd_reports), oldest
    /// first, as written (`file://host/path`, percent-encoded; ConEmu's a bare path). At most the
    /// last [`CWD_REPORTS_MAX`] are kept.
    pub fn cwd_reports(&self) -> &[String] {
        &self.cwd_reports[..self.cwd_reported]
    }

    /// Forget the reports, after reading them; their buffers are kept.
    pub fn clear_cwd_reports(&mut self) {
        self.cwd_reported = 0;
    }
}
