//! Changing the window from outside it: selecting a terminal, naming one, asking to close one, and the
//! window verb this app does not have. The rules (the switch, the caps, the budgets) are core's
//! (`wormhole_core::ipc::control::structure`).
//!
//! Nothing here takes the keyboard from the person unless it was asked to (`focus --activate`), and `close`
//! never closes anything: it asks, and the person answers.

use std::sync::Arc;
use std::sync::atomic::Ordering;

use wormhole_core::ipc::Peer;
use wormhole_core::ipc::access::Access;
use wormhole_core::ipc::control::event::{Body, Event, Topic};
use wormhole_core::ipc::control::structure::{launch_check, name_check};
use wormhole_core::ipc::control::{
    CloseAsked, CloseSpec, Error, ErrorCode, FocusSpec, Focused, Limited, NewWindowSpec, Reason, RenameSpec,
    Renamed, Reply, Target, WindowOpened, writes_off,
};
use wormhole_core::workspace::Action;

use super::{automation_off, not_found};
use crate::hook_server::core::Core;
use crate::hook_server::events::HookEvent;

/// The flag and the switch, in that order, for every verb of this module.
fn gate(core: &Core, access: Access) -> Result<(), Reply> {
    if let Some(off) = automation_off(core) {
        return Err(off.into());
    }
    if !access.write {
        return Err(writes_off().into());
    }
    Ok(())
}

fn found(core: &Core, target: Target, peer: &Peer) -> Result<i64, Reply> {
    core.find(target, peer).ok_or_else(|| not_found(target))
}

/// `new-window`: a window like the first one (its panels, no groups) with one new terminal in `folder` (the
/// selected terminal's folder without), running `command`. The ids are planned now and the window is opened by
/// Swift (`HookEvent::NewWindow`); behind the other windows unless `activate`.
pub(super) fn new_window(core: &Arc<Core>, spec: &NewWindowSpec, from: Option<i64>, access: Access) -> Reply {
    if let Err(reply) = gate(core, access) {
        return reply;
    }
    if let Err(error) = launch_check(spec.folder.as_deref(), spec.command.as_deref()) {
        return error.into();
    }
    if let Err(reply) = core.budget(Limited::NewWindow, from) {
        return reply;
    }
    let Some((windows, _)) = core.shared.windows() else {
        return Reply::error(
            ErrorCode::Unsupported,
            "this workspace is not one of several windows: use new-terminal",
        );
    };
    let planned = windows.plan_like(Some(&core.shared), spec.folder.clone());
    let folder = planned.folder.clone();
    core.ui.push(HookEvent::NewWindow {
        window: planned.window,
        terminal: planned.terminal,
        folder: folder.clone(),
        activate: spec.activate,
    });
    if let Some(command) = spec.command.clone() {
        super::open::run_first(core, planned.terminal, command);
    }
    tracing::info!(
        window = planned.window,
        terminal = planned.terminal,
        ?from,
        "control: a window was opened"
    );
    Reply::ok(WindowOpened { window: planned.window, terminal: planned.terminal, folder })
}

/// `focus`: select the terminal's row, which shows its split. `activate` also brings the app forward.
pub(super) fn focus(core: &Core, spec: &FocusSpec, from: Option<i64>, access: Access, peer: &Peer) -> Reply {
    if let Err(reply) = gate(core, access) {
        return reply;
    }
    let tab = match found(core, spec.target, peer) {
        Ok(tab) => tab,
        Err(reply) => return reply,
    };
    if let Err(reply) = core.budget(Limited::Focus, from) {
        return reply;
    }
    let Some(owner) = core.owner_of(tab) else { return not_found(spec.target) };
    let events = owner.with_model(|model| model.apply(Action::Select(tab)));
    owner.post(events);
    if spec.activate {
        core.ui.push(HookEvent::Activate);
        core.ui.push(HookEvent::Front { terminal: tab });
    }
    tracing::info!(terminal = tab, ?from, activate = spec.activate, "control: a terminal was focused");
    Reply::ok(Focused { window: owner.window_id(), terminal: tab })
}

/// `rename`: name the terminal (the sidebar's rename), or with no name call it after what runs in it again.
pub(super) fn rename(
    core: &Core,
    spec: &RenameSpec,
    from: Option<i64>,
    access: Access,
    peer: &Peer,
) -> Reply {
    if let Err(reply) = gate(core, access) {
        return reply;
    }
    let name = match name_check(spec.name.as_deref()) {
        Ok(name) => name,
        Err(error) => return error.into(),
    };
    let tab = match found(core, spec.target, peer) {
        Ok(tab) => tab,
        Err(reply) => return reply,
    };
    if let Err(reply) = core.budget(Limited::Rename, from) {
        return reply;
    }
    // A blank name goes back to what runs in the terminal.
    let Some(owner) = core.owner_of(tab) else { return not_found(spec.target) };
    let events = owner
        .with_model(|model| model.apply(Action::Rename { id: tab, name: name.clone().unwrap_or_default() }));
    owner.post(events);
    let shown = core
        .listing()
        .windows
        .iter()
        .flat_map(|window| &window.terminals)
        .find(|terminal| terminal.id == tab)
        .map(|terminal| terminal.name.clone());
    let Some(shown) = shown else { return not_found(spec.target) };
    let (window, terminal) = (owner.window_id(), Some(tab));
    core.bus.publish(Topic::Terminals, || Event {
        window: Some(window),
        terminal,
        body: Body::Renamed { name: name.clone() },
    });
    tracing::info!(terminal = tab, ?from, "control: a terminal was renamed");
    Reply::ok(Renamed { terminal: tab, name: shown, custom: name.is_some() })
}

/// `close`: the question before closing the terminal, which the person answers (`HookEvent::CloseAsked`).
pub(super) fn close(core: &Core, spec: &CloseSpec, from: Option<i64>, access: Access, peer: &Peer) -> Reply {
    if let Err(reply) = gate(core, access) {
        return reply;
    }
    let tab = match found(core, spec.target, peer) {
        Ok(tab) => tab,
        Err(reply) => return reply,
    };
    // A question already up is a refusal that costs nothing: said before the budget is spent.
    if core.close_question.load(Ordering::Acquire) {
        return Error::new(ErrorCode::Refused, "a question is already up: it has to be answered first")
            .because(Reason::QuestionOpen)
            .into();
    }
    if let Err(reply) = core.budget(Limited::Close, from) {
        return reply;
    }
    let window = core.owner_of(tab).map_or(core.window, |owner| owner.window_id());
    core.close_question.store(true, Ordering::Release);
    core.ui.push(HookEvent::CloseAsked { terminal: tab });
    tracing::info!(terminal = tab, ?from, "control: the close question was asked");
    Reply::ok(CloseAsked { asked: true, window, terminal: tab })
}
