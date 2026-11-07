//! What agents' hooks send over the socket besides control requests: an agent's event for a terminal, the
//! plan usage its status line reports, a second launch asking to come forward. Nothing is answered.

use std::sync::Arc;

use wormhole_core::ipc::Message;
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
        // The model and context window of a terminal's agent are not among the facts a row holds yet.
        Message::Gauge { .. } => {}
        // A request never comes this way: the server answers it instead of forwarding it.
        Message::Control(_) => {}
    }
}
