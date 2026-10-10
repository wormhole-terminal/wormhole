//! The wormhole intro: a Metal effect over the whole screen around a window the first time Wormhole
//! opens, and again from About. The styles and their timing are `wormhole_core::intro`; the overlay window and the
//! shader are [`metal`]. One plays at a time: a new one replaces the one playing.

#[cfg(target_os = "macos")]
mod metal;

use std::cell::Cell;
use std::rc::Rc;
use std::time::Instant;

use gpui_kit::*;
use wormhole_core::intro::Start;
use wormhole_core::intro::Style;

use crate::workspace::AppState;

/// The style the intro window plays, first launch and replay alike: the one whose ship docks on its
/// logo.
pub(crate) const FIRST_LAUNCH: Style = Style::Voyage;

/// How an intro plays.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Options {
    /// Whether the window was showing as it began (a replay) or never was (a first launch).
    pub start: Start,
    /// Whether the traffic lights fade away as the run begins: a replay over a window that is showing
    /// them. They are the window's chrome, and the page is what the run is about. The run never brings
    /// them back; the intro window does, when it is closed.
    pub lights: bool,
}

/// How far the intro playing now has got, for what plays along with it (the intro window's page).
#[derive(Clone, Copy, Debug)]
pub(crate) struct Progress {
    pub style: Style,
    /// Seconds since it began.
    pub seconds: f32,
    /// Whether the window was showing as it began: a replay, which leaves, or a first launch.
    pub start: Start,
    /// It was cut short: the page is whole, and the window is settling.
    pub skipped: bool,
}

/// The intro playing now; dropping its task closes the overlay and puts the window back.
#[derive(Default)]
struct Playing(Option<Run>);

impl Global for Playing {}

struct Run {
    _task: Task<()>,
    style: Style,
    start: Start,
    started: Instant,
    /// Set from outside to cut the run short; the run settles instead of stopping dead.
    skip: Rc<Cell<bool>>,
    skipped: Rc<Cell<bool>>,
    done: Rc<Cell<bool>>,
    /// Where the page's logo is in the window: its middle and its side, points from the top left, for
    /// the ship that docks there.
    landing: Rc<Cell<Option<[f32; 3]>>>,
}

/// Where the intro playing now has got to; `None` when none is (or it has finished).
pub(crate) fn progress(cx: &App) -> Option<Progress> {
    let run = cx.try_global::<Playing>()?.0.as_ref()?;
    (!run.done.get()).then(|| Progress {
        style: run.style,
        seconds: run.started.elapsed().as_secs_f32(),
        start: run.start,
        skipped: run.skipped.get(),
    })
}

/// The page's logo is at `rect` (its middle's x and y in the window, and its side): where Voyage's ship
/// docks. The page says it every frame it draws, so a window that is resized is followed.
pub(crate) fn set_landing(cx: &App, rect: [f32; 3]) {
    if let Some(run) = cx.try_global::<Playing>().and_then(|playing| playing.0.as_ref()) {
        run.landing.set(Some(rect));
    }
}

/// Cut the intro playing now short: the window settles back and the overlay fades away.
pub(crate) fn skip(cx: &App) {
    if let Some(run) = cx.try_global::<Playing>().and_then(|playing| playing.0.as_ref()) {
        run.skip.set(true);
    }
}

/// Play `style` around `window` (the glow instead under Reduce Motion), over a window that is
/// showing.
pub(crate) fn play(window: &Window, style: Style, cx: &mut App) {
    play_with(window, style, Options::default(), cx);
}

/// [`play`] as `options` ask.
pub(crate) fn play_with(window: &Window, style: Style, options: Options, cx: &mut App) {
    let style = style.with_motion(cx.reduce_motion());
    // The glow moves nothing, so there is nothing for the lights to fade out for.
    let options = Options { lights: options.lights && style != Style::Glow, ..options };
    let run = start(window, style, options, colors(cx), cx);
    cx.default_global::<Playing>().0 = run;
}

/// Play `style` around the window in front (About's buttons, the keys): brought forward first, as
/// About may be in front of it.
pub(crate) fn play_in_front(style: Style, cx: &mut App) {
    cx.defer(move |cx| {
        AppState::with_front_workspace(cx, |_, window, cx| {
            crate::background::activate_window(window, cx);
            play(window, style, cx);
        });
    });
}

/// The theme's primary and info colours and its terminal magenta (FTL's violet), as the
/// shader's RGBA.
fn colors(cx: &App) -> [[f32; 4]; 3] {
    let store = cx.global::<crate::settings::SettingsStore>();
    [store.ui.primary, store.ui.info, store.palette.ansi[5]].map(|color| {
        let rgba = Rgba::from(color);
        [rgba.r, rgba.g, rgba.b, rgba.a]
    })
}

