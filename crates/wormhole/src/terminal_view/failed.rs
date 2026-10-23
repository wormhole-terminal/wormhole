//! A command that failed, and who tells a terminal's state.
//!
//! A shell with integration says how a command ended (OSC 133 `D;code`). A non-zero code is a
//! failure the row keeps saying ("cargo build · exit 101") until the next command starts, whether
//! or not anyone was looking when it ended; the status `Failed` is only for the failure nobody
//! saw, and clears when the terminal is opened, like `Done`.

use std::time::{Duration, Instant};

use wormhole_core::notify::{self, Source};
use wormhole_core::view::sidebar::names::{command_name, command_name_is};

use super::TerminalView;

/// A command that ended with a failing exit code (see [`notify::failed_exit`]).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    /// Counts this terminal's failures: a new one is news even with the code of the last.
    pub serial: u64,
    pub code: i32,
    /// When the shell said so.
    pub at: Instant,
    /// The command line, how long it ran and the last thing it printed, as far as the shell's
    /// blocks could say (`Pty::ended`).
    pub command: Option<String>,
    pub took: Option<Duration>,
    pub last_line: Option<String>,
}

impl TerminalView {
    /// The last command's failure, until the next command starts. The row says it whether or not
    /// the status still does.
    pub fn failure(&self) -> Option<&Failure> {
        self.failure.as_ref()
    }

    /// The shell said a command ended with `code`: a failing one is remembered with what the
    /// blocks know of it, any other ends the last failure's story.
    pub(super) fn command_ended(&mut self, code: Option<i32>) {
        let Some(code) = notify::failed_exit(code) else {
            self.failure = None;
            return;
        };
        let ended = self.pty.as_ref().and_then(|pty| pty.ended(code));
        self.failures += 1;
        self.failure = Some(Failure {
            serial: self.failures,
            code,
            at: Instant::now(),
            command: ended.as_ref().map(|e| e.command.clone()).filter(|c| !c.is_empty()),
            took: ended.as_ref().and_then(|e| e.took),
            last_line: ended.and_then(|e| e.last_line),
        });
    }

    /// What tells this terminal's state: an agent, a program that reports its own status without
    /// being a coding agent, or nothing (the output, the bell and the shell's marks).
    pub fn source(&self) -> Source {
        match () {
            _ if self.agent.is_some() => Source::Agent,
            _ if self.program_driven => Source::Program,
            _ => Source::Plain,
        }
    }

    /// The name of the program that reports its status: what it calls itself (`app`, which a
    /// report may leave to its parent's), else the command it runs. `None` for a terminal no
    /// program reports in, and where neither is known.
    pub fn program_name(&self) -> Option<String> {
        if self.source() != Source::Program {
            return None;
        }
        let app = self.program.as_ref().and_then(|program| program.app.clone());
        let command = || {
            let command =
                self.foreground.as_ref().map(|f| f.command.as_str()).or(self.finished_command.as_deref());
            command.map(command_name).filter(|name| !name.is_empty())
        };
        app.or_else(command)
    }

    /// Whether [`program_name`](Self::program_name) is `expected`, without making the name: the
    /// window compares its copy of a terminal with the terminal on every redraw of it, and a
    /// reporting program's name is built from the command line when it gave none of its own.
    pub fn program_name_is(&self, expected: Option<&str>) -> bool {
        if self.source() != Source::Program {
            return expected.is_none();
        }
        if let Some(app) = self.program.as_ref().and_then(|program| program.app.as_deref()) {
            return expected == Some(app);
        }
        let command =
            self.foreground.as_ref().map(|f| f.command.as_str()).or(self.finished_command.as_deref());
        match (command, expected) {
            (Some(command), Some(name)) => !name.is_empty() && command_name_is(command, name),
            // A command with no words has no name.
            (Some(command), None) => command.split_whitespace().next().is_none(),
            (None, expected) => expected.is_none(),
        }
    }
}
