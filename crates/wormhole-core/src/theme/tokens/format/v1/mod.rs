//! Version 1 of the theme file: `version`, `id`, `name`, a `base` of copied values and the
//! person's own blocks on top, `variants` for a theme with a light and a dark appearance.
//! [`read`] turns its JSON into the resolver's model; [`write_v1`] is the other way.

mod read;
mod write;

pub(super) use read::read;
pub(super) use write::expression_text;
pub use write::write_v1;
