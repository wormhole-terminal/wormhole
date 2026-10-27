//! `doctor`: what the app tells a caller about how it stands, for an agent that wants to know what it may
//! do before it tries. Always answered, with the Automation flag off too: that is when it matters most.

use gpui_kit::*;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::ipc::Peer;
use wormhole_core::ipc::control::doctor::{Caller, Facts};
use wormhole_core::ipc::control::{PROTOCOL_VERSION, Reply, Target};

use super::find;
use crate::app_state::app;
use crate::settings::settings;

pub(super) fn doctor(from: Option<i64>, peer: &Peer, cx: &App) -> Reply {
    let settings = settings(cx);
    Reply::ok(Facts {
        app_version: env!("CARGO_PKG_VERSION").to_string(),
        protocol: PROTOCOL_VERSION,
        automation: settings.feature_enabled(FeatureFlag::Automation),
        control_write: settings.control_write,
        control_read: settings.control_read,
        caller: caller(from, peer, cx),
    })
}

/// The terminal the caller runs in: the one whose shell it runs under (what the typing rules go by), else
/// the one it says it is in, marked as not confirmed.
fn caller(from: Option<i64>, peer: &Peer, cx: &App) -> Option<Caller> {
    let under = app(cx).windows.iter().find_map(|window| {
        window
            .workspace
            .read_with(cx, |ws, cx| ws.control_terminals_under(peer, cx).first().copied())
            .ok()
            .flatten()
    });
    let (terminal, verified) = match (under, from) {
        (Some(terminal), _) => (terminal, true),
        (None, Some(claimed)) => (claimed, false),
        (None, None) => return None,
    };
    let found = find(Target::Terminal(terminal), cx)?;
    // `None` once the shell has ended: there is nothing to have reported.
    let shell_integration = found
        .window
        .workspace
        .read_with(cx, |ws, cx| ws.control_reader(found.tab, cx))
        .ok()
        .flatten()
        .map(|facts| facts.reader.has_blocks());
    Some(Caller { terminal, verified, shell_integration })
}
