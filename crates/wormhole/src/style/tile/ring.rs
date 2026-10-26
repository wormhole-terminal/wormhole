//! What the light and the amber edge share: how far they sit outside the tile and how thick
//! their line is for a tile of a size.

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
