//! The app's OS edge: everything the app asks of the operating system beyond GPUI, behind one
//! neutral API, so a port to another system writes a sibling of `macos/` and nothing else
//! changes. Each concern is a neutral file here (plain functions and types, no OS names: `window`,
//! `vibrancy`, `look`, `dock`, `a11y`, `auth`, `hotkey`, `keys`, `sound`, `scroller`, `fonts`,
//! `font_face`, `locale`, `status_item`, `menu`, `popover`, `notice`, `notifications`, `default_app`, `apps`,
//! `sheet`, `trash`, `image` (what the system decodes that GPUI can't), `panel`, `pointer`, `intro`, devctl's `inspect`) that calls `os::…`: `macos/` is AppKit's implementation
//! (the only code in the app with `objc2`, `block2`, raw window handles and `unsafe` for the
//! system), `fallback/` the portable one, GPUI's own call where GPUI has one and "not available"
//! where it hasn't (used only off macOS, not a target yet; compiled on macOS too, so it can't drift
//! from `macos/`, which a test pins). `macos::appkit` holds the one cast from a GPUI window to its
//! `NSView` and `NSWindow`. `spawn` starts the helper programs nobody waits for.
//!
//! What a feature owns is split: the OS part is here (the notification centre, Launch Services'
//! default app, apps by bundle identifier and `open -b`/`-t`, the quit sheet, the quick panel's
//! window, the intro's Metal overlay), the logic stays with the feature, which asks "is it
//! there?" through the answer (`None`, an error) rather than through an OS condition. Revealing a
//! file, opening a folder or a URL go through GPUI (`reveal_path`, `open_with_system`, `open_url`).
//!
//! Test and background-mode runs are decided inside these functions (`background_mode`,
//! `cfg!(test)`), so no feature checks them for a platform call: the Dock, VoiceOver, the bell,
//! menus and popovers recorded instead of shown, and the status item and the hotkey, which
//! background mode keeps out of the menu bar and the keys while the app shows them as they would be.
//!
//! The platform is the lowest layer of the app: it must not use `settings`, `style`, `window`
//! or any other feature. Where drawing needs the app's look (the GPUI-drawn menus and popovers),
//! the app hands it over (`Drawn`); the intro's colours come with the call. It still names the
//! status item's model and the menu key type from `menubar` (the layer below it, whose `build` the
//! status item calls as its menu opens); moving them down needs the platform to be handed a builder.

pub mod a11y;
mod anchor;
pub mod apps;
pub mod auth;
pub mod background_mode;
pub mod clipboard;
pub mod default_app;
pub mod dock;
// Built on macOS too, though only used elsewhere, so it can't drift from `macos/` (`parity`).
#[cfg_attr(target_os = "macos", allow(dead_code))]
pub mod fallback;
pub mod font_face;
pub mod fonts;
pub mod hotkey;
#[cfg(feature = "devctl")]
pub mod image;
pub mod inspect;
pub mod intro;
pub mod keys;
pub mod live_resize;
pub mod locale;
pub mod look;
#[cfg(target_os = "macos")]
pub mod macos;
pub mod menu;
pub mod notice;
pub mod notifications;
pub mod panel;
// Type-checked in every macOS build; only a test calls it.
#[cfg(target_os = "macos")]
#[cfg_attr(not(test), allow(dead_code))]
mod parity;
pub mod pointer;
pub mod popover;
#[cfg(any(test, feature = "devctl"))]
pub mod record;
pub mod scroller;
pub mod sheet;
pub mod sound;
pub mod spawn;
pub mod status_item;
pub mod trash;
pub mod vibrancy;
pub mod window;

use gpui_kit::*;

/// The implementation the neutral files call: AppKit's on macOS, the portable one elsewhere.
#[cfg(not(target_os = "macos"))]
use fallback as os;
#[cfg(target_os = "macos")]
use macos as os;

pub use anchor::{Anchor, on_right_click, on_right_click_except};
pub use menu::{Icon, Item, Menu, Swatch, run_on};
pub use popover::{ColourChange, Handle, Popover};

/// What Finder calls the volume the system starts from, as the last line of a path menu.
pub fn boot_volume_name() -> String {
    #[cfg(target_os = "macos")]
    if let Some(name) = macos::appkit::boot_volume_name() {
        return name;
    }
    "/".to_string()
}

/// Open `menu` at `at` (window coordinates) as the platform's own menu, and run what is chosen.
///
/// Returns at once: the menu opens, and its item runs, a moment later, outside the borrow of the
/// app that the caller (a mouse handler, an update) is in. See `macos::menu` for why that matters.
/// Call it from anywhere with a `Window` and an `App`, once per menu; an empty menu opens nothing.
pub fn show_menu(menu: Menu, at: Point<Pixels>, window: &mut Window, cx: &mut App) {
    if menu.is_empty() {
        return;
    }
    #[cfg(any(test, feature = "devctl"))]
    if record::keep(&menu, window) {
        return;
    }
    #[cfg(target_os = "macos")]
    let menu = match macos::menu::show(menu, at, window, cx) {
        Ok(()) => return,
        Err(menu) => menu,
    };
    match cx.try_global::<Drawn>().copied() {
        Some(drawn) => (drawn.menu)(menu, at, window, cx),
        None => {
            // A programming error (`style::menu_layer::install` not called): loud in development.
            debug_assert!(false, "no drawn menus handed over");
            tracing::warn!("no drawn menus handed over: the menu can't open");
        }
    }
}

/// Open `popover` under `anchor` (the element's bounds in the window) as the platform's own, and
/// run what is done in it, a moment later and outside every borrow of the app (see `popover`).
/// The handle closes it from the app (a hover popover is closed by the app).
pub fn show_popover(popover: Popover, anchor: Bounds<Pixels>, window: &mut Window, cx: &mut App) -> Handle {
    #[cfg(any(test, feature = "devctl"))]
    if let Some(handle) = record::keep_popover(&popover, window) {
        return handle;
    }
    #[cfg(target_os = "macos")]
    let popover = match macos::popover::show(popover, anchor, window, cx) {
        Ok(handle) => return handle,
        Err(popover) => popover,
    };
    match cx.try_global::<Drawn>().copied() {
        Some(drawn) => (drawn.popover)(popover, anchor, window, cx),
        None => {
            debug_assert!(false, "no drawn popovers handed over");
            tracing::warn!("no drawn popovers handed over: the popover can't open");
            Handle::new(|_| {})
        }
    }
}

/// Menus and popovers drawn by GPUI in the app's look, for a window the platform can't show its
/// own in (one without a native view, or a system without native menus). Drawing them takes the
/// app's colours and components, which sit above the platform, so the app hands them over as it
/// starts (`style::menu_layer::install`) instead of the platform reaching up for them.
#[derive(Clone, Copy)]
pub struct Drawn {
    pub menu: fn(Menu, Point<Pixels>, &mut Window, &mut App),
    pub popover: fn(Popover, Bounds<Pixels>, &mut Window, &mut App) -> Handle,
}

impl Global for Drawn {}

/// Open the system's Colours window showing `start`; each colour chosen in it reaches `on_change`,
/// the latest per tick, outside every borrow of the app. Nowhere to open it, nothing happens.
pub fn pick_colour(start: Hsla, on_change: ColourChange, window: &mut Window, cx: &mut App) {
    #[cfg(any(test, feature = "devctl"))]
    if record::keep_colour(&on_change) {
        return;
    }
    #[cfg(target_os = "macos")]
    macos::colour::pick(start, on_change, window, cx);
    #[cfg(not(target_os = "macos"))]
    let _ = (start, on_change, window, cx);
}
