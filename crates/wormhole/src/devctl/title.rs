//! The terminal's title row: what it draws, and its path menu.

use anyhow::Result;

use super::Ctx;

/// `title`: what the title row draws (see the module header).
pub(super) fn title(ctx: &mut Ctx, _: &str) -> Result<String> {
    Ok(ctx.workspace.devctl_title(ctx.window, ctx.cx))
}

/// `path-menu`: open the folder's menu as a click on the folder does (read it with `menu`).
pub(super) fn path_menu(ctx: &mut Ctx, _: &str) -> Result<String> {
    ctx.workspace.devctl_path_menu(ctx.window, ctx.cx);
    Ok(String::new())
}
