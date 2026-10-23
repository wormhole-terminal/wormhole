//! What the window looks like and says: screenshots, text dumps, element bounds and frame timings.

use std::path::PathBuf;

use anyhow::Result;
use gpui_kit::*;

use super::Ctx;
use crate::window::Workspace;

pub(super) fn shot(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let name = super::file_name(arg, "shot")?;
    let path = ctx.dir.join(format!("{name}.png"));
    ctx.window.render_to_image().and_then(|img| Ok(img.save(&path)?))?;
    Ok(format!("saved {}\n", path.display()))
}

pub(super) fn screen(ctx: &mut Ctx, _: &str) -> Result<String> {
    Ok(ctx.workspace.active_terminal().read(ctx.cx).screen_text())
}

pub(super) fn state(ctx: &mut Ctx, _: &str) -> Result<String> {
    let focus = ctx.workspace.focus_detail(ctx.window, ctx.cx);
    // The Studio's sheets are native, so no shot shows them: its line says when one is up.
    let studio = crate::studio::devctl::state(ctx.cx).map(|line| format!("{line}\n")).unwrap_or_default();
    // The front terminal's find, while its bar is open: the query and the bar's count.
    let find = ctx.workspace.active_terminal().read(ctx.cx).find_summary();
    let find = find.map(|line| format!("find {line}\n")).unwrap_or_default();
    // The intro window, and the traffic lights it keeps away for as long as it is up. With the intro
    // closed the lights are named only when they aren't at rest, so one left away shows up here.
    let lights = crate::platform::window::opacity(ctx.window);
    let say =
        |(alpha, hidden): (f32, bool)| format!(" lights={alpha:.2}{}", if hidden { " hidden" } else { "" });
    let intro = match ctx.workspace.devctl_intro_state(ctx.cx) {
        Some(line) => format!("{line}{}\n", lights.map(say).unwrap_or_default()),
        None => lights
            .filter(|(alpha, hidden)| *hidden || *alpha < 1.0)
            .map(|lights| format!("intro closed{}\n", say(lights)))
            .unwrap_or_default(),
    };
    let welcome = ctx.workspace.devctl_welcome_state(crate::notifications::permission_asks(ctx.cx), ctx.cx);
    let language = wormhole_text::language().tag();
    Ok(format!(
        "{}focus {focus}\nlanguage {language}\n{find}{studio}{intro}{welcome}",
        ctx.workspace.debug_state(ctx.cx)
    ))
}

pub(super) fn studio_shot(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let name = super::file_name(arg, "studio")?;
    let path = ctx.dir.join(format!("{name}.png"));
    crate::studio::devctl::shot(&path, ctx.cx)?;
    Ok(format!("saved {}\n", path.display()))
}

/// The settings window is drawn later (it reads app state that this tick is changing), so
/// its result is appended to this window's results file when it is ready.
pub(super) fn settings_shot(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let name = super::file_name(arg, "settings")?;
    crate::settings_window::devctl::shot(ctx.dir.join(format!("{name}.png")), ctx.out.to_path_buf(), ctx.cx);
    Ok(String::new())
}

/// `bounds <x> <y> [levels]`: the element at a point and `levels - 1` of its parents, as the
/// inspector picks them, one line each: `bounds x=… y=… w=… h=… <file:line> <id path>`.
#[cfg(any(feature = "inspector", debug_assertions))]
pub(super) fn bounds(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let at = super::coordinates(arg)?;
    let levels = match arg.split_whitespace().nth(2) {
        Some(n) => {
            n.parse::<usize>().map_err(|_| anyhow::anyhow!("expected a number of levels, got {n:?}"))?
        }
        None => 1,
    };
    let levels = levels.clamp(1, 16);
    ctx.wait = Some(ctx.cx.spawn_in(ctx.window, async move |_, cx| {
        pick::measure(at, levels, cx).await.unwrap_or_else(|err| format!("bounds failed: {err:#}\n"))
    }));
    Ok(String::new())
}

