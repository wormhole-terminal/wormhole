//! A window opened for a torn-off terminal growing from the chip it was let go as (core
//! `view::motion::tear_off`). The window opens at its final size, so its terminal is sized once and the
//! grid never reflows while it grows; what grows is the part of it that shows, a rectangle from the chip's
//! to the whole window, on a body the colour of the chip, with the content fading in over it. GPUI clips to
//! rectangles, so the corners are the body's alone and the content's are square, which the fade hides.
//!
//! Until it is over the window has no shadow (the system would outline the whole transparent frame), no
//! blur or fill behind it and no traffic lights; each comes back when the growth ends. The state is a
//! global keyed by the window's id, set before the window opens (`begin`), not a field of the workspace.
use std::collections::HashMap;
use std::time::Duration;

use gpui_kit::*;
use wormhole_core::view::motion::tear_off::{DURATION, Grow as Pose, pose};
use wormhole_core::view::panes::Rect;

use crate::platform;
use crate::settings::{UiColors, ui_in};
use crate::style::motion::elapsed;
use crate::style::{self, StatusGlyph, TextRole as _};
use crate::window::Workspace;

/// How long after the growth's clock a window waits before it goes back to its plain frame.
const GRACE: Duration = Duration::from_millis(60);

#[derive(Clone)]
struct Grow {
    chip: Rect,
    /// The chip's face: the terminal's name and glyph, which the window's content fades in over.
    face: (SharedString, StatusGlyph),
    /// The end has been scheduled (by the first frame).
    timed: bool,
}

#[derive(Default)]
struct Growing {
    windows: HashMap<i64, Grow>,
    /// devctl: every growing window stands at this point of its growth (0.0 to 1.0) and stays there.
    #[cfg(feature = "devctl")]
    hold: Option<f32>,
}

impl Global for Growing {}

/// Window `id`, about to open, grows from `chip` (in its own space), whose face is `face`.
pub(super) fn begin(id: i64, chip: Rect, face: (SharedString, StatusGlyph), cx: &mut App) {
    cx.default_global::<Growing>().windows.insert(id, Grow { chip, face, timed: false });
}

/// Window `id` is gone or its window never opened: nothing is left of its growth.
pub(super) fn forget(id: i64, cx: &mut App) {
    if let Some(growing) = cx.try_global::<Growing>()
        && growing.windows.contains_key(&id)
    {
        cx.global_mut::<Growing>().windows.remove(&id);
    }
}

/// Whether window `id` is growing, or about to.
pub(super) fn active(id: i64, cx: &App) -> bool {
    cx.try_global::<Growing>().is_some_and(|growing| growing.windows.contains_key(&id))
}

impl Workspace {
    /// The window has just been made: a window that grows hides what would show its full frame.
    pub(super) fn prepare_grow(&self, window: &Window, cx: &mut Context<Self>) {
        if active(self.window_id, cx) {
            platform::window::set_shadow(window, false);
            platform::window::set_shown(window, false);
            // Closed before it grew: nothing to wait for.
            let id = self.window_id;
            cx.on_release(move |_, cx| forget(id, cx)).detach();
        }
    }

    /// The window's frame: `root` as it is, or while the window grows, in the part of it that shows.
    pub(super) fn framed(&mut self, root: Div, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let id = self.window_id;
        // Read only until there is something to change: a global that is touched in a frame tells its
        // observers, and every frame of every window comes through here.
        let Some(growing) = cx.try_global::<Growing>() else { return root.into_any_element() };
        #[cfg(feature = "devctl")]
        let hold = growing.hold;
        let Some(grow) = growing.windows.get(&id).cloned() else {
            return root.into_any_element();
        };
        let (chip, face) = (grow.chip, grow.face.clone());
        #[cfg(feature = "devctl")]
        let held = hold.is_some();
        #[cfg(not(feature = "devctl"))]
        let held = false;
        if !held && !grow.timed {
            if let Some(grow) = cx.global_mut::<Growing>().windows.get_mut(&id) {
                grow.timed = true;
            }
            cx.spawn_in(window, async move |this, cx| {
                cx.background_executor().timer(DURATION + GRACE).await;
                this.update_in(cx, |this, window, cx| this.finish_grow(window, cx)).ok();
            })
            .detach();
        }
        let ui = ui_in(&self.profile, cx);
        let view = window.viewport_size();
        let (w, h) = (f32::from(view.width), f32::from(view.height));
        let radius = f32::from(ui.radius_lg);
        let at = move |t: f32| pose(chip, w, h, radius, elapsed(DURATION, t));
        let whole = div().absolute().w(px(w)).h(px(h)).child(root);
        let body = div().absolute().overflow_hidden();
        // What the chip showed, at the body's corner: where the eye was, until the window is there.
        let glyph = style::status_glyph(("grow-glyph", id as u64), ui, face.1, cx);
        let face = div()
            .absolute()
            .top_0()
            .left_0()
            .flex()
            .items_center()
            .gap_2()
            .px_2()
            .py_1()
            .text_body()
            .text_color(ui.text)
            .child(glyph)
            .child(div().min_w_0().truncate().child(face.0));
        #[cfg(feature = "devctl")]
        if let Some(t) = hold {
            let g = at(t);
            let body = clip(body.child(inside(whole, g)).child(face.opacity(g.face)), g, ui);
            return div().relative().size_full().child(body).into_any_element();
        }
        let animation = Animation::new(DURATION);
        let whole = whole
            .with_animation(("grow-content", id as u64), animation.clone(), move |d, t| inside(d, at(t)));
        let face = face
            .with_animation(("grow-face", id as u64), animation.clone(), move |d, t| d.opacity(at(t).face));
        let body =
            body.child(whole)
                .child(face)
                .with_animation(("grow", id as u64), animation, move |d, t| clip(d, at(t), ui));
        div().relative().size_full().child(body).into_any_element()
    }

    /// The growth is over: the window is whole again, with its shadow, its lights and its blur.
    fn finish_grow(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        forget(self.window_id, cx);
        platform::window::set_shadow(window, true);
        platform::window::set_shown(window, true);
        self.sync_appearance(cx);
        cx.notify();
    }
}

/// The window held where it is in the growth `g`: moved back by the clip's corner, so it stays where it is in
/// the window, and as much of it as has faded in.
fn inside(d: Div, g: Pose) -> Div {
    d.left(px(-g.shown.x)).top(px(-g.shown.y)).opacity(g.content)
}

/// The clip at `g`: the rectangle that shows, its corners, and the chip's own colour, which is gone once the
/// content is all there.
fn clip(d: Div, g: Pose, ui: UiColors) -> Div {
    d.left(px(g.shown.x))
        .top(px(g.shown.y))
        .w(px(g.shown.w))
        .h(px(g.shown.h))
        .rounded(px(g.radius))
        .bg(ui.elevated.opacity(1.0 - g.content))
}

/// devctl's `grow-hold`: hold every growing window at `at` of its growth (`None`: let it grow).
#[cfg(feature = "devctl")]
pub(crate) fn hold(at: Option<f32>, cx: &mut App) {
    cx.default_global::<Growing>().hold = at;
}

/// Whether the global exists (a test: a plain frame must not make it).
#[cfg(test)]
pub(super) fn exists(cx: &App) -> bool {
    cx.try_global::<Growing>().is_some()
}
