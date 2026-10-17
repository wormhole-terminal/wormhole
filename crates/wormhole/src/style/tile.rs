//! The status tile: a terminal's face and state in one rounded square, drawn like an app's icon:
//! a squircle, black unless an agent has colours of its own (Claude on its orange, Gemini on its
//! blue; the colours are tokens, picked by the agent registry), with a faint sheen across its top
//! and a hairline edge. It is [`TILE`] high beside a two-line row, and [`TILE_MINI`],
//! [`TILE_TITLE`], [`TILE_SMALL`] or [`TILE_DENSE_FOLD`] where a row, a tab, the title row or the
//! status bar is smaller (see [`status_tile_at`]). It holds what the terminal is (an agent's logo
//! at half the tile, its letters, or the terminal symbol) and says what it is doing round it: a
//! light circles its edge while an agent works (grey while a command runs), its edge breathes
//! amber while the agent needs you, and a badge in its corner tells the rest (finished, failed,
//! new output, the bell). Under Reduce Motion the light stands still as a ring and the amber edge
//! holds. The tile is drawn per row each frame: nothing here allocates per frame beyond the
//! elements themselves.

use std::f32::consts::PI;

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::agents::{self, AgentId, TileColor};
use wormhole_core::sidebar::Urgency;

use super::motion::{ANIMATION_FPS, TILE_BREATHE, TILE_BREATHE_LEAST, TILE_ORBIT};
use super::scale::{TILE, TILE_MINI, TextRole};
use super::status::StatusGlyph;
use crate::icons::glyph;
use crate::settings::UiColors;

/// What a [`status_tile`] shows inside.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TileFace {
    /// A logo (`Agent::icon`) on a black tile. [`TileFace::Agent`] is what the sidebar uses; this
    /// stays for a mark that is not a registered agent's.
    #[allow(dead_code)]
    Logo(&'static str),
    /// Two letters on a black tile (see [`TileFace::Logo`]).
    #[allow(dead_code)]
    Monogram(SharedString),
    /// A terminal without an agent.
    Terminal,
    /// An agent: its logo, or its monogram when it has none, on the colour the registry gives it
    /// (`Agent::tile_color`).
    Agent(AgentId),
}

/// The parts of a [`status_tile`].
pub struct StatusTile {
    pub face: TileFace,
    /// What the terminal is doing: the same states as its lead glyph.
    pub glyph: StatusGlyph,
    /// On the selected row or tab: the badge is cut out in the selection's colour, not the panel's.
    pub selected: bool,
    /// [`TILE_MINI`] instead of [`TILE`]: in a tab, the title row. [`status_tile_at`] takes any
    /// size and ignores this.
    pub mini: bool,
}

/// See [`StatusTile`]. `id` keys its animations (a terminal's id), so the light keeps circling
/// when the list around it changes.
#[track_caller]
pub fn status_tile(id: impl Into<ElementId>, ui: UiColors, tile: StatusTile, cx: &App) -> AnyElement {
    let size = if tile.mini { TILE_MINI } else { TILE };
    status_tile_at(id, ui, tile, size, cx)
}

/// [`status_tile`] at `size` points: [`TILE`] in a row, [`TILE_MINI`] in a compact one and a
/// folded head, [`TILE_TITLE`] in the title row, [`TILE_SMALL`] in a tab, a dense row and the
/// status bar, [`TILE_DENSE_FOLD`] in a dense folded head. The cut-out round the badge is the
/// panel's colour, or the selected row's.
#[track_caller]
pub fn status_tile_at(
    id: impl Into<ElementId>,
    ui: UiColors,
    tile: StatusTile,
    size: f32,
    cx: &App,
) -> AnyElement {
    let surface = if tile.selected { ui.selected_row } else { ui.panel };
    status_tile_on(id, ui, tile, size, surface, cx)
}

