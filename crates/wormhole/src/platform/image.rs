//! Pictures GPUI's own image loader can't decode (HEIC), turned into a format it can by the system's
//! decoders, which also handle what a camera or a phone makes.

use super::os::image as os;

/// The picture in `bytes` (HEIC, HEIF) as TIFF, which GPUI decodes; `None` when the system can't read
/// it. Decodes the whole picture: not for the main thread.
pub fn to_tiff(bytes: &[u8]) -> Option<Vec<u8>> {
    os::to_tiff(bytes)
}
