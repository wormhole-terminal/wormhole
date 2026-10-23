//! What the program in the terminal is doing and reports about itself: its folder, whether the shell is at its prompt, progress, status records and the foreground program.

use crate::Pty;
use std::path::PathBuf;
use std::sync::PoisonError;

use crate::{Progress, ReportedCwd, agents, program_status, sys};

impl Pty {
    /// The shell's current working directory.
    ///
    /// A shell with integration says where it is (OSC 7) at every prompt, and that is the truth
    /// even where polling its process can't see it (a nested shell's own folder, a container).
    /// The report wins for as long as polling says what it said when the report came: a prompt
    /// is the only time the shell's folder changes, and the next prompt brings the next report.
    /// When polling moves without a report, a shell without integration has taken over (`exec
    /// bash`), and polling is the truth again until the next report. Reports from another host
    /// (ssh) or of a folder that isn't here never get this far (`EventProxy::osc`), so those
    /// terminals show the local shell's folder, as before.
    pub fn cwd(&self) -> Option<PathBuf> {
        let polled = sys::process::cwd(self.pid()?);
        let mut reported = self.reported_cwd.lock().unwrap_or_else(PoisonError::into_inner);
        if let Some(path) = self.shared.take_cwd_report() {
            *reported = Some(ReportedCwd { path, polled_then: polled.clone() });
        }
        match &*reported {
            Some(report) if report.polled_then == polled => Some(report.path.clone()),
            _ => polled,
        }
    }

    /// Whether the shell is at its prompt, as its OSC 133 marks say; `None` for a shell without
    /// integration (nothing has marked anything yet). Exact where `shell_in_foreground` guesses.
    pub fn at_prompt(&self) -> Option<bool> {
        self.shared.at_prompt()
    }

    /// The exit code of the last command the shell reported finished (OSC 133 `D`).
    pub fn last_exit_code(&self) -> Option<i32> {
        self.shared.last_exit_code()
    }

    /// The progress a program reports (OSC 9;4), until it removes it.
    pub fn progress(&self) -> Option<Progress> {
        self.shared.progress()
    }

    /// What programs in this terminal report about themselves (OSC 7501). The PTY thread keeps it
    /// (a hostile program's reports pile up in 256 records, not in a queue), and a
    /// [`PtyEvent::ProgramStatus`] says it changed. The lock is held for a lookup or an update,
    /// never across a wait.
    pub fn program_status(&self) -> std::sync::MutexGuard<'_, program_status::ProgramStatus> {
        self.shared.program_status()
    }

    /// The coding agent running in the foreground, if any.
    pub fn foreground_agent(&self) -> Option<agents::AgentId> {
        agents::foreground_agent(self.pid()?)
    }

    /// Whether the shell is back at its prompt (nothing else in the foreground).
    pub fn shell_in_foreground(&self) -> bool {
        self.pid().is_some_and(agents::shell_in_foreground)
    }

    /// Whether a program other than the shell holds the terminal; `None` where
    /// the platform can't tell. While one does, the shell's folder can't change.
    pub fn program_in_foreground(&self) -> Option<bool> {
        agents::shell_holds_terminal(self.pid()?).map(|shell| !shell)
    }

    /// The program in the foreground and since when, unless it's the shell.
    pub fn foreground_command(&self) -> Option<agents::Foreground> {
        agents::foreground_command(self.pid()?)
    }
}
