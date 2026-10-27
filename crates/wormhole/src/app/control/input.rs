//! Typing into a terminal from the control line: `send-text` as a paste, `send-keys` as key presses
//! and `mouse` as pointer events. The rules (the switch, the caps, never into the caller's own
//! terminal nor one waiting for the user's answer) are core's, in one place for all three; this finds
//! the terminal and carries out what passes.

use gpui_kit::*;
use wormhole_core::ipc::Peer;
use wormhole_core::ipc::control::{
    ErrorCode, KeysSent, Limited, MouseSpec, Reached, Reason, Reply, SendKeysSpec, SendTarget, Target,
    mouse_check, send_keys_check, send_text_check, writes_off,
};

use super::{Found, automation_off, budget, find, not_found};
use crate::app_state::AppState;
use crate::settings::settings;
use crate::terminal_view::MouseSent;
use crate::window::TabFacts;

/// What the typing rules need of the target: its state and whether the caller runs inside it.
pub(super) fn target_facts(found: &Found, peer: &Peer, cx: &App) -> Option<SendTarget> {
    let facts =
        found.window.workspace.read_with(cx, |ws, cx| ws.control_tab_facts(found.tab, cx)).ok().flatten();
    let TabFacts { state, shell, asked } = facts?;
    // Where the caller runs, not what it says: a process can leave `from` out, but it can't move out
    // from under the shell it was started by, nor off its terminal. In part: a program that starts a
    // session of its own (`setsid`) leaves both behind.
    let caller_inside = shell.is_some_and(|shell| peer.runs_in(shell));
    Some(SendTarget { tab: found.tab, state, caller_inside, asked })
}

/// `send-text`: type into the target's tab as a paste, where the rules allow it.
pub(super) fn send_text(
    target: Target,
    text: &str,
    submit: bool,
    from: Option<i64>,
    peer: &Peer,
    cx: &mut App,
) -> Reply {
    // The switch before anything about the terminal, as the rules order it.
    let enabled = settings(cx).control_write;
    if !enabled {
        let error = writes_off();
        return Reply::from(error);
    }
    let Some(found) = find(target, cx) else { return not_found(target) };
    let Some(facts) = target_facts(&found, peer, cx) else { return not_found(target) };
    if let Err(error) = send_text_check(enabled, text, from, facts) {
        return Reply::from(error);
    }
    if let Err(reply) = budget(Limited::SendText, from, cx) {
        return reply;
    }
    let tab = found.tab;
    let text = text.trim_end_matches(['\n', '\r']).to_string();
    let typed =
        AppState::with_workspace(&found.window, cx, |ws, _, cx| ws.control_send_text(tab, &text, submit, cx));
    if typed != Some(true) {
        return Reply::error(ErrorCode::Failed, format!("tab {tab} could not be reached"));
    }
    tracing::info!(tab, ?from, submit, bytes = text.len(), "control: text sent to a terminal");
    Reply::ok(Reached { tab })
}

/// `send-keys`: press the keys and type the texts in the target's tab, each encoded for the modes the
/// program has when it goes out.
pub(super) fn send_keys(spec: &SendKeysSpec, from: Option<i64>, peer: &Peer, cx: &mut App) -> Reply {
    if let Some(off) = automation_off(cx) {
        return off.into();
    }
    let enabled = settings(cx).control_write;
    if !enabled {
        return Reply::from(writes_off());
    }
    let Some(found) = find(spec.target, cx) else { return not_found(spec.target) };
    let Some(facts) = target_facts(&found, peer, cx) else { return not_found(spec.target) };
    let strokes = match send_keys_check(enabled, &spec.keys, from, facts) {
        Ok(strokes) => strokes,
        Err(error) => return Reply::from(error),
    };
    if let Err(reply) = budget(Limited::SendKeys, from, cx) {
        return reply;
    }
    let tab = found.tab;
    let sent =
        AppState::with_workspace(&found.window, cx, |ws, _, cx| ws.control_send_keys(tab, &strokes, cx))
            .flatten();
    let Some(keys) = sent else {
        return Reply::error(ErrorCode::Failed, format!("tab {tab} could not be reached"));
    };
    tracing::info!(tab, ?from, keys, "control: keys sent to a terminal");
    Reply::ok(KeysSent { tab, keys })
}

/// `mouse`: a click, press, release, move or wheel turn in the target's tab, for a program that asked
/// for the mouse.
pub(super) fn mouse(spec: &MouseSpec, from: Option<i64>, peer: &Peer, cx: &mut App) -> Reply {
    if let Some(off) = automation_off(cx) {
        return off.into();
    }
    let enabled = settings(cx).control_write;
    if !enabled {
        return Reply::from(writes_off());
    }
    let Some(found) = find(spec.target, cx) else { return not_found(spec.target) };
    let Some(facts) = target_facts(&found, peer, cx) else { return not_found(spec.target) };
    if let Err(error) = mouse_check(enabled, &spec.op, from, facts) {
        return Reply::from(error);
    }
    if let Err(reply) = budget(Limited::Mouse, from, cx) {
        return reply;
    }
    let tab = found.tab;
    let sent = AppState::with_workspace(&found.window, cx, |ws, _, cx| ws.control_mouse(tab, &spec.op, cx))
        .flatten();
    match sent {
        Some(MouseSent::Sent) => Reply::ok(Reached { tab }),
        Some(MouseSent::Unwanted) => Reply::from(
            wormhole_core::ipc::control::Error::new(
                ErrorCode::Refused,
                format!(
                    "the program in tab {tab} did not ask for the mouse (or not for this): nothing was sent"
                ),
            )
            .because(Reason::NoMouse),
        ),
        Some(MouseSent::OutOfGrid { rows, cols }) => Reply::error(
            ErrorCode::BadRequest,
            format!("tab {tab}'s screen is {rows} rows by {cols} columns: rows and columns count from 0"),
        ),
        None => Reply::error(ErrorCode::Failed, format!("tab {tab} could not be reached")),
    }
}
