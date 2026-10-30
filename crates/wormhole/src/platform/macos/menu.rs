//! A [`Menu`] as an `NSMenu`, popped up in the window's view: the system's own look, VoiceOver,
//! type-to-select, the keyboard and the dark and light of the Mac.
//!
//! `popUpMenuPositioningItem:atLocation:inView:` runs AppKit's own event loop until the menu
//! closes. GPUI keeps its `App` borrowed while it runs a handler, an update or a deferred
//! callback, and every frame, timer and task that fires meanwhile borrows it again, so calling
//! it from inside any of them panics ("already borrowed") or deadlocks a lease. [`show`] therefore
//! only prepares the menu and hands the pop-up to a foreground task, which GPUI polls from the
//! run loop with nothing borrowed; the loop AppKit runs inside it keeps the other tasks going.
//! The chosen item's callback and the menu's `on_close` then run in a window update of their own,
//! after the loop has returned.

use std::cell::Cell;
use std::rc::Rc;

use block2::RcBlock;
use gpui_kit::*;
use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Bool, NSObjectProtocol as _};
use objc2::{AnyThread, DefinedClass as _, MainThreadMarker, sel};
use objc2_app_kit::{
    NSAppearanceCustomization as _, NSAttributedStringNSStringDrawing as _, NSBezierPath, NSColor,
    NSControlStateValueMixed, NSControlStateValueOn, NSFont, NSFontAttributeName,
    NSForegroundColorAttributeName, NSImage, NSMenu, NSMenuItem, NSView,
};
use objc2_foundation::{NSData, NSMutableAttributedString, NSPoint, NSRange, NSRect, NSSize, NSString};

use super::appkit;
use super::swatches::{self, Chosen};
use crate::platform::menu::{Callback, Check, Entry, Icon, Item, Menu, Pick};

/// The pictures in front of titles are this many points square, as the system's own menus have
/// them.
const IMAGE_SIDE: f64 = 16.0;

/// What `build` collects for the pop-up to look up afterwards: the callbacks of the items (an item's
/// tag is the place of its callback), the `on_pick` of each row of colour dots, and where a dot
/// writes down that it was clicked.
struct Parts {
    callbacks: Vec<Callback>,
    picks: Vec<Pick>,
    chosen: Chosen,
}

thread_local! {
    /// Whether a menu is open (or about to be): a second right-click during that moment, or a
    /// menu asked for by a task already queued, is dropped instead of nesting two loops.
    static OPEN: Cell<bool> = const { Cell::new(false) };
}

#[allow(unsafe_code)] // `define_class!` declares an Objective-C class; objc2 checks the signatures.
mod target {
    use std::cell::Cell;

    use objc2::rc::Retained;
    use objc2::runtime::NSObjectProtocol;
    use objc2::{DefinedClass as _, MainThreadMarker, MainThreadOnly, define_class, msg_send};
    use objc2_app_kit::NSMenuItem;
    use objc2_foundation::NSObject;

    pub struct Ivars {
        /// The tag of the item chosen, or -1.
        pub chosen: Cell<isize>,
    }

    define_class!(
        // SAFETY: NSObject has no subclassing requirements, and we don't implement `Drop`.
        #[unsafe(super(NSObject))]
        #[thread_kind = MainThreadOnly]
        #[name = "WormholePopUpMenuTarget"]
        #[ivars = Ivars]
        pub struct Target;

        impl Target {
            #[unsafe(method(menuItemClicked:))]
            fn menu_item_clicked(&self, sender: &NSMenuItem) {
                self.ivars().chosen.set(sender.tag());
            }
        }

        unsafe impl NSObjectProtocol for Target {}
    );

    impl Target {
        pub fn new(mtm: MainThreadMarker) -> Retained<Self> {
            let this = Self::alloc(mtm).set_ivars(Ivars { chosen: Cell::new(-1) });
            // SAFETY: `-[NSObject init]` is the designated initializer of the superclass.
            unsafe { msg_send![super(this), init] }
        }
    }
}

use target::Target;

