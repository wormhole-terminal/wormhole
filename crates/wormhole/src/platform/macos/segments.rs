//! System segmented controls hosted in a GPUI window. AppKit actions are queued for GPUI.
use std::cell::RefCell;
use std::collections::HashMap;

use gpui_kit::{App, Window, px};
use objc2::rc::Retained;
use objc2::runtime::NSObjectProtocol;
use objc2::{DefinedClass as _, MainThreadMarker, MainThreadOnly, define_class, msg_send, sel};
use objc2_app_kit::{NSAccessibility as _, NSSegmentedControl};
use objc2_foundation::{NSObject, NSRect, NSString};

use super::{appkit, native_controls::NativeViews};
use crate::platform::controls::{SegmentEvent, SegmentEvents, SegmentMetrics, SegmentOption, Segmented};

pub struct TargetIvars {
    id: String,
    events: SegmentEvents,
}
define_class!(
    // SAFETY: NSObject has no subclassing requirements; the action queues a value.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "WormholeNativeSegmentsTarget"]
    #[ivars = TargetIvars]
    struct Target;
    impl Target {
        #[unsafe(method(changed:))]
        fn changed(&self, sender: &NSSegmentedControl) {
            if let Ok(selected) = usize::try_from(sender.selectedSegment()) {
                self.ivars().events.try_send(SegmentEvent { id: self.ivars().id.clone(), selected }).ok();
            }
        }
    }
    unsafe impl NSObjectProtocol for Target {}
);
impl Target {
    #[allow(unsafe_code)]
    fn new(mtm: MainThreadMarker, id: &str, events: SegmentEvents) -> Retained<Self> {
        let object = Self::alloc(mtm).set_ivars(TargetIvars { id: id.to_owned(), events });
        // SAFETY: NSObject's designated initializer.
        unsafe { msg_send![super(object), init] }
    }
}

struct Entry {
    control: Retained<NSSegmentedControl>,
    _target: Retained<Target>,
    options: Vec<SegmentOption>,
}

pub struct Segments {
    views: NativeViews<Entry>,
    events: SegmentEvents,
    metrics: RefCell<HashMap<Vec<SegmentOption>, SegmentMetrics>>,
}

fn configure(control: &NSSegmentedControl, options: &[SegmentOption]) {
    control.setSegmentCount(options.len() as isize);
    for (index, option) in options.iter().enumerate() {
        let index = index as isize;
        control.setLabel_forSegment(&NSString::from_str(&option.label), index);
        let image = option.symbol.and_then(|symbol| appkit::symbol(symbol, &option.label));
        control.setImage_forSegment(image.as_deref(), index);
    }
}

impl Segments {
    pub fn new(window: &Window, events: SegmentEvents, cx: &App) -> Option<Self> {
        Some(Self { views: NativeViews::new(window, cx)?, events, metrics: RefCell::new(HashMap::new()) })
    }

    /// Ask AppKit for the control's size. Use individual fitting widths to give the GPUI
    /// accessibility and keyboard proxies the same segment boundaries.
    pub fn metrics(&self, options: &[SegmentOption]) -> SegmentMetrics {
        if let Some(metrics) = self.metrics.borrow().get(options) {
            return metrics.clone();
        }
        let control = NSSegmentedControl::initWithFrame(self.views.mtm.alloc(), NSRect::ZERO);
        configure(&control, options);
        control.sizeToFit();
        let frame = control.frame().size;
        let mut widths = Vec::with_capacity(options.len());
        for option in options {
            let single = NSSegmentedControl::initWithFrame(self.views.mtm.alloc(), NSRect::ZERO);
            configure(&single, std::slice::from_ref(option));
            single.sizeToFit();
            widths.push(single.frame().size.width as f32);
        }
        let sum: f32 = widths.iter().sum();
        let factor = if sum > 0.0 { frame.width as f32 / sum } else { 1.0 };
        let metrics = SegmentMetrics {
            width: px(frame.width as f32),
            height: px(frame.height as f32),
            widths: widths.into_iter().map(|w| px(w * factor)).collect(),
        };
        self.metrics.borrow_mut().insert(options.to_vec(), metrics.clone());
        metrics
    }
    pub fn begin(&self) {
        self.views.begin();
    }
    pub fn set_suspended(&self, suspended: bool) {
        self.views.set_suspended(suspended);
    }
    #[allow(unsafe_code)]
    pub fn place(&self, item: &Segmented<'_>) {
        self.views.place(
            item.id,
            item.bounds,
            item.clip,
            item.appearance,
            |container| {
                let control = NSSegmentedControl::initWithFrame(self.views.mtm.alloc(), NSRect::ZERO);
                control.setRefusesFirstResponder(true);
                control.setAccessibilityElement(false);
                let target = Target::new(self.views.mtm, item.id, self.events.clone());
                // SAFETY: the entry retains the target and its selector has the AppKit action signature.
                unsafe {
                    control.setTarget(Some(&*target));
                    control.setAction(Some(sel!(changed:)));
                }
                container.addSubview(&control);
                Entry { control, _target: target, options: Vec::new() }
            },
            |entry, _, frame| {
                if entry.options != item.options {
                    configure(&entry.control, item.options);
                    entry.options = item.options.to_vec();
                }
                entry.control.setEnabled(item.enabled);
                entry.control.setSelectedSegment(item.selected as isize);
                entry.control.setFrame(frame);
            },
        );
    }
    pub fn finish(&self) {
        self.views.finish();
    }
    pub fn accepts(&self, id: &str) -> bool {
        self.views.accepts(id, |entry| entry.control.isEnabled())
    }
    pub fn forget_prefix_except(&self, prefix: &str, keep: Option<&str>) {
        self.views.forget_prefix_except(prefix, keep);
    }
}
