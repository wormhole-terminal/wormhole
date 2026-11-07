//! An agent's hooks as the terminal's status, a key that answers its question, and the person
//! looking (which spends a result).

use crate::agents::AgentId;
use crate::agents::integrations::{self, Signal};
use crate::view::sidebar::Status;

use super::super::facts::{Millis, TerminalFacts};
use super::{Ask, Context, StatusMachine};

impl StatusMachine {
    /// A hook event for this terminal: what it means for the status, given the work the agent waits
    /// on (a turn that ends while a background task still runs isn't finished). A result that lands
    /// where the person is looking is seen at once.
    pub(super) fn on_hook(
        &mut self,
        facts: &mut TerminalFacts,
        agent: AgentId,
        event: &str,
        message: Option<String>,
        at: Millis,
        cx: Context,
    ) -> Option<Ask> {
        let signal = self.wakes.signal(agent, event)?;
        self.apply_signal(facts, agent, signal, message, at);
        if cx.looking() && matches!(facts.status, Status::Done | Status::Failed) {
            return self.acknowledge(facts, at);
        }
        None
    }

    /// Apply a status event from an agent's hooks.
    pub(super) fn apply_signal(
        &mut self,
        facts: &mut TerminalFacts,
        agent: AgentId,
        signal: Signal,
        message: Option<String>,
        at: Millis,
    ) {
        if facts.status == Status::Exited {
            return;
        }
        facts.agent = Some(agent);
        self.hook_driven = signal != Signal::Ended;
        // The last word is the hook's: a program's records, if any, don't reset what it set.
        self.program_driven = false;
        // The prompt's own words ("Approve cargo test?") beat the notification that the same prompt is
        // still waiting, which comes seconds later.
        let asked = facts.status == Status::Attention && facts.message.is_some();
        let (status, message) = match signal {
            Signal::Busy => (Status::Busy, None),
            Signal::Attention if asked => (Status::Attention, facts.message.take()),
            Signal::Attention => (Status::Attention, message),
            Signal::Done => (Status::Done, None),
            Signal::Failed => (Status::Failed, message),
            Signal::Quiet | Signal::Ended => (Status::Idle, None),
        };
        facts.message = message;
        self.set_status(facts, status, at);
    }

    /// A key typed while the agent's question is up: Return or a choice answers it, Esc turns it
    /// down. No hook says so until the tool has run, which can take minutes.
    pub(super) fn answer(&mut self, facts: &mut TerminalFacts, key: &str, control: bool, at: Millis) {
        if !self.hook_driven || facts.status != Status::Attention {
            return;
        }
        let (Some(agent), Some(signal)) = (facts.agent, integrations::answer_key(key, control)) else {
            return;
        };
        self.apply_signal(facts, agent, signal, None, at);
    }

    /// Clear "done", "failed" and "attention" and the bell once the person looks. A pending
    /// question, from an agent's hooks or a program's own report, stays until the program moves on.
    pub(super) fn acknowledge(&mut self, facts: &mut TerminalFacts, at: Millis) -> Option<Ask> {
        facts.bell = false;
        facts.unread = false;
        if (self.hook_driven || self.blocked_by_program()) && facts.status == Status::Attention {
            return None;
        }
        let seen = matches!(facts.status, Status::Done | Status::Failed | Status::Attention);
        if !seen {
            return None;
        }
        // What a program reported as finished has been seen: forgotten, so the next derivation
        // doesn't bring "done" back. What it still has going on is the status now, if it is the
        // program's status at all.
        if !self.program_driven {
            self.set_status(facts, Status::Idle, at);
        }
        self.program.is_some().then_some(Ask::AcknowledgeReports)
    }
}