/// [`status_tile_at`] on a surface of `surface`'s colour (the badge's rim, the amber edge's inside).
#[track_caller]
pub fn status_tile_on(
    id: impl Into<ElementId>,
    ui: UiColors,
    tile: StatusTile,
    size: f32,
    surface: Hsla,
    cx: &App,
) -> AnyElement {
    let StatusTile { face, glyph: state, .. } = tile;
    let id: ElementId = id.into();
    let corner = size * SQUIRCLE;
    let still = cx.reduce_motion();
    let (paint, mark_color, face) = resolve(ui, face);
    let face_size = size * FACE_SHARE;
    let face_el = match face {
        Resolved::Logo(path) => {
            svg().path(path).size(px(face_size)).text_color(mark_color).into_any_element()
        }
        Resolved::Terminal => {
            svg().path(glyph::TERMINAL).size(px(face_size)).text_color(mark_color).into_any_element()
        }
        Resolved::Letters(letters) => div()
            .text_size(px(size * MONOGRAM_SHARE))
            .font_weight(FontWeight::BOLD)
            .text_color(mark_color)
            .child(letters)
            .into_any_element(),
    };
    // The light round the edge, or the amber edge breathing.
    let ring = match state {
        StatusGlyph::Working => Some(orbit((id.clone(), "orbit"), ui.agent_working, size, corner, still)),
        StatusGlyph::Running => Some(orbit((id.clone(), "orbit"), ui.muted, size, corner, still)),
        StatusGlyph::NeedsYou => Some(breathing_edge((id.clone(), "breathe"), ui, size, corner, still)),
        _ => None,
    };
    let badge = badge(ui, state, size, surface);
    let sheen = div().absolute().top_0().left_0().size_full().rounded(px(corner)).bg(linear_gradient(
        180.0,
        linear_color_stop(ui.tile_sheen, 0.0),
        linear_color_stop(ui.tile_sheen.opacity(0.0), SHEEN_REACH),
    ));
    let mut face_box = div()
        .size_full()
        .relative()
        .flex()
        .items_center()
        .justify_center()
        .rounded(px(corner))
        .shadow(vec![BoxShadow {
            color: ui.shadow,
            offset: point(px(0.0), px(1.0)),
            blur_radius: px(1.5),
            spread_radius: px(0.0),
            inset: false,
        }])
        .border_color(ui.tile_edge);
    face_box = match paint {
        Paint::Flat(color) => face_box.bg(color),
        Paint::Gradient(from, to) => {
            face_box.bg(linear_gradient(135.0, linear_color_stop(from, 0.0), linear_color_stop(to, 1.0)))
        }
    };
    let edge = Some(ui.border_width.into());
    let b = &mut face_box.style().border_widths;
    (b.top, b.right, b.bottom, b.left) = (edge, edge, edge, edge);
    div()
        .relative()
        .flex_none()
        .size(px(size))
        .child(face_box.child(sheen).child(face_el))
        .children(ring)
        .children(badge)
        .into_any_element()
}

/// What a tile is filled with.
#[derive(Clone, Copy)]
enum Paint {
    Flat(Hsla),
    /// From the top left to the bottom right.
    Gradient(Hsla, Hsla),
}

