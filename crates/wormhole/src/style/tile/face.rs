//! What the tile is filled with and what it shows inside: the face resolved against the agent
//! registry. Every colour is a token but an agent's own, which its registry entry holds
//! (`agents::Tile`). Must not draw anything itself.

use gpui_kit::*;
use wormhole_core::agents::{self, Tile};

use super::TileFace;
use crate::settings::UiColors;

/// What a tile is filled with.
#[derive(Clone, Copy)]
pub(super) enum Paint {
    Flat(Hsla),
    /// From the top left to the bottom right.
    Gradient(Hsla, Hsla),
}

/// The face once the agent is looked up: what to draw inside.
pub(super) enum Resolved {
    Logo(&'static str),
    Letters(SharedString),
    Terminal,
}

/// The tile's fill, the colour of what is drawn on it, and that drawing. The theme's colours are
/// tokens (`UiColors`); an agent's own colour comes from the registry, the same in every theme.
pub(super) fn resolve(ui: UiColors, face: TileFace) -> (Paint, Hsla, Resolved) {
    let black = Paint::Flat(ui.tile_background);
    match face {
        TileFace::Terminal => (black, ui.tile_mark, Resolved::Terminal),
        TileFace::Logo(path) => (black, ui.tile_mark_brand, Resolved::Logo(path)),
        TileFace::Monogram(letters) => (black, ui.tile_mark_brand, Resolved::Letters(letters)),
        TileFace::Agent(id) => {
            let agent = agents::agent(id);
            let paint = match agent.tile {
                Tile::Black => black,
                Tile::Solid(color) => Paint::Flat(rgb(color).into()),
                Tile::Gradient(from, to) => Paint::Gradient(rgb(from).into(), rgb(to).into()),
            };
            let drawing = match agent.icon() {
                Some(path) => Resolved::Logo(path),
                None => Resolved::Letters(agent.monogram.into()),
            };
            (paint, ui.tile_mark_brand, drawing)
        }
    }
}
