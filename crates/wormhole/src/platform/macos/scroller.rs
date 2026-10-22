//! AppKit's side of `platform::scroller`: `NSScroller`'s preferred style and its change
//! notification, and the paging behaviour from the user's defaults.

use std::ptr::NonNull;

use block2::RcBlock;
use gpui_kit::App;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::{NSObjectProtocol, ProtocolObject};
use objc2_app_kit::{NSPreferredScrollerStyleDidChangeNotification, NSScroller, NSScrollerStyle};
use objc2_foundation::{NSNotification, NSNotificationCenter, NSOperationQueue, NSUserDefaults, ns_string};
use wormhole_term::scrollbar::ScrollerStyle;

/// An observer of the style; dropping it stops the calls.
pub struct StyleObserver {
    token: Retained<ProtocolObject<dyn NSObjectProtocol>>,
}

impl Drop for StyleObserver {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        // SAFETY: `token` is the observer `addObserverForName…` returned on this center.
        unsafe { NSNotificationCenter::defaultCenter().removeObserver(self.token.as_ref()) };
    }
}

pub fn style(_: &App) -> ScrollerStyle {
    let Some(mtm) = MainThreadMarker::new() else { return ScrollerStyle::default() };
    if NSScroller::preferredScrollerStyle(mtm) == NSScrollerStyle::Legacy {
        ScrollerStyle::Legacy
    } else {
        ScrollerStyle::Overlay
    }
}

/// `AppleScrollerPagingBehavior`: 0 pages, 1 jumps to the spot.
pub fn jump_to_spot() -> bool {
    NSUserDefaults::standardUserDefaults().integerForKey(ns_string!("AppleScrollerPagingBehavior")) == 1
}

#[allow(unsafe_code)]
pub fn observe(changed: impl Fn() + Send + Sync + 'static) -> Option<StyleObserver> {
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
