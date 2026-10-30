use crate::platform::controls::{Stepper, StepperEvents};
use gpui_kit::{App, Pixels, Size, Window, px, size};
pub struct Steppers;
impl Steppers {
    pub fn new(_: &Window, _: StepperEvents, _: &App) -> Option<Self> {
        None
    }
    pub fn size(&self) -> Size<Pixels> {
        size(px(19.), px(27.))
    }
    pub fn begin(&self) {}
    pub fn place(&self, _: &Stepper<'_>) {}
    pub fn finish(&self) {}
    pub fn forget_prefix(&self, _: &str) {}
    pub fn accepts(&self, _: &str) -> bool {
        false
    }
}
