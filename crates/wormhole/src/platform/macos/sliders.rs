//! AppKit sliders. Actions are queued until the GPUI entity borrow is released.
use super::native_controls::NativeViews;
use crate::platform::controls::{Slider, SliderEvent, SliderEvents};
use gpui_kit::{App, Window};
use objc2::rc::Retained;
use objc2::runtime::NSObjectProtocol;
use objc2::{DefinedClass as _, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{NSAccessibility as _, NSSlider};
use objc2_foundation::{NSObject, NSRect};

pub struct TargetIvars {
    id: String,
    events: SliderEvents,
}
define_class!(
    // SAFETY: NSObject has no subclassing requirements.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "WormholeNativeSliderTarget"]
    #[ivars = TargetIvars]
    struct Target;
    impl Target {
        #[unsafe(method(changed:))]
        fn changed(&self, sender: &NSSlider) {
            self.ivars().events.try_send(SliderEvent {
                id: self.ivars().id.clone(), value: sender.doubleValue()
            }).ok();
        }
    }
    unsafe impl NSObjectProtocol for Target {}
);
impl Target {
    #[allow(unsafe_code)]
    fn new(mtm: MainThreadMarker, id: &str, events: SliderEvents) -> Retained<Self> {
        let object = Self::alloc(mtm).set_ivars(TargetIvars { id: id.to_owned(), events });
        // SAFETY: NSObject's designated initializer.
        unsafe { msg_send![super(object), init] }
    }
}
struct Entry {
    control: Retained<NSSlider>,
    _target: Retained<Target>,
}
pub struct Sliders {
    views: NativeViews<Entry>,
    events: SliderEvents,
}
impl Sliders {
    pub fn new(window: &Window, events: SliderEvents, cx: &App) -> Option<Self> {
        Some(Self { views: NativeViews::new(window, cx)?, events })
    }
    pub fn begin(&self) {
        self.views.begin();
    }
    #[allow(unsafe_code)]
    pub fn place(&self, item: &Slider<'_>) {
        self.views.place(
            item.id,
            item.bounds,
            item.clip,
            item.appearance,
            |container| {
                let control = NSSlider::initWithFrame(self.views.mtm.alloc(), NSRect::ZERO);
                control.setContinuous(true);
                control.setRefusesFirstResponder(true);
                control.setAccessibilityElement(false);
                let target = Target::new(self.views.mtm, item.id, self.events.clone());
                // SAFETY: the entry owns the target for the lifetime of the control.
                unsafe {
                    control.setTarget(Some(&*target));
                    control.setAction(Some(sel!(changed:)));
                }
                container.addSubview(&control);
                Entry { control, _target: target }
            },
            |entry, _, frame| {
                entry.control.setMinValue(item.min);
                entry.control.setMaxValue(item.max);
                entry.control.setDoubleValue(item.value);
                entry.control.setEnabled(item.enabled);
                entry.control.setFrame(frame);
            },
        );
    }
    pub fn finish(&self) {
        self.views.finish();
    }
    pub fn forget_prefix(&self, prefix: &str) {
        self.views.forget_prefix(prefix);
    }
    pub fn accepts(&self, id: &str) -> bool {
        self.views.accepts(id, |entry| entry.control.isEnabled())
    }
}
