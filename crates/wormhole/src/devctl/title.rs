//! The terminal's title row: what it draws, and its path's hover card.

use anyhow::{Result, anyhow};

use super::Ctx;

/// `title`: what the title row draws (see the module header).
pub(super) fn title(ctx: &mut Ctx, _: &str) -> Result<String> {
    Ok(ctx.workspace.devctl_title(ctx.window, ctx.cx))
}

/// `path-card <open|close>`: the path's hover card, at once.
pub(super) fn path_card(ctx: &mut Ctx, arg: &str) -> Result<String> {
    match arg.trim() {
        "open" => ctx.workspace.devctl_path_card(true, ctx.cx),
        "close" => ctx.workspace.devctl_path_card(false, ctx.cx),
        other => return Err(anyhow!("path-card <open|close>, not {other:?}")),
    }
    Ok(String::new())
}
