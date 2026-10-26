//! What the window does with a terminal that was let go of outside it (the Tear off feature; the sidebar
//! follows the drag out, `sidebar::tear_off`): over empty desktop it opens a window of its own there and
//! the terminal goes into it, growing from where its chip was let go; over another Wormhole window it goes
//! into that window's list; anywhere else nothing happens.
//!
//! AppKit sends a drag's mouse events to the window it began in only, so what lies under the pointer is
//! worked out from where the app's windows are, front to back (`wormhole_core::view::tear_off`). A terminal
//! moves as a whole, running (`terminal_move`); it leaves its split behind, the others of which stay. The
//! last terminal of a window is never torn off into a window of its own, which would only move the window
//! (and leave this one with a new, empty terminal); it can still go into another window, and this one
//! then starts a new terminal, as when its last one is closed.
use gpui_kit::*;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::storage::store::WindowRecord;
use wormhole_core::view::panes::Rect;
use wormhole_core::view::sidebar::WindowSidebar;
use wormhole_core::view::tear_off::{Landing, Occupant, Placed, Screen, Spot, landing, place, to_global};

use crate::app_state::{AppState, OpenWindow, app, app_mut};
use crate::platform;
use crate::platform::pointer::Pointer;
use crate::settings::settings_in;
use crate::sidebar::terminal_drag::pane_glyph;
use crate::style::{GHOST_POINTER_GAP, GHOST_POINTER_RISE};
use crate::window::terminal_move::{Departing, open_for};
use crate::window::{Layout, Workspace, window_bounds};

/// Where the pointer lies in a torn-off window: over the new terminal's row, which was under it a moment
/// ago (the sidebar's width and the title row's height, about).
const ANCHOR: (f32, f32) = (120.0, 90.0);
/// The chip the window grows from when the drag did not measure its own (a script's release): its width, at its
/// narrowest, and height.
const CHIP: (f32, f32) = (160.0, 28.0);

/// A terminal let go outside its window.
#[derive(Clone, Copy)]
pub(super) struct Release {
    pub(super) id: i64,
    pub(super) pointer: Pointer,
    /// The size its chip was drawn at, if it was.
    pub(super) chip: Option<(f32, f32)>,
}

impl Workspace {
    /// A terminal picked up by its row was let go outside the window with the pointer at `pointer`: what
    /// lies there says what happens (see the module). Nothing happens with the feature off or in a
    /// window that is full screen (it has no outside: another Space lies there).
    pub(super) fn terminal_released_outside(
        &mut self,
        release: Release,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.terminal_drag = None;
        cx.notify();
        if !settings_in(&self.profile, cx).feature_enabled(FeatureFlag::TearOff) || window.is_fullscreen() {
            return;
        }
        let source = self.window_id;
        // Deferred: the other windows are read, and the window the terminal goes into updated, none of
        // which may happen while this one is being updated.
        cx.defer(move |cx| land(source, release, cx));
    }

    /// Terminal `id` goes to a window of its own, which opens at `spot` of its display (the pointer where it
    /// was let go). A terminal alone in its window stays.
    fn tear_off_to_new_window(&mut self, release: Release, window: &mut Window, cx: &mut Context<Self>) {
        let Release { id, pointer, chip: chip_size } = release;
        let spot = pointer.spot;
        let Some(ix) = self.find_session(id) else { return };
        if self.sessions.len() < 2 {
            return;
        }
        let display = DisplayId::new(spot.display);
        let Some(visible) = cx.displays().into_iter().find(|d| d.id() == display).map(|d| d.visible_bounds())
        else {
            return;
        };
        let wide = window.bounds().size;
        let (chip_w, chip_h) = chip_size.unwrap_or(CHIP);
        let chip = Rect::new(spot.x + GHOST_POINTER_GAP, spot.y - GHOST_POINTER_RISE, chip_w, chip_h);
        let placed = place(
            (spot.x, spot.y),
            chip,
            (f32::from(wide.width), f32::from(wide.height)),
            ANCHOR,
            Rect::new(
                f32::from(visible.origin.x),
                f32::from(visible.origin.y),
                f32::from(visible.size.width),
                f32::from(visible.size.height),
            ),
        );
        let bounds = Bounds::new(
            point(px(placed.bounds.x), px(placed.bounds.y)),
            size(px(placed.bounds.w), px(placed.bounds.h)),
        );
        let summary = &self.sessions[ix].pane.summary;
        let face = (self.sessions[ix].name(), pane_glyph(summary.status, summary.source.told()));
        // Out of its split, if it was in one: the terminal goes alone.
        self.leave_layout(ix, cx);
        let Some(Departing { records, terminals, active, .. }) = self.take_out(&[id], window, cx) else {
            return;
        };
        let layout = Layout::new(self.panes, self.right_tab, WindowSidebar::default());
        let layout = serde_json::to_string(&layout).ok();
        let profile = self.profile.clone();
        let grows = self.plays_motion(cx).then_some((placed.chip, face));
        let home = self.window_id;
        cx.defer(move |cx| {
            let window_id = app_mut(cx).alloc_window_id();
            let record = WindowRecord {
                id: window_id,
                active,
                layout,
                bounds: window_bounds::encode(WindowBounds::Windowed(bounds), Some(display)),
                sessions: records,
                splits: Vec::new(),
            };
            if let Some((chip, face)) = grows {
                super::grow::begin(window_id, chip, face, cx);
            }
            open_for(&profile, record, terminals, None, home, cx);
        });
    }

