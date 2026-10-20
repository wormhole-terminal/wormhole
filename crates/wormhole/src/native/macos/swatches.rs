//! A row of colour dots as a menu item's own view, the way Finder shows its tags in a file's
//! context menu: nine round buttons in a line, the current one ringed, a highlight under the
//! pointer, a tooltip with the colour's name and the name for VoiceOver. One click picks and closes
//! the menu.
//!
//! A view can't run the choice itself: AppKit is still inside the menu's loop, where GPUI's `App`
//! is not to be touched (see `menu`). The dot only writes down which one was clicked in a cell it
//! shares with the pop-up, and cancels the menu's tracking; the pop-up reads the cell once the
//! loop has returned and runs `on_pick` in a window update of its own.

use std::cell::Cell;
use std::rc::Rc;

use objc2::rc::Retained;
use objc2::runtime::NSObjectProtocol;
use objc2::{AnyThread as _, DefinedClass as _, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSAccessibility as _, NSAccessibilityButtonRole, NSBezierPath, NSColor, NSEvent, NSTrackingArea,
    NSTrackingAreaOptions, NSView,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};

use super::appkit;
use crate::native::menu::Swatches;

/// The row's left margin, a dot's cell (the dot is centred in it), and the row's height.
const INSET: f64 = 12.0;
const CELL: f64 = 22.0;
const HEIGHT: f64 = 28.0;
/// A dot's diameter, a hovered one's, and the ring round the chosen one.
const DOT: f64 = 14.0;
const DOT_HOVER: f64 = 16.0;
const RING: f64 = 20.0;

/// Which dot was clicked: the row's place in its menu and the dot's in the row.
pub type Chosen = Rc<Cell<Option<(usize, usize)>>>;

pub struct DotIvars {
    rgba: [f32; 4],
    name: String,
    row: usize,
    index: usize,
    selected: bool,
    hovered: Cell<bool>,
    chosen: Chosen,
}

define_class!(
    // SAFETY: NSView may be subclassed; we override only drawing, the mouse and VoiceOver's press,
    // keep our state in ivars and don't implement `Drop`.
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "WormholeSwatchDot"]
    #[ivars = DotIvars]
    pub struct Dot;

    impl Dot {
        #[unsafe(method(drawRect:))]
        fn draw_rect(&self, _dirty: NSRect) {
            let ivars = self.ivars();
            let bounds = self.bounds();
            let around = |side: f64| {
                let x = bounds.origin.x + (bounds.size.width - side) / 2.0;
                let y = bounds.origin.y + (bounds.size.height - side) / 2.0;
                NSRect::new(NSPoint::new(x, y), NSSize::new(side, side))
            };
            let side = if ivars.hovered.get() { DOT_HOVER } else { DOT };
            let dot = NSBezierPath::bezierPathWithOvalInRect(around(side));
            appkit::ns_color(ivars.rgba).setFill();
            dot.fill();
            // A hairline so a pale colour on a pale menu still has an edge.
            NSColor::separatorColor().setStroke();
            dot.setLineWidth(0.5);
            dot.stroke();
            if ivars.selected {
                // Inside the cell, with the stroke's own width left.
                let ring = NSBezierPath::bezierPathWithOvalInRect(around(RING - 1.5));
                NSColor::labelColor().setStroke();
                ring.setLineWidth(1.5);
                ring.stroke();
            }
        }

        // A click may be the first thing a menu's view gets.
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
                self.choose();
            }
        }

        #[unsafe(method(mouseEntered:))]
        fn mouse_entered(&self, _event: &NSEvent) {
            self.ivars().hovered.set(true);
            self.setNeedsDisplay(true);
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            self.ivars().hovered.set(false);
            self.setNeedsDisplay(true);
        }

        // VoiceOver's press on the button (and the keyboard through it).
        #[unsafe(method(accessibilityPerformPress))]
        fn accessibility_perform_press(&self) -> bool {
            self.choose();
            true
        }
    }

    unsafe impl NSObjectProtocol for Dot {}
);

impl Dot {
    #[allow(unsafe_code)]
    fn new(mtm: MainThreadMarker, frame: NSRect, ivars: DotIvars) -> Retained<Self> {
        let this = Self::alloc(mtm).set_ivars(ivars);
        // SAFETY: `-[NSView initWithFrame:]` is the designated initializer of the superclass, and
        // the ivars are set before it.
        unsafe { msg_send![super(this), initWithFrame: frame] }
    }

    /// Write the choice down and close the menu; the pop-up runs it afterwards.
    #[allow(unsafe_code)]
    fn choose(&self) {
        let ivars = self.ivars();
        ivars.chosen.set(Some((ivars.row, ivars.index)));
        if let Some(item) = self.enclosingMenuItem() {
            // SAFETY: `menu` is the item's weak back-reference; the menu is alive while it is
            // tracking, and a menu that is gone leaves `None`.
            if let Some(menu) = unsafe { item.menu() } {
                menu.cancelTracking();
            }
        }
    }
}

#[allow(unsafe_code)]
/// The row as one view for a menu item: a dot per swatch, left to right. `row` is the row's place
/// among the menu's, which `chosen` is told with the dot's.
pub fn row_view(mtm: MainThreadMarker, row: usize, swatches: &Swatches, chosen: &Chosen) -> Retained<NSView> {
    let width = 2.0 * INSET + CELL * swatches.swatches.len() as f64;
    let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, HEIGHT));
    let container = NSView::initWithFrame(NSView::alloc(mtm), frame);
    container.setAccessibilityElement(false);
    for (index, swatch) in swatches.swatches.iter().enumerate() {
        let rgba = gpui_kit::Rgba::from(swatch.color);
        let cell = NSRect::new(
            NSPoint::new(INSET + CELL * index as f64, (HEIGHT - CELL) / 2.0),
            NSSize::new(CELL, CELL),
        );
        let dot = Dot::new(
            mtm,
            cell,
            DotIvars {
                rgba: [rgba.r, rgba.g, rgba.b, rgba.a],
                name: swatch.name.to_string(),
                row,
                index,
                selected: swatches.selected == Some(index),
                hovered: Cell::new(false),
                chosen: chosen.clone(),
            },
        );
        let name = NSString::from_str(&dot.ivars().name);
        dot.setToolTip(Some(&name));
        dot.setAccessibilityElement(true);
        dot.setAccessibilityLabel(Some(&name));
        dot.setAccessibilitySelected(dot.ivars().selected);
        // SAFETY: a constant AppKit defines, read on the main thread.
        dot.setAccessibilityRole(Some(unsafe { NSAccessibilityButtonRole }));
        // The area follows the view's bounds, so the highlight needs no layout code.
        // SAFETY: the owner is the dot, which implements `mouseEntered:` and `mouseExited:`;
        // AppKit keeps the area weakly pointing at it, and the view keeps the area.
        let area = unsafe {
            NSTrackingArea::initWithRect_options_owner_userInfo(
                NSTrackingArea::alloc(),
                NSRect::ZERO,
                NSTrackingAreaOptions::MouseEnteredAndExited
                    | NSTrackingAreaOptions::ActiveAlways
                    | NSTrackingAreaOptions::InVisibleRect,
                Some(&dot),
                None,
            )
        };
        dot.addTrackingArea(&area);
        container.addSubview(&dot);
    }
    container
}
