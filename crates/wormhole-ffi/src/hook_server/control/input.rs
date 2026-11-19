//! Typing into a terminal from the control line: `send-text` as a paste, `send-keys` as key presses and
//! `mouse` as pointer events. The rules (the switch, the caps, never into the caller's own terminal nor one
//! waiting for the user's answer) are core's, in one place for all three; this finds the terminal and
//! carries out what passes.

use std::time::Duration;

use wormhole_core::ipc::Peer;
use wormhole_core::ipc::access::Access;
use wormhole_core::ipc::control::{
    ErrorCode, KeysSent, Limited, MouseSpec, Reached, Reason, Reply, SendKeysSpec, SendTarget, Target,
    mouse_check, send_keys_check, send_text_check, writes_off,
};
use wormhole_core::workspace::StatusEvent;

use super::{automation_off, not_found};
use crate::hook_server::core::Core;
use crate::hook_server::terminals::MouseSent;

/// The pause between a pasted text and the Return that submits it: the program reads the paste first.
const SUBMIT_DELAY: Duration = Duration::from_millis(60);

/// What the typing rules need of the target: its state and whether the caller runs inside it.
pub(super) fn target_facts(core: &Core, tab: i64, peer: &Peer) -> Option<SendTarget> {
    let facts = core.tab_facts(tab)?;
    // Where the caller runs, not what it says: a process can leave `from` out, but it can't move out from
    // under the shell it was started by, nor off its terminal. In part: a program that starts a session of
    // its own (`setsid`) leaves both behind.
    let caller_inside = facts.shell.is_some_and(|shell| peer.runs_in(shell));
    Some(SendTarget { tab, state: facts.state, caller_inside, asked: facts.asked })
}

/// `send-text`: type into the target's terminal as a paste, where the rules allow it.
pub(super) fn send_text(
    core: &Core,
    target: Target,
    text: &str,
    submit: bool,
    from: Option<i64>,
    access: Access,
    peer: &Peer,
) -> Reply {
    // The switch before anything about the terminal, as the rules order it.
    let enabled = access.write;
    if !enabled {
        return Reply::from(writes_off());
    }
    let Some(tab) = core.find(target, peer) else { return not_found(target) };
    let Some(facts) = target_facts(core, tab, peer) else { return not_found(target) };
    if let Err(error) = send_text_check(enabled, text, from, facts) {
        return Reply::from(error);
    }
    if let Err(reply) = core.budget(Limited::SendText, from) {
        return reply;
    }
    let text = text.trim_end_matches(['\n', '\r']).to_string();
    let Some(terminal) = core.terminal(tab) else {
        return Reply::error(ErrorCode::Failed, format!("tab {tab} could not be reached"));
    };
    terminal.paste(text.clone());
    terminal.control_typed();
    if submit {
        // A thread for the pause: the paste is read first, and the dispatch thread has others to answer.
        let spawned = std::thread::Builder::new().name("wormhole-hook-submit".into()).spawn(move || {
            std::thread::sleep(SUBMIT_DELAY);
            terminal.write(b"\r".to_vec());
        });
        if let Err(err) = spawned {
            tracing::warn!("couldn't press Return after a pasted text: {err}");
        }
    }
    tracing::info!(tab, ?from, submit, bytes = text.len(), "control: text sent to a terminal");
    Reply::ok(Reached { tab })
}

/// `send-keys`: press the keys and type the texts in the target's terminal, each encoded for the modes the
/// program has when it goes out.
pub(super) fn send_keys(
    core: &Core,
    spec: &SendKeysSpec,
    from: Option<i64>,
    access: Access,
    peer: &Peer,
) -> Reply {
    if let Some(off) = automation_off(core) {
        return off.into();
    }
    let enabled = access.write;
    if !enabled {
        return Reply::from(writes_off());
    }
    let Some(tab) = core.find(spec.target, peer) else { return not_found(spec.target) };
    let Some(facts) = target_facts(core, tab, peer) else { return not_found(spec.target) };
    let strokes = match send_keys_check(enabled, &spec.keys, from, facts) {
        Ok(strokes) => strokes,
        Err(error) => return Reply::from(error),
    };
    if let Err(reply) = core.budget(Limited::SendKeys, from) {
        return reply;
    }
    let Some(terminal) = core.terminal(tab) else {
        return Reply::error(ErrorCode::Failed, format!("tab {tab} could not be reached"));
    };
    let keys = terminal.send_strokes(&strokes);
    if keys > 0 {
        // The echo is the person's typing, as far as the status heuristics go.
        core.shared.feed_quiet(tab, StatusEvent::Typed);
        terminal.control_typed();
    }
    tracing::info!(tab, ?from, keys, "control: keys sent to a terminal");
    Reply::ok(KeysSent { tab, keys })
}

/// `mouse`: a click, press, release, move or wheel turn in the target's terminal, for a program that asked
/// for the mouse.
pub(super) fn mouse(core: &Core, spec: &MouseSpec, from: Option<i64>, access: Access, peer: &Peer) -> Reply {
    if let Some(off) = automation_off(core) {
        return off.into();
    }
    let enabled = access.write;
    if !enabled {
        return Reply::from(writes_off());
    }
    let Some(tab) = core.find(spec.target, peer) else { return not_found(spec.target) };
    let Some(facts) = target_facts(core, tab, peer) else { return not_found(spec.target) };
    if let Err(error) = mouse_check(enabled, &spec.op, from, facts) {
        return Reply::from(error);
    }
    if let Err(reply) = core.budget(Limited::Mouse, from) {
        return reply;
    }
    match core.terminal(tab).map(|terminal| terminal.send_mouse(&spec.op)) {
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