    /// Terminal `id` leaves this window, alone and running, for another one to take in.
    fn take_out_alone(&mut self, id: i64, window: &mut Window, cx: &mut Context<Self>) -> Option<Departing> {
        let ix = self.find_session(id)?;
        self.leave_layout(ix, cx);
        self.take_out(&[id], window, cx)
    }
}

/// The app's windows as they lie on the screens, front to back, in the global space, with what each is for a
/// terminal that is let go over it: only another window of the same profile that shows terminals takes one.
/// And the system's numbers of all of them, to tell them from other applications' windows.
fn windows_in_front(
    open: &[OpenWindow],
    source: &OpenWindow,
    screens: &[Screen],
    cx: &mut App,
) -> (Vec<Placed<i64>>, Vec<i64>) {
    let Some(stack) = cx.window_stack() else { return (Vec::new(), Vec::new()) };
    let (mut placed, mut numbers) = (Vec::new(), Vec::new());
    for handle in stack {
        let seen = handle
            .update(cx, |_, window, cx| {
                let display = window.display(cx).map(|display| u64::from(display.id()));
                (
                    window.bounds(),
                    display,
                    platform::window::is_minimised(window),
                    platform::window::number(window),
                )
            })
            .ok();
        let Some((bounds, Some(display), false, number)) = seen else { continue };
        numbers.extend(number);
        let local = Rect::new(
            f32::from(bounds.origin.x),
            f32::from(bounds.origin.y),
            f32::from(bounds.size.width),
            f32::from(bounds.size.height),
        );
        let Some(bounds) = to_global(display, local, screens) else { continue };
        let occupant = match open.iter().find(|w| w.handle.window_id() == handle.window_id()) {
            Some(w) if !w.hidden && w.profile == source.profile => Occupant::Terminals(w.id),
            _ => Occupant::Other,
        };
        placed.push(Placed { occupant, bounds });
    }
    (placed, numbers)
}

/// Terminal `id` of window `source` was let go with the pointer at `pointer`: do what lies there.
fn land(source: i64, release: Release, cx: &mut App) {
    let windows = app(cx).windows.clone();
    let Some(from) = windows.iter().find(|w| w.id == source) else { return };
    let (stack, ours) = windows_in_front(&windows, from, &platform::pointer::screens(), cx);
    // Whatever the front-most window there is, if it is not one of this app's it is another application's
    // (or the desktop's): outside Wormhole.
    let covered =
        platform::pointer::window_number_at(release.pointer.global).is_some_and(|n| !ours.contains(&n));
    land_over(&windows, &stack, covered, source, release, cx);
}

/// [`land`] with the windows' places given (a test says where they are).
pub(super) fn land_over(
    windows: &[OpenWindow],
    stack: &[Placed<i64>],
    covered: bool,
    source: i64,
    release: Release,
    cx: &mut App,
) {
    let Release { id, pointer, .. } = release;
    let Some(from) = windows.iter().find(|w| w.id == source).cloned() else { return };
    match landing(stack, pointer.global.0, pointer.global.1, source, covered) {
        Landing::Stay => {}
        Landing::NewWindow => {
            AppState::with_workspace(&from, cx, |ws, window, cx| {
                ws.tear_off_to_new_window(release, window, cx)
            });
        }
        Landing::Into(target) => {
            let Some(to) = windows.iter().find(|w| w.id == target).cloned() else { return };
            let Some(Some(departing)) =
                AppState::with_workspace(&from, cx, |ws, window, cx| ws.take_out_alone(id, window, cx))
            else {
                return;
            };
            let mut waiting = Some(departing);
            let taken = AppState::with_workspace(&to, cx, |ws, window, cx| {
                if let Some(departing) = waiting.take() {
                    ws.receive(departing, window, cx);
                }
            });
            // The window it was going to is gone: it goes back where it came from, still running.
            if taken.is_none()
                && let Some(departing) = waiting
            {
                AppState::with_workspace(&from, cx, |ws, window, cx| ws.receive(departing, window, cx));
            }
        }
    }
}

/// A pointer at (`x`, `y`) of the primary display, counted from its top left: what a script or a test
/// says instead of moving a mouse (`platform::pointer::pretend`).
#[cfg(any(test, feature = "devctl"))]
pub(crate) fn pointer_at(x: f32, y: f32, cx: &App) -> Option<Pointer> {
    let display = cx.primary_display()?;
    let height = f32::from(display.bounds().size.height);
    Some(Pointer { global: (x, height - y), spot: Spot { display: u64::from(display.id()), x, y } })
}

#[cfg(any(test, feature = "devctl"))]
impl Workspace {
    /// Terminal `id` let go outside the window as a drag would, with the pointer at (`x`, `y`) of the
    /// primary display (devctl, the tests).
    pub(crate) fn tear_off_at(
        &mut self,
        id: i64,
        x: f32,
        y: f32,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(pointer) = pointer_at(x, y, cx) {
            self.terminal_released_outside(Release { id, pointer, chip: None }, window, cx);
        }
    }
}
