//! The light that circles a working terminal's tile: a line of a few steps fading behind a head,
//! over a faint track, along the rounded rectangle's outside edge.

use std::f32::consts::PI;

use gpui_kit::prelude::*;
use gpui_kit::*;

use super::ring::{RING_OUT, line};
use crate::style::motion::{ANIMATION_FPS, TILE_ORBIT};

/// How much of the edge the light covers, and how many steps its tail fades in.
const ORBIT_SPAN: f32 = 0.32;
const ORBIT_STEPS: usize = 12;
/// What is left of the ring under the light: a faint track so the whole edge reads as busy.
const ORBIT_TRACK: f32 = 0.22;

/// A light that circles the tile's edge once every [`TILE_ORBIT`], its tail fading behind it, over
/// a faint track. Still, it is the whole ring at the light's strength. It runs along the edge's
/// outside, so it never covers the tile's own.
///
/// Its turn is read from the clock every animation shares (`repeat_synced`) and moved on by `phase`
/// (a share of a turn, the tile's own), not counted from when the element was made: a light that is
/// drawn again under another element (a status change playing over the tile, the row moving) keeps
/// its place on the edge instead of starting over, and tiles' lights are not all at the same point.
#[track_caller]
pub(super) fn orbit(
    id: impl Into<ElementId>,
    color: Hsla,
    size: f32,
    corner: f32,
    phase: f32,
    still: bool,
) -> AnyElement {
    let stroke = line(size);
    let (outer, round) = (size + 2.0 * RING_OUT, corner + RING_OUT);
    let layer = || div().absolute().top(px(-RING_OUT)).left(px(-RING_OUT)).size(px(outer));
    if still {
        return layer().child(ring_canvas(color, outer, round, stroke, None)).into_any_element();
    }
    layer()
        .with_animation(
            id,
            Animation::new(TILE_ORBIT).repeat_synced().with_max_fps(ANIMATION_FPS),
            move |d, turn| d.child(ring_canvas(color, outer, round, stroke, Some((turn + phase).fract()))),
        )
        .into_any_element()
}

/// The ring itself: the light at `turn` round the edge (from the top's middle, clockwise) and the
/// track under it, or with no `turn` the whole ring.
fn ring_canvas(color: Hsla, size: f32, corner: f32, stroke: f32, turn: Option<f32>) -> Canvas<()> {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let origin = bounds.origin;
            // The line runs along the middle of the edge, half its width in.
            let inset = stroke / 2.0;
            let (w, h, r) = (size - stroke, size - stroke, (corner - inset).max(0.0));
            let at = |s: f32| {
                let (x, y) = perimeter_point(w, h, r, s);
                point(origin.x + px(x + inset), origin.y + px(y + inset))
            };
            let paint = |window: &mut Window, from: f32, to: f32, alpha: f32| {
                let mut path = PathBuilder::stroke(px(stroke));
                let steps = ((to - from) * RING_SAMPLES).ceil().max(1.0) as usize;
                path.move_to(at(from));
                for step in 1..=steps {
                    path.line_to(at(from + (to - from) * step as f32 / steps as f32));
                }
                if let Ok(path) = path.build() {
                    window.paint_path(path, color.opacity(alpha));
                }
            };
            match turn {
                None => paint(window, 0.0, 1.0, 1.0),
                Some(turn) => {
                    paint(window, 0.0, 1.0, ORBIT_TRACK);
                    // The tail, faint to bright, ending at the light's head.
                    let tail = turn - ORBIT_SPAN;
                    for step in 0..ORBIT_STEPS {
                        let from = tail + ORBIT_SPAN * step as f32 / ORBIT_STEPS as f32;
                        let to = tail + ORBIT_SPAN * (step + 1) as f32 / ORBIT_STEPS as f32;
                        let alpha = (step + 1) as f32 / ORBIT_STEPS as f32;
                        paint(window, from, to, alpha * alpha);
                    }
                }
            }
        },
    )
    .size_full()
}

/// How finely the ring's curve is drawn: points per turn round the edge.
const RING_SAMPLES: f32 = 96.0;

