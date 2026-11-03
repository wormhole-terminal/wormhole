//! `platform::sheet` without native sheets: none can be shown, so the app asks with GPUI's prompt.

use gpui_kit::Window;
use smol::channel::Receiver;

use crate::platform::sheet::{Choice, Question};

pub fn ask_with_checkbox(_: &Window, _: &Question) -> Option<Receiver<Option<bool>>> {
    None
}

pub fn choose(_: &Window, _: &Choice) -> Option<Receiver<Option<usize>>> {
    None
}

pub fn dismiss(_: &Window) {}

#[cfg(feature = "devctl")]
pub fn answer(_: &Window, _: isize, _: Option<bool>) -> bool {
    false
}
