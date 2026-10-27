//! Windows and splits from the control line: `new-window`, `split`, `focus`, `rename` and `close`. The
//! rules (the switch, the caps, the budgets) are core's (`wormhole_core::ipc::control::structure`); this
//! finds the window and the terminal a request names and carries out what passes.
//!
//! Where it matters for the person at the Mac: nothing here brings a window forward or takes the keyboard
//! unless the request says so (`--activate`, `--select`), a window opens behind the others, and `close`
//! never closes anything, it asks.
//!
//! No verb here refuses the caller's own terminal. None types into a terminal or reads one; an agent that
//! splits its own terminal for a dev server, names itself or asks to close itself is using them for what they
//! are for, and a close is the person's to answer either way.

use gpui_kit::*;
use wormhole_core::ipc::control::structure::{launch_check, name_check, size_check};
use wormhole_core::ipc::control::{
    CloseSpec, ErrorCode, FocusSpec, Limited, NewWindowSpec, RenameSpec, Reply, SplitSpec, writes_off,
};

use super::{automation_off, budget, find, not_found};
use crate::app_state::AppState;
use crate::settings::settings;
use crate::window::SplitPlan;

/// The flag and the switch, in that order, for every verb of this module.
fn gate(cx: &App) -> Result<(), Reply> {
    if let Some(off) = automation_off(cx) {
        return Err(off.into());
    }
    if !settings(cx).control_write {
        return Err(writes_off().into());
    }
    Ok(())
}

fn unreachable_window() -> Reply {
    Reply::error(ErrorCode::Failed, "the window could not be reached")
}

/// `new-window`.
pub(super) fn new_window(spec: &NewWindowSpec, from: Option<i64>, cx: &mut App) -> Reply {
    if let Err(reply) = gate(cx) {
        return reply;
    }
    if let Err(error) = launch_check(spec.folder.as_deref(), spec.command.as_deref()) {
        return error.into();
    }
    if let Err(reply) = budget(Limited::NewWindow, from, cx) {
        return reply;
    }
    match crate::window::window_lifecycle::open_control_window(
        spec.folder.clone(),
        spec.command.as_deref(),
        spec.activate,
        cx,
    ) {
        Some(opened) => {
            tracing::info!(
                window = opened.window,
                terminal = opened.terminal,
                ?from,
                "control: a window was opened"
            );
            Reply::ok(opened)
        }
        None => Reply::error(ErrorCode::Failed, "the window could not be opened"),
    }
}

/// `split`.
pub(super) fn split(spec: &SplitSpec, from: Option<i64>, cx: &mut App) -> Reply {
    if let Err(reply) = gate(cx) {
        return reply;
    }
    if let Err(error) =
        size_check(spec.size).and(launch_check(spec.folder.as_deref(), spec.command.as_deref()))
    {
        return error.into();
    }
    let Some(found) = find(spec.target, cx) else { return not_found(spec.target) };
    // A split opens a terminal: it counts with `new-terminal`.
    if let Err(reply) = budget(Limited::NewTerminal, from, cx) {
        return reply;
    }
    let tab = found.tab;
    let made = AppState::with_workspace(&found.window, cx, |ws, window, cx| {
        let plan = SplitPlan {
            side: spec.direction,
            size: spec.size,
            folder: spec.folder.clone(),
            command: spec.command.as_deref(),
            select: spec.select,
        };
        ws.control_split(tab, plan, window, cx)
    });
    match made {
        Some(Ok(made)) => {
            tracing::info!(
                terminal = made.terminal,
                split = made.split,
                ?from,
                "control: a terminal was split off"
            );
            Reply::ok(made)
        }
        Some(Err(error)) => error.into(),
        None => unreachable_window(),
    }
}

/// `focus`.
pub(super) fn focus(spec: &FocusSpec, from: Option<i64>, cx: &mut App) -> Reply {
    if let Err(reply) = gate(cx) {
        return reply;
    }
    let Some(found) = find(spec.target, cx) else { return not_found(spec.target) };
    if let Err(reply) = budget(Limited::Focus, from, cx) {
        return reply;
    }
    let (tab, activate) = (found.tab, spec.activate);
    let focused = AppState::with_workspace(&found.window, cx, |ws, window, cx| {
        ws.control_focus(tab, activate, window, cx)
    });
    match focused {
        Some(Some(focused)) => Reply::ok(focused),
        _ => unreachable_window(),
    }
}

/// `rename`.
pub(super) fn rename(spec: &RenameSpec, from: Option<i64>, cx: &mut App) -> Reply {
    if let Err(reply) = gate(cx) {
        return reply;
    }
    let name = match name_check(spec.name.as_deref()) {
        Ok(name) => name,
        Err(error) => return error.into(),
    };
    let Some(found) = find(spec.target, cx) else { return not_found(spec.target) };
    if let Err(reply) = budget(Limited::Rename, from, cx) {
        return reply;
    }
    let tab = found.tab;
    let renamed = AppState::with_workspace(&found.window, cx, |ws, _, cx| ws.control_rename(tab, name, cx));
    match renamed {
        Some(Some(renamed)) => {
            tracing::info!(terminal = tab, ?from, "control: a terminal was renamed");
            Reply::ok(renamed)
        }
        _ => unreachable_window(),
    }
}

/// `close`: the question in the terminal's window, which the person answers.
pub(super) fn close(spec: &CloseSpec, from: Option<i64>, cx: &mut App) -> Reply {
    if let Err(reply) = gate(cx) {
        return reply;
    }
    let Some(found) = find(spec.target, cx) else { return not_found(spec.target) };
    if let Err(reply) = budget(Limited::Close, from, cx) {
        return reply;
    }
    let tab = found.tab;
    let asked =
        AppState::with_workspace(&found.window, cx, |ws, window, cx| ws.control_ask_close(tab, window, cx));
    match asked {
        Some(Ok(asked)) => {
            tracing::info!(terminal = tab, ?from, "control: the close question was asked");
            Reply::ok(asked)
        }
        Some(Err(error)) => error.into(),
        None => unreachable_window(),
    }
}
