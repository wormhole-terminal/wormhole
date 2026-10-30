//! AppKit push buttons hosted in a GPUI window.
use super::native_controls::NativeViews;
use crate::platform::controls::{ButtonEvents, PushButton};
use gpui_kit::{App, Pixels, Size, Window, px, size};
use objc2::rc::Retained;
use objc2::runtime::NSObjectProtocol;
use objc2::{DefinedClass as _, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{NSAccessibility as _, NSBezelStyle, NSButton, NSColor};
use objc2_foundation::{NSObject, NSRect, NSString};
use std::cell::RefCell;
use std::collections::HashMap;

pub struct TargetIvars {
    id: String,
    events: ButtonEvents,
}
define_class!(
    // SAFETY: NSObject has no subclassing requirements; the action only enqueues its id.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "WormholeNativeButtonTarget"]
    #[ivars = TargetIvars]
    struct Target;
    impl Target {
        #[unsafe(method(pressed:))]
        fn pressed(&self, _: &NSButton) {
            self.ivars().events.try_send(self.ivars().id.clone()).ok();
        }
    }
    unsafe impl NSObjectProtocol for Target {}
);
impl Target {
    #[allow(unsafe_code)]
    fn new(mtm: MainThreadMarker, id: &str, events: ButtonEvents) -> Retained<Self> {
        let object = Self::alloc(mtm).set_ivars(TargetIvars { id: id.to_owned(), events });
        // SAFETY: NSObject's designated initializer.
        unsafe { msg_send![super(object), init] }
    }
}
struct Entry {
    control: Retained<NSButton>,
    _target: Retained<Target>,
}
pub struct Buttons {
    views: NativeViews<Entry>,
    events: ButtonEvents,
    sizes: RefCell<HashMap<String, Size<Pixels>>>,
}
impl Buttons {
    pub fn new(window: &Window, events: ButtonEvents, cx: &App) -> Option<Self> {
        Some(Self { views: NativeViews::new(window, cx)?, events, sizes: RefCell::new(HashMap::new()) })
    }
    pub fn size(&self, label: &str) -> Size<Pixels> {
        if let Some(size) = self.sizes.borrow().get(label) {
            return *size;
        }
        let control = NSButton::initWithFrame(self.views.mtm.alloc(), NSRect::ZERO);
        control.setBezelStyle(NSBezelStyle::Push);
        control.setTitle(&NSString::from_str(label));
        control.sizeToFit();
        let frame = control.frame().size;
        let result = size(px(frame.width as f32), px(frame.height as f32));
        self.sizes.borrow_mut().insert(label.to_string(), result);
        result
    }
    pub fn begin(&self) {
        self.views.begin();
    }
    #[allow(unsafe_code)]
    pub fn place(&self, item: &PushButton<'_>) {
        self.views.place(
            item.id,
            item.bounds,
            item.clip,
            item.appearance,
            |container| {
                let control = NSButton::initWithFrame(self.views.mtm.alloc(), NSRect::ZERO);
                control.setBezelStyle(NSBezelStyle::Push);
                control.setRefusesFirstResponder(true);
                control.setAccessibilityElement(false);
                let target = Target::new(self.views.mtm, item.id, self.events.clone());
                // SAFETY: the entry retains the target for the control's lifetime.
                unsafe {
                    control.setTarget(Some(&*target));
                    control.setAction(Some(sel!(pressed:)));
                }
                container.addSubview(&control);
                Entry { control, _target: target }
            },
            |entry, _, frame| {
                entry.control.setTitle(&NSString::from_str(item.label));
                entry.control.setToolTip(item.tooltip.map(NSString::from_str).as_deref());
                entry.control.setEnabled(item.enabled);
                entry.control.setHasDestructiveAction(item.destructive);
                entry.control.setBezelColor(item.primary.then(|| NSColor::controlAccentColor()).as_deref());
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
