//! What `list`, `status`, `notify` and `doctor` tell and do.

use std::sync::Arc;
use std::time::Instant;

use wormhole_core::config::AgentAccess;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::ipc::Peer;
use wormhole_core::ipc::access::Scope;
use wormhole_core::ipc::control::doctor::{Access as DoctorAccess, Caller, Facts, Standing};
use wormhole_core::ipc::control::{
    ErrorCode, Limited, PROTOCOL_VERSION, Reached, Reply, StatusInfo, Summary, Target, notice_text,
};
use wormhole_core::workspace::StatusEvent;

use super::not_found;
use crate::hook_server::core::Core;
use crate::hook_server::gate::identify;

/// `status`: the target's terminal, the caller's own without one, the app's summary outside any tab.
pub(super) fn status(core: &Core, target: Option<Target>, from: Option<i64>, peer: &Peer) -> Reply {
    let listing = core.listing();
    let Some(target) = target.or(from.map(Target::Tab)) else {
        let states = listing.windows.iter().flat_map(|w| w.terminals.iter().map(|t| t.status));
        return Reply::ok(Summary::of(listing.windows.len(), states));
    };
    let Some(id) = core.find(target, peer) else { return not_found(target) };
    let found = listing
        .windows
        .iter()
        .find_map(|window| window.terminals.iter().find(|t| t.id == id).map(|t| (window.id, t.clone())));
    match found {
        Some((window, terminal)) => Reply::ok(StatusInfo { window, terminal, tab: target.id() }),
        None => not_found(target),
    }
}

/// `notify`: a notification about the target's terminal (the caller's own without one), in the caller's
/// words: the terminal keeps it as its latest notification (`StatusEvent::ControlNotice`) and the notification
/// rules tell it like a program's own, from the terminal's facts, with the status left alone.
pub(super) fn notify(
    core: &Core,
    target: Option<Target>,
    title: &str,
    body: &str,
    from: Option<i64>,
    peer: &Peer,
) -> Reply {
    let Some(words) = notice_text(title, body) else {
        return Reply::error(ErrorCode::BadRequest, "nothing to say once the text is cleaned");
    };
    let Some(target) = target.or(from.map(Target::Tab)) else {
        return Reply::error(
            ErrorCode::BadRequest,
            "name a terminal (--terminal, --tab or --pane); inside one of Wormhole's terminals its own is the default",
        );
    };
    let Some(tab) = core.find(target, peer) else { return not_found(target) };
    if let Err(reply) = core.budget(Limited::Notify, from) {
        return reply;
    }
    // Told by the notification rules in the caller's words, as GPUI's `notice_from_control`: the status stays.
    core.shared.feed(tab, StatusEvent::ControlNotice { title: words.title, body: words.body });
    Reply::ok(Reached { tab })
}

/// `doctor`: what the app tells a caller about how it stands. Always answered, with the Automation flag
/// off too: that is when it matters most.
pub(super) fn doctor(core: &Arc<Core>, from: Option<i64>, peer: &Peer) -> Reply {
    let settings = core.settings();
    let automation = settings.feature_enabled(FeatureFlag::Automation);
    // With the feature on, agent access decides and says where the caller stands; the two older switches
    // are what an older command line reads, so they say whether anything is let in at all.
    let access = automation.then(|| standing(core, peer));
    let (control_write, control_read) = match &access {
        Some(access) => (access.mode != AgentAccess::Off, access.mode != AgentAccess::Off),
        None => (settings.control_write, false),
    };
    Reply::ok(Facts {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        protocol: PROTOCOL_VERSION,
        automation,
        control_write,
        control_read,
        caller: caller(core, from, peer),
        access,
    })
}

/// The terminal the caller runs in: the one whose shell it runs under (what the typing rules go by), else
/// the one it says it is in, marked as not confirmed.
fn caller(core: &Core, from: Option<i64>, peer: &Peer) -> Option<Caller> {
    let (terminal, verified) = match (core.terminals_under(peer).first().copied(), from) {
        (Some(terminal), _) => (terminal, true),
        (None, Some(claimed)) => (claimed, false),
        (None, None) => return None,
    };
    let id = core.find(Target::Terminal(terminal), peer)?;
    // `None` once the shell has ended: there is nothing to have reported.
    let shell_integration = core.terminal(id).and_then(|t| t.reader()).map(|reader| reader.has_blocks());
    Some(Caller { terminal, verified, shell_integration })
}

/// Where the caller stands, for `doctor`: the mode, who the app takes it for and what covers it. Reads,
/// never asks.
fn standing(core: &Core, peer: &Peer) -> DoctorAccess {
    let mode = core.settings().agent_access;
    let resolved = identify(core, peer).ok();
    let who = resolved.as_ref().map_or_else(|| "Scripts".to_string(), |resolved| resolved.label.clone());
    let standing = match (mode, &resolved) {
        (AgentAccess::Off, _) => Standing::Off,
        (AgentAccess::Open, _) => Standing::Open,
        (AgentAccess::Ask, None) => Standing::NotAsked,
        (AgentAccess::Ask, Some(resolved)) => {
            let now = Instant::now();
            let held = core.desk().with_state(|grants, _| {
                grants.find(&resolved.identity, resolved.process).map(|held| held.scope)
            });
            match held {
                Some(Scope::Session(_)) => Standing::Session,
                Some(Scope::UntilQuit) => Standing::UntilQuit,
                None if core.desk().stored_caps(&resolved.identity.key()).is_some() => Standing::Always,
                None if core.desk().with_state(|grants, _| grants.cooling(&resolved.identity, now)) => {
                    Standing::DeclinedRecently
                }
                None => Standing::NotAsked,
            }
        }
    };
    DoctorAccess { mode, who, standing }
}
