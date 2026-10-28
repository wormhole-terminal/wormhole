//! `platform::picker` without a panel: GPUI has no window over another with a system material, so no picker
//! is shown off macOS.

use gpui_kit::{App, Window};
use wormhole_core::view::panes::Rect;

use crate::platform::picker::{Events, View};

pub struct Panel;

impl Panel {
    pub fn new(_window: &Window, _opaque: bool, _events: Events, _cx: &App) -> Option<Self> {
        None
    }

    pub fn show(&mut self, _view: &View<'_>, _rect: Rect, _cx: &mut App) {}

    pub fn update(&mut self, _view: &View<'_>, _rect: Rect) {}

    pub fn close(self, _cx: &mut App) {}

    #[cfg(feature = "devctl")]
    pub fn info(&self) -> Option<crate::platform::picker::Info> {
        None
    }
}
