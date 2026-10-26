//! The status tile: a terminal's face and state in one rounded square, drawn like an app's icon:
//! a squircle, black unless an agent has colours of its own (Claude on its orange, Gemini on its
//! blue; the colours are tokens, picked by the agent registry), with a faint sheen across its top
//! and a hairline edge. It is [`TILE`] high beside a two-line row, and [`TILE_MINI`],
//! [`TILE_TITLE`], [`TILE_SMALL`] or [`TILE_DENSE_FOLD`] where a row, a tab, the title row or the
//! status bar is smaller (see [`status_tile_at`]). It holds what the terminal is (an agent's logo
//! at half the tile, its letters, or the terminal symbol) and says what it is doing round it: a
//! light circles its edge while an agent works (grey while a command runs), its edge breathes
//! amber while the agent needs input, and a badge in its corner tells the rest (finished, failed,
//! new output, the bell). Under Reduce Motion the light stands still as a ring and the amber edge
//! holds. The tile is drawn per row each frame: nothing here allocates per frame beyond the
//! elements themselves.

mod badge;
mod edge;
mod face;
mod orbit;
mod ring;
mod transition;

use gpui_kit::prelude::*;
use gpui_kit::*;
use wormhole_core::agents::AgentId;

use self::badge::badge;
use self::face::{Paint, Resolved, resolve};
use self::ring::{light, phase_of};
use super::scale::{TILE, TILE_MINI};
use super::status::StatusGlyph;
use super::status_shift::StatusShift;
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
    status_tile_shifting(id, ui, tile, size, surface, None, cx)
}

/// [`status_tile_on`] while its terminal's status changes to the one it shows, as `shift` says (what it
/// changed from, how it is played, how far along it is): the old light, edge and badge go and the new
/// ones come as the pose of the moment has them. No shift, a status that is not another one, a variant with
/// nothing to say of it and Reduce Motion all draw the tile as it always is.
#[track_caller]
pub fn status_tile_shifting(
    id: impl Into<ElementId>,
    ui: UiColors,
    tile: StatusTile,
    size: f32,
    surface: Hsla,
    shift: Option<StatusShift>,
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
    // The light round the edge, or the amber edge breathing, and the badge: as they are, or the layer
    // that plays the change from the old ones.
    let phase = phase_of(&id);
    let (ring, badge, shifting) = match shift.filter(|shift| !still && shift.is_shown(state)) {
        None => (light(&id, ui, state, (size, corner), phase, still), badge(ui, state, size, surface), None),
        Some(shift) => {
            let shape = transition::Shape { id: id.clone(), ui, size, corner, surface, phase };
            (None, None, Some(transition::shifting(shape, shift, state)))
        }
    };
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
        .children(shifting)
        .into_any_element()
}

/// How far down the tile the sheen reaches, from its full strength at the top.
const SHEEN_REACH: f32 = 0.55;

/// The tile's corner, as a share of its size: rounder than a control, the way macOS draws an
/// app's icon.
const SQUIRCLE: f32 = 0.3;
/// The logo or the terminal symbol, and a monogram's letters, as shares of the tile.
const FACE_SHARE: f32 = 0.5;
const MONOGRAM_SHARE: f32 = 0.36;
