//! What the light and the amber edge share: how far they sit outside the tile and how thick
//! their line is for a tile of a size, and which of the two a status has (`light`).

use std::hash::{Hash, Hasher};

use gpui_kit::*;

use super::edge::breathing_edge;
use super::orbit::orbit;
use crate::settings::UiColors;
use crate::style::status::StatusGlyph;

/// How far the light and the amber edge sit outside the tile's own edge.
pub(super) const RING_OUT: f32 = 1.0;

/// The light's line, in a tile and in a smaller one (below [`LINE_THIN_BELOW`]).
const ORBIT_STROKE: f32 = 2.0;
const ORBIT_STROKE_MINI: f32 = 1.5;
pub(super) const LINE_THIN_BELOW: f32 = 28.0;

/// The light's and the edge's line for a tile of `size`.
pub(super) fn line(size: f32) -> f32 {
    if size < LINE_THIN_BELOW { ORBIT_STROKE_MINI } else { ORBIT_STROKE }
}

/// Where in its turn a tile's light and its edge's breath are at any moment, as a share of it: the tile's
/// own, from its id, so the tiles of a list are not all at the same point of theirs.
pub(super) fn phase_of(id: &ElementId) -> f32 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut hasher);
    (hasher.finish() % 1000) as f32 / 1000.0
}

/// The light round the edge of a tile in `state`, or its amber edge breathing, or neither. `id` keys
/// the animation, `phase` is the tile's ([`phase_of`]).
#[track_caller]
pub(super) fn light(
    id: &ElementId,
    ui: UiColors,
    state: StatusGlyph,
    (size, corner): (f32, f32),
    phase: f32,
    still: bool,
) -> Option<AnyElement> {
    match state {
        StatusGlyph::Working => {
            Some(orbit((id.clone(), "orbit"), ui.agent_working, size, corner, phase, still))
        }
        StatusGlyph::Running => Some(orbit((id.clone(), "orbit"), ui.muted, size, corner, phase, still)),
        StatusGlyph::NeedsYou => {
            Some(breathing_edge((id.clone(), "breathe"), ui, size, corner, phase, still))
        }
        _ => None,
    }
}
