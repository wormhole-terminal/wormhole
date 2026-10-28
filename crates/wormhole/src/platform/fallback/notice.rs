//! `platform::notice` without a panel: GPUI has no window over another with a system material, so no
//! chip is shown off macOS.

use gpui_kit::{App, Window};
use wormhole_core::view::motion::notice::Entrance;
use wormhole_core::view::notice::Spot;

use crate::platform::notice::{Clicks, Content};

pub struct Chip;

impl Chip {
    pub fn new(_window: &Window, _opaque: bool, _clicks: Clicks) -> Option<Self> {
        None
    }

    pub fn show(
        &mut self,
        _content: &Content<'_>,
        _spot: Spot,
        _entrance: Entrance,
        _clickable: bool,
        _cx: &mut App,
    ) {
    }

    pub fn swap(&mut self, _content: &Content<'_>, _spot: Spot, _clickable: bool) {}

    pub fn hide(&mut self, _entrance: Entrance) {}

    pub fn set_clickable(&mut self, _clickable: bool) {}

    pub fn set_hidden(&mut self, _hidden: bool, _cx: &mut App) {}

    pub fn restyle(&mut self, _content: &Content<'_>) {}

    pub fn place(&mut self, _spot: Spot) {}

    pub fn close(self, _cx: &mut App) {}

    #[cfg(feature = "devctl")]
    pub fn info(&self) -> Option<crate::platform::notice::Info> {
        None
    }
}

#[cfg(feature = "devctl")]
pub fn force_material(_: Option<crate::platform::notice::Material>) {}
