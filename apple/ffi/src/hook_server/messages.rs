//! What agents' hooks send over the socket besides control requests: an agent's event for a terminal, the
//! plan usage its status line reports, a second launch asking to come forward. Nothing is answered.

use std::sync::Arc;

use wormhole_core::ipc::Message;
use wormhole_core::ipc::access::agent_word;
use wormhole_core::workspace::StatusEvent;

use super::core::Core;
use super::events::HookEvent;

/// Act on one message from the socket.
pub(crate) fn apply(core: &Arc<Core>, message: Message) {
    match message {
        // The terminal's id is what `WORMHOLE_TAB_ID` held; one this window does not have (another window's,
        // or one that closed) is ignored by the model. What the event means for the status is core's.
        Message::Hook { tab, agent, event, message } => {
            let Some(id) = tab.trim().parse::<i64>().ok() else { return };
            core.shared.feed(id, StatusEvent::Hook { agent, event, message });
        }
        // Every agent's plan limits, for the Usage tab: the latest report of an agent wins.
        Message::Usage { usage } => core.usage.record(usage),
        Message::Activate => core.ui.push(HookEvent::Activate),
        // The terminal's own: its agent's model and context window, for its title row. A row's facts hold
        // neither, so it goes to Swift, which keeps the latest per terminal (a status line runs many times a
        // minute: only a change is worth a wake).
        Message::Gauge { tab, gauge } => {
            let Some(terminal) = tab.trim().parse::<i64>().ok() else { return };
            let agent = agent_word(gauge.agent);
            core.ui.push_gauge(HookEvent::Gauge {
                terminal,
                agent,
                model: gauge.model,
                context: gauge.context,
            });
        }
        // A request never comes this way: the server answers it instead of forwarding it.
        Message::Control(_) => {}
    }
}
