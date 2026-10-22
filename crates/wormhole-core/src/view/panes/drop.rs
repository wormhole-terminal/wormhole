//! Where a drag lands on the panes of a split: which pane, and which of its four edges. A terminal
//! dragged from the sidebar onto an edge joins the split on that side of the pane, and a pane's header
//! dragged onto another pane's edge moves it there. Pure geometry over the rectangles
//! [`Tree::rects`](super::Tree::rects) gives; what the drop does to the tree is `Tree::insert_beside`
//! and `TabPanes::join` / `move_to`.

use super::geometry::Rect;
use super::{Axis, PaneId};

/// The side of a pane a drop lands on, which is the side the dragged pane ends up on.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Zone {
    Left,
    Right,
    Top,
    Bottom,
}

impl Zone {
    pub const ALL: [Zone; 4] = [Zone::Left, Zone::Right, Zone::Top, Zone::Bottom];

    /// The axis the dragged pane and the target share afterwards: side by side for the left and
    /// right edges, stacked for the top and bottom ones.
    pub fn axis(self) -> Axis {
        match self {
            Zone::Left | Zone::Right => Axis::Row,
            Zone::Top | Zone::Bottom => Axis::Column,
        }
    }

    /// Whether the dragged pane goes before the target along [`Zone::axis`] (left, above).
    pub fn before(self) -> bool {
        matches!(self, Zone::Left | Zone::Top)
    }
}

/// How much of a pane's side an edge's band takes: a quarter, so the middle half is left for the
/// drop on a row that is not a pane edge.
pub const EDGE_BAND: f32 = 0.25;

/// The widest an edge's band gets, in points. A quarter of a wide pane is a slab of screen, and a
/// band that big would make the near edge of a big pane the only thing a pointer ever hits.
pub const EDGE_BAND_MAX: f32 = 96.0;

/// The depth of an edge's band on a pane side `side` points long.
pub fn edge_band(side: f32) -> f32 {
    (side * EDGE_BAND).min(EDGE_BAND_MAX)
}

/// The band along `zone`'s edge of `rect`, what the app tints while a drag is over it.
pub fn band_rect(rect: Rect, zone: Zone) -> Rect {
    let (band_w, band_h) = (edge_band(rect.w), edge_band(rect.h));
    match zone {
        Zone::Left => Rect::new(rect.x, rect.y, band_w, rect.h),
        Zone::Right => Rect::new(rect.right() - band_w, rect.y, band_w, rect.h),
        Zone::Top => Rect::new(rect.x, rect.y, rect.w, band_h),
        Zone::Bottom => Rect::new(rect.x, rect.bottom() - band_h, rect.w, band_h),
    }
}

/// The share of `rect` a terminal dropped on `zone` would take: the half of the pane on that side
/// (a join gives the newcomer half of the target's room, `Tree::insert_beside`), what the app tints
/// under the pointer so the person sees the result before letting go.
pub fn preview_rect(rect: Rect, zone: Zone) -> Rect {
    let (half_w, half_h) = (rect.w / 2.0, rect.h / 2.0);
    match zone {
        Zone::Left => Rect::new(rect.x, rect.y, half_w, rect.h),
        Zone::Right => Rect::new(rect.x + half_w, rect.y, half_w, rect.h),
        Zone::Top => Rect::new(rect.x, rect.y, rect.w, half_h),
        Zone::Bottom => Rect::new(rect.x, rect.y + half_h, rect.w, half_h),
    }
}

