//! AppKit's side of `platform::look`: the app's effective appearance and `NSWorkspace`'s
//! accessibility display options, with their change notification.

use std::ptr::NonNull;

use block2::RcBlock;
use objc2::MainThreadMarker;
use objc2::rc::Retained;
use objc2::runtime::{NSObjectProtocol, ProtocolObject};
use objc2_app_kit::{
    NSAppearanceNameAqua, NSAppearanceNameDarkAqua, NSApplication, NSWorkspace,
    NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification,
};
use objc2_foundation::{NSArray, NSNotification, NSOperationQueue};
use wormhole_core::theme::tokens::Appearance;

/// An observer of the display options; dropping it stops the calls.
pub struct DisplayObserver {
    token: Retained<ProtocolObject<dyn NSObjectProtocol>>,
}

impl Drop for DisplayObserver {
    #[allow(unsafe_code)]
    fn drop(&mut self) {
        let center = NSWorkspace::sharedWorkspace().notificationCenter();
        // SAFETY: `token` is the observer `addObserverForName…` returned on this center.
        unsafe { center.removeObserver(self.token.as_ref()) };
    }
}

#[allow(unsafe_code)]
pub fn appearance() -> Option<Appearance> {
    let mtm = MainThreadMarker::new()?;
    // SAFETY: AppKit's appearance names are immutable statics, set before main runs.
    let (aqua, dark) = unsafe { (NSAppearanceNameAqua, NSAppearanceNameDarkAqua) };
    let names = NSArray::from_slice(&[aqua, dark]);
    // The app's own appearance is never set, so it is the system's. The best match folds
    // the high-contrast and vibrant variants into the two that matter here.
    let best = NSApplication::sharedApplication(mtm)
        .effectiveAppearance()
        .bestMatchFromAppearancesWithNames(&names)?;
    Some(if &*best == dark { Appearance::Dark } else { Appearance::Light })
}

pub fn reduce_transparency() -> bool {
    NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceTransparency()
}

pub fn increase_contrast() -> bool {
    NSWorkspace::sharedWorkspace().accessibilityDisplayShouldIncreaseContrast()
}

pub fn reduce_motion() -> Option<bool> {
    Some(NSWorkspace::sharedWorkspace().accessibilityDisplayShouldReduceMotion())
}

#[allow(unsafe_code)]
pub fn observe(changed: impl Fn() + Send + Sync + 'static) -> Option<DisplayObserver> {
    let center = NSWorkspace::sharedWorkspace().notificationCenter();
    let block = RcBlock::new(move |_: NonNull<NSNotification>| changed());
    // SAFETY: a notification name AppKit defines, no sender filter, and the main queue, so
    // the block runs on the main thread; it is `Send + Sync` as the center requires.
    let token = unsafe {
        center.addObserverForName_object_queue_usingBlock(
            Some(NSWorkspaceAccessibilityDisplayOptionsDidChangeNotification),
            None,
            Some(&NSOperationQueue::mainQueue()),
            &block,
        )
    };
    Some(DisplayObserver { token })
}
