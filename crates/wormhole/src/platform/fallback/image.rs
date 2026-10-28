//! `platform::image` without a system decoder: the picture can't be shown.

pub fn to_tiff(_bytes: &[u8]) -> Option<Vec<u8>> {
    None
}
