//! The wormhole intro: an effect over the whole screen around a window the first time Wormhole
//! opens, and again from About. The styles and their timing are `wormhole_core::intro`; the overlay
//! window and the shader are the platform's (`platform::intro`). One plays at a time: a new one
//! replaces the one playing.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

use gpui_kit::*;
use wormhole_core::intro::Style;
use wormhole_core::intro::{Handover, Start, WindowMotion, page};

use crate::platform::intro::{Frame, Overlay, Setup};
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
    /// The run hands over to the terminal welcome, not the intro window's page (`None`): Portal has
    /// no ship, and for the others the terminal says where the ship docks (`set_landing`).
    pub handover: Option<Handover>,
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
/// docks. The page says it every frame it draws, so a window that is resized is followed. The terminal
/// welcome says it from the cell its program draws the mark in (`TerminalView`'s dock).
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
const SETTLE: f32 = 0.3;

/// Put the overlay up and drive it frame by frame; `None` where it can't play (no overlay on this
/// system or window, or it failed to start).
fn start(
    window: &Window,
    style: Style,
    options: Options,
    colors: [[f32; 4]; 3],
    cx: &mut App,
) -> Option<Run> {
    /// About 60 frames a second: a whole screen of the shader each frame.
    const FRAME: Duration = Duration::from_millis(16);

    let landing = Rc::new(Cell::new(None));
    let setup = Setup {
        style,
        colors,
        start: options.start,
        lights: options.lights,
        ship: options.handover.is_none_or(|handover| handover.ship()),
    };
    let overlay = match Overlay::new(window, setup, landing.clone()) {
        Ok(overlay) => overlay?,
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
        let mut cut: Option<(f32, f32, WindowMotion)> = None;
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
                    (Frame { t, time, lights, settling: None }, time >= seconds)
                }
                Some((at, t, from)) => {
                    let since = time - at;
                    let settled = (since / SETTLE).clamp(0.0, 1.0);
                    // The lights are the intro window's now: a skip leaves them where they are.
                    let frame = Frame { t, time, lights: None, settling: Some((from, settled)) };
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

#[cfg(feature = "devctl")]
pub(crate) mod devctl {
    use std::path::Path;

    use anyhow::{Context as _, Result};
    use gpui_kit::{App, Window};
    use wormhole_core::intro::Style;

    use crate::platform::intro::pictures;

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
    pub(crate) fn frames(arg: &str, dir: &Path, cx: &mut App) -> Result<String> {
        let mut args = arg.split_whitespace();
        let style = parse(args.next().unwrap_or_default())?;
        let count: usize = args.next().map(str::parse).transpose()?.unwrap_or(8).clamp(1, 60);
        let ui = crate::settings::ui(cx);
        let rgb = |c: gpui_kit::Hsla| {
            let c = gpui_kit::Rgba::from(c);
            [c.r, c.g, c.b]
        };
        let stand_in = [rgb(ui.panel_right), rgb(ui.background), rgb(ui.border_strong)];
        pictures::frames(style, count, super::colors(cx), stand_in, dir)
    }

    /// `welcome-dock`: Voyage's ship at `landing` (where a terminal's dock says, in window points),
    /// drawn over the window's own picture at four moments of the landing, to `<dir>/<name>-<n>.png`:
    /// the ship on its way in, docked, at the touchdown, and dissolving into what is under it.
    pub(crate) fn dock_frames(
        window: &Window,
        landing: wormhole_core::intro::voyage::Landing,
        dir: &Path,
        name: &str,
        cx: &mut App,
    ) -> Result<String> {
        pictures::dock_frames(window, landing, super::colors(cx), dir, name)
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
}
