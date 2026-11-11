//! One terminal of the window, as the model keeps it: who it is, what the person decided about it
//! (a name, a pin, its split) and what the shell last said of it.

use crate::view::sidebar::{SplitId, names};

use super::facts::TerminalFacts;

#[derive(Clone, Debug)]
pub(super) struct Session {
    pub id: i64,
    /// The n-th terminal made in this window, for the name of one that has none yet.
    pub number: usize,
    /// A name the person gave it, which wins over what runs in it.
    pub custom_name: Option<String>,
    pub pinned: bool,
    /// "Hide from Agents": a program that controls Wormhole cannot list, read or type into it.
    pub agent_hidden: bool,
    /// "Forget text": its screen is not saved with the window.
    pub forget_text: bool,
    pub split: Option<SplitId>,
    /// The shell it was opened with (`terminal::shells`' key), `None` for the default: a restart of its
    /// shell and the next launch run the same thing.
    pub shell: Option<String>,
    /// Its record said "Done, unseen" when the window was restored: the result comes back if its program
    /// is the same one (its session host attached again), and is dropped once that is decided.
    pub done_unseen: bool,
    pub facts: TerminalFacts,
}

impl Session {
    pub(super) fn new(id: i64, number: usize) -> Self {
        Self {
            id,
            number,
            custom_name: None,
            pinned: false,
            agent_hidden: false,
            forget_text: false,
            split: None,
            shell: None,
            done_unseen: false,
            facts: TerminalFacts::default(),
        }
    }

    /// The name: the person's, else the title its program set, the agent, the program in front or
    /// the shell (`names::process_name`), else "Terminal n".
    pub(super) fn name(&self) -> String {
        match &self.custom_name {
            Some(name) => name.clone(),
            None => self.label(),
        }
    }

    /// What the terminal is called by what runs in it, whatever the person named it (the title row's last chip,
    /// the GPUI app's `Pane::label`).
    pub(super) fn label(&self) -> String {
        let facts = &self.facts;
        let agent = facts.agent.map(|id| crate::agents::agent(id).command);
        let named = names::process_name(&names::NameFacts {
            title: facts.title.as_deref(),
            agent,
            command: facts.command.as_deref().or(facts.finished_command.as_deref()),
            shell: &facts.shell,
        });
        // text: the fallback for a terminal nothing has said anything about yet.
        if named.is_empty() { format!("Terminal {}", self.number) } else { named }
    }
}
