//! AppKit switches hosted above a GPUI view. Values stay with the application; actions go to a
//! queue. Each switch sits in a clipping view so scrolling cannot expose it outside its row.
use gpui_kit::{App, Pixels, Size, Window, px, size};
use objc2::rc::Retained;
use objc2::runtime::NSObjectProtocol;
use objc2::{DefinedClass as _, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{NSAccessibility as _, NSButton, NSButtonType, NSSwitch};
use objc2_foundation::{NSObject, NSRect, NSString};

use super::native_controls::NativeViews;
use crate::platform::controls::{Events, Switch, SwitchEvent};

pub struct TargetIvars {
    id: String,
    events: Events,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements and the action only queues a value.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "WormholeNativeSwitchTarget"]
    #[ivars = TargetIvars]
    struct Target;

    impl Target {
        #[unsafe(method(changed:))]
        fn changed(&self, sender: &NSSwitch) {
            self.ivars().events.try_send(SwitchEvent { id: self.ivars().id.clone(), on: sender.state() != 0 }).ok();
        }
    }
    unsafe impl NSObjectProtocol for Target {}
);

impl Target {
    #[allow(unsafe_code)]
    fn new(mtm: MainThreadMarker, id: &str, events: Events) -> Retained<Self> {
        let object = Self::alloc(mtm).set_ivars(TargetIvars { id: id.to_owned(), events });
        // SAFETY: NSObject's designated initializer.
        unsafe { msg_send![super(object), init] }
    }
}

struct Entry {
    control: Retained<NSSwitch>,
    // AppKit's target is weak; retain it as long as the control is present.
    _target: Retained<Target>,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "WormholeNativeCheckboxTarget"]
    #[ivars = TargetIvars]
    struct CheckboxTarget;
    impl CheckboxTarget {
        #[unsafe(method(changed:))]
        fn changed(&self, sender: &NSButton) {
            self.ivars().events.try_send(SwitchEvent { id: self.ivars().id.clone(), on: sender.state() != 0 }).ok();
        }
    }
    unsafe impl NSObjectProtocol for CheckboxTarget {}
);
impl CheckboxTarget {
    #[allow(unsafe_code)]
    fn new(mtm: MainThreadMarker, id: &str, events: Events) -> Retained<Self> {
        let object = Self::alloc(mtm).set_ivars(TargetIvars { id: id.to_owned(), events });
        // SAFETY: NSObject's designated initializer.
        unsafe { msg_send![super(object), init] }
    }
}
struct CheckboxEntry {
    control: Retained<NSButton>,
    _target: Retained<CheckboxTarget>,
}

pub struct Switches {
    views: NativeViews<Entry>,
    events: Events,
    size: Size<Pixels>,
    checkboxes: NativeViews<CheckboxEntry>,
    checkbox_size: Size<Pixels>,
}

impl Switches {
    pub fn new(window: &Window, events: Events, cx: &App) -> Option<Self> {
        let views = NativeViews::new(window, cx)?;
        let mtm = views.mtm;
        let prototype = NSSwitch::initWithFrame(mtm.alloc(), NSRect::ZERO);
        prototype.sizeToFit();
        let fitting = prototype.frame().size;
        let checkboxes = NativeViews::new(window, cx)?;
        let checkbox = NSButton::initWithFrame(mtm.alloc(), NSRect::ZERO);
        checkbox.setButtonType(NSButtonType::Switch);
        checkbox.setTitle(&NSString::from_str(""));
        checkbox.sizeToFit();
        let checkbox_frame = checkbox.frame().size;
        Some(Self {
            views,
            events,
            size: size(px(fitting.width as f32), px(fitting.height as f32)),
            checkboxes,
            checkbox_size: size(px(checkbox_frame.width as f32), px(checkbox_frame.height as f32)),
        })
    }

    pub fn size(&self) -> Size<Pixels> {
        self.size
    }

    pub fn begin(&self) {
        self.views.begin();
        self.checkboxes.begin();
    }

    /// Hide the current views while their cached surface is covered or off screen.
    pub fn set_suspended(&self, suspended: bool) {
        self.views.set_suspended(suspended);
        self.checkboxes.set_suspended(suspended);
    }

    pub fn checkbox_size(&self) -> Size<Pixels> {
        self.checkbox_size
    }

    #[allow(unsafe_code)]
    pub fn place_checkbox(&self, item: &Switch<'_>) {
        self.checkboxes.place(
            item.id,
            item.bounds,
            item.clip,
            item.appearance,
            |container| {
                let control = NSButton::initWithFrame(self.checkboxes.mtm.alloc(), NSRect::ZERO);
                control.setButtonType(NSButtonType::Switch);
                control.setTitle(&NSString::from_str(""));
                control.setRefusesFirstResponder(true);
                control.setAccessibilityElement(false);
                let target = CheckboxTarget::new(self.checkboxes.mtm, item.id, self.events.clone());
                // SAFETY: the target is retained by the entry while AppKit holds its weak reference.
                unsafe {
                    control.setTarget(Some(&*target));
                    control.setAction(Some(sel!(changed:)));
                }
                container.addSubview(&control);
                CheckboxEntry { control, _target: target }
            },
            |entry, _, frame| {
                entry.control.setEnabled(item.enabled);
                entry.control.setState(if item.on { 1 } else { 0 });
                entry.control.setFrame(frame);
            },
        );
    }

    #[allow(unsafe_code)]
    pub fn place(&self, item: &Switch<'_>) {
        self.views.place(
            item.id,
            item.bounds,
            item.clip,
            item.appearance,
            |container| {
                let control = NSSwitch::initWithFrame(self.views.mtm.alloc(), NSRect::ZERO);
                // GPUI owns tab order and accessibility for the row. AppKit still handles pointer
                // interaction and draws the system control; the matching GPUI switch owns focus.
                control.setRefusesFirstResponder(true);
                control.setAccessibilityElement(false);
                let target = Target::new(self.views.mtm, item.id, self.events.clone());
                // SAFETY: the entry owns the target for the lifetime of the control; the selector
                // is an AppKit target-action method with a control sender.
                unsafe {
                    control.setTarget(Some(&*target));
                    control.setAction(Some(sel!(changed:)));
                }
                container.addSubview(&control);
                Entry { control, _target: target }
            },
            |entry, _, frame| {
                entry.control.setAccessibilityLabel(Some(&NSString::from_str(item.label)));
                entry.control.setEnabled(item.enabled);
                entry.control.setState(if item.on { 1 } else { 0 });
                entry.control.setFrame(frame);
            },
        );
    }

    /// Hide controls not placed in this frame, such as rows scrolled out or a previous page.
    pub fn finish(&self) {
        self.views.finish();
        self.checkboxes.finish();
    }

    /// Reject queued actions after a row has been hidden or disabled.
    pub fn accepts(&self, id: &str) -> bool {
        self.views.accepts(id, |entry| entry.control.isEnabled())
            || self.checkboxes.accepts(id, |entry| entry.control.isEnabled())
    }

    /// Remove views belonging to a completed transient surface without accumulating one set
    /// per opening. Removal is deferred because AppKit can send notifications while detaching.
    pub fn forget_prefix(&self, prefix: &str) {
        self.views.forget_prefix(prefix);
        self.checkboxes.forget_prefix(prefix);
    }
}
