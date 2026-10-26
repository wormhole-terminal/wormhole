//! A terminal's row dragged out of the window (the Tear off feature). GPUI keeps reporting the drag to the
//! window it began in while the pointer is far outside, but paints its picture only inside that window, so
//! once the pointer is out the picture is a small transparent window of its own that follows the pointer
//! over the screen and lets the mouse through (`platform::window::make_ghost`): the drag, which belongs to
//! the source window, goes on under it. Back inside the window it is put away.
//!
//! What a release outside means (a window of its own where it was let go, or another Wormhole window's list)
//! is the window's to decide, with the other windows in view: this only tells it where the pointer was
//! (`SidebarEvent::TerminalReleasedOutside`). There is one drag at a time in the whole app, so its state
//! is a global ([`InFlight`]), not a field of any window; the window ends it however the drag ends
//! (`end`: a drop, a release, no drop at all), and a window that stops being the active one cancels it
//! (`cancel`), since the system may take the mouse away then and leave GPUI's drag set for good.
use gpui_kit::*;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::view::tear_off::outside;

use super::terminal_drag::{DragOrigin, DraggedTerminal};
use super::{Sidebar, SidebarEvent};
use crate::platform;
use crate::platform::pointer::Pointer;
use crate::settings::{settings_in, ui};
use crate::style::{self, StatusGlyph};

/// The chip's window: room for the chip at its widest (`style::drag_ghost` stops at 240) and its height.
const CHIP_W: f32 = 248.0;
const CHIP_H: f32 = 36.0;

/// What the chip shows.
#[derive(Clone)]
struct Info {
    id: i64,
    name: SharedString,
    glyph: StatusGlyph,
}

/// The drag in flight, for as long as it can leave the window.
struct Drag {
    source: WindowId,
    info: Info,
    chip: Option<Chip>,
}

struct Chip {
    handle: AnyWindowHandle,
    view: Entity<TearChip>,
    shown: bool,
}

#[derive(Default)]
struct InFlight(Option<Drag>);

impl Global for InFlight {}

/// A row was picked up in `window`: from now on its drag is followed out of the window.
pub(super) fn picked_up(dragged: &DraggedTerminal, window: &Window, cx: &mut App) {
    // One drag at a time: whatever an earlier one left (its chip) goes first.
    end(cx);
    let info = Info { id: dragged.id, name: dragged.name.clone(), glyph: dragged.glyph };
    let source = window.window_handle().window_id();
    cx.default_global::<InFlight>().0 = Some(Drag { source, info, chip: None });
}

/// Whether a drag of this feature is in flight (a chip may be out).
pub(crate) fn in_flight(cx: &App) -> bool {
    cx.try_global::<InFlight>().is_some_and(|flight| flight.0.is_some())
}

/// The drag is over, however it ended: the chip goes. Does nothing without a drag of this feature.
pub(crate) fn end(cx: &mut App) {
    let Some(drag) = cx.default_global::<InFlight>().0.take() else { return };
    if let Some(chip) = drag.chip {
        // Deferred: this may run inside the chip's own update or the source window's.
        cx.defer(move |cx| {
            let _ = chip.handle.update(cx, |_, window, _| window.remove_window());
        });
    }
}

/// `window` stopped being the active one: if its drag is the one in flight, GPUI's drag is dropped
/// (it would otherwise stay set and be drawn) and the chip goes.
pub(crate) fn cancel(window: &mut Window, cx: &mut App) {
    let ours = cx
        .try_global::<InFlight>()
        .and_then(|flight| flight.0.as_ref())
        .is_some_and(|drag| drag.source == window.window_handle().window_id());
    if ours {
        cx.stop_active_drag(window);
        end(cx);
    }
}

/// Whether the drag in flight began in `window`.
fn began_in(window: &Window, cx: &App) -> bool {
    cx.try_global::<InFlight>()
        .and_then(|flight| flight.0.as_ref())
        .is_some_and(|drag| drag.source == window.window_handle().window_id())
}

/// Where the chip's window goes for a pointer at `at`, in the global space: where GPUI draws the picture
/// inside the window, to the pointer's right and a little above it.
fn chip_corner(at: &Pointer) -> (f32, f32) {
    (at.global.0 + style::GHOST_POINTER_GAP, at.global.1 + style::GHOST_POINTER_RISE)
}

