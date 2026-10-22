//! What the updater asks of a window's terminals before it restarts the app: which a restart
//! would cut off and which still work. The window answers with plain facts, so the updater
//! reads no session.

use gpui_kit::*;
use wormhole_core::agents::AgentId;

use crate::terminal_view::Status;
use crate::window::Workspace;

/// A terminal of a window as a restart sees it.
pub(crate) struct RestartFacts {
    pub name: String,
    pub agent: Option<AgentId>,
    pub status: Status,
    /// The program in front, as the terminal's row names it ("npm run dev").
    pub program: Option<String>,
    /// Something runs in it other than a shell at its prompt (`program_in_foreground`).
    pub program_in_foreground: bool,
    /// Ending it would cut something off: the rule for closing one, and for quitting.
    pub cut_off: bool,
}

impl Workspace {
    /// This window's terminals that a restart ends: a hosted terminal goes on running through the
    /// relaunch, so it is left out.
    pub(crate) fn restart_facts(&self, cx: &App) -> Vec<RestartFacts> {
        self.sessions
            .iter()
            .filter(|session| !session.is_hosted(cx))
            .map(|session| {
                let terminal = session.terminal().read(cx);
                RestartFacts {
                    name: session.name(cx).to_string(),
                    agent: terminal.agent,
                    status: terminal.status(),
                    program: terminal.foreground().map(|foreground| foreground.command.clone()),
                    program_in_foreground: terminal.program_in_foreground(),
                    cut_off: session.needs_close_confirmation(cx),
                }
            })
            .collect()
    }
}
