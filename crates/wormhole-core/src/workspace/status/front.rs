//! Who holds the terminal: the program in front, the coding agent among the foreground processes,
//! and the shell getting the terminal back. The shell looks at the process (`ForegroundLook`) every
//! few seconds, since the system sends no event for it, and after what may have changed it.

use crate::agents::AgentId;
use crate::view::sidebar::Status;

use super::super::facts::{Millis, TerminalFacts};
use super::rules::program_left_the_terminal;
use super::{Ask, ForegroundLook, StatusMachine};

impl StatusMachine {
    /// Take what a look at the process found. A program that held the terminal and is gone takes
    /// whatever only it could say with it (a crash leaves no word of its own): the shell is asked to
    /// drop its records.
    pub(super) fn on_foreground(
        &mut self,
        facts: &mut TerminalFacts,
        look: ForegroundLook,
        at: Millis,
    ) -> Option<Ask> {
        let left = program_left_the_terminal(self.program_in_front, look.in_front);
        self.program_in_front = look.in_front == Some(true);
        let foreground = if self.program_in_front { look.command } else { None };
        if foreground != self.foreground {
            let ended = std::mem::replace(&mut self.foreground, foreground);
            self.ended_command = match &self.foreground {
                None => ended.map(|front| front.command),
                Some(_) => None,
            };
        }
        self.refresh_agent(facts, look.agent, look.shell_in_front, at);
        (left && self.program.is_some()).then_some(Ask::ProgramLeft)
    }

    fn refresh_agent(
        &mut self,
        facts: &mut TerminalFacts,
        agent: Option<AgentId>,
        shell_in_front: bool,
        at: Millis,
    ) {
        // An agent reporting through hooks, or a program saying what it is, is known even when its
        // process doesn't look like one (it may run over ssh); forget it only once the shell is back.
        if agent == facts.agent || (agent.is_none() && self.reported() && !shell_in_front) {
            return;
        }
        // Hooks can't report an agent that has quit (or crashed): hand the status back to the output
        // heuristic.
        if agent.is_none() && self.hook_driven {
            self.hook_driven = false;
            self.wakes = Default::default();
            facts.message = None;
            if matches!(facts.status, Status::Busy | Status::Attention) {
                self.set_status(facts, Status::Idle, at);
            }
        }
        facts.agent = agent;
    }
}
