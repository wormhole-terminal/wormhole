//! The parts of the notice chip that take a click: clear views laid over the capsule while the chip shows a
//! sticky message, one for the whole capsule and one over each button. A view only tells the app's queue
//! where it was pressed (see `platform::notice::Clicks`); what a press means is `crate::notice`'s. Each takes
//! the first click even when the window is not in front, shows the pointing hand over itself, is a button for
//! VoiceOver, and a button's lights up a circle under the pointer.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Instant;

use objc2::rc::Retained;
use objc2::runtime::NSObjectProtocol;
use objc2::{AnyThread as _, DefinedClass as _, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSAccessibility as _, NSAccessibilityButtonRole, NSBezierPath, NSColor, NSCursor, NSEvent,
    NSTrackingArea, NSTrackingAreaOptions, NSView,
};
use objc2_foundation::{NSRect, NSString};
use wormhole_core::view::notice::{Buttons, Press};

/// How much of the label colour the circle under a hovered button takes.
const HOVER_ALPHA: f64 = 0.14;

pub struct ClickIvars {
    press: Rc<dyn Fn(Press, Instant)>,
    /// When the button last went down: a long press is told by when it began, not when it came up.
    down: Cell<Instant>,
    /// What a click on this view is.
    which: Press,
    /// Lights up a circle under the pointer: a button's, not the capsule's.
    lights: bool,
    hovered: Cell<bool>,
}

define_class!(
    // SAFETY: NSView may be subclassed; we override drawing, the mouse, the cursor and VoiceOver's press,
    // keep our state in an ivar and don't implement `Drop`.
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "WormholeNoticeClick"]
    #[ivars = ClickIvars]
    pub struct ClickView;

    impl ClickView {
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            let ivars = self.ivars();
            if ivars.lights && ivars.hovered.get() {
                NSColor::labelColor().colorWithAlphaComponent(HOVER_ALPHA).setFill();
                NSBezierPath::bezierPathWithOvalInRect(self.bounds()).fill();
            }
        }

        // A click on the chip counts at once, whether or not its window is the one in front.
        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool {
            true
        }

        // Taken, so the press is ours until the button comes up.
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, _event: &NSEvent) {
            self.ivars().down.set(Instant::now());
        }

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, event: &NSEvent) {
            let at = self.convertPoint_fromView(event.locationInWindow(), None);
            let bounds = self.bounds();
            let inside = at.x >= bounds.origin.x
                && at.x <= bounds.origin.x + bounds.size.width
                && at.y >= bounds.origin.y
                && at.y <= bounds.origin.y + bounds.size.height;
            if inside {
                self.press();
            }
        }

        // A cursor rect only counts in the window in front; a tracking area also in one that isn't.
        #[unsafe(method(cursorUpdate:))]
        fn cursor_update(&self, _event: &NSEvent) {
            NSCursor::pointingHandCursor().set();
        }

        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, _event: &NSEvent) {
            self.hover(true);
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            self.hover(false);
        }

        #[unsafe(method(accessibilityPerformPress))]
        fn accessibility_perform_press(&self) -> bool {
            self.ivars().down.set(Instant::now());
            self.press();
            true
        }
    }

    unsafe impl NSObjectProtocol for ClickView {}
);

impl ClickView {
    fn press(&self) {
        let ivars = self.ivars();
        (ivars.press)(ivars.which, ivars.down.get());
    }

    /// Put out the circle under the pointer: a view that is hidden gets no `mouseExited:` to do it.
    fn cool(&self) {
        let ivars = self.ivars();
        if ivars.lights && ivars.hovered.replace(false) {
            self.setNeedsDisplay(true);
        }
    }

    fn set_shown(&self, shown: bool) {
        self.setHidden(!shown);
        if !shown {
            self.cool();
        }
    }

    fn hover(&self, hovered: bool) {
        let ivars = self.ivars();
        if ivars.lights && ivars.hovered.replace(hovered) != hovered {
            self.setNeedsDisplay(true);
        }
    }
}