/// How long a skipped run takes to settle: the window back to rest and the overlay gone.
#[cfg(target_os = "macos")]
const SETTLE: f32 = 0.3;

#[cfg(target_os = "macos")]
fn start(
    window: &Window,
    style: Style,
    options: Options,
    colors: [[f32; 4]; 3],
    cx: &mut App,
) -> Option<Run> {
    use std::time::Duration;

    use wormhole_core::intro::page;

    /// About 60 frames a second: a whole screen of the shader each frame.
    const FRAME: Duration = Duration::from_millis(16);

    let view = ns_view(window)?;
    let landing = Rc::new(Cell::new(None));
    let overlay =
        match metal::Overlay::new(&view, style, colors, !crate::background::on(), options, landing.clone()) {
            Ok(overlay) => overlay,
            Err(err) => {
                tracing::warn!("the intro can't play: {err:#}");
                return None;
            }
        };
    let seconds = style.seconds();
    let started = Instant::now();
    let (skip, skipped, done) =
        (Rc::new(Cell::new(false)), Rc::new(Cell::new(false)), Rc::new(Cell::new(false)));
    let (flag, flagged, finished) = (skip.clone(), skipped.clone(), done.clone());
    let task = cx.spawn(async move |cx| {
        // Where a skip happened: when, how far the run had got, and how the window was.
        let mut cut: Option<(f32, f32, wormhole_core::intro::WindowMotion)> = None;
        loop {
            let time = started.elapsed().as_secs_f32();
            if flag.get() && cut.is_none() {
                let t = (time / seconds).min(1.0);
                cut = Some((time, t, overlay.motion_at(t)));
                flagged.set(true);
            }
            let (frame, over) = match cut {
                None => {
                    let t = (time / seconds).min(1.0);
                    let lights = Some(page::lights(time, options.start));
                    (metal::Frame { t, time, lights, settling: None }, time >= seconds)
                }
                Some((at, t, from)) => {
                    let since = time - at;
                    let settled = (since / SETTLE).clamp(0.0, 1.0);
                    // The lights are the intro window's now: a skip leaves them where they are.
                    let frame = metal::Frame { t, time, lights: None, settling: Some((from, settled)) };
                    (frame, since >= SETTLE)
                }
            };
            overlay.frame(&frame);
            if over {
                break;
            }
            cx.background_executor().timer(FRAME).await;
        }
        finished.set(true);
        drop(overlay);
    });
    Some(Run { _task: task, style, start: options.start, started, skip, skipped, done, landing })
}

#[cfg(not(target_os = "macos"))]
fn start(_: &Window, _: Style, _: Options, _: [[f32; 4]; 3], _: &mut App) -> Option<Run> {
    None
}

#[cfg(target_os = "macos")]
#[allow(unsafe_code)] // AppKit interop; see the SAFETY note below.
fn ns_view(window: &Window) -> Option<objc2::rc::Retained<objc2_app_kit::NSView>> {
    use objc2::MainThreadMarker;
    use objc2::rc::Retained;
    use objc2_app_kit::NSView;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};

    MainThreadMarker::new()?;
    let handle = HasWindowHandle::window_handle(window).ok()?;
    let RawWindowHandle::AppKit(handle) = handle.as_raw() else { return None };
    // SAFETY: GPUI hands out its live NSView and we are on the main thread.
    let view: &NSView = unsafe { handle.ns_view.cast::<NSView>().as_ref() };
    Some(Retained::from(view))
}

#[cfg(feature = "devctl")]
pub(crate) mod devctl {
    use std::path::Path;

    use anyhow::{Context as _, Result};
    use gpui_kit::{App, Window};
    use wormhole_core::intro::Style;

    /// `intro [style]`: play it around this window (`emerge`, `vortex`, `collapse`, `ftl`, `aurora`,
    /// `tentacles`, `voyage`, `glow`).
    pub(crate) fn play(window: &Window, arg: &str, cx: &mut App) -> Result<String> {
        let style = parse(arg)?;
        // devctl asks for exactly this style: no Reduce Motion swap, so `glow` can be seen too.
        let run = super::start(window, style, super::Options::default(), super::colors(cx), cx);
        cx.default_global::<super::Playing>().0 = run;
        Ok(String::new())
    }

