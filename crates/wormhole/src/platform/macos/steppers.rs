//! AppKit steppers; the setting owns its value, so each action reports a relative change.
use super::native_controls::NativeViews;
use crate::platform::controls::{Stepper, StepperEvent, StepperEvents};
use gpui_kit::{App, Pixels, Size, Window, px, size};
use objc2::rc::Retained;
use objc2::runtime::NSObjectProtocol;
use objc2::{DefinedClass as _, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{NSAccessibility as _, NSStepper};
use objc2_foundation::{NSObject, NSRect};

pub struct TargetIvars {
    id: String,
    events: StepperEvents,
}
define_class!(
    // SAFETY: NSObject has no subclassing requirements; actions only queue a relative step.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "WormholeNativeStepperTarget"]
    #[ivars = TargetIvars]
    struct Target;
    impl Target {
        #[unsafe(method(changed:))]
        fn changed(&self, sender: &NSStepper) {
            let value = sender.doubleValue();
            sender.setDoubleValue(0.0);
            let delta = if value > 0.0 { 1 } else if value < 0.0 { -1 } else { return };
            self.ivars().events.try_send(StepperEvent { id: self.ivars().id.clone(), delta }).ok();
        }
    }
    unsafe impl NSObjectProtocol for Target {}
);
impl Target {
    #[allow(unsafe_code)]
    fn new(mtm: MainThreadMarker, id: &str, events: StepperEvents) -> Retained<Self> {
        let object = Self::alloc(mtm).set_ivars(TargetIvars { id: id.to_owned(), events });
        // SAFETY: NSObject's designated initializer.
        unsafe { msg_send![super(object), init] }
    }
}
struct Entry {
    control: Retained<NSStepper>,
    _target: Retained<Target>,
}
pub struct Steppers {
    views: NativeViews<Entry>,
    events: StepperEvents,
    size: Size<Pixels>,
}
impl Steppers {
    pub fn new(window: &Window, events: StepperEvents, cx: &App) -> Option<Self> {
        let views = NativeViews::new(window, cx)?;
        let prototype = NSStepper::initWithFrame(views.mtm.alloc(), NSRect::ZERO);
        prototype.sizeToFit();
        let frame = prototype.frame().size;
        Some(Self { views, events, size: size(px(frame.width as f32), px(frame.height as f32)) })
    }
    pub fn size(&self) -> Size<Pixels> {
        self.size
    }
    pub fn begin(&self) {
        self.views.begin();
    }
    #[allow(unsafe_code)]
    pub fn place(&self, item: &Stepper<'_>) {
        self.views.place(
            item.id,
            item.bounds,
            item.clip,
            item.appearance,
            |container| {
                let control = NSStepper::initWithFrame(self.views.mtm.alloc(), NSRect::ZERO);
                control.setMinValue(-1.0);
                control.setMaxValue(1.0);
                control.setIncrement(1.0);
                control.setAutorepeat(true);
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
