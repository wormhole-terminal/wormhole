//! Portable platforms keep the GPUI segmented control.
use crate::platform::controls::{SegmentEvents, SegmentMetrics, SegmentOption, Segmented};
use gpui_kit::{App, Window};

pub struct Segments;
impl Segments {
    pub fn new(_: &Window, _: SegmentEvents, _: &App) -> Option<Self> {
        None
    }
    pub fn metrics(&self, _: &[SegmentOption]) -> SegmentMetrics {
        unreachable!("no native host")
    }
    pub fn begin(&self) {}
    pub fn set_suspended(&self, _: bool) {}
    pub fn place(&self, _: &Segmented<'_>) {}
    pub fn finish(&self) {}
    pub fn accepts(&self, _: &str) -> bool {
        false
    }
    pub fn forget_prefix_except(&self, _: &str, _: Option<&str>) {}
}
