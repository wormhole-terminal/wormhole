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

use gpui_kit::*;
use objc2::rc::Retained;
use objc2::{AnyThread, DefinedClass as _, MainThreadMarker, sel};
use objc2_app_kit::{NSControlStateValueMixed, NSControlStateValueOn, NSImage, NSMenu, NSMenuItem, NSView};
use objc2_foundation::{NSData, NSSize, NSString};

use super::appkit;
use super::swatches::{self, Chosen};
use crate::native::menu::{Callback, Check, Entry, Icon, Item, Menu, Pick};

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
    let ns_menu = build(mtm, menu, &target, &mut parts);
    let mut out = String::new();
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
            let icon =
                item.image().map(|image| format!(" [icon {}x{}]", image.size().width, image.size().height));
            out.push_str(&format!("{}{}\n", item.title(), icon.unwrap_or_default()));
        }
    }
    out.push_str(&format!("chosen dot: {:?}\n", parts.chosen.get()));
    Some(out)
}

/// Build `menu`, run AppKit's loop until it closes, and return the callback of the item chosen.
fn pop_up(view: &NSView, at: Point<Pixels>, menu: &Menu) -> Option<Callback> {
    let mtm = MainThreadMarker::new()?;
    let target = Target::new(mtm);
    let mut parts = Parts { callbacks: Vec::new(), picks: Vec::new(), chosen: Rc::default() };
    let ns_menu = build(mtm, menu, &target, &mut parts);
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
fn build(mtm: MainThreadMarker, menu: &Menu, target: &Target, parts: &mut Parts) -> Retained<NSMenu> {
    let ns_menu = NSMenu::new(mtm);
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
                    ns_item.setSubmenu(Some(&build(mtm, menu, target, parts)));
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
