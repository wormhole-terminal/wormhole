//! The terminal welcome's devctl commands: replay it with a hand-over, draw the ship where it docks,
//! and send the program's requests by hand to see them refused or answered.

use anyhow::{Result, anyhow};
use wormhole_core::ipc::control::{Action, Request, WelcomeOp};
use wormhole_core::onboarding::intro::{Handover, Start};

use super::Ctx;
use super::ambient::later;
use crate::workspace::control;

/// `welcome <cursor|portal|logo> [hidden]`: the window leaves into the wormhole and comes back with a
/// new terminal running the welcome, the ship (or not) landing as that hand-over has it. With `hidden`
/// the run begins as a first launch's does, over a window that was never shown (and the terminal opens
/// at once).
pub(super) fn welcome(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let mut words = arg.split_whitespace();
    let name = words.next().unwrap_or_default();
    let handover =
        Handover::parse(name).ok_or_else(|| anyhow!("expected cursor|portal|logo, got {name:?}"))?;
    let start = match words.next() {
        None => Start::Shown,
        Some("hidden") => Start::Hidden,
        Some(other) => return Err(anyhow!("expected `hidden` or nothing, got {other:?}")),
    };
    ctx.workspace.replay_welcome_from(handover, start, ctx.window, ctx.cx);
    Ok(String::new())
}

/// `welcome-dock [name]`: the ship at the cell (or chevron) the active terminal's welcome docks it on,
/// drawn over the window's own picture at four moments of the landing, to `<dir>/<name>-<n>.png`.
/// The window must be drawn with the welcome's page in it first (a `frames 2` before).
pub(super) fn dock(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let name = super::file_name(arg, "dock")?;
    let landing = ctx.workspace.active_terminal().read(ctx.cx).dock_landing().ok_or_else(|| {
        anyhow!(
            "the active terminal has no dock (a welcome terminal in the cursor or logo hand-over has one)"
        )
    })?;
    crate::intro::devctl::dock_frames(ctx.window, landing, ctx.dir, name, ctx.cx)
}

/// `welcome-probe <tab|-> <token> <op as JSON>`: a welcome request as the program would send it, from
/// that tab (`-`: from outside any), answered after this update; the reply's JSON line is appended.
pub(super) fn probe(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let mut parts = arg.trim().splitn(3, ' ');
    let (Some(from), Some(token), Some(op)) = (parts.next(), parts.next(), parts.next()) else {
        return Err(anyhow!("usage: welcome-probe <tab|-> <token> <op as JSON>"));
    };
    let op: WelcomeOp = serde_json::from_str(op).map_err(|err| anyhow!("not a welcome op: {err}"))?;
    let from = (from != "-").then(|| from.to_string());
    let request = Request::new(Action::Welcome { token: token.to_string(), op }, from);
    later(ctx, move |cx| {
        format!("{}\n", serde_json::to_string(&control::handle(request, cx)).unwrap_or_default())
    });
    Ok(String::new())
}
