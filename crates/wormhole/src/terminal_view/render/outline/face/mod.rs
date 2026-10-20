//! A font face as the outline code needs it: made from a family and its slant and weight, asked
//! for another face that has a character, and for a glyph's outline. CoreText has it on macOS;
//! everywhere else there is no face, and double-width rows keep their unstretched glyphs.

#[cfg(target_os = "macos")]
mod coretext;
#[cfg(not(target_os = "macos"))]
mod none;

#[cfg(target_os = "macos")]
pub(super) use coretext::Face;
#[cfg(not(target_os = "macos"))]
pub(super) use none::Face;
