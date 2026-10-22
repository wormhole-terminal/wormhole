//! The scrollbar along the pane's right edge, drawn here rather than by AppKit: a native view
//! over the Metal layer would cover GPUI's popovers and menus. Core's `terminal::scrollbar` says
//! where the thumb is, what a click on the track does and when the overlay style shows or fades;
//! this paints it and forwards the pointer. The system's style comes from `scroller`.
//!
//! A frame costs a handful of multiplications from numbers the grid's snapshot already holds.
//! The fade asks for frames only while it is fading, and a timer wakes the view when the idle
//! time is over, so a quiet terminal draws nothing for it.

use std::time::Instant;

use wormhole_core::terminal::scrollbar::{
    EDGE, GUTTER_W, Hit, Next, ScrollbarGeometry, ScrollerStyle, THUMB_W, THUMB_WIDE_W, Visibility, ZONE_W,
    jumps,
};

use super::*;
use crate::platform::scroller::{self, ScrollerPrefs};

/// What the scrollbar keeps in the view.
#[derive(Default)]
pub(in crate::terminal_view) struct ScrollbarState {
    visibility: Visibility,
    /// Where the pointer holds the thumb, below its top, while it's dragged.
    grab: Option<f32>,
    /// The pointer is over the scrollbar's zone.
    hovered: bool,
    /// Sleeps until the idle time is over, then draws the fade.
    wake: Option<Task<()>>,
}

/// Where the scrollbar was last drawn and what it stood for, for the pointer.
#[derive(Clone, Copy)]
pub(super) struct ScrollbarLayout {
    geometry: ScrollbarGeometry,
    bounds: Bounds<Pixels>,
}

impl TerminalView {
    /// The scrollbar's zone, below the title bar and as tall as the grid. It is there on every
    /// screen but only answers the pointer while a scrollbar is drawn: the alternate screen and
    /// a terminal without history leave the pane's right edge to the program and to selecting.
    pub(super) fn render_scrollbar(&self, cx: &mut Context<Self>) -> AnyElement {
        let style = ScrollerPrefs::style(cx);
        let store = store_in(&self.profile, cx);
        let (thumb, held_thumb, track) = (
            token(&store.tokens, "scrollbar.thumb"),
            token(&store.tokens, "text.faint"),
            token(&store.tokens, "state.hover"),
        );
        let entity = cx.entity();
        let prepaint_state = self.render_state.clone();
        let paint_state = self.render_state.clone();
        let weak = entity.downgrade();

        let bar = canvas(
            move |bounds, _window, cx| {
                let mut state = prepaint_state.borrow_mut();
                let snapshot = state.snapshot();
                let drawn = !snapshot.mode.contains(Modes::ALT_SCREEN);
                let geometry = ScrollbarGeometry {
                    history: snapshot.history_size,
                    rows: snapshot.rows().len(),
                    offset: snapshot.display_offset,
                    track: f32::from(bounds.size.height),
                };
                let layout = (drawn && geometry.scrollable()).then_some(ScrollbarLayout { geometry, bounds });
                state.scrollbar = layout;
                drop(state);
                entity.update(cx, |this, cx| this.scrollbar_frame(layout.map(|l| l.geometry), style, cx));
            },
            move |bounds, (), window, cx| {
                let Some(ScrollbarLayout { geometry, .. }) = paint_state.borrow().scrollbar else { return };
                let Ok((held, grabbed, reveal)) = weak.read_with(cx, |this, cx| {
                    let scrollbar = &this.scrollbar;
                    (scrollbar.held(), scrollbar.grab.is_some(), scrollbar.reveal(style, cx.reduce_motion()))
                }) else {
                    return;
                };
                if reveal.opacity > 0.0 {
                    let color = if grabbed { held_thumb } else { thumb };
                    paint_bar(bounds, geometry, style, held, (color, track), reveal.opacity, window);
                }
                if reveal.next == Next::Frame {
                    window.request_animation_frame();
                }
                if grabbed {
                    drag_listener(weak.clone(), window);
                }
            },
        )
        .size_full();

        let width = if style == ScrollerStyle::Legacy { GUTTER_W } else { ZONE_W };
        div()
            .id("scrollbar")
            .absolute()
            .top(px(PADDING_Y))
            .bottom(px(PADDING_Y))
            .right_0()
            .w(px(width))
            .when(self.render_state.borrow().scrollbar.is_some(), |this| this.cursor(CursorStyle::Arrow))
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| this.hover_scrollbar(*hovered, cx)))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_scrollbar_down))
            .child(bar)
            .into_any_element()
    }

    /// After the layout is known: the view's scrolling shows the overlay scrollbar, and the
    /// timer for the end of its idle time runs while it is waiting for it.
    fn scrollbar_frame(
        &mut self,
        geometry: Option<ScrollbarGeometry>,
        style: ScrollerStyle,
        cx: &mut Context<Self>,
    ) {
        let Some(geometry) = geometry else {
            self.scrollbar.hovered = false;
            self.scrollbar.grab = None;
            return;
        };
        self.scrollbar.visibility.watch(geometry.offset, Instant::now());
        if style != ScrollerStyle::Overlay {
            return;
        }
        let Next::After(wait) = self.scrollbar.reveal(style, cx.reduce_motion()).next else { return };
        if self.scrollbar.wake.is_some() {
            return;
        }
        self.scrollbar.wake = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(wait).await;
            // The fade (or, with Reduce Motion, the cut) is drawn by the frame this asks for.
            let _ = this.update(cx, |this, cx| {
                this.scrollbar.wake = None;
                this.notify_redraw(cx);
            });
        }));
    }

    fn hover_scrollbar(&mut self, hovered: bool, cx: &mut Context<Self>) {
        let drawn = self.render_state.borrow().scrollbar.is_some();
        if self.scrollbar.hovered == (hovered && drawn) {
            return;
        }
        self.scrollbar.hovered = hovered && drawn;
        // The idle time starts when the pointer leaves, as it does in AppKit.
        self.scrollbar.visibility.show(Instant::now());
        cx.notify();
    }

    /// A press on the thumb grabs it; on the track it pages or jumps (as the system's "Click in
    /// the scroll bar to" says, Option doing the other) and a jump keeps hold of the thumb.
    fn on_scrollbar_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let Some((geometry, y)) = self.scrollbar_at(event.position) else { return };
        // The pane under it would start a selection.
        cx.stop_propagation();
        window.focus(&self.focus_handle, cx);
        self.scrollbar.grab = match geometry.hit(y) {
            Hit::Thumb => Some(y - geometry.thumb_top()),
            _ => {
                let jump = jumps(scroller::jump_to_spot(), event.modifiers.alt);
                let click = geometry.track_click(y, jump);
                self.scroll_to_offset(click.offset, cx);
                // A page leaves nothing to drag: only a jump takes hold of the thumb.
                click.grab
            }
        };
        cx.notify();
    }

    /// The thumb follows the pointer while the button is held, wherever the pointer goes.
    fn drag_scrollbar(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        let Some(grab) = self.scrollbar.grab else { return };
        if event.pressed_button != Some(MouseButton::Left) {
            self.release_scrollbar(cx);
            return;
        }
        if let Some((geometry, y)) = self.scrollbar_at(event.position) {
            self.scroll_to_offset(geometry.offset_for_drag(y, grab), cx);
        }
    }

    fn release_scrollbar(&mut self, cx: &mut Context<Self>) {
        if self.scrollbar.grab.take().is_some() {
            self.scrollbar.visibility.show(Instant::now());
            cx.notify();
        }
    }

    /// The scrollbar as last drawn, and `position`'s height over the track's top; `None` while
    /// none is drawn.
    fn scrollbar_at(&self, position: Point<Pixels>) -> Option<(ScrollbarGeometry, f32)> {
        let layout = self.render_state.borrow().scrollbar?;
        Some((layout.geometry, f32::from(position.y - layout.bounds.origin.y)))
    }
}