/// Pop `menu` up at `at` (window coordinates) in `window`'s view, run what is chosen. Gives the
/// menu back when the window has no AppKit view to pop it up in, for the fallback to draw.
pub fn show(menu: Menu, at: Point<Pixels>, window: &mut Window, cx: &mut App) -> Result<(), Menu> {
    let Some(view) = appkit::ns_view(window) else { return Err(menu) };
    if OPEN.with(Cell::get) {
        // The caller may already have marked its trigger open. A dropped menu must
        // still close that state, outside the update that attempted to show it.
        if let Some(on_close) = menu.on_close {
            let handle = window.window_handle();
            cx.defer(move |cx| {
                let _ = handle.update(cx, |_, window, cx| on_close(window, cx));
            });
        }
        return Ok(());
    }
    OPEN.with(|open| open.set(true));
    let handle = window.window_handle();
    cx.spawn(async move |cx| {
        let chosen = pop_up(&view, at, &menu);
        OPEN.with(|open| open.set(false));
        cx.update(move |cx| {
            handle
                .update(cx, move |_, window, cx| {
                    if let Some(on_close) = &menu.on_close {
                        on_close(window, cx);
                    }
                    if let Some(run) = chosen {
                        run(window, cx);
                    }
                    // GPUI saw no mouse-up for the press that opened the menu and the window
                    // idles after the loop: draw it again.
                    window.refresh();
                })
                .ok();
        });
    })
    .detach();
    Ok(())
}

/// devctl: build `menu` as the real `NSMenu` without showing it (a background instance must not
/// take the keys), and say what AppKit has: each item's title, and for a row of colour dots each
/// dot's tooltip, VoiceOver label and ring. With `press` the dot of that place is pressed as
/// VoiceOver does, and which one the menu recorded as chosen is told. `None` off the main thread.
#[cfg(any(test, feature = "devctl"))]
pub fn inspect(menu: &Menu, press: Option<usize>) -> Option<String> {
    use objc2_app_kit::NSAccessibility as _;

    let mtm = MainThreadMarker::new()?;
    let target = Target::new(mtm);
    let mut parts = Parts { callbacks: Vec::new(), picks: Vec::new(), chosen: Rc::default() };
    let ns_menu = build(mtm, menu, &target, &mut parts, menu.appearance);
    let mut out = String::new();
    let appearance = ns_menu.appearance().map(|appearance| appearance.name().to_string());
    out.push_str(&format!("appearance: {}\n", appearance.as_deref().unwrap_or("system")));
    for item in ns_menu.itemArray() {
        if item.isSeparatorItem() {
            out.push_str("---\n");
        } else if let Some(view) = item.view() {
            out.push_str("colours\n");
            for (ix, dot) in view.subviews().iter().enumerate() {
                let tip = dot.toolTip().map(|tip| tip.to_string()).unwrap_or_default();
                let label = dot.accessibilityLabel().map(|label| label.to_string()).unwrap_or_default();
                let ring = if dot.isAccessibilitySelected() { " [x]" } else { "" };
                out.push_str(&format!("  {ix} tooltip={tip:?} voiceover={label:?}{ring}\n"));
                if press == Some(ix) {
                    dot.accessibilityPerformPress();
                }
            }
        } else {
            let icon = item.image().map(|image| {
                let (ink, colors) = image_ink(&image);
                format!(
                    " [icon {}x{} ink {}% colors {}]",
                    image.size().width,
                    image.size().height,
                    ink,
                    colors
                )
            });
            let check = if item.state() == NSControlStateValueOn {
                " [x]"
            } else if item.state() == NSControlStateValueMixed {
                " [-]"
            } else {
                ""
            };
            let enabled = if item.isEnabled() { "" } else { " [off]" };
            let subtitle = if item.respondsToSelector(sel!(subtitle)) {
                item.subtitle().map(|text| format!(" — {text}")).unwrap_or_default()
            } else {
                String::new()
            };
            out.push_str(&format!(
                "{}{subtitle}{}{}{}\n",
                item.title(),
                check,
                enabled,
                icon.unwrap_or_default()
            ));
        }
    }
    out.push_str(&format!("chosen dot: {:?}\n", parts.chosen.get()));
    Some(out)
}

