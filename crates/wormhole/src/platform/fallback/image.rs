//! `platform::image` without a system decoder: the picture can't be shown.

use crate::platform::image::{Decoded, Refused};

pub fn decode(_bytes: &[u8]) -> Result<Decoded, Refused> {
    Err(Refused::Unreadable)
}