/// What shows of `zone`'s band along `rect` while the pointer is on `active`: the band without the
/// corners the top and bottom bands take and without the part the lit half ([`preview_rect`] of
/// `active`) covers, so the faint outlines of the sides it isn't on never lie under it. `None` when
/// nothing is left of it.
pub fn hint_rect(rect: Rect, zone: Zone, active: Zone) -> Option<Rect> {
    let band = band_rect(rect, zone);
    let (top, bottom) = (edge_band(rect.h), edge_band(rect.h));
    // The left and right bands run between the top and bottom ones.
    let (mut left, mut right, mut up, mut down) = match zone {
        Zone::Left | Zone::Right => (band.x, band.right(), band.y + top, band.bottom() - bottom),
        Zone::Top | Zone::Bottom => (band.x, band.right(), band.y, band.bottom()),
    };
    let lit = preview_rect(rect, active);
    match active {
        Zone::Left => left = left.max(lit.right()),
        Zone::Right => right = right.min(lit.x),
        Zone::Top => up = up.max(lit.bottom()),
        Zone::Bottom => down = down.min(lit.y),
    }
    (right > left && down > up).then(|| Rect::new(left, up, right - left, down - up))
}

/// The edge of `rect` a drop at `point` (x, y from the same origin as the rectangle) lands on, or
/// `None` outside it. Every point inside resolves to one edge: the nearest, measured in bands
/// (so a wide pane isn't biased towards its top and bottom), which makes the corners go to the
/// edge the pointer is deeper in and the middle to whichever is closest. A tie (the centre of a
/// pane with equal bands) goes right, then below, as ⌘D and ⇧⌘D split. The left and top edges
/// belong to the pane, the right and bottom ones to its neighbour, so no point is in two panes.
pub fn zone_at(rect: Rect, point: (f32, f32)) -> Option<Zone> {
    let (x, y) = point;
    let inside = rect.w > 0.0 && rect.h > 0.0 && x >= rect.x && x < rect.right() && y >= rect.y;
    if !inside || y >= rect.bottom() {
        return None;
    }
    let (band_w, band_h) = (edge_band(rect.w), edge_band(rect.h));
    // `min_by` keeps the first of equals, so this order is the tie-break.
    [
        (Zone::Right, (rect.right() - x) / band_w),
        (Zone::Bottom, (rect.bottom() - y) / band_h),
        (Zone::Left, (x - rect.x) / band_w),
        (Zone::Top, (y - rect.y) / band_h),
    ]
    .into_iter()
    .min_by(|(_, a), (_, b)| a.total_cmp(b))
    .map(|(zone, _)| zone)
}

/// The pane under `point` and the edge of it the drop lands on, from the panes' rectangles
/// ([`Tree::rects`](super::Tree::rects)); `None` over no pane.
pub fn drop_target(rects: &[(PaneId, Rect)], point: (f32, f32)) -> Option<(PaneId, Zone)> {
    rects.iter().find_map(|&(id, rect)| zone_at(rect, point).map(|zone| (id, zone)))
}

#[cfg(test)]
mod tests {
    use super::{EDGE_BAND_MAX, Zone, band_rect, drop_target, edge_band, hint_rect, preview_rect, zone_at};
    use crate::panes::geometry::Rect;
    use crate::panes::{Axis, PANE_MIN_H, PANE_MIN_W};

    const PANE: Rect = Rect { x: 100.0, y: 50.0, w: 400.0, h: 300.0 };

    #[test]
    fn a_zone_names_the_axis_and_the_side_the_dragged_pane_lands_on() {
        let sides: Vec<_> = Zone::ALL.iter().map(|zone| (zone.axis(), zone.before())).collect();
        assert_eq!(
            sides,
            [(Axis::Row, true), (Axis::Row, false), (Axis::Column, true), (Axis::Column, false)]
        );
    }

    #[test]
    fn the_edge_band_is_a_quarter_of_the_side_up_to_a_cap() {
        assert_eq!(edge_band(400.0), 96.0, "a quarter would be 100");
        assert_eq!(edge_band(300.0), 75.0);
        assert_eq!(edge_band(PANE_MIN_W), 40.0);
        assert_eq!(edge_band(PANE_MIN_H), 24.0);
        assert_eq!(edge_band(4000.0), EDGE_BAND_MAX);
        assert_eq!(edge_band(0.0), 0.0);
    }

