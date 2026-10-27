//! `platform::sheet` without native sheets: none can be shown, so the app asks with GPUI's prompt.

use gpui_kit::Window;
use smol::channel::Receiver;

use crate::platform::sheet::Question;

pub fn ask_with_checkbox(_: &Window, _: &Question) -> Option<Receiver<Option<bool>>> {
    None
}

#[cfg(feature = "devctl")]
pub fn answer(_: &Window, _: isize) -> bool {
    false
}
