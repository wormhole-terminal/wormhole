//! A row of colour dots as a menu item's own view, the way Finder shows its tags in a file's
//! context menu: nine round buttons in a line (a popover's first may be an empty ring with a slash:
//! no colour), the current one ringed, a highlight under the
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
use crate::platform::menu::Swatches;

/// The row's left margin, a dot's cell (the dot is centred in it), and the row's height.
const INSET: f64 = 12.0;
const CELL: f64 = 22.0;
const HEIGHT: f64 = 28.0;
/// A dot's diameter, a hovered one's, and the ring round the chosen one.
const DOT: f64 = 14.0;
const DOT_HOVER: f64 = 16.0;
const RING: f64 = 20.0;
/// The colour wheel's wedges.
const WEDGES: usize = 6;

/// Which dot was clicked: the row's place in its menu and the dot's in the row.
pub type Chosen = Rc<Cell<Option<(usize, usize)>>>;

pub struct DotIvars {
    rgba: Cell<Option<[f32; 4]>>,
    name: String,
    row: usize,
    index: usize,
    selected: Cell<bool>,
    hovered: Cell<bool>,
    chosen: Chosen,
    /// In a popover: tell the app which dot was clicked, through its queue. A menu has none: its
    /// pop-up reads `chosen` once the loop has returned.
    notify: Option<Rc<dyn Fn(usize)>>,
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
            match ivars.rgba.get() {
                // No colour: no fill, a slash through the ring.
                Some(rgba) if rgba[3] == 0.0 => {
                    let slash = NSBezierPath::bezierPath();
                    let inset = side * 0.15;
                    slash.moveToPoint(NSPoint::new(
                        bounds.origin.x + (bounds.size.width - side) / 2.0 + inset,
                        bounds.origin.y + (bounds.size.height - side) / 2.0 + inset,
                    ));
                    slash.lineToPoint(NSPoint::new(
                        bounds.origin.x + (bounds.size.width + side) / 2.0 - inset,
                        bounds.origin.y + (bounds.size.height + side) / 2.0 - inset,
                    ));
                    NSColor::secondaryLabelColor().setStroke();
                    slash.setLineWidth(1.0);
                    slash.stroke();
                    dot.setLineWidth(1.0);
                    dot.stroke();
                }
                Some(rgba) => {
                    appkit::ns_color(rgba).setFill();
                    dot.fill();
                }
                // No colour of one's own yet: the wheel to pick one from.
                None => {
                    let centre = NSPoint::new(
                        bounds.origin.x + bounds.size.width / 2.0,
                        bounds.origin.y + bounds.size.height / 2.0,
                    );
                    for wedge in 0..WEDGES {
                        let (from, to) = (360.0 / WEDGES as f64 * wedge as f64, 360.0 / WEDGES as f64 * (wedge + 1) as f64);
                        let path = NSBezierPath::bezierPath();
                        path.moveToPoint(centre);
                        path.appendBezierPathWithArcWithCenter_radius_startAngle_endAngle(centre, side / 2.0, from, to);
                        path.closePath();
                        NSColor::colorWithHue_saturation_brightness_alpha(from / 360.0, 0.75, 0.95, 1.0).setFill();
                        path.fill();
                    }
                }
            }
            // A hairline so a pale colour on a pale menu still has an edge.
            NSColor::separatorColor().setStroke();
            dot.setLineWidth(0.5);
            dot.stroke();
            if ivars.selected.get() {
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

        // A stack view lays a view out by its size when it has no constraints of its own.
        #[unsafe(method(intrinsicContentSize))]
        fn intrinsic_content_size(&self) -> NSSize {
            NSSize::new(CELL, CELL)
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

    /// Write the choice down and close the menu; the pop-up runs it afterwards. In a popover tell
    /// the app's queue, and ring this dot instead of the one that was.
    #[allow(unsafe_code)]
    fn choose(&self) {
        let ivars = self.ivars();
        ivars.chosen.set(Some((ivars.row, ivars.index)));
        if let Some(notify) = &ivars.notify {
            // SAFETY: the superview is the row that holds the dot, alive while the dot is.
            if let Some(row) = unsafe { self.superview() } {
                for sibling in row.subviews().iter() {
                    if let Some(dot) = sibling.downcast_ref::<Dot>() {
                        dot.ivars().selected.set(std::ptr::eq(dot, self));
                        dot.setNeedsDisplay(true);
                    }
                }
            }
            notify(ivars.index);
        }
        if let Some(item) = self.enclosingMenuItem() {
            // SAFETY: `menu` is the item's weak back-reference; the menu is alive while it is
            // tracking, and a menu that is gone leaves `None`.
            if let Some(menu) = unsafe { item.menu() } {
                menu.cancelTracking();
            }
        }
    }
}

/// How the row is used: in a menu (no `notify`, a margin like a menu item's) or in a popover (which
/// is told of each click, and has the dot for a colour of one's own).
pub struct Use {
    pub inset: f64,
    pub notify: Option<Rc<dyn Fn(usize)>>,
    /// The colour of one's own: the last dot, which is ringed when it has a colour.
    pub custom: Option<Option<[f32; 4]>>,
}

impl Use {
    pub fn menu() -> Self {
        Self { inset: INSET, notify: None, custom: None }
    }
}

/// The row as one view: a dot per swatch, left to right, and the dot for a colour of one's own
/// when `how.custom` says. `row` is the row's place among the menu's, which `chosen` is told with
/// the dot's.
#[allow(unsafe_code)]
pub fn row_view(
    mtm: MainThreadMarker,
    row: usize,
    swatches: &Swatches,
    chosen: &Chosen,
    how: Use,
) -> Retained<NSView> {
    let count = swatches.swatches.len() + usize::from(how.custom.is_some());
    let width = 2.0 * how.inset + CELL * count as f64;
    let frame = NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(width, HEIGHT));
    let container = NSView::initWithFrame(NSView::alloc(mtm), frame);
    container.setAccessibilityElement(false);
    let named = swatches.swatches.iter().enumerate().map(|(index, swatch)| {
        let rgba = gpui_kit::Rgba::from(swatch.color);
        (
            index,
            Some([rgba.r, rgba.g, rgba.b, rgba.a]),
            swatch.name.to_string(),
            swatches.selected == Some(index),
        )
    });
    let custom = how.custom.map(|color| {
        let name = if color.is_some() { "Custom colour" } else { "Other colour…" };
        (swatches.swatches.len(), color, name.to_string(), color.is_some())
    });
    for (index, rgba, name, selected) in named.chain(custom) {
        let cell = NSRect::new(
            NSPoint::new(how.inset + CELL * index as f64, (HEIGHT - CELL) / 2.0),
            NSSize::new(CELL, CELL),
        );
        let dot = Dot::new(
            mtm,
            cell,
            DotIvars {
                rgba: Cell::new(rgba),
                name,
                row,
                index,
                selected: Cell::new(selected),
                hovered: Cell::new(false),
                chosen: chosen.clone(),
                notify: how.notify.clone(),
            },
        );
        let name = NSString::from_str(&dot.ivars().name);
        dot.setToolTip(Some(&name));
        dot.setAccessibilityElement(true);
        dot.setAccessibilityLabel(Some(&name));
        dot.setAccessibilitySelected(dot.ivars().selected.get());
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

/// The row's size: its width at `inset`, and its height.
pub fn row_size(swatches: &Swatches, custom: bool, inset: f64) -> NSSize {
    let count = swatches.swatches.len() + usize::from(custom);
    NSSize::new(2.0 * inset + CELL * count as f64, HEIGHT)
}
