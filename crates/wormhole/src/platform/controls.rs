//! Native controls hosted in a GPUI window. The app owns their values; AppKit reports requests
//! through a queue so it never calls application state while GPUI is drawing.

use gpui_kit::{Bounds, Pixels, px};
use wormhole_core::theme::tokens::Appearance;

pub use super::os::buttons::Buttons;
pub use super::os::controls::Switches;
pub use super::os::segments::Segments;
pub use super::os::sliders::Sliders;
pub use super::os::steppers::Steppers;

pub type ButtonEvents = smol::channel::Sender<String>;
pub struct PushButton<'a> {
    pub id: &'a str,
    pub label: &'a str,
    pub tooltip: Option<&'a str>,
    pub enabled: bool,
    pub primary: bool,
    pub destructive: bool,
    pub bounds: Bounds<Pixels>,
    pub clip: Bounds<Pixels>,
    pub appearance: Appearance,
}

#[derive(Clone, Debug)]
pub struct SliderEvent {
    pub id: String,
    pub value: f64,
}
pub type SliderEvents = smol::channel::Sender<SliderEvent>;
pub struct Slider<'a> {
    pub id: &'a str,
    pub value: f64,
    pub min: f64,
    pub max: f64,
    pub enabled: bool,
    pub bounds: Bounds<Pixels>,
    pub clip: Bounds<Pixels>,
    pub appearance: Appearance,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StepperEvent {
    pub id: String,
    pub delta: i32,
}
pub type StepperEvents = smol::channel::Sender<StepperEvent>;
pub struct Stepper<'a> {
    pub id: &'a str,
    pub enabled: bool,
    pub bounds: Bounds<Pixels>,
    pub clip: Bounds<Pixels>,
    pub appearance: Appearance,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SegmentEvent {
    pub id: String,
    pub selected: usize,
}
pub type SegmentEvents = smol::channel::Sender<SegmentEvent>;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct SegmentOption {
    pub label: String,
    pub symbol: Option<&'static str>,
}

pub struct Segmented<'a> {
    pub id: &'a str,
    pub options: &'a [SegmentOption],
    pub selected: usize,
    pub enabled: bool,
    pub bounds: Bounds<Pixels>,
    pub clip: Bounds<Pixels>,
    pub appearance: Appearance,
}

#[derive(Clone, Debug)]
pub struct SegmentMetrics {
    pub width: Pixels,
    pub height: Pixels,
    pub widths: Vec<Pixels>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SwitchEvent {
    pub id: String,
    pub on: bool,
}

pub type Events = smol::channel::Sender<SwitchEvent>;

/// The model and geometry supplied for a switch in one frame.
pub struct Switch<'a> {
    pub id: &'a str,
    pub label: &'a str,
    pub on: bool,
    pub enabled: bool,
    pub bounds: Bounds<Pixels>,
    pub clip: Bounds<Pixels>,
    pub appearance: Appearance,
}

/// A view is clipped to the same rectangle as the element that requested it.
pub(crate) fn visible(bounds: Bounds<Pixels>, clip: Bounds<Pixels>) -> Option<Bounds<Pixels>> {
    let visible = bounds.intersect(&clip);
    (visible.size.width > px(0.) && visible.size.height > px(0.)).then_some(visible)
}

/// A covered surface keeps its geometry but cannot show or accept input. Stale views never
/// return when the cover is removed.
#[cfg(any(target_os = "macos", test))]
pub(crate) fn generation_visible(suspended: bool, entry: u64, current: u64) -> bool {
    !suspended && entry == current
}

#[cfg(test)]
mod tests {
    use super::{generation_visible, visible};
    use gpui_kit::{Bounds, point, px, size};

    #[test]
    fn native_controls_are_clipped_and_fully_hidden_outside_the_scroll_region() {
        let bounds = Bounds::new(point(px(10.), px(20.)), size(px(40.), px(20.)));
        let clip = Bounds::new(point(px(0.), px(30.)), size(px(100.), px(100.)));
        let shown = visible(bounds, clip).unwrap();
        assert_eq!(shown.origin, point(px(10.), px(30.)));
        assert_eq!(shown.size, size(px(40.), px(10.)));
        let elsewhere = Bounds::new(point(px(100.), px(100.)), size(px(20.), px(20.)));
        assert!(visible(bounds, elsewhere).is_none());
    }

    #[test]
    fn suspension_does_not_restore_views_from_an_older_frame() {
        assert!(generation_visible(false, 7, 7));
        assert!(!generation_visible(true, 7, 7));
        assert!(!generation_visible(false, 6, 7));
        assert!(!generation_visible(true, 6, 7));
    }
}
