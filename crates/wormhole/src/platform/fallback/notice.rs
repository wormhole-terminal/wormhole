//! `platform::notice` without a panel: GPUI has no window over another with a system material, so no
//! chip is shown off macOS.

use gpui_kit::Window;
use wormhole_core::view::motion::notice::Entrance;

use crate::platform::notice::Content;

pub struct Chip;

impl Chip {
    pub fn new(_window: &Window) -> Option<Self> {
        None
    }

    pub fn show(&mut self, _content: &Content<'_>, _top_inset: f32, _entrance: Entrance) {}

    pub fn swap(&mut self, _content: &Content<'_>) {}

    pub fn hide(&mut self, _entrance: Entrance) {}

    pub fn place(&mut self, _top_inset: f32) {}

    #[cfg(feature = "devctl")]
    pub fn info(&self) -> Option<crate::platform::notice::Info> {
        None
    }
}

#[cfg(feature = "devctl")]
pub fn force_material(_: Option<crate::platform::notice::Material>) {}
