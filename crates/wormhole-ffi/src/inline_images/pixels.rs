//! The pixels of one picture, by the key a placement record carries.

use wormhole_term::Pty;

use crate::terminal::Terminal;

/// A decoded picture: straight (not premultiplied) RGBA, row-major, `width * height * 4` bytes.
#[derive(Debug, Clone, uniffi::Record)]
pub struct InlineImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

#[uniffi::export]
impl Terminal {
    /// The picture a placement's `key` names. `None` while it is still being decoded (the engine
    /// wakes the view when it is done, and the next frame is the moment to ask again) and for a
    /// picture the engine's store has let go of (its budget evicts the least recently drawn).
    pub fn inline_image(&self, key: u32) -> Option<InlineImage> {
        self.with_pty(|pty: &Pty| {
            let image = pty.images().get(key)?;
            Some(InlineImage { width: image.width, height: image.height, rgba: image.rgba.clone() })
        })
        .flatten()
    }
}