/// The pointer is `outside` the window or not: the chip shows or goes away, following it.
fn follow(outside: bool, cx: &mut App) {
    let Some(drag) = cx.default_global::<InFlight>().0.as_mut() else { return };
    if !outside {
        if let Some(chip) = drag.chip.as_mut().filter(|chip| chip.shown) {
            chip.shown = false;
            let handle = chip.handle;
            let _ = handle.update(cx, |_, window, _| platform::window::hide(window));
        }
        return;
    }
    let Some(pointer) = platform::pointer::on_screen() else { return };
    let info = drag.info.clone();
    let (x, y) = chip_corner(&pointer);
    match drag.chip.as_mut() {
        Some(chip) => {
            let was_shown = std::mem::replace(&mut chip.shown, true);
            let handle = chip.handle;
            let _ = handle.update(cx, |_, window, _| {
                platform::window::set_global_top_left(window, x, y);
                if !was_shown {
                    platform::window::show(window);
                }
            });
        }
        None => {
            if let Some((handle, view)) = open_chip(info, &pointer, cx)
                && let Some(drag) = cx.default_global::<InFlight>().0.as_mut()
            {
                drag.chip = Some(Chip { handle, view, shown: true });
            }
        }
    }
}

/// The chip's window, over the screen the pointer is on.
fn open_chip(info: Info, pointer: &Pointer, cx: &mut App) -> Option<(AnyWindowHandle, Entity<TearChip>)> {
    let spot = pointer.spot;
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::new(
            point(px(spot.x + style::GHOST_POINTER_GAP), px((spot.y - style::GHOST_POINTER_RISE).max(0.0))),
            size(px(CHIP_W), px(CHIP_H)),
        ))),
        display_id: Some(DisplayId::new(spot.display)),
        titlebar: None,
        // The drag has the mouse: the chip must not take the keys or the activation from the window.
        focus: false,
        show: true,
        kind: WindowKind::PopUp,
        is_movable: false,
        is_resizable: false,
        is_minimizable: false,
        window_background: WindowBackgroundAppearance::Transparent,
        ..Default::default()
    };
    match crate::window_open::open(options, cx, |window, cx| {
        platform::window::make_ghost(window);
        cx.new(|_| TearChip { info, measured: None })
    }) {
        Ok((handle, view)) => {
            let (x, y) = chip_corner(pointer);
            let _ = handle.update(cx, |_, window, _| platform::window::set_global_top_left(window, x, y));
            Some((handle, view))
        }
        Err(err) => {
            tracing::warn!("opening the tear-off chip failed: {err:#}");
            None
        }
    }
}

/// The chip's picture: what the drag shows inside the window (`DraggedTerminal`), plain.
struct TearChip {
    info: Info,
    /// The chip's size as it was drawn, which the window it grows into starts from.
    measured: Option<(f32, f32)>,
}

impl Render for TearChip {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let ui = ui(cx);
        let glyph = style::status_glyph(("drag-glyph", self.info.id as u64), ui, self.info.glyph, cx);
        div().size_full().flex().items_start().child(style::drag_ghost(
            ui,
            Some(glyph),
            self.info.name.clone(),
        ))
    }
}

impl Sidebar {
    /// Whether a row picked up in this list can be torn off.
    pub(super) fn tear_off_on(&self, cx: &App) -> bool {
        settings_in(&self.input.profile, cx).feature_enabled(FeatureFlag::TearOff)
    }

    /// The list follows a drag out of the window and tells the window where it ends.
    pub(super) fn with_tear_off(&self, list: Stateful<Div>, cx: &mut Context<Self>) -> Stateful<Div> {
        list.on_drag_move::<DraggedTerminal>(cx.listener(
            |_, event: &DragMoveEvent<DraggedTerminal>, window, cx| {
                if event.drag(cx).origin != DragOrigin::Row || !began_in(window, cx) {
                    return;
                }
                let at = event.event.position;
                let view = window.viewport_size();
                follow(outside(at.x.into(), at.y.into(), view.width.into(), view.height.into()), cx);
            },
        ))
        // Runs before GPUI drops the drag (so it is still set) and with the pointer where it was let go,
        // in the window's space even when that is outside it.
        .on_mouse_up_out(
            MouseButton::Left,
            cx.listener(|_, event: &MouseUpEvent, window, cx| {
                if !began_in(window, cx) || !cx.has_active_drag() {
                    return;
                }
                let at = event.position;
                let view = window.viewport_size();
                if !outside(at.x.into(), at.y.into(), view.width.into(), view.height.into()) {
                    return;
                }
                let drag = cx.try_global::<InFlight>().and_then(|flight| flight.0.as_ref());
                let id = drag.map(|d| d.info.id);
                let chip = drag.and_then(|d| d.chip.as_ref()).and_then(|chip| chip.view.read(cx).measured);
                end(cx);
                if let (Some(id), Some(pointer)) = (id, platform::pointer::on_screen()) {
                    cx.emit(SidebarEvent::TerminalReleasedOutside { id, pointer, chip });
                }
            }),
        )
    }
}
