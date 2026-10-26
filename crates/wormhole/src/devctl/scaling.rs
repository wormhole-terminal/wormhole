//! Scaling the window without a mouse (the Motion feature): what the window and its terminal hold for it, and
//! the end of a live resize, which no test instance can have (it needs the pointer on the window's edge).

use anyhow::Result;

use super::Ctx;

/// `scaling`: the window's resize steps and animation, the shown terminal's cell, grid and spare room.
pub(super) fn scaling(ctx: &mut Ctx, _: &str) -> Result<String> {
    Ok(ctx.workspace.devctl_scaling(ctx.window, ctx.cx))
}

/// `settle`: what happens when a live resize ends, now: the window gives up the strip, if any, that a window
/// off the cell grid leaves at its right and bottom edges.
pub(super) fn settle(ctx: &mut Ctx, _: &str) -> Result<String> {
    ctx.workspace.devctl_settle(ctx.window, ctx.cx);
    Ok(String::new())
}
