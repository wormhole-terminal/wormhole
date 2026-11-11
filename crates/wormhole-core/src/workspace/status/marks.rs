//! The shell's marks (OSC 133): a command starts running, finishes, and the shell is back at its
//! prompt. And the failure a finished command leaves on the row.

use wormhole_term::{Ended, Mark};

use crate::notify;
use crate::view::sidebar::Status;

use super::super::facts::{Failure, Millis, TerminalFacts};
use super::StatusMachine;

impl StatusMachine {
    /// A shell with integration says exactly where it is: a command starts running (`C`, busy even
    /// before it prints), finishes (`D`, settled at once), and the shell is back at its prompt, where
    /// output is prompts and echoes, not work.
    pub(super) fn on_mark(
        &mut self,
        facts: &mut TerminalFacts,
        mark: Mark,
        ended: Option<Ended>,
        at: Millis,
    ) {
        if self.hook_driven || facts.status == Status::Exited {
            return;
        }
        // Where the shell is stays known while a program's reports decide the status (they end with
        // the program, and the heuristics then need to know whether it is at its prompt).
        let heuristics = !self.reported();
        match mark {
            Mark::PromptStart(_) | Mark::CommandStart => self.at_prompt = Some(true),
            Mark::OutputStart => {
                self.at_prompt = Some(false);
                // The next command: the last one's failure is history.
                facts.failure = None;
                if heuristics {
                    self.finished_mark = false;
                    self.last_output = at;
                    if facts.status != Status::Attention {
                        self.set_status(facts, Status::Busy, at);
                    }
                }
            }
            Mark::Finished(code) => {
                self.at_prompt = Some(true);
                self.finished_mark |= heuristics;
                // A program that reports itself says how it went (the status is its own); a
                // command's exit code is only ever a command's.
                if heuristics {
                    self.command_ended(facts, code, ended, at);
                }
            }
        }
    }

    /// The shell said a command ended with `code`: a failing one is remembered with what the blocks
    /// know of it, any other ends the last failure's story. An interrupt (⌃C) is not a failure.
    fn command_ended(
        &mut self,
        facts: &mut TerminalFacts,
        code: Option<i32>,
        ended: Option<Ended>,
        at: Millis,
    ) {
        facts.failure = notify::failed_exit(code).map(|code| {
            self.failures += 1;
            let (command, took, last_line) = match ended {
                Some(Ended { command, took, last_line }) => (Some(command), took, last_line),
                None => (None, None, None),
            };
            Failure {
                serial: self.failures,
                code,
                command: command.filter(|command| !command.is_empty()),
                at,
                took: took.map(|took| u64::try_from(took.as_millis()).unwrap_or(u64::MAX)),
                last_line,
            }
        });
    }
}
