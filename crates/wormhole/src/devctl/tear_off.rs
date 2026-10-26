//! Tearing a terminal off without a mouse (the Tear off feature): where the pointer is said to be, and a
//! release at a point of the screen. A drag with the mouse commands (`mouse-down`, `mouse-drag`, `mouse-up`)
//! reaches the same code, with the pointer where `pointer` put it.

use anyhow::{Result, anyhow};

use super::{Ctx, coordinates};

/// `pointer <x> <y>` | `pointer off`: say where the pointer is, in points from the primary screen's top
/// left (what a release outside the window reads), or let it be where it really is.
pub(super) fn pointer(ctx: &mut Ctx, arg: &str) -> Result<String> {
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