    /// `intro-frames <style> [count]`: draw `count` frames of it off screen (default 8) over a
    /// stand-in screen and window, to `<dir>/intro-<style>-<n>.bmp`.
    #[cfg(target_os = "macos")]
    pub(crate) fn frames(arg: &str, dir: &Path, cx: &mut App) -> Result<String> {
        use super::metal::{Renderer, Uniforms};
        use wormhole_core::intro::window_motion;

        let mut args = arg.split_whitespace();
        let style = parse(args.next().unwrap_or_default())?;
        let count: usize = args.next().map(str::parse).transpose()?.unwrap_or(8).clamp(1, 60);
        let (w, h) = (1440, 900);
        let (width, height) = (860.0_f32, 540.0_f32);
        let origin = [290.0_f32, 180.0_f32];
        let ui = crate::settings::ui(cx);
        let rgb = |c: gpui_kit::Hsla| {
            let c = gpui_kit::Rgba::from(c);
            [c.r, c.g, c.b]
        };
        let (desk, win, edge) = (rgb(ui.panel_right), rgb(ui.background), rgb(ui.border_strong));
        let uniforms = Uniforms::new([w as f32, h as f32], origin, [width, height], 1.0, super::colors(cx));
        let centre = [origin[0] + width / 2.0, origin[1] + height / 2.0];
        let renderer = Renderer::new()?;
        let mut out = String::new();
        for n in 0..count {
            let t = (n as f32 + 0.5) / count as f32;
            let time = t * style.seconds();
            // The logo's spot on the stand-in window: its top, as the page has it.
            let landing =
                wormhole_core::intro::voyage::Landing { centre: [centre[0], origin[1] + 88.0], size: 96.0 };
            let (scene, trail) =
                uniforms.at(style, t, time).with_ship(style, time, [w as f32, h as f32], landing);
            let pixels = renderer.render_pixels(w, h, &scene, &trail)?;
            let motion = window_motion(style, t);
            let mut rgb_out = Vec::with_capacity(w * h * 3);
            for y in 0..h {
                for x in 0..w {
                    let (px, py) = (x as f32 - centre[0], y as f32 - centre[1]);
                    let (hw, hh) = (width / 2.0 * motion.scale, height / 2.0 * motion.scale);
                    let mut base = desk;
                    if px.abs() <= hw && py.abs() <= hh {
                        let on_edge = hw - px.abs() < 1.0 || hh - py.abs() < 1.0;
                        let fill = if on_edge { edge } else { win };
                        base = std::array::from_fn(|i| base[i] + (fill[i] - base[i]) * motion.alpha);
                    }
                    let i = (y * w + x) * 4;
                    let a = f32::from(pixels[i + 3]) / 255.0;
                    for c in 0..3 {
                        let lit = f32::from(pixels[i + c]) / 255.0 + base[c] * (1.0 - a);
                        rgb_out.push((lit.clamp(0.0, 1.0) * 255.0) as u8);
                    }
                }
            }
            let path = dir.join(format!("intro-{}-{n}.bmp", style.key()));
            std::fs::write(&path, bmp(w, h, &rgb_out)).with_context(|| path.display().to_string())?;
            out += &format!("saved {}\n", path.display());
        }
        Ok(out)
    }

    #[cfg(not(target_os = "macos"))]
    pub(crate) fn frames(_: &str, _: &Path, _: &mut App) -> Result<String> {
        anyhow::bail!("the intro draws with Metal: macOS only")
    }

    fn parse(arg: &str) -> Result<Style> {
        let arg = arg.trim();
        if arg.is_empty() {
            return Ok(super::FIRST_LAUNCH);
        }
        Style::parse(arg).with_context(|| {
            format!("no intro style {arg:?} (emerge, vortex, collapse, ftl, aurora, tentacles, voyage, glow)")
        })
    }

    /// A 24-bit BMP of `rgb` rows, top first: no image crate in the app for a debug picture.
    #[cfg_attr(not(target_os = "macos"), allow(dead_code))]
    fn bmp(width: usize, height: usize, rgb: &[u8]) -> Vec<u8> {
        let row = (width * 3).div_ceil(4) * 4;
        let size = 54 + row * height;
        let mut out = Vec::with_capacity(size);
        out.extend_from_slice(b"BM");
        out.extend_from_slice(&(size as u32).to_le_bytes());
        out.extend_from_slice(&[0; 4]);
        out.extend_from_slice(&54u32.to_le_bytes());
        out.extend_from_slice(&40u32.to_le_bytes());
        out.extend_from_slice(&(width as i32).to_le_bytes());
        // Negative: rows top first.
        out.extend_from_slice(&(-(height as i32)).to_le_bytes());
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&24u16.to_le_bytes());
        out.extend_from_slice(&[0; 24]);
        for y in 0..height {
            for x in 0..width {
                let i = (y * width + x) * 3;
                out.extend_from_slice(&[rgb[i + 2], rgb[i + 1], rgb[i]]);
            }
            out.resize(out.len() + row - width * 3, 0);
        }
        out
    }
}
