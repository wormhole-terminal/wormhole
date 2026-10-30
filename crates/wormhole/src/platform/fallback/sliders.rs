use crate::platform::controls::{Slider, SliderEvents};
use gpui_kit::{App, Window};

pub struct Sliders;
impl Sliders {
    pub fn new(_: &Window, _: SliderEvents, _: &App) -> Option<Self> {
        None
    }
    pub fn begin(&self) {}
    pub fn place(&self, _: &Slider<'_>) {}
    pub fn finish(&self) {}
    pub fn forget_prefix(&self, _: &str) {}
    pub fn accepts(&self, _: &str) -> bool {
        false
    }
}
