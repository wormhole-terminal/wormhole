//! Background work that wakes an agent again, kept per terminal: a turn that ends while it runs
//! isn't finished. The hooks are separate processes, so the terminal remembers what each one named.
//!
//! Claude lists a turn's leftover shells in `Stop`, but not who started them or why: the main
//! agent's dev server runs on by design, while a `Monitor` or a shell a subagent waits on ends and
//! wakes the session. `PostToolUse:wakes=<id>` names the second kind as it starts (see
//! [`super::describe`]), and a `Stop:shells=…` that still lists one of them means the agent works on.

use std::collections::VecDeque;

use super::{AgentId, Signal, signal};

/// More than any session has in flight; the oldest go first.
const KEPT: usize = 64;

/// The ids of the background tasks that wake the terminal's agent, as its hooks named them.
#[derive(Debug, Default)]
pub struct Wakes {
    ids: VecDeque<String>,
}

impl Wakes {
    /// What a hook event means for the terminal's status, given the work it is waiting on. Learns
    /// from the event first: a task that wakes it, or which of them still run.
    pub fn signal(&mut self, agent: AgentId, event: &str) -> Option<Signal> {
        let (name, detail) = event.split_once(':').unwrap_or((event, ""));
        if let Some(id) = detail.strip_prefix("wakes=") {
            self.remember(id);
        }
        let meant = signal(agent, event);
        match (name, detail.strip_prefix("shells=")) {
            ("Stop", Some(running)) => {
                // A task the turn doesn't list any more has ended.
                self.ids.retain(|id| running.split(',').any(|shell| shell == id));
                if meant == Some(Signal::Done) && !self.ids.is_empty() {
                    return Some(Signal::Busy);
                }
            }
            // A background agent keeps the turn going on its own; it may still be waiting on a shell.
            ("Stop", None) if detail != "background" => self.ids.clear(),
            ("SessionEnd" | "sessionEnd", _) => self.ids.clear(),
            _ => {}
        }
        meant
    }

    fn remember(&mut self, id: &str) {
        if self.ids.iter().any(|known| known == id) {
            return;
        }
        if self.ids.len() == KEPT {
            self.ids.pop_front();
        }
        self.ids.push_back(id.to_string());
    }
}

#[cfg(test)]
mod tests {
    use super::{AgentId, Signal, Wakes};

    fn run(wakes: &mut Wakes, events: &[&str]) -> Vec<Option<Signal>> {
        events.iter().map(|event| wakes.signal(AgentId::Claude, event)).collect()
    }

    #[test]
    fn a_turn_waiting_on_a_subagents_shell_works_on() {
        let mut wakes = Wakes::default();
        let got = run(
            &mut wakes,
            &["Stop:background", "PostToolUse:wakes=b7", "Stop:shells=b7", "UserPromptSubmit", "Stop"],
        );
        use Signal::{Busy, Done};
        assert_eq!(got, [Some(Busy), Some(Busy), Some(Busy), Some(Busy), Some(Done)]);
    }

    #[test]
    fn the_main_agents_own_server_does_not_keep_it_working() {
        let mut wakes = Wakes::default();
        assert_eq!(wakes.signal(AgentId::Claude, "Stop:shells=b1"), Some(Signal::Done));
        // A waking task ended (the turn no longer lists it) while the server runs on.
        run(&mut wakes, &["PostToolUse:wakes=m1"]);
        assert_eq!(wakes.signal(AgentId::Claude, "Stop:shells=b1,m1"), Some(Signal::Busy));
        assert_eq!(wakes.signal(AgentId::Claude, "Stop:shells=b1"), Some(Signal::Done));
        assert_eq!(
            wakes.signal(AgentId::Claude, "Stop:shells=b1,m1"),
            Some(Signal::Done),
            "m1 was forgotten"
        );
    }

    #[test]
    fn a_new_session_forgets_the_old_ones_work() {
        let mut wakes = Wakes::default();
        run(&mut wakes, &["PostToolUse:wakes=m1", "SessionEnd:clear"]);
        assert_eq!(wakes.signal(AgentId::Claude, "Stop:shells=m1"), Some(Signal::Done));
    }

    #[test]
    fn only_so_many_are_kept() {
        let mut wakes = Wakes::default();
        for n in 0..=super::KEPT {
            wakes.signal(AgentId::Claude, &format!("PostToolUse:wakes=t{n}"));
        }
        assert_eq!(wakes.ids.len(), super::KEPT);
        assert_eq!(wakes.signal(AgentId::Claude, "Stop:shells=t0"), Some(Signal::Done));
    }
}
