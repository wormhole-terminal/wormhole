//! Saying that a program is typing at a terminal or reading it (the GPUI app's `agent_access/drive.rs`): a request
//! that drives a terminal, once its handler has said yes, makes the notice chip of the window that shows it
//! say who and what ("Claude Code is typing", "A script is reading").

use wormhole_core::agents::agent;
use wormhole_core::ipc::access::Resolved;
use wormhole_core::ipc::control::{Action, Target};
use wormhole_core::view::notice::Driver;

use super::events::HookEvent;

/// The terminal `action` drives, and whether it types (else it only reads): the target a request names, or the
/// caller's own tab for a read that names none.
pub(crate) fn drive_of(action: &Action, from: Option<i64>) -> Option<(Target, bool)> {
    let own = |target: Option<Target>| target.or(from.map(Target::Tab));
    match action {
        Action::SendText { target, .. } => Some((*target, true)),
        Action::SendKeys(spec) => Some((spec.target, true)),
        Action::Mouse(spec) => Some((spec.target, true)),
        Action::Run(spec) => Some((spec.target, true)),
        Action::Read(spec) => own(spec.target).map(|target| (target, false)),
        Action::Blocks(spec) => own(spec.target).map(|target| (target, false)),
        Action::Wait(spec) if spec.pattern.is_some() || spec.gone.is_some() => Some((spec.target, false)),
        _ => None,
    }
}

/// Who is behind the request, for the chip.
pub(crate) fn driver_of(resolved: &Resolved) -> Driver {
    resolved.identity.agent.map_or(Driver::Scripts, Driver::Agent)
}

/// The event that tells Swift to say it on `terminal`'s chip. `agent` is the agent's name as the sentence has it.
pub(crate) fn announcement(terminal: i64, typing: bool, driver: Driver) -> HookEvent {
    let agent = match driver {
        Driver::Agent(id) => Some(agent(id).name.to_string()),
        Driver::Scripts => None,
    };
    HookEvent::Driven { terminal, typing, agent }
}

#[cfg(test)]
mod tests {
    use wormhole_core::agents::AgentId;

    use super::*;

    #[test]
    fn an_agent_is_named_and_a_script_is_not() {
        assert_eq!(
            announcement(3, true, Driver::Agent(AgentId::Claude)),
            HookEvent::Driven { terminal: 3, typing: true, agent: Some("Claude Code".into()) }
        );
        assert_eq!(
            announcement(3, false, Driver::Scripts),
            HookEvent::Driven { terminal: 3, typing: false, agent: None }
        );
    }
}
