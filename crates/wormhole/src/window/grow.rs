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
use wormhole_core::view::motion::tear_off::{DURATION, pose};
use wormhole_core::view::panes::Rect;

use crate::platform;
use crate::settings::ui_in;
use crate::style::motion::elapsed;
use crate::window::Workspace;

/// How long after the growth's clock a window waits before it goes back to its plain frame.
const GRACE: Duration = Duration::from_millis(60);

struct Grow {
    chip: Rect,
    /// The end has been scheduled (by the first frame).
    timed: bool,
}

#[derive(Default)]
struct Growing(HashMap<i64, Grow>);

impl Global for Growing {}

/// Window `id`, about to open, grows from `chip` (in its own space).
pub(super) fn begin(id: i64, chip: Rect, cx: &mut App) {
    cx.default_global::<Growing>().0.insert(id, Grow { chip, timed: false });
}

/// Whether window `id` is growing, or about to.
pub(super) fn active(id: i64, cx: &App) -> bool {
    cx.try_global::<Growing>().is_some_and(|growing| growing.0.contains_key(&id))
}

impl Workspace {
    /// The window has just been made: a window that grows hides what would show its full frame.
    pub(super) fn prepare_grow(&self, window: &Window, cx: &App) {
        if active(self.window_id, cx) {
            platform::window::set_shadow(window, false);
            platform::window::set_shown(window, false);
        }
    }

    /// The window's frame: `root` as it is, or while the window grows, in the part of it that shows.
    pub(super) fn framed(&mut self, root: Div, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let id = self.window_id;
        let Some(grow) = cx.default_global::<Growing>().0.get_mut(&id) else {
            return root.into_any_element();
        };
        let chip = grow.chip;
        if !std::mem::replace(&mut grow.timed, true) {
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
        let animation = Animation::new(DURATION);
        // The window at its full size, held where it is in the window: the clip moves over it.
        let whole = div().absolute().w(px(w)).h(px(h)).child(root).with_animation(
            ("grow-content", id as u64),
            animation.clone(),
            move |d, t| {
                let g = at(t);
                d.left(px(-g.shown.x)).top(px(-g.shown.y)).opacity(g.content)
            },
        );
        let body = div().absolute().overflow_hidden().child(whole).with_animation(
            ("grow", id as u64),
            animation,
            move |d, t| {
                let g = at(t);
                d.left(px(g.shown.x))
                    .top(px(g.shown.y))
                    .w(px(g.shown.w))
                    .h(px(g.shown.h))
                    .rounded(px(g.radius))
                    // The chip's own colour, gone once the content is all there.
                    .bg(ui.elevated.opacity(1.0 - g.content))
            },
        );
        div().relative().size_full().child(body).into_any_element()
    }

    /// The growth is over: the window is whole again, with its shadow, its lights and its blur.
    fn finish_grow(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.default_global::<Growing>().0.remove(&self.window_id);
        platform::window::set_shadow(window, true);
        platform::window::set_shown(window, true);
        self.sync_appearance(cx);
        cx.notify();
    }
}
