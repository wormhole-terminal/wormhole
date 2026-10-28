//! The part of the notice chip that takes a click: a clear view laid over the capsule while the chip shows
//! a sticky message. It only tells the app's queue that it was pressed (see `platform::notice::Clicks`);
//! what a press means is `crate::notice`'s. It takes the first click even when the window is not in front,
//! shows the pointing hand over itself, and is a button for VoiceOver.

use std::rc::Rc;

use objc2::rc::Retained;
use objc2::runtime::NSObjectProtocol;
use objc2::{AnyThread as _, DefinedClass as _, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSAccessibility as _, NSAccessibilityButtonRole, NSCursor, NSEvent, NSTrackingArea,
    NSTrackingAreaOptions, NSView,
};
use objc2_foundation::{NSRect, NSString};

pub struct ClickIvars {
    press: Rc<dyn Fn()>,
}

define_class!(
    // SAFETY: NSView may be subclassed; we override the mouse, the cursor and VoiceOver's press, keep our
    // state in an ivar and don't implement `Drop`.
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "WormholeNoticeClick"]
    #[ivars = ClickIvars]
    pub struct ClickView;

    impl ClickView {
        // A click on the chip counts at once, whether or not its window is the one in front.
        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool {
            true
        }

        // Taken, so the press is ours until the button comes up.
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, _event: &NSEvent) {}

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, event: &NSEvent) {
            let at = self.convertPoint_fromView(event.locationInWindow(), None);
            let bounds = self.bounds();
            let inside = at.x >= bounds.origin.x
                && at.x <= bounds.origin.x + bounds.size.width
                && at.y >= bounds.origin.y
                && at.y <= bounds.origin.y + bounds.size.height;
            if inside {
                (self.ivars().press)();
            }
        }

        // A cursor rect only counts in the window in front; a tracking area also in one that isn't.
        #[unsafe(method(cursorUpdate:))]
        fn cursor_update(&self, _event: &NSEvent) {
            NSCursor::pointingHandCursor().set();
        }

        #[unsafe(method(accessibilityPerformPress))]
        fn accessibility_perform_press(&self) -> bool {
            (self.ivars().press)();
            true
        }
    }

    unsafe impl NSObjectProtocol for ClickView {}
);

/// A view that calls `press` when it is clicked, `label` for VoiceOver. It has no frame yet.
#[allow(unsafe_code)]
pub fn click_view(mtm: MainThreadMarker, label: &str, press: Rc<dyn Fn()>) -> Retained<ClickView> {
    let this = ClickView::alloc(mtm).set_ivars(ClickIvars { press });
    // SAFETY: `-[NSView initWithFrame:]` is the designated initializer of the superclass, and the ivars
    // are set before it.
    let view: Retained<ClickView> = unsafe { msg_send![super(this), initWithFrame: NSRect::ZERO] };
    view.setAccessibilityElement(true);
    view.setAccessibilityLabel(Some(&NSString::from_str(label)));
    // SAFETY: a constant AppKit defines, read on the main thread.
    view.setAccessibilityRole(Some(unsafe { NSAccessibilityButtonRole }));
    // SAFETY: the owner is the view, which implements `cursorUpdate:`; the view keeps the area.
    let area = unsafe {
        NSTrackingArea::initWithRect_options_owner_userInfo(
            NSTrackingArea::alloc(),
            NSRect::ZERO,
            NSTrackingAreaOptions::CursorUpdate
                | NSTrackingAreaOptions::ActiveAlways
                | NSTrackingAreaOptions::InVisibleRect,
            Some(&view),
            None,
        )
    };
    view.addTrackingArea(&area);
    view
}
