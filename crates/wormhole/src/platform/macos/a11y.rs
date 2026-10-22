//! AppKit's side of `platform::a11y`: VoiceOver announcements, Full Keyboard Access, and waking
//! the accessibility tree GPUI builds.

pub fn keyboard_navigation() -> bool {
    let Some(mtm) = objc2::MainThreadMarker::new() else { return false };
    objc2_app_kit::NSApplication::sharedApplication(mtm).isFullKeyboardAccessEnabled()
}

#[allow(unsafe_code)]
pub fn post(text: &str, interrupt: bool) {
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
pub fn wake_tree(window: &gpui_kit::Window) {
    use objc2::rc::Retained;
    use objc2::runtime::AnyObject;

    // GPUI's AccessKit adapter sits on the window's content view, which holds GPUI's.
    let content = crate::platform::macos::appkit::ns_window(window).and_then(|window| window.contentView());
    let Some(content) = content else { return };
    // SAFETY: asking the content view for its accessibility children is what an assistive app
    // does first; the adapter answers it by switching the tree on.
    let _children: Option<Retained<AnyObject>> =
        unsafe { objc2::msg_send![&*content, accessibilityChildren] };
}