impl ScrollbarState {
    /// The pointer keeps the scrollbar up: over it, or holding the thumb.
    fn held(&self) -> bool {
        self.hovered || self.grab.is_some()
    }

    /// How the scrollbar looks now. The legacy style is always there.
    fn reveal(
        &self,
        style: ScrollerStyle,
        reduce_motion: bool,
    ) -> wormhole_core::terminal::scrollbar::Reveal {
        use wormhole_core::terminal::scrollbar::Reveal;
        match style {
            ScrollerStyle::Legacy => Reveal { opacity: 1.0, next: Next::Nothing },
            ScrollerStyle::Overlay => self.visibility.reveal(Instant::now(), self.held(), reduce_motion),
        }
    }
}

/// The track (behind the thumb under the pointer, or always in the legacy gutter) and the thumb.
fn paint_bar(
    zone: Bounds<Pixels>,
    geometry: ScrollbarGeometry,
    style: ScrollerStyle,
    held: bool,
    (thumb, track): (Hsla, Hsla),
    opacity: f32,
    window: &mut Window,
) {
    let (width, with_track) = match style {
        ScrollerStyle::Legacy => (GUTTER_W - 2.0 * EDGE, true),
        ScrollerStyle::Overlay if held => (THUMB_WIDE_W, true),
        ScrollerStyle::Overlay => (THUMB_W, false),
    };
    if with_track {
        let strip = width + 2.0 * EDGE;
        let bounds =
            Bounds::new(point(zone.right() - px(strip), zone.top()), size(px(strip), zone.size.height));
        window.paint_quad(fill(bounds, track.opacity(opacity)));
    }
    let bounds = Bounds::new(
        point(zone.right() - px(EDGE + width), zone.top() + px(geometry.thumb_top())),
        size(px(width), px(geometry.thumb_len())),
    );
    window.paint_quad(fill(bounds, thumb.opacity(opacity)).corner_radii(px(width / 2.0)));
}

/// While the thumb is held, follow the pointer outside the scrollbar too, and let go wherever
/// the button goes up.
fn drag_listener(entity: WeakEntity<TerminalView>, window: &mut Window) {
    let moved = entity.clone();
    window.on_mouse_event(move |event: &MouseMoveEvent, phase, _window, cx| {
        if phase == DispatchPhase::Bubble {
            let _ = moved.update(cx, |this, cx| this.drag_scrollbar(event, cx));
        }
    });
    window.on_mouse_event(move |_: &MouseUpEvent, phase, _window, cx| {
        if phase == DispatchPhase::Bubble {
            let _ = entity.update(cx, |this, cx| this.release_scrollbar(cx));
        }
    });
}
