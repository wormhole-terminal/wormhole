use crate::platform::controls::{ButtonEvents, PushButton};
use gpui_kit::{App, Pixels, Size, Window, px, size};

pub struct Buttons;
impl Buttons {
    pub fn new(_: &Window, _: ButtonEvents, _: &App) -> Option<Self> {
        None
    }
    pub fn size(&self, _: &str) -> Size<Pixels> {
        size(px(80.), px(24.))
    }
    pub fn begin(&self) {}
    pub fn place(&self, _: &PushButton<'_>) {}
    pub fn finish(&self) {}
    pub fn forget_prefix(&self, _: &str) {}
    pub fn accepts(&self, _: &str) -> bool {
        false
    }
}
