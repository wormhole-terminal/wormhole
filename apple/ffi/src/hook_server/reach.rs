//! Which profile a request reaches (`agent-permissions.md`, section 9; the GPUI app's `agent_access/profile.rs`): a
//! profile is a trust domain with its own mode and approvals, so a request is judged by the profile of the window
//! it reaches, never by the one that happens to be shown.

use std::sync::Arc;

use wormhole_core::ipc::Peer;
use wormhole_core::ipc::control::{Action, Request, Target};

use super::core::{Core, Seat};

/// The terminal an action names, or defaults to (the caller's own, `from`).
fn target_of(action: &Action, from: Option<i64>) -> Option<Target> {
    let own = |target: Option<Target>| target.or(from.map(Target::Tab));
    match action {
        Action::SendText { target, .. } => Some(*target),
        Action::SendKeys(spec) => Some(spec.target),
        Action::Mouse(spec) => Some(spec.target),
        Action::Run(spec) => Some(spec.target),
        Action::Wait(spec) => Some(spec.target),
        Action::Close(spec) => Some(spec.target),
        Action::Focus(spec) => Some(spec.target),
        Action::Rename(spec) => Some(spec.target),
        Action::Split(spec) => Some(spec.target),
        Action::Read(spec) => own(spec.target),
        Action::Blocks(spec) => own(spec.target),
        Action::Status { target } | Action::Notify { target, .. } => own(*target),
        Action::Events(spec) => {
            spec.filter().ok().and_then(|f| f.terminals.first().map(|id| Target::Terminal(*id)))
        }
        _ => None,
    }
}

impl Core {
    /// The seat of the window `request` reaches: the one with the terminal it names, the one named for a new
    /// terminal, else the profile that is shown (where a request with no target acts).
    pub(crate) fn seat_reached(&self, request: &Request, peer: &Peer) -> Arc<Seat> {
        let named = match &request.action {
            Action::NewTerminal { window: Some(id), .. } => self.share_of_window(*id),
            action => target_of(action, request.from_tab())
                .and_then(|target| self.find(target, peer))
                .and_then(|id| self.owner_of(id)),
        };
        named.map_or_else(|| self.seat(), |shared| self.seat_of_window(&shared))
    }
}
