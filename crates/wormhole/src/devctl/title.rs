//! The terminal's title row: what it draws, and its path menu.

use anyhow::Result;

use super::Ctx;

/// `title`: what the title row draws (see the module header).
pub(super) fn title(ctx: &mut Ctx, _: &str) -> Result<String> {
    Ok(ctx.workspace.devctl_title(ctx.window, ctx.cx))
}

/// `path-card <open|close>`: the path's hover card, at once.
pub(super) fn path_card(ctx: &mut Ctx, arg: &str) -> Result<String> {
    match arg.trim() {
        "open" => ctx.workspace.devctl_path_card(true, ctx.window, ctx.cx),
        "close" => ctx.workspace.devctl_path_card(false, ctx.window, ctx.cx),
        other => return Err(anyhow::anyhow!("path-card <open|close>, not {other:?}")),
    }
    Ok(String::new())
}

/// `path-menu`: open the folder's menu as a right-click on the folder does (read it with `menu`).
pub(super) fn path_menu(ctx: &mut Ctx, _: &str) -> Result<String> {
    ctx.workspace.devctl_path_menu(ctx.window, ctx.cx);
    Ok(String::new())
}
