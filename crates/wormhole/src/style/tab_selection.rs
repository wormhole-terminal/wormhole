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
//!
//! The places the tabs were laid out in are kept in a [`TabSlots`] the caller holds from one build to the next,
//! so a build that draws the bar makes nothing of its own to hold them: a bar that is built for every
//! change of a count is on a budget of what it allocates.

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

/// Where a bar's tabs lie in the frame being drawn. Made once for a bar and given to every build of it.
#[derive(Clone, Default)]
pub struct TabSlots(Rc<RefCell<Laid>>);

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
fn travel(id: &GlobalElementId, target: f32, window: &mut Window, cx: &mut App) -> f32 {
    window.with_element_state(id, |state: Option<Travel>, window| {
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
}

/// The shape: no size of its own, it paints at the selected tab's place (`Element`, since it keeps a
/// spring and reads the frame's layout, and a closure for each of those is an allocation a build makes).
struct Shape {
    id: ElementId,
    slots: TabSlots,
    active: Option<usize>,
    fill: Hsla,
    radius: Pixels,
}

impl IntoElement for Shape {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for Shape {
    /// Where the spring is: asked in the layout, which is where an element keeps its state.
    type RequestLayoutState = Option<f32>;
    type PrepaintState = ();

    fn id(&self) -> Option<ElementId> {
        Some(self.id.clone())
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Option<f32>) {
        let at = match (id, self.active) {
            (Some(id), Some(active)) => Some(travel(id, active as f32, window, cx)),
            _ => None,
        };
        let mut style = Style::default();
        style.position = Position::Absolute;
        (window.request_layout(style, [], cx), at)
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        _: &mut Option<f32>,
        _: &mut Window,
        _: &mut App,
    ) {
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        _: Bounds<Pixels>,
        at: &mut Option<f32>,
        _: &mut (),
        window: &mut Window,
        _: &mut App,
    ) {
        let laid = self.slots.0.borrow();
        let Some(slot) = at.and_then(|at| selection_at(&laid.slots, at)) else { return };
        let bounds = Bounds::new(point(px(slot.x), px(laid.top)), size(px(slot.w), px(laid.height)));
        window.paint_quad(gpui_kit::fill(bounds, self.fill).corner_radii(self.radius));
    }
}

/// A row of `tabs` (laid out left to right, 4 pt apart, centred) with a shape of `ui.selected` under
/// the `active`th of them, which springs to the one that is selected next. `None` for no selection: no
/// shape. `slots` is the bar's, made once and given to every build of it. Give the selected tab no fill of its
/// own (`.bg(transparent_black())`), and chain the bar's role and the rest on the result.
#[track_caller]
pub fn selection_bar(
    id: impl Into<ElementId>,
    ui: UiColors,
    active: Option<usize>,
    slots: &TabSlots,
    tabs: impl IntoIterator<Item = AnyElement>,
) -> Stateful<Div> {
    let id: ElementId = id.into();
    let tracked = slots.clone();
    let shape = Shape { id: id.clone(), slots: slots.clone(), active, fill: ui.selected, radius: ui.radius };
    // The shape is the first child, so it paints under every tab and the bar's listener skips it.
    div()
        .on_children_prepainted(move |bounds, _, _| {
            let mut laid = tracked.0.borrow_mut();
            let mut tabs = bounds.iter().skip(1);
            laid.slots.clear();
            laid.slots
                .extend(tabs.clone().map(|b| Slot { x: b.origin.x.as_f32(), w: b.size.width.as_f32() }));
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