    #[test]
    fn a_point_near_an_edge_lands_on_it() {
        assert_eq!(zone_at(PANE, (110.0, 200.0)), Some(Zone::Left));
        assert_eq!(zone_at(PANE, (490.0, 200.0)), Some(Zone::Right));
        assert_eq!(zone_at(PANE, (300.0, 60.0)), Some(Zone::Top));
        assert_eq!(zone_at(PANE, (300.0, 340.0)), Some(Zone::Bottom));
    }

    #[test]
    fn the_middle_resolves_to_the_nearest_edge_and_a_corner_to_the_one_the_pointer_is_deeper_in() {
        // Well outside every band, nearer the left side than any other.
        assert_eq!(zone_at(PANE, (220.0, 200.0)), Some(Zone::Left));
        assert_eq!(zone_at(PANE, (300.0, 130.0)), Some(Zone::Top));
        assert_eq!(zone_at(PANE, (300.0, 270.0)), Some(Zone::Bottom));
        // Top left corner: 10 in from the left, 40 down from the top.
        assert_eq!(zone_at(PANE, (110.0, 90.0)), Some(Zone::Left));
        assert_eq!(zone_at(PANE, (140.0, 60.0)), Some(Zone::Top));
        // Bottom right corner.
        assert_eq!(zone_at(PANE, (490.0, 300.0)), Some(Zone::Right));
        assert_eq!(zone_at(PANE, (440.0, 340.0)), Some(Zone::Bottom));
    }

    #[test]
    fn a_tie_goes_right_like_a_split_to_the_right_and_a_narrow_pane_is_not_biased() {
        let square = Rect::new(0.0, 0.0, 200.0, 200.0);
        assert_eq!(zone_at(square, (100.0, 100.0)), Some(Zone::Right), "the centre: all four are equal");
        assert_eq!(zone_at(square, (100.0, 120.0)), Some(Zone::Bottom), "off the centre, the nearest");
        // A tall narrow pane: the centre of its width is nearer its sides than its top, in bands.
        let tall = Rect::new(0.0, 0.0, 200.0, 900.0);
        assert_eq!(zone_at(tall, (60.0, 450.0)), Some(Zone::Left));
        assert_eq!(zone_at(tall, (100.0, 120.0)), Some(Zone::Top));
        // A wide short one is nearer its top and bottom.
        let wide = Rect::new(0.0, 0.0, 900.0, 200.0);
        assert_eq!(zone_at(wide, (450.0, 40.0)), Some(Zone::Top));
        assert_eq!(zone_at(wide, (450.0, 160.0)), Some(Zone::Bottom));
        assert_eq!(zone_at(wide, (30.0, 100.0)), Some(Zone::Left));
    }

    #[test]
    fn a_big_pane_keeps_its_bands_narrow_and_still_resolves_its_middle() {
        let big = Rect::new(0.0, 0.0, 2000.0, 1000.0);
        assert_eq!(zone_at(big, (1000.0, 300.0)), Some(Zone::Top));
        assert_eq!(zone_at(big, (700.0, 600.0)), Some(Zone::Bottom));
        assert_eq!(zone_at(big, (150.0, 500.0)), Some(Zone::Left));
        assert_eq!(band_rect(big, Zone::Left), Rect::new(0.0, 0.0, 96.0, 1000.0));
        assert_eq!(band_rect(big, Zone::Right), Rect::new(1904.0, 0.0, 96.0, 1000.0));
        assert_eq!(band_rect(big, Zone::Top), Rect::new(0.0, 0.0, 2000.0, 96.0));
        assert_eq!(band_rect(big, Zone::Bottom), Rect::new(0.0, 904.0, 2000.0, 96.0));
    }

    #[test]
    fn the_preview_is_the_half_of_the_pane_the_dropped_terminal_would_take() {
        assert_eq!(preview_rect(PANE, Zone::Left), Rect::new(100.0, 50.0, 200.0, 300.0));
        assert_eq!(preview_rect(PANE, Zone::Right), Rect::new(300.0, 50.0, 200.0, 300.0));
        assert_eq!(preview_rect(PANE, Zone::Top), Rect::new(100.0, 50.0, 400.0, 150.0));
        assert_eq!(preview_rect(PANE, Zone::Bottom), Rect::new(100.0, 200.0, 400.0, 150.0));
    }

