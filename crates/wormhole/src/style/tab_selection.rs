//! A bar of tabs whose selection is one shape under them, which springs from tab to tab (the right
//! panel's, `window::panel`): the quick spring of core's `view::motion::tabs`, on a coordinate that counts
//! tabs, so a switch made while it is on its way carries on from where the shape is.
//!
//! The shape is painted by the bar itself, behind its tabs, from where the tabs were laid out *in this
//! frame* (the bar hears its children's bounds after they are prepainted, and the shape paints after every
//! prepaint), so a tab that grows for a badge never leaves the shape a frame behind. The tabs are
//! the caller's: the selected one is drawn without a fill of its own, since the shape is its fill. While the
//! shape travels the bar's view is drawn again every frame, and nothing else is (an animation of a
//! cached region); Reduce Motion puts the shape at its tab at once.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::motion::tabs::{SELECTION, Slot, selection_at};

use super::motion::spring;
use crate::settings::UiColors;

/// How far from its tab the shape may be, in tabs, and still count as arrived: a hundredth of a tab is
/// a fraction of a point, which nobody sees.
const ARRIVED: f32 = 0.01;

/// Where the bar's tabs lie in the frame being drawn.
#[derive(Default)]
struct Laid {
    slots: Vec<Slot>,
    /// The tabs' top and height (they share them).
    top: f32,
    height: f32,
}

/// The shape's spring, kept by the bar from one frame to the next.
struct Travel {
    spring: SpringState,
    /// The tab the spring is heading for, which a change of tab replaces from the moment it is drawn.
    target: f32,
    at: Instant,
}

/// The coordinate the shape is at now, for a bar whose selected tab is the `target`th: the spring stepped
/// by the time since it was last drawn, and asked for another frame until it has arrived.
fn travel(id: &ElementId, target: f32, window: &mut Window, cx: &mut App) -> f32 {
    window.with_global_id(id.clone(), |global_id, window| {
        window.with_element_state(global_id, |state: Option<Travel>, window| {
            let now = Instant::now();
            let mut travel = state.unwrap_or(Travel {
                spring: SpringState { position: target, velocity: 0.0 },
                target,
                at: now,
            });
            let config = spring(SELECTION);
            travel.spring =
                config.step(travel.spring, travel.target, now.duration_since(travel.at).as_secs_f32());
            travel.target = target;
            travel.at = now;
            if cx.reduce_motion() || config.is_settled(travel.spring, target, ARRIVED) {
                travel.spring = SpringState { position: target, velocity: 0.0 };
            } else {
                window.request_animation_frame();
            }
            (travel.spring.position, travel)
        })
    })
}

/// A row of `tabs` (laid out left to right, 4 pt apart, centred) with a shape of `ui.selected` under
/// the `active`th of them, which springs to the one that is selected next. `None` for no selection: no
/// shape. Give the selected tab no fill of its own (`.bg(transparent_black())`), and chain the bar's role
/// and the rest on the result.
#[track_caller]
pub fn selection_bar(
    id: impl Into<ElementId>,
    ui: UiColors,
    active: Option<usize>,
    tabs: impl IntoIterator<Item = AnyElement>,
) -> Stateful<Div> {
    let id: ElementId = id.into();
    let laid = Rc::new(RefCell::new(Laid::default()));
    let (paint_laid, track_laid) = (laid.clone(), laid);
    let (travel_id, fill, radius) = (id.clone(), ui.selected, ui.radius);
    // The shape is the first child, so it paints under every tab and the bar's listener skips it.
    let shape = canvas(
        move |_, window, cx| active.map(|active| travel(&travel_id, active as f32, window, cx)),
        move |_, at, window, _| {
            let laid = paint_laid.borrow();
            let Some(slot) = at.and_then(|at| selection_at(&laid.slots, at)) else { return };
            let bounds = Bounds::new(point(px(slot.x), px(laid.top)), size(px(slot.w), px(laid.height)));
            window.paint_quad(fill_quad(bounds, fill, radius));
        },
    )
    .absolute()
    .size_0();
    div()
        .on_children_prepainted(move |bounds, _, _| {
            let mut laid = track_laid.borrow_mut();
            let mut tabs = bounds.iter().skip(1);
            laid.slots =
                tabs.clone().map(|b| Slot { x: b.origin.x.as_f32(), w: b.size.width.as_f32() }).collect();
            if let Some(first) = tabs.next() {
                (laid.top, laid.height) = (first.origin.y.as_f32(), first.size.height.as_f32());
            }
        })
        .id(id)
        .relative()
        .flex()
        .items_center()
        .gap_1()
        .child(shape)
        .children(tabs)
}

/// A filled quad with round corners.
fn fill_quad(bounds: Bounds<Pixels>, color: Hsla, radius: Pixels) -> PaintQuad {
    gpui_kit::fill(bounds, color).corner_radii(radius)
}
