//! Which terminals a request reaches, and who owns the ones a program opened (`agent-permissions.md`,
//! section 6). Core's `Scopes` holds the answers in memory (never stored: after a restart the terminals a
//! program opened earlier are "others"); this finds out, for a request, which terminals it touches and what
//! it needs of each, and keeps the answers in step with the windows (a closed terminal drops out).
//!
//! A terminal hidden from agents is not among the terminals a request touches: it is not found, so there is
//! nothing to ask about, and the handler answers `not_found` as for a terminal that does not exist.

use gpui_kit::*;
use wormhole_core::ipc::Peer;
use wormhole_core::ipc::access::{Capability, Identity};
use wormhole_core::ipc::control::{Action, Reply, Request, Target};

use super::state;
use crate::app_state::app;

/// Whether `action` opens terminals (a terminal, a window, a split): those become the program's own.
pub(super) fn opens(action: &Action) -> bool {
    matches!(action, Action::NewTerminal { .. } | Action::NewWindow(_) | Action::Split(_))
}

/// The terminals `peer` runs inside of, by its place in the process tree: always in its own scope.
pub(super) fn own_terminals(peer: &Peer, cx: &App) -> Vec<i64> {
    app(cx)
        .windows
        .iter()
        .filter_map(|window| {
            window.workspace.read_with(cx, |ws, cx| ws.control_terminals_under(peer, cx)).ok()
        })
        .flatten()
        .collect()
}

/// Every terminal `peer` may know of (not the ones hidden from it).
pub(super) fn visible_terminals(peer: &Peer, cx: &App) -> Vec<i64> {
    app(cx)
        .windows
        .iter()
        .filter_map(|window| window.workspace.read_with(cx, |ws, cx| ws.control_visible_ids(peer, cx)).ok())
        .flatten()
        .collect()
}

/// The terminal `target` names, if there is one and `peer` may know of it.
fn terminal_of(target: Target, peer: &Peer, cx: &App) -> Option<i64> {
    app(cx)
        .windows
        .iter()
        .find_map(|window| {
            window.workspace.read_with(cx, |ws, cx| ws.control_find_as(target, peer, cx)).ok().flatten()
        })
        .map(|(terminal, _)| terminal)
}

/// What `request` needs of each terminal it names, for the ones that exist and that its caller may know of:
/// reading, typing or running, and what rename and focus need. Not `split` (opening is not scoped: what it
/// makes is the program's own), not `close` (every close is the user's to answer, on its own question).
pub(super) fn touched(request: &Request, peer: &Peer, cx: &App) -> Vec<(i64, Capability)> {
    let from = request.from_tab().map(Target::Tab);
    let wanted: Vec<(Target, Capability)> = match &request.action {
        Action::Read(spec) => spec.target.or(from).map(|t| (t, Capability::Read)).into_iter().collect(),
        Action::Blocks(spec) => spec.target.or(from).map(|t| (t, Capability::Read)).into_iter().collect(),
        Action::Wait(spec) if spec.plan().is_ok_and(|plan| plan.needs_read()) => {
            vec![(spec.target, Capability::Read)]
        }
        Action::Events(spec) => spec
            .filter()
            .ok()
            .filter(|filter| filter.topics.contains(wormhole_core::ipc::control::Topic::Output))
            .map(|filter| {
                filter.terminals.iter().map(|id| (Target::Terminal(*id), Capability::Read)).collect()
            })
            .unwrap_or_default(),
        Action::SendText { target, .. } => vec![(*target, Capability::Type)],
        Action::SendKeys(spec) => vec![(spec.target, Capability::Type)],
        Action::Mouse(spec) => vec![(spec.target, Capability::Type)],
        Action::Run(spec) => vec![(spec.target, Capability::Run)],
        Action::Focus(spec) => vec![(spec.target, Capability::Open)],
        Action::Rename(spec) => vec![(spec.target, Capability::Open)],
        _ => Vec::new(),
    };
    wanted
        .into_iter()
        .filter_map(|(target, capability)| Some((terminal_of(target, peer, cx)?, capability)))
        .collect()
}

/// `reply` answers a request of `owner` that opened a terminal or a window: the terminal it names is the
/// program's own from now on. Nothing for a reply that opened none (a refusal; a worktree that waits for the
/// user's dialog).
pub(crate) fn own_opened(owner: Identity, reply: &Reply, cx: &mut App) {
    let opened = reply
        .ok
        .then_some(reply.result.as_ref())
        .flatten()
        .and_then(|result| result.get("terminal"))
        .and_then(serde_json::Value::as_i64);
    if let Some(terminal) = opened {
        state(cx).scopes.own(terminal, owner);
    }
}

/// Forget what is kept about terminals that are closed, and what a session allowed for programs that ended.
pub(super) fn sweep(sessions_too: bool, cx: &mut App) {
    let open: Vec<i64> = app(cx)
        .windows
        .iter()
        .filter_map(|window| window.workspace.read_with(cx, |ws, _| ws.control_ids()).ok())
        .flatten()
        .collect();
    state(cx).scopes.retain_open(&open);
    if sessions_too {
        let now = std::time::Instant::now();
        let running = |process: wormhole_core::ipc::access::ProcessRef| {
            wormhole_core::platform::process::start_time(process.pid) == Some(process.started)
        };
        state(cx).scopes.prune(now, running);
    }
}
