//! Pictures the app's image loader can't decode.

use std::sync::Arc;

use gpui_kit::*;

/// A HEIC picture as GPUI can draw it. `None` until the system's decoder is wired in.
pub(super) fn from_heic(_bytes: &[u8]) -> Option<Arc<Image>> {
    None
}
