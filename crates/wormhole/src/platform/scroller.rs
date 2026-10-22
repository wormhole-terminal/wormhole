//! How the system draws scrollbars, and what a click on a track does: System Settings →
//! Appearance → Show scroll bars (the style, read at start and followed while the app runs) and
//! "Click in the scroll bar to" (read at the click, nothing to follow).
//!
//! The terminal draws its scrollbar itself, so it asks here. The legacy style takes a gutter from
//! the pane, which is why a change of style re-lays out every window and nothing else does.

use gpui_kit::{App, Global};
use wormhole_core::terminal::scrollbar::{GUTTER_W, ScrollerStyle};

/// The system's scrollbar style, kept as a global. Without one (tests) the overlay style is used.
pub struct ScrollerPrefs {
    system: ScrollerStyle,
    /// What devctl says in place of the system.
    stand_in: Option<ScrollerStyle>,
    /// Hears the style change while the app runs.
    _observer: Option<StyleObserver>,
}

impl Global for ScrollerPrefs {}

impl ScrollerPrefs {
    /// Read the style now and follow it from here on.
    pub fn install(cx: &mut App) {
        let (changed, changes) = smol::channel::unbounded::<()>();
        let observer = system::observe(move || {
            let _ = changed.try_send(());
        });
        cx.set_global(Self { system: system::style(), stand_in: None, _observer: observer });
        cx.spawn(async move |cx| {
            while changes.recv().await.is_ok() {
                cx.update(|cx| Self::set_system(cx, system::style()));
            }
        })
        .detach();
    }

    fn set_system(cx: &mut App, style: ScrollerStyle) {
        let before = Self::style(cx);
        cx.global_mut::<Self>().system = style;
        if Self::style(cx) != before {
            // The gutter changes the panes' width; every terminal lays out again.
            cx.refresh_windows();
        }
    }

    pub fn style(cx: &App) -> ScrollerStyle {
        cx.try_global::<Self>()
            .map_or_else(ScrollerStyle::default, |prefs| prefs.stand_in.unwrap_or(prefs.system))
    }

    /// The width a terminal gives up for the scrollbar: the gutter in the legacy style, which is
    /// always there so that showing or hiding the thumb never reflows the text.
    pub fn gutter(cx: &App) -> f32 {
        if Self::style(cx) == ScrollerStyle::Legacy { GUTTER_W } else { 0.0 }
    }

    /// devctl's `scroller`: say the system is in one style, or follow it again.
    #[cfg(feature = "devctl")]
    pub(crate) fn stand_in(cx: &mut App, style: Option<ScrollerStyle>) {
        cx.global_mut::<Self>().stand_in = style;
        cx.refresh_windows();
    }
}

/// Whether a click on a track jumps to the spot ("Jump to the spot that's clicked") rather than
/// pages ("Jump to the next page"); `false` where the system can't be asked.
pub fn jump_to_spot() -> bool {
    system::jump_to_spot()
}

/// An observer of the style; dropping it stops the calls.
pub struct StyleObserver {
    #[cfg(all(target_os = "macos", not(test)))]
    token: objc2::rc::Retained<objc2::runtime::ProtocolObject<dyn objc2::runtime::NSObjectProtocol>>,
}

#[cfg(all(target_os = "macos", not(test)))]
impl Drop for StyleObserver {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        // SAFETY: `token` is the observer `addObserverForName…` returned on this center.
        unsafe {
            objc2_foundation::NSNotificationCenter::defaultCenter().removeObserver(self.token.as_ref())
        };
    }
}

#[cfg(all(target_os = "macos", not(test)))]
mod system {
    use std::ptr::NonNull;

    use block2::RcBlock;
    use objc2::MainThreadMarker;
    use objc2_app_kit::{NSPreferredScrollerStyleDidChangeNotification, NSScroller, NSScrollerStyle};
    use objc2_foundation::{
        NSNotification, NSNotificationCenter, NSOperationQueue, NSUserDefaults, ns_string,
    };
    use wormhole_core::terminal::scrollbar::ScrollerStyle;

    use super::StyleObserver;

    pub(super) fn style() -> ScrollerStyle {
        let Some(mtm) = MainThreadMarker::new() else { return ScrollerStyle::default() };
        if NSScroller::preferredScrollerStyle(mtm) == NSScrollerStyle::Legacy {
            ScrollerStyle::Legacy
        } else {
            ScrollerStyle::Overlay
        }
    }

    /// `AppleScrollerPagingBehavior`: 0 pages, 1 jumps to the spot.
    pub(super) fn jump_to_spot() -> bool {
        NSUserDefaults::standardUserDefaults().integerForKey(ns_string!("AppleScrollerPagingBehavior")) == 1
    }

    #[allow(unsafe_code)]
    pub(super) fn observe(changed: impl Fn() + Send + Sync + 'static) -> Option<StyleObserver> {
        let block = RcBlock::new(move |_: NonNull<NSNotification>| changed());
        // SAFETY: a notification name AppKit defines, no sender filter, and the main queue, so
        // the block runs on the main thread; it is `Send + Sync` as the center requires.
        let token = unsafe {
            NSNotificationCenter::defaultCenter().addObserverForName_object_queue_usingBlock(
                Some(NSPreferredScrollerStyleDidChangeNotification),
                None,
                Some(&NSOperationQueue::mainQueue()),
                &block,
            )
        };
        Some(StyleObserver { token })
    }
}

/// Off macOS, and in tests (which never ask the real system): nothing to ask or observe.
#[cfg(not(all(target_os = "macos", not(test))))]
mod system {
    use wormhole_core::terminal::scrollbar::ScrollerStyle;

    use super::StyleObserver;

    pub(super) fn style() -> ScrollerStyle {
        ScrollerStyle::default()
    }

    pub(super) fn jump_to_spot() -> bool {
        false
    }

    pub(super) fn observe(changed: impl Fn() + Send + Sync + 'static) -> Option<StyleObserver> {
        let _ = changed;
        None
    }
}
