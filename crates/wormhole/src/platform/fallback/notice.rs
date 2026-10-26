//! `platform::notice` without a panel: GPUI has no window over another with a system material, so no
//! chip is shown off macOS.

use gpui_kit::{App, Window};
use wormhole_core::view::motion::notice::Entrance;
use wormhole_core::view::notice::Spot;

use crate::platform::notice::Content;

pub struct Chip;

impl Chip {
    pub fn new(_window: &Window, _opaque: bool) -> Option<Self> {
        None
    }

    pub fn show(&mut self, _content: &Content<'_>, _spot: Spot, _entrance: Entrance, _cx: &mut App) {}

    pub fn swap(&mut self, _content: &Content<'_>, _spot: Spot) {}

    pub fn hide(&mut self, _entrance: Entrance) {}

    pub fn place(&mut self, _spot: Spot) {}

    pub fn close(self, _cx: &mut App) {}

    #[cfg(feature = "devctl")]
    pub fn info(&self) -> Option<crate::platform::notice::Info> {
        None
    }
}

#[cfg(feature = "devctl")]
pub fn force_material(_: Option<crate::platform::notice::Material>) {}
