//! A portable window has no hosted system controls yet; callers keep their GPUI fallback.
use crate::platform::controls::{Events, Switch};
use gpui_kit::{App, Pixels, Size, Window, px, size};

pub struct Switches;

impl Switches {
    pub fn checkbox_size(&self) -> Size<Pixels> {
        size(px(16.), px(16.))
    }
    pub fn place_checkbox(&self, _: &Switch<'_>) {}
    pub fn new(_: &Window, _: Events, _: &App) -> Option<Self> {
        None
    }
    pub fn begin(&self) {}
    pub fn set_suspended(&self, _: bool) {}
    pub fn place(&self, _: &Switch<'_>) {}
    pub fn finish(&self) {}
    pub fn forget_prefix(&self, _: &str) {}
    pub fn size(&self) -> Size<Pixels> {
        size(px(54.), px(24.))
    }
    pub fn accepts(&self, _: &str) -> bool {
        false
    }
}
