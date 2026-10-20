//! The Theme Studio window.

use anyhow::Result;

use super::Ctx;

pub(super) fn toggle(ctx: &mut Ctx, _: &str) -> Result<String> {
    ctx.workspace.toggle_studio(ctx.window, ctx.cx);
    Ok(String::new())
}

/// One of `studio-select <token>`, `studio-scroll <y>`, `studio-save <name>`,
/// `studio-set <value>`, `studio-type <text>`, `studio-filter <show> [text]`, `studio-area <area>`,
/// `studio-resize <w> <h>` and `studio-menu [entry]`, run on the open Studio.
pub(super) fn with(ctx: &mut Ctx, command: &str, arg: &str) -> Result<String> {
    let arg = arg.trim().to_string();
    let command = command.to_string();
    ctx.workspace.with_studio(ctx.cx, |studio, window, cx| match command.as_str() {
        "studio-select" => studio.devctl_select(&arg, window, cx),
        "studio-type" => studio.devctl_type(&arg, window, cx),
        "studio-scroll" => studio.devctl_scroll(arg.parse().unwrap_or(0.0), cx),
        "studio-save" => studio.devctl_save(&arg, window, cx),
        "studio-filter" => studio.devctl_filter(&arg, window, cx),
        "studio-area" => studio.devctl_area(&arg, cx),
        "studio-resize" => studio.devctl_resize(&arg, window),
        "studio-menu" => studio.devctl_menu(&arg, cx),
        _ => studio.devctl_set(&arg, window, cx),
    })?;
    Ok(String::new())
}

/// `studio-frames <n> [scroll]`: draw the Studio window n times and append the timings to
/// `<dir>/frames.txt`. With `scroll`, each draw follows a wheel tick over the gallery.
pub(super) fn frames(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let mut words = arg.split_whitespace();
    let frames = words.next().and_then(|n| n.parse::<usize>().ok()).unwrap_or(40).max(1);
    let scrolling = words.next() == Some("scroll");
    ctx.workspace.devctl_studio_frames(frames, scrolling, ctx.dir.join("frames.txt"), ctx.cx)?;
    Ok(String::new())
}
