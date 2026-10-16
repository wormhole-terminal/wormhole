//! The status tile: a terminal's face and state in one rounded square, as tall as a two-line row's
//! two lines ([`TILE`]), or [`TILE_MINI`] in a tab and the title row. It holds what the terminal
//! is (an agent's logo, its monogram, or the terminal symbol) and says what it is doing round it:
//! a light circles its edge while an agent works (grey while a command runs), its edge breathes
//! amber while the agent needs you, and a badge in its corner tells the rest (finished, failed,
//! new output, the bell). Under Reduce Motion the light stands still as a ring.

use std::f32::consts::PI;

use gpui_kit::prelude::*;
use gpui_kit::*;

use super::motion::{ANIMATION_FPS, PULSE_RANGE, STATUS_PULSE, TILE_ORBIT};
use super::scale::{TILE, TILE_MINI, TextRole};
use super::status::StatusGlyph;
use crate::icons::glyph;
use crate::settings::UiColors;

/// What a [`status_tile`] shows inside.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TileFace {
    /// An agent's logo (`Agent::icon`).
    Logo(&'static str),
    /// An agent without a bundled logo: its two letters.
    Monogram(SharedString),
    /// A terminal without an agent.
    Terminal,
}

/// The parts of a [`status_tile`].
pub struct StatusTile {
    pub face: TileFace,
    /// What the terminal is doing: the same states as its lead glyph.
    pub glyph: StatusGlyph,
    /// On the selected row or tab: the face in the selection's text color.
    pub selected: bool,
    /// [`TILE_MINI`] instead of [`TILE`]: in a tab, the title row.
    pub mini: bool,
}

/// See [`StatusTile`]. `id` keys its animations (a terminal's id), so the light keeps circling
/// when the list around it changes.
#[track_caller]
pub fn status_tile(id: impl Into<ElementId>, ui: UiColors, tile: StatusTile, cx: &App) -> AnyElement {
    let StatusTile { face, glyph: state, selected, mini } = tile;
    let id: ElementId = id.into();
    let size = if mini { TILE_MINI } else { TILE };
    let corner = size * SQUIRCLE;
    let still = cx.reduce_motion();
    let quiet = matches!(state, StatusGlyph::Shell | StatusGlyph::Idle);
    let face_color = match (selected, quiet) {
        (true, _) => ui.selected_text,
        (false, true) => ui.muted,
        (false, false) => ui.text,
    };
    let face_size = size * FACE_SHARE;
    let face_el = match face {
        TileFace::Logo(path) => {
            svg().path(path).size(px(face_size)).text_color(face_color).into_any_element()
        }
        TileFace::Terminal => {
            svg().path(glyph::TERMINAL).size(px(face_size)).text_color(face_color).into_any_element()
        }
        TileFace::Monogram(letters) => div()
            .text_size(px(size * MONOGRAM_SHARE))
            .font_weight(FontWeight::BOLD)
            .text_color(face_color)
            .child(letters)
            .into_any_element(),
    };
    // The light round the edge, or the amber edge breathing.
    let ring = match state {
        StatusGlyph::Working => {
            Some(orbit((id.clone(), "orbit"), ui.agent_working, size, corner, mini, still))
        }
        StatusGlyph::Running => Some(orbit((id.clone(), "orbit"), ui.muted, size, corner, mini, still)),
        StatusGlyph::NeedsYou => Some(breathing_edge((id.clone(), "breathe"), ui, corner, mini, still)),
        _ => None,
    };
    let badge = badge(ui, state, mini);
    div()
        .relative()
        .flex_none()
        .size(px(size))
        .child(
            div()
                .size_full()
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(corner))
                .bg(if selected { ui.selected_inset } else { ui.band })
                .border_color(ui.border)
                .map(|mut d| {
                    let edge = Some(ui.border_width.into());
                    let b = &mut d.style().border_widths;
                    (b.top, b.right, b.bottom, b.left) = (edge, edge, edge, edge);
                    d
                })
                .child(face_el),
        )
        .children(ring)
        .children(badge)
        .into_any_element()
}

/// The tile's corner, as a share of its size: rounder than a control, the way macOS draws an
/// app's icon.
const SQUIRCLE: f32 = 0.3;
/// The logo or the terminal symbol, and a monogram's letters, as shares of the tile.
const FACE_SHARE: f32 = 0.5;
const MONOGRAM_SHARE: f32 = 0.36;
/// The light's line, in a tile and in a mini one.
const ORBIT_STROKE: f32 = 2.0;
const ORBIT_STROKE_MINI: f32 = 1.5;
/// How much of the edge the light covers, and how many steps its tail fades in.
const ORBIT_SPAN: f32 = 0.32;
const ORBIT_STEPS: usize = 12;
/// What is left of the ring under the light: a faint track so the whole edge reads as busy.
const ORBIT_TRACK: f32 = 0.22;
/// The corner badge, in a tile and in a mini one, and the panel-coloured rim that cuts it out.
const BADGE: f32 = 14.0;
const BADGE_MINI: f32 = 10.0;
const BADGE_RIM: f32 = 2.0;
/// How far the badge hangs past the tile's corner.
const BADGE_OUT: f32 = 3.0;