/// How much of a picture is drawn rather than transparent, in percent of a grid of its pixels: an
/// icon that is there but blank is the thing a size alone doesn't show.
#[cfg(any(test, feature = "devctl"))]
fn image_ink(image: &NSImage) -> (usize, usize) {
    use objc2_app_kit::{NSBitmapImageRep, NSColorSpace};
    use std::collections::HashSet;

    let Some(rep) = image.TIFFRepresentation().and_then(|tiff| NSBitmapImageRep::imageRepWithData(&tiff))
    else {
        return (0, 0);
    };
    let (w, h) = (rep.pixelsWide().max(1), rep.pixelsHigh().max(1));
    let (mut drawn, mut seen) = (0, 0);
    let mut colors = HashSet::new();
    let srgb = NSColorSpace::sRGBColorSpace();
    for y in (0..h).step_by((h / 16).max(1) as usize) {
        for x in (0..w).step_by((w / 16).max(1) as usize) {
            seen += 1;
            if let Some(color) = rep.colorAtX_y(x, y) {
                if color.alphaComponent() > 0.1 {
                    drawn += 1;
                }
                // Count opaque interior colors: a flat colored disk without its
                // contrasting letter has one, regardless of edge antialiasing.
                if x >= w / 4
                    && x < 3 * w / 4
                    && y >= h / 4
                    && y < 3 * h / 4
                    && color.alphaComponent() > 0.99
                    && let Some(color) = color.colorUsingColorSpace(&srgb)
                {
                    colors.insert((
                        (color.redComponent() * 255.0).round() as u8,
                        (color.greenComponent() * 255.0).round() as u8,
                        (color.blueComponent() * 255.0).round() as u8,
                    ));
                }
            }
        }
    }
    (drawn * 100 / seen.max(1), colors.len())
}

/// Build `menu`, run AppKit's loop until it closes, and return the callback of the item chosen.
fn pop_up(view: &NSView, at: Point<Pixels>, menu: &Menu) -> Option<Callback> {
    let mtm = MainThreadMarker::new()?;
    let target = Target::new(mtm);
    let mut parts = Parts { callbacks: Vec::new(), picks: Vec::new(), chosen: Rc::default() };
    let ns_menu = build(mtm, menu, &target, &mut parts, menu.appearance);
    ns_menu.popUpMenuPositioningItem_atLocation_inView(None, appkit::location(view, at), Some(view));
    // A dot of a row of colours: its `on_pick` with its place, run later like any item's callback.
    if let Some((row, index)) = parts.chosen.get() {
        let pick = parts.picks.get(row)?.clone();
        return Some(Rc::new(move |window, cx| pick(index, window, cx)));
    }
    let chosen = target.ivars().chosen.get();
    usize::try_from(chosen).ok().and_then(|ix| parts.callbacks.get(ix).cloned())
}

/// `menu` as an `NSMenu`. An item's tag is the place of its callback in `parts.callbacks`.
fn build(
    mtm: MainThreadMarker,
    menu: &Menu,
    target: &Target,
    parts: &mut Parts,
    appearance: Option<wormhole_core::theme::tokens::Appearance>,
) -> Retained<NSMenu> {
    let ns_menu = NSMenu::new(mtm);
    // GPUI keeps the window's system appearance to observe system changes. A menu
    // follows the opening window's theme, including in a parked profile. If no
    // window look was supplied, use the app's current native-material appearance.
    let ns_appearance =
        appearance.and_then(super::appearance::ns_appearance).or_else(super::appearance::current);
    ns_menu.setAppearance(ns_appearance.as_deref());
    // Every item says whether it is enabled; AppKit isn't to guess from the responder chain.
    ns_menu.setAutoenablesItems(false);
    for entry in &menu.entries {
        let item = match entry {
            Entry::Separator => NSMenuItem::separatorItem(mtm),
            // macOS 14 is the minimum: native section headers.
            Entry::Header(title) => NSMenuItem::sectionHeaderWithTitle(&NSString::from_str(title), mtm),
            Entry::Item(item) => item_for(mtm, item, target, &mut parts.callbacks),
            Entry::Swatches(row) => {
                let ns_item = NSMenuItem::new(mtm);
                ns_item.setView(Some(&swatches::row_view(
                    mtm,
                    parts.picks.len(),
                    row,
                    &parts.chosen,
                    swatches::Use::menu(),
                )));
                parts.picks.push(row.on_pick.clone());
                ns_item
            }
            Entry::Submenu { title, enabled, menu } => {
                let ns_item = NSMenuItem::new(mtm);
                ns_item.setTitle(&NSString::from_str(title));
                ns_item.setEnabled(*enabled);
                if *enabled {
                    ns_item.setSubmenu(Some(&build(mtm, menu, target, parts, appearance)));
                }
                ns_item
            }
        };
        ns_menu.addItem(&item);
    }
    ns_menu
}

