//! No glyph outlines off macOS.

use super::Seg;

pub(crate) struct Face;

impl Face {
    pub fn named(_family: &str, _bold: bool, _italic: bool) -> Option<Face> {
        None
    }

    pub fn for_char(&self, _c: char) -> Option<Face> {
        None
    }

    pub fn glyph_of(&self, _c: char) -> Option<u16> {
        None
    }

    pub fn outline(&self, _glyph: u32, _size: f32, _scale: (f32, f32), _shift_x: f32) -> Option<Vec<Seg>> {
        None
    }
}