/// A view that calls `press` with `which` (and when the click began) when it is clicked, `label` for VoiceOver; `lights`: a circle
/// shows under the pointer. It has no frame yet.
#[allow(unsafe_code)]
pub fn click_view(
    mtm: MainThreadMarker,
    label: &str,
    which: Press,
    lights: bool,
    press: Rc<dyn Fn(Press, Instant)>,
) -> Retained<ClickView> {
    let this = ClickView::alloc(mtm).set_ivars(ClickIvars {
        press,
        down: Cell::new(Instant::now()),
        which,
        lights,
        hovered: Cell::new(false),
    });
    // SAFETY: `-[NSView initWithFrame:]` is the designated initializer of the superclass, and the ivars
    // are set before it.
    let view: Retained<ClickView> = unsafe { msg_send![super(this), initWithFrame: NSRect::ZERO] };
    view.setAccessibilityElement(true);
    view.setAccessibilityLabel(Some(&NSString::from_str(label)));
    // SAFETY: a constant AppKit defines, read on the main thread.
    view.setAccessibilityRole(Some(unsafe { NSAccessibilityButtonRole }));
    // SAFETY: the owner is the view, which implements `cursorUpdate:`, `mouseEntered:` and `mouseExited:`;
    // the view keeps the area.
    let area = unsafe {
        NSTrackingArea::initWithRect_options_owner_userInfo(
            NSTrackingArea::alloc(),
            NSRect::ZERO,
            NSTrackingAreaOptions::CursorUpdate
                | NSTrackingAreaOptions::MouseEnteredAndExited
                | NSTrackingAreaOptions::ActiveAlways
                | NSTrackingAreaOptions::InVisibleRect,
            Some(&view),
            None,
        )
    };
    view.addTrackingArea(&area);
    view
}

/// The three views over the chip: one over the whole capsule and one over each button, the buttons above it so
/// they take their own clicks. All out of the way until the chip is clickable.
pub struct Presses {
    body: Retained<ClickView>,
    info: Retained<ClickView>,
    close: Retained<ClickView>,
}

impl Presses {
    /// Put the views in `holder`; a click anywhere on them calls `tell`.
    pub fn new(mtm: MainThreadMarker, holder: &NSView, tell: &Rc<dyn Fn(Press, Instant)>) -> Self {
        let view = |which, lights| {
            let view = click_view(mtm, "", which, lights, tell.clone());
            view.setHidden(true);
            holder.addSubview(&view);
            view
        };
        // The body first, so the buttons are over it.
        Self {
            body: view(Press::Body, false),
            info: view(Press::Info, true),
            close: view(Press::Close, true),
        }
    }

    /// The views show while the chip is clickable, a button's only where it is there. A view that goes out
    /// of sight puts out its circle.
    pub fn show(&self, clickable: bool, buttons: Buttons) {
        self.body.set_shown(clickable);
        self.info.set_shown(clickable && buttons.info);
        self.close.set_shown(clickable && buttons.close);
    }

    /// What VoiceOver (and a tooltip) calls the capsule and each button.
    pub fn label(&self, body: &str, info: &str, close: &str) {
        self.body.setAccessibilityLabel(Some(&NSString::from_str(body)));
        for (view, text) in [(&self.info, info), (&self.close, close)] {
            view.setAccessibilityLabel(Some(&NSString::from_str(text)));
            view.setToolTip(Some(&NSString::from_str(text)));
        }
    }

    /// Where the views are, in the panel's space: the capsule, and the buttons that are there.
    pub fn place(&self, capsule: NSRect, info: Option<NSRect>, close: Option<NSRect>) {
        self.body.setFrame(capsule);
        for (view, frame) in [(&self.info, info), (&self.close, close)] {
            if let Some(frame) = frame {
                view.setFrame(frame);
            }
        }
    }

    /// The panel went out of sight under the pointer: no circle stays lit.
    pub fn cool(&self) {
        self.info.cool();
        self.close.cool();
    }
}