#[allow(unsafe_code)]
fn item_for(
    mtm: MainThreadMarker,
    item: &Item,
    target: &Target,
    callbacks: &mut Vec<Callback>,
) -> Retained<NSMenuItem> {
    let ns_item = NSMenuItem::new(mtm);
    ns_item.setTitle(&NSString::from_str(&item.title));
    if let Some(subtitle) = &item.subtitle {
        let subtitle = NSString::from_str(subtitle);
        ns_item.setToolTip(Some(&subtitle));
        if ns_item.respondsToSelector(sel!(setSubtitle:)) {
            ns_item.setSubtitle(Some(&subtitle));
        } else {
            // macOS 14.0–14.3 has no subtitle; keep the detail visible.
            ns_item.setTitle(&NSString::from_str(&crate::platform::menu::display_title(
                &item.title,
                item.subtitle.as_deref(),
                false,
            )));
        }
    }
    ns_item.setEnabled(item.enabled);
    ns_item.setState(match item.check {
        Check::Off => 0,
        Check::On => NSControlStateValueOn,
        Check::Mixed => NSControlStateValueMixed,
    });
    if let Some(key) = &item.key {
        ns_item.setKeyEquivalent(&NSString::from_str(&key.key));
        ns_item.setKeyEquivalentModifierMask(appkit::modifier_mask(key));
    }
    ns_item.setImage(item.icon.as_ref().and_then(image).as_deref());
    if let Some(run) = &item.on_select {
        ns_item.setTag(callbacks.len() as isize);
        callbacks.push(run.clone());
        // SAFETY: `menuItemClicked:` is defined on `Target` with the `(id)sender` signature
        // AppKit uses for actions. AppKit holds the target weakly; the caller keeps it alive
        // until the menu has closed.
        unsafe {
            ns_item.setTarget(Some(target));
            ns_item.setAction(Some(sel!(menuItemClicked:)));
        }
    }
    ns_item
}

fn image(icon: &Icon) -> Option<Retained<NSImage>> {
    match icon {
        Icon::File(path) => appkit::file_icon(path, IMAGE_SIDE),
        Icon::Folder => appkit::folder_icon(IMAGE_SIDE),
        // Only where there is no icon service; the app has a file icon here.
        Icon::Monogram(_) => None,
        Icon::Avatar { letter, background, foreground } => {
            Some(avatar_image(letter, *background, *foreground))
        }
        // A name the system doesn't know gives `None`: the row simply has no picture. Symbols are
        // template images already, which is what makes the menu tint them like its text.
        Icon::Symbol(name) => {
            NSImage::imageWithSystemSymbolName_accessibilityDescription(&NSString::from_str(name), None)
        }
        Icon::Image(file) => {
            if file.bytes.is_empty() {
                return None;
            }
            let image = NSImage::initWithData(NSImage::alloc(), &NSData::with_bytes(&file.bytes))?;
            // AppKit doesn't fit a picture to the row: a large file would overflow it.
            image.setSize(NSSize::new(IMAGE_SIDE, IMAGE_SIDE));
            Some(image)
        }
    }
}

/// A profile mark keeps its color in an AppKit menu; it is not a template icon.
#[allow(unsafe_code)] // AppKit text attributes have the documented font and color value types.
fn avatar_image(letter: &str, background: Hsla, foreground: Hsla) -> Retained<NSImage> {
    let text = letter.to_owned();
    let bg = Rgba::from(background);
    let fg = Rgba::from(foreground);
    let handler = RcBlock::new(move |rect: NSRect| -> Bool {
        let colour = |c: Rgba| {
            NSColor::colorWithSRGBRed_green_blue_alpha(
                f64::from(c.r),
                f64::from(c.g),
                f64::from(c.b),
                f64::from(c.a),
            )
        };
        colour(bg).setFill();
        NSBezierPath::bezierPathWithOvalInRect(rect).fill();
        let label = NSMutableAttributedString::initWithString(
            NSMutableAttributedString::alloc(),
            &NSString::from_str(&text),
        );
        let range = NSRange::new(0, text.encode_utf16().count());
        let font = NSFont::boldSystemFontOfSize(10.0);
        let ink = colour(fg);
        // SAFETY: these attribute keys take NSFont and NSColor; the range covers the string.
        unsafe {
            label.addAttribute_value_range(NSFontAttributeName, font.as_ref() as &AnyObject, range);
            label.addAttribute_value_range(NSForegroundColorAttributeName, ink.as_ref() as &AnyObject, range);
        }
        let size = label.size();
        label.drawAtPoint(NSPoint::new(
            (rect.size.width - size.width) / 2.0,
            (rect.size.height - size.height) / 2.0,
        ));
        Bool::YES
    });
    let image =
        NSImage::imageWithSize_flipped_drawingHandler(NSSize::new(IMAGE_SIDE, IMAGE_SIDE), false, &handler);
    image.setTemplate(false);
    image
}
