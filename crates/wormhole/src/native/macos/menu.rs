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

use gpui_kit::*;
use objc2::rc::Retained;
use objc2::{AnyThread, DefinedClass as _, MainThreadMarker, sel};
use objc2_app_kit::{
    NSControlStateValueMixed, NSControlStateValueOn, NSImage, NSImageSymbolConfiguration, NSImageSymbolScale,
    NSMenu, NSMenuItem, NSView,
};
use objc2_foundation::{NSArray, NSData, NSSize, NSString};

use super::appkit;
use crate::native::menu::{Callback, Check, Entry, Icon, Item, Menu};

/// The pictures in front of titles are this many points square, as the system's own menus have
/// them.
const IMAGE_SIDE: f64 = 16.0;

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

/// Build `menu`, run AppKit's loop until it closes, and return the callback of the item chosen.
fn pop_up(view: &NSView, at: Point<Pixels>, menu: &Menu) -> Option<Callback> {
    let mtm = MainThreadMarker::new()?;
    let target = Target::new(mtm);
    let mut callbacks = Vec::new();
    let ns_menu = build(mtm, menu, &target, &mut callbacks);
    ns_menu.popUpMenuPositioningItem_atLocation_inView(None, appkit::location(view, at), Some(view));
    let chosen = target.ivars().chosen.get();
    usize::try_from(chosen).ok().and_then(|ix| callbacks.get(ix).cloned())
}

/// `menu` as an `NSMenu`. An item's tag is the place of its callback in `callbacks`.
fn build(
    mtm: MainThreadMarker,
    menu: &Menu,
    target: &Target,
    callbacks: &mut Vec<Callback>,
) -> Retained<NSMenu> {
    let ns_menu = NSMenu::new(mtm);
    // Every item says whether it is enabled; AppKit isn't to guess from the responder chain.
    ns_menu.setAutoenablesItems(false);
    for entry in &menu.entries {
        let item = match entry {
            Entry::Separator => NSMenuItem::separatorItem(mtm),
            // macOS 14 is the minimum: native section headers.
            Entry::Header(title) => NSMenuItem::sectionHeaderWithTitle(&NSString::from_str(title), mtm),
            Entry::Item(item) => item_for(mtm, item, target, callbacks),
            Entry::Submenu { title, enabled, menu } => {
                let ns_item = NSMenuItem::new(mtm);
                ns_item.setTitle(&NSString::from_str(title));
                ns_item.setEnabled(*enabled);
                if *enabled {
                    ns_item.setSubmenu(Some(&build(mtm, menu, target, callbacks)));
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
        Icon::Swatch(color) => {
            let rgba = gpui_kit::Rgba::from(*color);
            let color = appkit::ns_color([rgba.r, rgba.g, rgba.b, rgba.a]);
            let palette =
                NSImageSymbolConfiguration::configurationWithPaletteColors(&NSArray::from_retained_slice(&[
                    color,
                ]));
            let config = palette.configurationByApplyingConfiguration(
                &NSImageSymbolConfiguration::configurationWithScale(NSImageSymbolScale::Medium),
            );
            appkit::symbol("circle.fill", "Colour")?.imageWithSymbolConfiguration(&config)
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
