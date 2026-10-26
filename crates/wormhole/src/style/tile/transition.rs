//! A tile while its terminal's status changes: the old light, edge and badge on their way out, the new
//! ones on their way in and the flare's ring, drawn from the pose core gives (`StatusPose`). One layer
//! over the tile's face that is drawn again for every frame of the change and leaves the face alone;
//! when the change is over the tile is drawn as it always is, and the light, which is read from the
//! clock every animation shares, goes on where it was (see `orbit`).

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::view::motion::pose::status::{Flare, Part};

use super::badge::grown;
use super::ring::{RING_OUT, light};
use crate::settings::UiColors;
use crate::style::status::StatusGlyph;
use crate::style::status_shift::StatusShift;

/// What the layer is drawn on and with: the tile's size and corner, its surface (the badge's rim) and
/// the phase of its light.
#[derive(Clone)]
pub(super) struct Shape {
    pub(super) id: ElementId,
    pub(super) ui: UiColors,
    pub(super) size: f32,
    pub(super) corner: f32,
    pub(super) surface: Hsla,
    pub(super) phase: f32,
}

/// The layer for `shift`, the tile going from its `from` to `to`.
#[track_caller]
pub(super) fn shifting(shape: Shape, shift: StatusShift, to: StatusGlyph) -> AnyElement {
    let layer = div().absolute().top_0().left_0().size_full();
    let id = ElementId::from((shape.id.clone(), "shift"));
    shift.drive(layer, id, to, move |layer, pose| {
        let from = shift.from;
        layer
            .children(parts(&shape, from, "from", Part { opacity: pose.leaving, scale: 1.0 }, pose.leaving))
            .children(parts(&shape, to, "to", pose.badge, pose.ring))
            .children(flare(&shape, pose.flare))
    })
}

/// The light or edge of `state` at `ring` strength and its badge as `badge` says, in one box over the
/// tile; `None` while none of it shows. `tag` keeps the old parts' animations apart from the new.
#[track_caller]
fn parts(shape: &Shape, state: StatusGlyph, tag: &'static str, badge: Part, ring: f32) -> Option<AnyElement> {
    let Shape { id, ui, size, corner, surface, phase } = shape;
    let light_id = ElementId::from((id.clone(), tag));
    let ring_part = light(&light_id, *ui, state, (*size, *corner), *phase, false).filter(|_| ring > 0.0);
    let badge_part = grown(*ui, state, *size, *surface, badge.scale);
    (ring_part.is_some() || (badge_part.is_some() && badge.opacity > 0.0)).then(|| {
        div()
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .children(ring_part.map(|ring_part| faded(ring_part, ring)))
            .children(badge_part.map(|badge_part| faded(badge_part, badge.opacity)))
            .into_any_element()
    })
}

/// `part` (a light, an edge or a badge, placed by the tile's corner) at `opacity`.
#[track_caller]
fn faded(part: AnyElement, opacity: f32) -> AnyElement {
    let layer = div().absolute().top_0().left_0().size_full().child(part);
    if opacity < 1.0 { layer.opacity(opacity.max(0.0)) } else { layer }.into_any_element()
}

/// The ring a flare sends out from the amber edge: as thick as it reaches far, so it fills the room
/// between the edge and where it has got to, and fades as it goes.
#[track_caller]
fn flare(shape: &Shape, flare: Flare) -> Option<AnyElement> {
    if flare.reach <= 0.0 || flare.strength <= 0.0 {
        return None;
    }
    let Shape { ui, size, corner, .. } = shape;
    let out = RING_OUT + flare.reach;
    let mut ring = div()
        .absolute()
        .top(px(-out))
        .left(px(-out))
        .size(px(size + 2.0 * out))
        .rounded(px(corner + out))
        .border_color(ui.agent_attention.opacity(flare.strength));
    let width = Some(px(flare.reach).into());
    let b = &mut ring.style().border_widths;
    (b.top, b.right, b.bottom, b.left) = (width, width, width, width);
    Some(ring.into_any_element())
}