/// The face once the agent is looked up: what to draw inside.
enum Resolved {
    Logo(&'static str),
    Letters(SharedString),
    Terminal,
}

/// The tile's fill, the colour of what is drawn on it, and that drawing. Every colour is a token
/// (`UiColors`); the registry only says which agent has colours of its own.
fn resolve(ui: UiColors, face: TileFace) -> (Paint, Hsla, Resolved) {
    let black = Paint::Flat(ui.tile_background);
    match face {
        TileFace::Terminal => (black, ui.tile_mark, Resolved::Terminal),
        TileFace::Logo(path) => (black, ui.tile_mark_brand, Resolved::Logo(path)),
        TileFace::Monogram(letters) => (black, ui.tile_mark_brand, Resolved::Letters(letters)),
        TileFace::Agent(id) => {
            let agent = agents::agent(id);
            let paint = match agent.tile_color() {
                TileColor::Black => black,
                TileColor::Claude => Paint::Flat(ui.agent_claude),
                TileColor::Gemini => Paint::Gradient(ui.agent_gemini, ui.agent_gemini_end),
            };
            let drawing = match agent.icon() {
                Some(path) => Resolved::Logo(path),
                None => Resolved::Letters(agent.monogram.into()),
            };
            (paint, ui.tile_mark_brand, drawing)
        }
    }
}

/// What a tile in this state asks of the person, for choosing which of several terminals a folded
/// group, a tab switcher card or the status bar shows (core's order of urgency).
impl From<StatusGlyph> for Urgency {
    fn from(glyph: StatusGlyph) -> Self {
        match glyph {
            StatusGlyph::NeedsYou => Urgency::NeedsYou,
            StatusGlyph::Failed => Urgency::Failed,
            StatusGlyph::Working => Urgency::Working,
            StatusGlyph::Bell => Urgency::Bell,
            StatusGlyph::Terminal { unread: true } => Urgency::NewOutput,
            StatusGlyph::Running | StatusGlyph::Terminal { unread: false } => Urgency::Running,
            StatusGlyph::Done => Urgency::Done,
            StatusGlyph::Idle | StatusGlyph::Shell => Urgency::Idle,
        }
    }
}

/// How far down the tile the sheen reaches, from its full strength at the top.
const SHEEN_REACH: f32 = 0.55;
/// How far the light and the amber edge sit outside the tile's own edge.
const RING_OUT: f32 = 1.0;

/// The tile's corner, as a share of its size: rounder than a control, the way macOS draws an
/// app's icon.
const SQUIRCLE: f32 = 0.3;
/// The logo or the terminal symbol, and a monogram's letters, as shares of the tile.
const FACE_SHARE: f32 = 0.5;
const MONOGRAM_SHARE: f32 = 0.36;
/// The light's line, in a tile and in a smaller one (below [`LINE_THIN_BELOW`]).
const ORBIT_STROKE: f32 = 2.0;
const ORBIT_STROKE_MINI: f32 = 1.5;
const LINE_THIN_BELOW: f32 = 28.0;
/// How much of the edge the light covers, and how many steps its tail fades in.
const ORBIT_SPAN: f32 = 0.32;
const ORBIT_STEPS: usize = 12;
/// What is left of the ring under the light: a faint track so the whole edge reads as busy.
const ORBIT_TRACK: f32 = 0.22;
/// The corner badge, in a tile and in one below [`LINE_THIN_BELOW`], and the rim in the surface's
/// colour that cuts it out.
const BADGE: f32 = 14.0;
const BADGE_MINI: f32 = 10.0;
const BADGE_RIM: f32 = 2.0;
/// How far the badge hangs past the tile's corner.
const BADGE_OUT: f32 = 3.0;

/// A light that circles the tile's edge once every [`TILE_ORBIT`], its tail fading behind it, over
/// a faint track. Still, it is the whole ring at the light's strength. It runs along the edge's
/// outside, so it never covers the tile's own.
#[track_caller]
fn orbit(id: impl Into<ElementId>, color: Hsla, size: f32, corner: f32, still: bool) -> AnyElement {
    let stroke = line(size);
    let (outer, round) = (size + 2.0 * RING_OUT, corner + RING_OUT);
    let layer = || div().absolute().top(px(-RING_OUT)).left(px(-RING_OUT)).size(px(outer));
    if still {
        return layer().child(ring_canvas(color, outer, round, stroke, None)).into_any_element();
    }
    layer()
        .with_animation(
            id,
            Animation::new(TILE_ORBIT).repeat().with_max_fps(ANIMATION_FPS),
            move |d, turn| d.child(ring_canvas(color, outer, round, stroke, Some(turn))),
        )
        .into_any_element()
}

/// The light's and the edge's line for a tile of `size`.
fn line(size: f32) -> f32 {
    if size < LINE_THIN_BELOW { ORBIT_STROKE_MINI } else { ORBIT_STROKE }
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

/// The edge of a tile whose agent needs you: amber, breathing down to [`TILE_BREATHE_LEAST`] every
/// [`TILE_BREATHE`]; steady under Reduce Motion.
#[track_caller]
fn breathing_edge(id: impl Into<ElementId>, ui: UiColors, size: f32, corner: f32, still: bool) -> AnyElement {
    let stroke = px(line(size));
    let mut edge = div()
        .absolute()
        .top(px(-RING_OUT))
        .left(px(-RING_OUT))
        .size(px(size + 2.0 * RING_OUT))
        .rounded(px(corner + RING_OUT))
        .border_color(ui.agent_attention);
    let b = &mut edge.style().border_widths;
    (b.top, b.right, b.bottom, b.left) =
        (Some(stroke.into()), Some(stroke.into()), Some(stroke.into()), Some(stroke.into()));
    if still {
        return edge.into_any_element();
    }
    edge.with_animation(
        id,
        Animation::new(TILE_BREATHE)
            .repeat()
            .with_max_fps(ANIMATION_FPS)
            .with_easing(pulsating_between(TILE_BREATHE_LEAST, 1.0)),
        |edge, opacity| edge.opacity(opacity),
    )
    .into_any_element()
}

/// The badge in the tile's bottom-right corner for the states the edge doesn't tell: needs you
/// (!), finished (✓), failed (×), the bell, and new output (a dot).
#[track_caller]
fn badge(ui: UiColors, state: StatusGlyph, tile: f32, surface: Hsla) -> Option<AnyElement> {
    let size = if tile < LINE_THIN_BELOW { BADGE_MINI } else { BADGE };
    let mark = size - 2.0 * BADGE_RIM - 2.0;
    let (fill, inside): (Hsla, Option<AnyElement>) = match state {
        StatusGlyph::NeedsYou => (
            ui.agent_attention,
            Some(
                div()
                    .text_size(px(mark + 2.0))
                    .line_height(px(mark + 2.0))
                    .font_weight(FontWeight::BLACK)
                    .text_color(surface)
                    .child("!")
                    .into_any_element(),
            ),
        ),
        StatusGlyph::Done => (ui.agent_done, Some(badge_mark(glyph::CHECK, mark, surface))),
        StatusGlyph::Failed => (ui.danger, Some(badge_mark(glyph::FAILED, mark, surface))),
        StatusGlyph::Bell => (ui.agent_attention, Some(badge_mark(glyph::BELL, mark, surface))),
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
        .border_color(surface)
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
