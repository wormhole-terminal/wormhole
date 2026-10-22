//! A font face as the outline code needs it: made from a family and its slant and weight, asked
//! for another face that has a character, and for a glyph's outline. CoreText has it on macOS;
//! everywhere else there is no face, and double-width rows keep their unstretched glyphs.

#[cfg(not(target_os = "macos"))]
pub(crate) use super::fallback::font_face::Face;
#[cfg(target_os = "macos")]
pub(crate) use super::macos::font_face::Face;

/// One piece of an outline, in device pixels from the glyph's origin, y down.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Seg {
    Move(f32, f32),
    Line(f32, f32),
    Quad([f32; 2], [f32; 2]),
    Cubic([f32; 2], [f32; 2], [f32; 2]),
    Close,
}
