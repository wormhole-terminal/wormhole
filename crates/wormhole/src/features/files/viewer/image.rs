//! Turning a picture's bytes into what GPUI draws, without trusting the file: the system's decoder reads
//! the size the file declares and refuses an absurd one, scales what it decodes to at most
//! `platform::image::MAX_SIDE` on a side, and gives the first frame only (`platform::image`). The pixels
//! are handed to GPUI as a `RenderImage` that this app owns, so GPUI keeps no decode of its own, and
//! the one atlas tile it draws from is dropped when the picture is replaced or the viewer goes
//! ([`Files::release_image`](super::super::Files)).

use std::sync::Arc;

use gpui_kit::*;

use crate::platform::image::{Refused, decode};

/// The picture in `bytes`, or why it can't be shown. Blocks while it decodes: not for the main thread.
pub(super) fn picture(bytes: &[u8]) -> Result<Arc<RenderImage>, Refused> {
    let decoded = decode(bytes)?;
    // GPUI's frames are BGRA with straight alpha, as the terminal's inline images are.
    let mut bgra = decoded.rgba;
    for pixel in bgra.as_chunks_mut::<4>().0 {
        pixel.swap(0, 2);
    }
    let buffer =
        image::ImageBuffer::from_raw(decoded.width, decoded.height, bgra).ok_or(Refused::Unreadable)?;
    Ok(Arc::new(RenderImage::new(vec![image::Frame::new(buffer)])))
}