    #[test]
    fn a_hint_is_its_band_between_the_others_and_clear_of_the_lit_half() {
        // The left band runs between the top and bottom ones, and stops where the lit bottom half starts.
        assert_eq!(hint_rect(PANE, Zone::Left, Zone::Bottom), Some(Rect::new(100.0, 125.0, 96.0, 75.0)));
        assert_eq!(hint_rect(PANE, Zone::Right, Zone::Top), Some(Rect::new(404.0, 200.0, 96.0, 75.0)));
        // The top band ends where the lit right half starts.
        assert_eq!(hint_rect(PANE, Zone::Top, Zone::Right), Some(Rect::new(100.0, 50.0, 200.0, 75.0)));
        assert_eq!(hint_rect(PANE, Zone::Bottom, Zone::Left), Some(Rect::new(300.0, 275.0, 200.0, 75.0)));
        // Sides on the lit side's own axis are clear of it already.
        assert_eq!(hint_rect(PANE, Zone::Right, Zone::Left), Some(Rect::new(404.0, 125.0, 96.0, 150.0)));
        assert_eq!(hint_rect(PANE, Zone::Top, Zone::Bottom), Some(Rect::new(100.0, 50.0, 400.0, 75.0)));
        assert_eq!(hint_rect(Rect::new(0.0, 0.0, 0.0, 0.0), Zone::Top, Zone::Left), None, "no area, no hint");
    }

    #[test]
    fn the_band_of_a_small_pane_is_a_quarter_of_its_sides() {
        let small = Rect::new(10.0, 20.0, PANE_MIN_W, PANE_MIN_H);
        assert_eq!(band_rect(small, Zone::Left), Rect::new(10.0, 20.0, 40.0, PANE_MIN_H));
        assert_eq!(band_rect(small, Zone::Bottom), Rect::new(10.0, 92.0, PANE_MIN_W, 24.0));
    }

    #[test]
    fn nothing_outside_a_pane_lands_and_a_shared_edge_belongs_to_one_pane() {
        assert_eq!(zone_at(PANE, (99.0, 200.0)), None);
        assert_eq!(zone_at(PANE, (300.0, 49.0)), None);
        assert_eq!(zone_at(PANE, (500.0, 200.0)), None, "the right edge is the neighbour's");
        assert_eq!(zone_at(PANE, (300.0, 350.0)), None, "so is the bottom one");
        assert!(zone_at(PANE, (100.0, 50.0)).is_some(), "the top left corner is inside");
        assert_eq!(zone_at(Rect::new(0.0, 0.0, 0.0, 100.0), (0.0, 50.0)), None, "no area, no drop");
        assert_eq!(zone_at(PANE, (f32::NAN, 100.0)), None);
    }

    #[test]
    fn the_target_is_the_pane_under_the_pointer_with_its_edge() {
        // Two columns, the right one split in two rows.
        let rects = [
            (1, Rect::new(0.0, 0.0, 400.0, 600.0)),
            (2, Rect::new(400.0, 0.0, 400.0, 300.0)),
            (3, Rect::new(400.0, 300.0, 400.0, 300.0)),
        ];
        assert_eq!(drop_target(&rects, (10.0, 300.0)), Some((1, Zone::Left)));
        assert_eq!(drop_target(&rects, (399.0, 300.0)), Some((1, Zone::Right)));
        assert_eq!(drop_target(&rects, (400.0, 300.0)), Some((3, Zone::Left)), "the edge is pane 3's");
        assert_eq!(drop_target(&rects, (600.0, 100.0)), Some((2, Zone::Top)));
        assert_eq!(drop_target(&rects, (600.0, 290.0)), Some((2, Zone::Bottom)));
        assert_eq!(drop_target(&rects, (600.0, 310.0)), Some((3, Zone::Top)));
        assert_eq!(drop_target(&rects, (900.0, 300.0)), None, "past the panes");
        assert_eq!(drop_target(&[], (0.0, 0.0)), None);
    }
}
