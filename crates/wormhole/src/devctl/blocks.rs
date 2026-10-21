//! The active terminal's command blocks: what the shell's integration marked, and the
//! Edit menu's actions on one of them; and its inline images.

use anyhow::{Result, anyhow};

use super::Ctx;

/// `blocks`: one line per block, as `TerminalView::devctl_blocks` writes them.
pub(super) fn blocks(ctx: &mut Ctx, _: &str) -> Result<String> {
    Ok(ctx.workspace.active_terminal().read(ctx.cx).devctl_blocks())
}

/// `images`: the active terminal's image store, as `TerminalView::devctl_images` writes it.
pub(super) fn images(ctx: &mut Ctx, _: &str) -> Result<String> {
    Ok(ctx.workspace.active_terminal().read(ctx.cx).devctl_images())
}

/// `block <n> <select|copy-output|copy-command|rerun>`: the action on the n-th block.
pub(super) fn block(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let mut words = arg.split_whitespace();
    let n = words.next().and_then(|n| n.parse::<usize>().ok()).filter(|n| *n > 0);
    let (Some(n), Some(action)) = (n, words.next()) else {
        return Err(anyhow!("expected <n> <select|copy-output|copy-command|rerun>"));
    };
    let terminal = ctx.workspace.active_terminal();
    terminal.update(ctx.cx, |terminal, cx| terminal.devctl_block(n - 1, action, cx))?;
    Ok(String::new())
}

/// `pane-bar`: what the bar over the active terminal says (a question, a notice, or none).
pub(super) fn pane_bar(ctx: &mut Ctx, _: &str) -> Result<String> {
    Ok(ctx.workspace.active_terminal().read(ctx.cx).devctl_pane_bar())
}

/// `clipboard-answer <once|always|deny>`: the bar's buttons.
pub(super) fn clipboard_answer(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let terminal = ctx.workspace.active_terminal();
    terminal.update(ctx.cx, |terminal, cx| terminal.devctl_clipboard_answer(arg.trim(), cx))?;
    Ok(String::new())
}
