//! The marker (`agent-permissions.md`, 10): while a program that controls Wormhole acts on a terminal, the
//! terminal says so, with the notice chip that drops from its top edge ("Claude Code is typing"). Only who
//! and what, never the text. The chip is native, so it costs no frame, and a request posts it after the
//! handler has said yes, outside any draw.

use gpui_kit::*;
use wormhole_core::ipc::access::Resolved;
use wormhole_core::ipc::control::{Action, Target};
use wormhole_core::view::notice::{Driven, Driver, Notice};

use crate::app_state::{AppState, app};

/// What `action` does to which terminal, if it is something the chip tells: typing into one, having a
/// command run in it, or reading it (a read, or a wait on its text). `typing` says which.
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

/// Who `resolved` is, for the chip.
pub(super) fn driver_of(resolved: &Resolved) -> Driver {
    resolved.identity.agent.map_or(Driver::Scripts, Driver::Agent)
}

/// Say on `target` that `driver` is typing (or reading), if the notice chip is on for its window. Nothing
/// when the terminal is gone.
pub(crate) fn announce(target: Target, typing: bool, driver: Driver, cx: &mut App) {
    let driven = if typing { Driven::Typing(driver) } else { Driven::Reading(driver) };
    let window = app(cx).windows.iter().find_map(|window| {
        window
            .workspace
            .read_with(cx, |ws, _| ws.control_find(target))
            .ok()
            .flatten()
            .map(|(_, tab)| (window.clone(), tab))
    });
    let Some((window, tab)) = window else { return };
    AppState::with_workspace(&window, cx, |ws, _, cx| ws.control_driven(tab, Notice::Driven(driven), cx));
}