/// Finding elements with the inspector, from outside its panel.
#[cfg(any(feature = "inspector", debug_assertions))]
mod pick {
    use std::fmt::Write as _;
    use std::time::Duration;

    use anyhow::Result;
    use gpui_kit::*;

    use crate::inspector::probe::{self, Picked};

    /// How wide GPUI makes the inspector's panel; the app gets the window minus that.
    const PANEL_REMS: f32 = 30.0;
    const POLL: Duration = Duration::from_millis(30);

    pub(super) async fn measure(
        at: Point<Pixels>,
        levels: usize,
        cx: &mut AsyncWindowContext,
    ) -> Result<String> {
        // One frame says whether the panel is open already (`inspect`): then it stays as it is.
        let (open, viewport, pointer, panel) = cx.update(|window, cx| {
            probe::take_drawn();
            window.draw(cx).clear(cx);
            let panel = rems(PANEL_REMS).to_pixels(window.rem_size());
            (probe::take_drawn(), window.viewport_size(), window.mouse_position(), panel)
        })?;
        // The panel takes its room from the app: the window grows by as much meanwhile, so
        // everything is measured where it is without the panel.
        let wide = size(viewport.width + panel, viewport.height);
        if !open {
            cx.update(|window, cx| {
                window.resize(wide);
                window.toggle_inspector(cx);
            })?;
            until(cx, |window| window.viewport_size() == wide).await?;
        }
        let narrowed = cx.update(|window, _| wide.width - window.viewport_size().width)?;

        probe::forget();
        cx.update(|window, cx| {
            // A frame first, so every element has the hitbox picking goes by.
            window.draw(cx).clear(cx);
            let event =
                MouseMoveEvent { position: at, pressed_button: None, modifiers: Modifiers::default() };
            window.dispatch_event(PlatformInput::MouseMove(event), cx);
        })?;
        let mut found: Vec<Picked> = Vec::new();
        while found.len() < levels {
            if !found.is_empty() {
                // Out to the parent, as the scroll wheel does in the inspector.
                cx.update(|window, cx| window.dispatch_event(step_out(at), cx))?;
            }
            match settled(cx, found.last()).await? {
                Some(picked) => found.push(picked),
                None => break,
            }
        }

        if !open {
            cx.update(|window, cx| {
                window.toggle_inspector(cx);
                window.resize(viewport);
            })?;
            until(cx, |window| window.viewport_size() == viewport).await?;
        }
        cx.update(|window, cx| {
            let event =
                MouseMoveEvent { position: pointer, pressed_button: None, modifiers: Modifiers::default() };
            window.dispatch_event(PlatformInput::MouseMove(event), cx);
            window.refresh();
        })?;

        let mut out = String::new();
        if found.is_empty() {
            writeln!(out, "bounds: nothing at {} {}", f32::from(at.x), f32::from(at.y)).ok();
        }
        if narrowed > px(0.5) {
            writeln!(out, "bounds: the window could not grow for the panel; measured {narrowed} narrower")
                .ok();
        }
        for picked in found {
            let b = picked.bounds;
            writeln!(
                out,
                "bounds x={} y={} w={} h={} {} {}",
                tenths(b.origin.x),
                tenths(b.origin.y),
                tenths(b.size.width),
                tenths(b.size.height),
                picked.location,
                picked.name
            )
            .ok();
        }
        Ok(out)
    }

    fn step_out(at: Point<Pixels>) -> PlatformInput {
        PlatformInput::ScrollWheel(ScrollWheelEvent {
            position: at,
            delta: ScrollDelta::Pixels(point(px(0.0), px(36.0))),
            modifiers: Modifiers::default(),
            touch_phase: TouchPhase::Moved,
        })
    }

