//! OSC 7: the folder the shell says it is in (and ConEmu's `9;9`, which says the same).
//!
//! The host hears of it as [`Event::Cwd`] and reads [`Terminal::cwd`] or, when a prompt says it
//! several times between two looks, every report in [`Terminal::cwd_reports`].

use crate::terminal::event::Event;
use crate::terminal::osc::dispatch::fill;
use crate::terminal::reports::title::drop_controls;
use crate::terminal::{CWD_MAX, CWD_REPORTS_MAX, Terminal};

/// The shell's folder as the last OSC 7 said it (empty before any), and every report since the
/// host last looked: the first `reported` of `reports`, whose strings are reused.
#[derive(Debug, Default)]
pub(in crate::terminal) struct Folder {
    pub(in crate::terminal) now: String,
    pub(in crate::terminal) reports: Vec<String>,
    pub(in crate::terminal) reported: usize,
}

impl Terminal {
    /// OSC 7.
    pub(in crate::terminal) fn set_cwd(&mut self, text: &[u8]) {
        fill(&mut self.folder.now, text, CWD_MAX);
        // A folder is drawn and passed on (to a new terminal's shell): no controls, as in a title.
        drop_controls(&mut self.folder.now);
        // The report is kept in a buffer that is reused, so a prompt that says it every time
        // allocates nothing once the host has taken the first.
        if self.folder.reported < self.folder.reports.len() {
            self.folder.reports[self.folder.reported].clone_from(&self.folder.now);
        } else {
            self.folder.reports.push(self.folder.now.clone());
        }
        self.folder.reported = (self.folder.reported + 1).min(CWD_REPORTS_MAX);
        self.push_event(Event::Cwd);
    }

    /// The folders reported (OSC 7) since [`clear_cwd_reports`](Self::clear_cwd_reports), oldest
    /// first, as written (`file://host/path`, percent-encoded; ConEmu's a bare path). At most the
    /// last [`CWD_REPORTS_MAX`] are kept.
    pub fn cwd_reports(&self) -> &[String] {
        &self.folder.reports[..self.folder.reported]
    }

    /// Forget the reports, after reading them; their buffers are kept.
    pub fn clear_cwd_reports(&mut self) {
        self.folder.reported = 0;
    }
}
