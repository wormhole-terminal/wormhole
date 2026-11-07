//! Inline pictures for the Swift view (kitty, iTerm2 and Sixel, which the engine already decodes):
//! where each one sits goes into the packed frame (`section`), its pixels are fetched once by key
//! (`pixels`). Swift draws them as textures; nothing here decodes or places anything itself.

mod pixels;
mod placeholders;
mod section;

pub use pixels::InlineImage;
pub(crate) use placeholders::from_cells as placeholder_placements;
pub(crate) use section::PlacementsSent;