/// The point `s` of the way round a `w`×`h` rectangle with corners of radius `r`, starting at the
/// top's middle and going clockwise; `s` wraps, so -0.1 is 0.9.
fn perimeter_point(w: f32, h: f32, r: f32, s: f32) -> (f32, f32) {
    let r = r.min(w / 2.0).min(h / 2.0);
    let (straight_w, straight_h) = (w - 2.0 * r, h - 2.0 * r);
    let arc = PI * r / 2.0;
    let total = 2.0 * straight_w + 2.0 * straight_h + 4.0 * arc;
    if total <= 0.0 {
        return (w / 2.0, h / 2.0);
    }
    let mut d = s.rem_euclid(1.0) * total;
    // The legs in order: half the top, the top-right corner, the right, the bottom-right corner,
    // the bottom, the bottom-left corner, the left, the top-left corner, the other half of the top.
    let corner = |cx: f32, cy: f32, start: f32, d: f32| {
        let angle = start + d / arc.max(f32::EPSILON) * PI / 2.0;
        (cx + r * angle.cos(), cy + r * angle.sin())
    };
    let half = straight_w / 2.0;
    if d <= half {
        return (w / 2.0 + d, 0.0);
    }
    d -= half;
    if d <= arc {
        return corner(w - r, r, -PI / 2.0, d);
    }
    d -= arc;
    if d <= straight_h {
        return (w, r + d);
    }
    d -= straight_h;
    if d <= arc {
        return corner(w - r, h - r, 0.0, d);
    }
    d -= arc;
    if d <= straight_w {
        return (w - r - d, h);
    }
    d -= straight_w;
    if d <= arc {
        return corner(r, h - r, PI / 2.0, d);
    }
    d -= arc;
    if d <= straight_h {
        return (0.0, h - r - d);
    }
    d -= straight_h;
    if d <= arc {
        return corner(r, r, PI, d);
    }
    d -= arc;
    (r + d.min(half), 0.0)
}

#[cfg(test)]
mod tests {
    use super::perimeter_point;

    fn close(a: (f32, f32), b: (f32, f32)) -> bool {
        (a.0 - b.0).abs() < 0.01 && (a.1 - b.1).abs() < 0.01
    }

    #[test]
    fn the_light_starts_at_the_top_and_goes_clockwise_round_the_corners() {
        let (w, h, r) = (32.0, 32.0, 8.0);
        assert!(close(perimeter_point(w, h, r, 0.0), (16.0, 0.0)));
        // A quarter of the way round is the middle of the right side, half the bottom's middle.
        assert!(close(perimeter_point(w, h, r, 0.25), (32.0, 16.0)));
        assert!(close(perimeter_point(w, h, r, 0.5), (16.0, 32.0)));
        assert!(close(perimeter_point(w, h, r, 0.75), (0.0, 16.0)));
        assert!(close(perimeter_point(w, h, r, 1.0), (16.0, 0.0)), "wraps");
        assert!(close(perimeter_point(w, h, r, -0.25), perimeter_point(w, h, r, 0.75)));
    }

    #[test]
    fn every_point_lies_on_the_rounded_edge() {
        let (w, h, r) = (32.0, 20.0, 6.0);
        for step in 0..400 {
            let (x, y) = perimeter_point(w, h, r, step as f32 / 400.0);
            assert!((-0.01..=w + 0.01).contains(&x) && (-0.01..=h + 0.01).contains(&y), "{x},{y}");
            // In a corner's square the point is r from the corner's centre.
            let cx = if x < r {
                Some(r)
            } else if x > w - r {
                Some(w - r)
            } else {
                None
            };
            let cy = if y < r {
                Some(r)
            } else if y > h - r {
                Some(h - r)
            } else {
                None
            };
            if let (Some(cx), Some(cy)) = (cx, cy) {
                let d = ((x - cx).powi(2) + (y - cy).powi(2)).sqrt();
                assert!((d - r).abs() < 0.05, "{x},{y} is {d} from the corner");
            }
        }
    }

    #[test]
    fn a_square_corner_and_a_round_tile_work_too() {
        assert!(close(perimeter_point(10.0, 10.0, 0.0, 0.125), (10.0, 0.0)));
        // r larger than half the side is a circle.
        let (x, y) = perimeter_point(20.0, 20.0, 40.0, 0.25);
        assert!(close((x, y), (20.0, 10.0)));
    }
}
