//! Tearing a terminal off without a mouse (the Tear off feature): where the pointer is said to be, and a
//! release at a point of the screen. A drag with the mouse commands (`mouse-down`, `mouse-drag`, `mouse-up`)
//! reaches the same code, with the pointer where `pointer` put it.

use anyhow::{Result, anyhow};

use super::{Ctx, coordinates};

/// `pointer` | `pointer <x> <y>` | `pointer off`: where the pointer is; say where the pointer is, in points from the primary screen's top
/// left (what a release outside the window reads), or let it be where it really is.
pub(super) fn pointer(ctx: &mut Ctx, arg: &str) -> Result<String> {
    if arg.trim().is_empty() {
        return Ok(format!("{:?}\n", crate::platform::pointer::on_screen()));
    }
    if arg.trim() == "off" {
        crate::platform::pointer::pretend(None);
        return Ok(String::new());
    }
    let at = coordinates(arg)?;
    let pointer = crate::window::tear_off::pointer_at(at.x.into(), at.y.into(), ctx.cx)
        .ok_or_else(|| anyhow!("no primary display"))?;
    crate::platform::pointer::pretend(Some(pointer));
    Ok(String::new())
}

/// `tear-off <row> <x> <y>`: the terminal in that row is let go outside the window with the pointer at
/// that point of the primary screen: a window of its own opens there, or the Wormhole window under it takes
/// the terminal. Does nothing with the feature off.
pub(super) fn tear_off(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let (row, at) = arg.trim().split_once(' ').ok_or_else(|| anyhow!("expected <row> <x> <y>"))?;
    let at = coordinates(at)?;
    let id = ctx.workspace.devctl_session_id(super::sessions::row_arg(ctx, row)?);
    ctx.workspace.tear_off_at(id, at.x.into(), at.y.into(), ctx.window, ctx.cx);
    Ok(String::new())
}

/// `grow-hold <t>` | `grow-hold off`: every window growing from a torn-off terminal's chip stands at that
/// point of its growth (0.0 the chip, 1.0 the window) and stays, for a `shot` of it (the growth takes
/// 280 ms, a window's commands wait for its tick); `off` lets it grow on.
pub(super) fn grow_hold(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let at = match arg.trim() {
        "off" => None,
        t => Some(t.parse::<f32>().map_err(|_| anyhow!("expected a point of the growth (0 to 1) or off"))?),
    };
    crate::window::grow::hold(at.map(|t| t.clamp(0.0, 1.0)), ctx.cx);
    ctx.cx.notify();
    Ok(String::new())
}