    /// The element the panel settles on (past GPUI Kit's layers, the same two looks in a row)
    /// that isn't `previous`; `None` when there is no other one (the pick is as far out as it goes).
    async fn settled(cx: &mut AsyncWindowContext, previous: Option<&Picked>) -> Result<Option<Picked>> {
        let mut last = None;
        for _ in 0..40 {
            cx.update(|window, _| window.refresh())?;
            cx.background_executor().timer(POLL).await;
            let now = probe::picked().filter(|picked| !picked.layer && Some(picked) != previous);
            if now.is_some() && now == last {
                return Ok(now);
            }
            last = now;
        }
        Ok(None)
    }

    /// Wait (a second at most) for the window to get there.
    async fn until(cx: &mut AsyncWindowContext, done: impl Fn(&Window) -> bool) -> Result<()> {
        for _ in 0..40 {
            if cx.update(|window, _| done(window))? {
                break;
            }
            cx.background_executor().timer(Duration::from_millis(25)).await;
        }
        Ok(())
    }

    /// Rounded to a tenth of a pixel, which is all a layout check needs.
    fn tenths(value: Pixels) -> f32 {
        (f32::from(value) * 10.0).round() / 10.0
    }
}

/// How many terminal grids were painted since the last call, and how many a second.
pub(super) fn paints(_ctx: &mut Ctx, _arg: &str) -> Result<String> {
    use std::sync::Mutex;
    use std::sync::atomic::Ordering;
    static LAST: Mutex<Option<(std::time::Instant, u64)>> = Mutex::new(None);
    let now = std::time::Instant::now();
    let count = crate::terminal_view::PAINTS.load(Ordering::Relaxed);
    let mut last = LAST.lock().map_err(|_| anyhow::anyhow!("paints lock poisoned"))?;
    let (since, before) = last.unwrap_or((now, 0));
    *last = Some((now, count));
    let secs = now.duration_since(since).as_secs_f64();
    let fps = if secs > 0.0 { (count - before) as f64 / secs } else { 0.0 };
    Ok(format!("paints {} in {secs:.2}s ({fps:.1} per second)", count - before))
}

pub(super) fn frames(ctx: &mut Ctx, arg: &str) -> Result<String> {
    let frames = arg.trim().parse::<usize>().unwrap_or(40).max(1);
    bench_frames(frames, ctx.dir.join("frames.txt"), ctx.window, ctx.cx);
    Ok(String::new())
}

/// Redraw the workspace `frames` times, as the display would (the workspace is not marked dirty:
/// what the terminals' own output dirtied is drawn again), and append the draw times to `out`. Runs
/// outside the tick's entity update because drawing renders (and so borrows) the workspace.
fn bench_frames(frames: usize, out: PathBuf, window: &mut Window, cx: &mut Context<Workspace>) {
    cx.spawn_in(window, async move |this, cx| {
        let mut times = Vec::with_capacity(frames);
        for _ in 0..frames {
            if this.upgrade().is_none() {
                return;
            }
            let Ok(elapsed) = cx.update(|window, cx| {
                let start = std::time::Instant::now();
                let arena = window.draw(cx);
                arena.clear(cx);
                start.elapsed()
            }) else {
                return;
            };
            times.push(elapsed.as_secs_f64() * 1000.0);
            // Let the app breathe between frames, like the display link would.
            cx.background_executor().timer(std::time::Duration::from_millis(2)).await;
        }
        // The first frames warm caches (shaped text, icons); report the rest.
        let steady = &times[times.len().min(3)..];
        let avg = steady.iter().sum::<f64>() / steady.len().max(1) as f64;
        let max = steady.iter().cloned().fold(0.0, f64::max);
        let line =
            format!("frames n={} avg={avg:.2}ms max={max:.2}ms first={:.2}ms\n", times.len(), times[0]);
        if let Ok(mut file) = std::fs::OpenOptions::new().append(true).create(true).open(out) {
            use std::io::Write as _;
            let _ = file.write_all(line.as_bytes());
        }
    })
    .detach();
}
