//! What VoiceOver hears beyond the accessibility tree GPUI builds from the views' roles and
//! labels: announcements, since GPUI has no live regions. A terminal that needs you is
//! said at once, interrupting; one that finished waits its turn. Which changes are said, and in
//! what words, is core's (`wormhole_core::notify::Announcer`); this only posts them. And
//! whether macOS's Keyboard navigation is on, which makes buttons ⇥ stops.
//!
//! The system's Reduce Motion, Increase Contrast and Reduce Transparency are read and watched in
//! `vibrancy` with the rest of what the system says about how windows look.

/// Say `text` through VoiceOver (when it runs; otherwise nothing happens). `interrupt` cuts into
/// what it is saying (needs you); without it the announcement is queued (finished).
pub fn announce(text: &str, interrupt: bool) {
    tracing::debug!(interrupt, "announce: {text}");
    #[cfg(feature = "devctl")]
    devctl::remember(text, interrupt);
    // A test instance in background mode keeps them to devctl (`a11y said`).
    #[cfg(all(target_os = "macos", not(test)))]
    if !crate::background::on() {
        platform::post(text, interrupt);
    }
}

/// Whether macOS's Keyboard navigation is on (System Settings → Keyboard): ⇥ then reaches buttons
/// too, the title row's among them. AppKit can't be watched for it, so it is read when ⇥
/// is pressed; it is cheap.
pub fn keyboard_navigation() -> bool {
    #[cfg(feature = "devctl")]
    if let Some(on) = devctl::KEYBOARD_NAVIGATION.get() {
        return on;
    }
    system_keyboard_navigation()
}

#[cfg(all(target_os = "macos", not(test)))]
fn system_keyboard_navigation() -> bool {
    platform::keyboard_navigation()
}

/// Off macOS, and in tests (which never ask the real system): buttons are not ⇥ stops.
#[cfg(not(all(target_os = "macos", not(test))))]
fn system_keyboard_navigation() -> bool {
    false
}

/// devctl's `keyboard-navigation`: say it is on or off, or (`None`) follow the system again.
#[cfg(feature = "devctl")]
pub fn set_keyboard_navigation(on: Option<bool>) {
    devctl::KEYBOARD_NAVIGATION.set(on);
}

/// The announcements made lately, oldest first, for devctl's `a11y said`.
#[cfg(feature = "devctl")]
pub fn recent() -> Vec<(String, bool)> {
    devctl::recent()
}

/// Wake the window's accessibility tree, as VoiceOver or Accessibility Inspector would by asking
/// for it: GPUI builds the tree only once something has. It is there from the next frame on.
#[cfg(feature = "devctl")]
pub fn wake_tree(window: &gpui_kit::Window) {
    #[cfg(target_os = "macos")]
    platform::wake_tree(window);
    #[cfg(not(target_os = "macos"))]
    let _ = window;
}

#[cfg(feature = "devctl")]
mod devctl {
    use std::cell::{Cell, RefCell};
    use std::collections::VecDeque;

    /// Enough to see what a scripted check caused.
    const KEEP: usize = 16;

    thread_local! {
        static SAID: RefCell<VecDeque<(String, bool)>> = const { RefCell::new(VecDeque::new()) };
        /// What devctl says in place of the system's Keyboard navigation.
        pub(super) static KEYBOARD_NAVIGATION: Cell<Option<bool>> = const { Cell::new(None) };
    }

    pub(super) fn remember(text: &str, interrupt: bool) {
        SAID.with_borrow_mut(|said| {
            if said.len() == KEEP {
                said.pop_front();
            }
            said.push_back((text.to_string(), interrupt));
        });
    }

    pub(super) fn recent() -> Vec<(String, bool)> {
        SAID.with_borrow(|said| said.iter().cloned().collect())
    }
}

#[cfg(target_os = "macos")]
mod platform {
    #[cfg(not(test))]
    pub(super) fn keyboard_navigation() -> bool {
        let Some(mtm) = objc2::MainThreadMarker::new() else { return false };
        objc2_app_kit::NSApplication::sharedApplication(mtm).isFullKeyboardAccessEnabled()
    }

    #[cfg(not(test))]
    #[allow(unsafe_code)]
    pub(super) fn post(text: &str, interrupt: bool) {
        use objc2::MainThreadMarker;
        use objc2::runtime::AnyObject;
        use objc2_app_kit::{
            NSAccessibilityAnnouncementKey, NSAccessibilityAnnouncementRequestedNotification,
            NSAccessibilityPostNotificationWithUserInfo, NSAccessibilityPriorityKey,
            NSAccessibilityPriorityLevel, NSApplication,
        };
        use objc2_foundation::{NSDictionary, NSNumber, NSString};

        let Some(mtm) = MainThreadMarker::new() else { return };
        let app = NSApplication::sharedApplication(mtm);
        let level =
            if interrupt { NSAccessibilityPriorityLevel::High } else { NSAccessibilityPriorityLevel::Medium };
        let message = NSString::from_str(text);
        let priority = NSNumber::new_isize(level.0);
        let values: [&AnyObject; 2] = [&message, &priority];
        // SAFETY: the keys and the notification name are immutable AppKit statics, set before
        // main runs; the user info holds an NSString and an NSNumber, as AppKit documents for
        // them; the element is the main window or the application object, on the main thread.
        unsafe {
            let keys = [NSAccessibilityAnnouncementKey, NSAccessibilityPriorityKey];
            let info = NSDictionary::from_slices(&keys, &values);
            // The main window speaks for the app when there is one, as AppKit's own examples do.
            match app.mainWindow() {
                Some(window) => NSAccessibilityPostNotificationWithUserInfo(
                    &window,
                    NSAccessibilityAnnouncementRequestedNotification,
                    Some(&info),
                ),
                None => NSAccessibilityPostNotificationWithUserInfo(
                    &app,
                    NSAccessibilityAnnouncementRequestedNotification,
                    Some(&info),
                ),
            }
        }
    }

    #[cfg(feature = "devctl")]
    #[allow(unsafe_code)]
    pub(super) fn wake_tree(window: &gpui_kit::Window) {
        use objc2::rc::Retained;
        use objc2::runtime::AnyObject;
        use objc2_app_kit::NSView;
        use raw_window_handle::{HasWindowHandle, RawWindowHandle};

        let Ok(handle) = HasWindowHandle::window_handle(window) else { return };
        let RawWindowHandle::AppKit(handle) = handle.as_raw() else { return };
        // SAFETY: GPUI hands out its live view, and this runs on the main thread.
        let view: &NSView = unsafe { handle.ns_view.cast::<NSView>().as_ref() };
        // GPUI's AccessKit adapter sits on the window's content view, which holds GPUI's.
        let Some(content) = view.window().and_then(|window| window.contentView()) else { return };
        // SAFETY: asking the content view for its accessibility children is what an assistive app
        // does first; the adapter answers it by switching the tree on.
        let _children: Option<Retained<AnyObject>> =
            unsafe { objc2::msg_send![&*content, accessibilityChildren] };
    }
}
