//! The batch itself: waiting until the window has settled, and telling a script it is done.

use std::hash::{Hash as _, Hasher as _};
use std::time::Duration;

use anyhow::{Result, anyhow};
use gpui_kit::*;

use super::Ctx;
use crate::window::Workspace;

/// How long `wait-idle` waits by default.
const IDLE_TIMEOUT: Duration = Duration::from_millis(5000);
/// How often it looks again while git work is in flight.
const GIT_POLL: Duration = Duration::from_millis(25);
/// How far apart the frames it compares are drawn.
const FRAME_GAP: Duration = Duration::from_millis(100);
/// How many frames in a row after the first must look the same.
const STEADY_FRAMES: usize = 2;

pub(super) fn sync(ctx: &mut Ctx, arg: &str) -> Result<String> {
    ctx.sync.push(arg.trim().to_string());
    Ok(String::new())
}

pub(super) fn wait_idle(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let timeout = match arg.trim() {
        "" => IDLE_TIMEOUT,
        ms => Duration::from_millis(ms.parse().map_err(|_| anyhow!("expected a timeout in ms, got {ms:?}"))?),
    };
    ctx.wait = Some(ctx.cx.spawn_in(ctx.window, async move |this, cx| idle(this, cx, timeout).await));
    Ok(String::new())
}

/// Wait until no git status or history refresh of this window is in flight, then until every
/// window draws the same picture `STEADY_FRAMES + 1` times in a row. The second part catches
/// what has no flag to look at: a diff being read and built, lists measuring their rows, images
/// loading, a refresh that started while waiting.
async fn idle(this: WeakEntity<Workspace>, cx: &mut AsyncWindowContext, timeout: Duration) -> String {
    let start = cx.background_executor().now();
    let mut last: Option<Vec<Option<u64>>> = None;
    let mut steady = 0;
    loop {
        let Ok(busy) = this.update(cx, |workspace, cx| workspace.devctl_git_busy(cx)) else {
            return "wait-idle failed: the window closed\n".into();
        };
        if busy {
            (last, steady) = (None, 0);
        } else {
            let Ok(frame) = cx.update(fingerprints) else {
                return "wait-idle failed: the window closed\n".into();
            };
            if last.as_ref() == Some(&frame) {
                steady += 1;
                if steady >= STEADY_FRAMES {
                    return "idle\n".into();
                }
            } else {
                steady = 0;
            }
            last = Some(frame);
        }
        if cx.background_executor().now() - start >= timeout {
            let why = if busy { "git still busy" } else { "the windows still change" };
            return format!("timeout: {why}\n");
        }
        cx.background_executor().timer(if busy { GIT_POLL } else { FRAME_GAP }).await;
    }
}

/// Draw every window and fingerprint what it shows, this one first. `None` for a window that
/// can't be captured (the test platform has no renderer): it counts as unchanged.
fn fingerprints(window: &mut Window, cx: &mut App) -> Vec<Option<u64>> {
    let own = window.window_handle();
    let mut prints = vec![fingerprint(window, cx)];
    for handle in cx.windows() {
        if handle != own {
            prints.push(handle.update(cx, |_, window, cx| fingerprint(window, cx)).ok().flatten());
        }
    }
    prints
}

fn fingerprint(window: &mut Window, cx: &mut App) -> Option<u64> {
    window.draw(cx).clear(cx);
    let image = window.render_to_image().ok()?;
    let mut hasher = std::hash::DefaultHasher::new();
    (image.width(), image.height()).hash(&mut hasher);
    image.as_raw().hash(&mut hasher);
    Some(hasher.finish())
}