/// A light that circles the tile's edge once every [`TILE_ORBIT`], its tail fading behind it, over
/// a faint track. Still, it is the whole ring at the light's strength.
#[track_caller]
fn orbit(
    id: impl Into<ElementId>,
    color: Hsla,
    size: f32,
    corner: f32,
    mini: bool,
    still: bool,
) -> AnyElement {
    let stroke = if mini { ORBIT_STROKE_MINI } else { ORBIT_STROKE };
    let layer = || div().absolute().top_0().left_0().size_full();
    if still {
        return layer().child(ring_canvas(color, size, corner, stroke, None)).into_any_element();
    }
    layer()
        .with_animation(
            id,
            Animation::new(TILE_ORBIT).repeat().with_max_fps(ANIMATION_FPS),
            move |d, turn| d.child(ring_canvas(color, size, corner, stroke, Some(turn))),
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

/// The edge of a tile whose agent needs you: amber, breathing like the needs-you halo; steady
/// under Reduce Motion.
#[track_caller]
fn breathing_edge(
    id: impl Into<ElementId>,
    ui: UiColors,
    corner: f32,
    mini: bool,
    still: bool,
) -> AnyElement {
    let stroke = px(if mini { ORBIT_STROKE_MINI } else { ORBIT_STROKE });
    let mut edge = div()
        .absolute()
        .top_0()
        .left_0()
        .size_full()
        .rounded(px(corner))
        .bg(ui.attention_tint)
        .border_color(ui.agent_attention);
    let b = &mut edge.style().border_widths;
    (b.top, b.right, b.bottom, b.left) =
        (Some(stroke.into()), Some(stroke.into()), Some(stroke.into()), Some(stroke.into()));
    if still {
        return edge.into_any_element();
    }
    let (low, high) = PULSE_RANGE;
    edge.with_animation(
        id,
        Animation::new(STATUS_PULSE)
            .repeat()
            .with_max_fps(ANIMATION_FPS)
            .with_easing(pulsating_between(low.max(BREATH_LEAST), high)),
        |edge, opacity| edge.opacity(opacity),
    )
    .into_any_element()
}

/// The amber edge never fades further than this, so the tile always reads as waiting.
const BREATH_LEAST: f32 = 0.45;

/// The badge in the tile's bottom-right corner for the states the edge doesn't tell: needs you
/// (!), finished (✓), failed (×), the bell, and new output (a dot).
#[track_caller]
fn badge(ui: UiColors, state: StatusGlyph, mini: bool) -> Option<AnyElement> {
    let size = if mini { BADGE_MINI } else { BADGE };
    let mark = size - 2.0 * BADGE_RIM - 2.0;
    let (fill, inside): (Hsla, Option<AnyElement>) = match state {
        StatusGlyph::NeedsYou => (
            ui.agent_attention,
            Some(
                div()
                    .text_size(px(mark + 2.0))
                    .line_height(px(mark + 2.0))
                    .font_weight(FontWeight::BLACK)
                    .text_color(ui.panel)
                    .child("!")
                    .into_any_element(),
            ),
        ),
        StatusGlyph::Done => (ui.agent_done, Some(badge_mark(glyph::CHECK, mark, ui.panel))),
        StatusGlyph::Failed => (ui.danger, Some(badge_mark(glyph::FAILED, mark, ui.panel))),
        StatusGlyph::Bell => (ui.agent_attention, Some(badge_mark(glyph::BELL, mark, ui.panel))),
        StatusGlyph::Terminal { unread: true } => (ui.primary, None),
        _ => return None,
    };
    // A dot for new output: smaller than the other badges, nothing inside.
    let size = if inside.is_none() { size - BADGE_RIM * 2.0 } else { size };
    let mut badge = div()
        .absolute()
        .right(px(-BADGE_OUT))
        .bottom(px(-BADGE_OUT))
        .size(px(size))
        .flex()
        .items_center()
        .justify_center()
        .rounded_full()
        .bg(fill)
        .border_color(ui.panel)
        .text_caption();
    let rim = Some(px(BADGE_RIM).into());
    let b = &mut badge.style().border_widths;
    (b.top, b.right, b.bottom, b.left) = (rim, rim, rim, rim);
    Some(badge.children(inside).into_any_element())
}

fn badge_mark(path: &'static str, size: f32, color: Hsla) -> AnyElement {
    svg().path(path).size(px(size.max(1.0))).text_color(color).into_any_element()
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
