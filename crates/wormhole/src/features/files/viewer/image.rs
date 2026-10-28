//! Pictures the app's image loader can't decode.

use std::sync::Arc;

use gpui_kit::*;

/// A HEIC picture as GPUI can draw it: the system decodes it and hands it back as TIFF. `None` when the
/// system can't read it. Decodes the whole picture: not for the main thread.
pub(super) fn from_heic(bytes: &[u8]) -> Option<Arc<Image>> {
    let tiff = crate::platform::image::to_tiff(bytes)?;
    Some(Arc::new(Image::from_bytes(ImageFormat::Tiff, tiff)))
}
