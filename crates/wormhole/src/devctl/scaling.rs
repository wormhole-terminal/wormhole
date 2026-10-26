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

/// `reveal-test`: what opening a terminal window with the system's animation does, on this window and as far
/// as a test instance may: after this update its first frame is asked for by hand (`Reveal::paint`, which
/// needs the app and the window free), and the window goes behind the others as background mode keeps it, not
/// in front (the instance never takes the focus). Says so in the log when it is through.
pub(super) fn reveal_test(ctx: &mut Ctx, _: &str) -> Result<String> {
    let handle = ctx.window.window_handle();
    ctx.cx
        .spawn(async move |_, cx| {
            let ready = cx.update(|cx| {
                handle.update(cx, |_, window, _| crate::platform::window::Reveal::of(window)).ok().flatten()
            });
            let Some(ready) = ready else {
                tracing::info!("reveal-test: the window has no reveal");
                return;
            };
            ready.paint();
            cx.update(|cx| {
                let _ = handle.update(cx, |_, window, _| crate::background::send_behind(window));
            });
            tracing::info!(
                "reveal-test: the first frame was asked for outside any update and the window is behind"
            );
        })
        .detach();
    Ok(String::new())
}
