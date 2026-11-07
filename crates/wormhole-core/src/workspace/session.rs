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
    pub split: Option<SplitId>,
    pub facts: TerminalFacts,
}

impl Session {
    pub(super) fn new(id: i64, number: usize) -> Self {
        Self { id, number, custom_name: None, pinned: false, split: None, facts: TerminalFacts::default() }
    }

    /// The name: the person's, else the title its program set, the agent, the program in front or
    /// the shell (`names::process_name`), else "Terminal n".
    pub(super) fn name(&self) -> String {
        if let Some(name) = &self.custom_name {
            return name.clone();
        }
        let facts = &self.facts;
        let agent = facts.agent.map(|id| crate::agents::agent(id).command);
        let named = names::process_name(&names::NameFacts {
            title: facts.title.as_deref(),
            agent,
            command: facts.command.as_deref(),
            shell: &facts.shell,
        });
        // text: the fallback for a terminal nothing has said anything about yet.
        if named.is_empty() { format!("Terminal {}", self.number) } else { named }
    }
}
