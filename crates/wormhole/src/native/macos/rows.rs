//! A row like a menu's in a popover: its title at the left, a rounded highlight in the accent
//! colour under the pointer, white text on it. AppKit has no such control (a borderless button
//! centres its title and shows no highlight), so it is a small view, as `swatches::Dot` is.
//! Like every control of a popover it only tells the app's queue what happened; see `popover`.

use std::cell::Cell;
use std::rc::Rc;

use objc2::rc::Retained;
use objc2::runtime::NSObjectProtocol;
use objc2::{AnyThread as _, DefinedClass as _, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSAccessibility as _, NSAccessibilityButtonRole, NSBezierPath, NSColor, NSEvent, NSFont, NSTextField,
    NSTrackingArea, NSTrackingAreaOptions, NSView,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

const HEIGHT: f64 = 24.0;
const PAD: f64 = 8.0;
const RADIUS: f64 = 5.0;

pub struct RowIvars {
    label: Retained<NSTextField>,
    enabled: bool,
    hovered: Cell<bool>,
    tag: usize,
    notify: Rc<dyn Fn(usize)>,
}

define_class!(
    // SAFETY: NSView may be subclassed; we override drawing, the mouse and VoiceOver's press, keep
    // our state in ivars and don't implement `Drop`.
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "WormholeActionRow"]
    #[ivars = RowIvars]
    pub struct ActionRow;

    impl ActionRow {
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            let ivars = self.ivars();
            if ivars.hovered.get() && ivars.enabled {
                NSColor::controlAccentColor().setFill();
                NSBezierPath::bezierPathWithRoundedRect_xRadius_yRadius(self.bounds(), RADIUS, RADIUS).fill();
            }
        }

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
                self.press();
            }
        }

        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, _event: &NSEvent) {
            self.hover(true);
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            self.hover(false);
        }

        #[unsafe(method(intrinsicContentSize))]
        fn intrinsic_content_size(&self) -> NSSize {
            NSSize::new(self.ivars().label.frame().size.width + 2.0 * PAD, HEIGHT)
        }

        #[unsafe(method(accessibilityPerformPress))]
        fn accessibility_perform_press(&self) -> bool {
            self.press();
            true
        }
    }

    unsafe impl NSObjectProtocol for ActionRow {}
);

impl ActionRow {
    fn hover(&self, hovered: bool) {
        let ivars = self.ivars();
        if !ivars.enabled {
            return;
        }
        ivars.hovered.set(hovered);
        let color = if hovered { NSColor::selectedMenuItemTextColor() } else { NSColor::labelColor() };
        ivars.label.setTextColor(Some(&color));
        self.setNeedsDisplay(true);
    }

    fn press(&self) {
        let ivars = self.ivars();
        if ivars.enabled {
            (ivars.notify)(ivars.tag);
        }
    }
}

/// A row titled `title`; pressing it calls `notify` with `tag`.
#[allow(unsafe_code)]
pub fn action_row(
    mtm: MainThreadMarker,
    title: &str,
    enabled: bool,
    tag: usize,
    notify: Rc<dyn Fn(usize)>,
) -> Retained<ActionRow> {
    let label = NSTextField::labelWithString(&NSString::from_str(title), mtm);
    label.setFont(Some(&NSFont::systemFontOfSize(13.0)));
    let color = if enabled { NSColor::labelColor() } else { NSColor::disabledControlTextColor() };
    label.setTextColor(Some(&color));
    label.sizeToFit();
    let size = label.frame().size;
    label.setFrameOrigin(NSPoint::new(PAD, ((HEIGHT - size.height) / 2.0).floor()));
    let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(size.width + 2.0 * PAD, HEIGHT));
    let this = ActionRow::alloc(mtm).set_ivars(RowIvars {
        label: label.clone(),
        enabled,
        hovered: Cell::new(false),
        tag,
        notify,
    });
    // SAFETY: `-[NSView initWithFrame:]` is the designated initializer of the superclass, and the
    // ivars are set before it.
    let row: Retained<ActionRow> = unsafe { msg_send![super(this), initWithFrame: frame] };
    row.addSubview(&label);
    row.setAccessibilityElement(true);
    row.setAccessibilityLabel(Some(&NSString::from_str(title)));
    // SAFETY: a constant AppKit defines, read on the main thread.
    row.setAccessibilityRole(Some(unsafe { NSAccessibilityButtonRole }));
    // SAFETY: the owner is the row, which implements `mouseEntered:` and `mouseExited:`; the view
    // keeps the area.
    let area = unsafe {
        NSTrackingArea::initWithRect_options_owner_userInfo(
            NSTrackingArea::alloc(),
            NSRect::ZERO,
            NSTrackingAreaOptions::MouseEnteredAndExited
                | NSTrackingAreaOptions::ActiveAlways
                | NSTrackingAreaOptions::InVisibleRect,
            Some(&row),
            None,
        )
    };
    row.addTrackingArea(&area);
    row
}
