//! The macOS notification center (`UNUserNotificationCenter`), for the bundled app only: GPUI's
//! own notifications carry a tag, a title, a body and buttons, but not the interruption level
//! (Time Sensitive), the thread, the sound or a Reply field. So the app posts here itself.
//!
//! The center has one delegate per app. GPUI sets its own when GPUI Component registers a
//! response handler in `gpui_kit::init`; [`Center::open`] runs later and replaces it, so every
//! response, Reply's text included, arrives here and goes to the main thread over a channel.
//! Nothing posts through GPUI's route in the bundled app, so nothing is lost by that.
//!
//! The center aborts a process that isn't an app bundle, so nothing here is touched without one.

use std::cell::Cell;
use std::ffi::c_void;
use std::sync::OnceLock;

use block2::RcBlock;
use objc2::rc::Retained;
use objc2::runtime::{Bool, ProtocolObject};
use objc2_foundation::{NSArray, NSBundle, NSError, NSSet, NSString};
use objc2_user_notifications::{
    UNAuthorizationOptions, UNAuthorizationStatus, UNMutableNotificationContent, UNNotificationAction,
    UNNotificationActionOptions, UNNotificationCategory, UNNotificationCategoryOptions,
    UNNotificationInterruptionLevel, UNNotificationRequest, UNNotificationSetting, UNNotificationSettings,
    UNNotificationSound, UNTextInputNotificationAction, UNUserNotificationCenter,
};
use smol::channel::Sender;

use crate::workspace::notifications::{
    Category, FromCenter, Notice, OPEN, Permission, REPLY, SHOW_CHANGES, actions,
};
use delegate::Delegate;
use wormhole_core::notify::Level;

/// The categories (sets of buttons) our notifications use, by `Category`.
const NEEDS_YOU: &str = "wormhole.needs-you";
const FINISHED: &str = "wormhole.finished";

/// The entitlement behind Time Sensitive notifications (added to Developer ID builds with the
/// provisioning profile by `scripts/bundle.sh`).
const TIME_SENSITIVE: &str = "com.apple.developer.usernotifications.time-sensitive";

pub(crate) struct Center {
    center: Retained<UNUserNotificationCenter>,
    /// The center's `delegate` is weak: this keeps ours alive.
    _delegate: Retained<Delegate>,
    tx: Sender<FromCenter>,
    /// Authorization was asked for in this run (macOS shows its prompt only the first time).
    asked: Cell<bool>,
}

impl Center {
    /// The center, with our delegate and categories; `None` outside an app bundle.
    pub(crate) fn open(tx: Sender<FromCenter>) -> Option<Self> {
        if NSBundle::mainBundle().bundleIdentifier().is_none() {
            tracing::debug!("notifications: no bundle identifier, using osascript");
            return None;
        }
        let center = UNUserNotificationCenter::currentNotificationCenter();
        let delegate = Delegate::new(tx.clone());
        center.setDelegate(Some(ProtocolObject::from_ref(&*delegate)));
        center.setNotificationCategories(&categories());
        if !entitled_to_time_sensitive() {
            tracing::debug!("notifications: no Time Sensitive entitlement, needs you is delivered at Active");
        }
        Some(Self { center, _delegate: delegate, tx, asked: Cell::new(false) })
    }

    pub(crate) fn post(&self, notice: &Notice) {
        if !self.asked.get() {
            self.ask();
        }
        let content = UNMutableNotificationContent::new();
        content.setTitle(&NSString::from_str(&notice.title));
        if !notice.subtitle.is_empty() {
            content.setSubtitle(&NSString::from_str(&notice.subtitle));
        }
        content.setBody(&NSString::from_str(&notice.body));
        content.setThreadIdentifier(&NSString::from_str(&notice.thread));
        content.setInterruptionLevel(match notice.level {
            Level::Passive => UNNotificationInterruptionLevel::Passive,
            Level::Active => UNNotificationInterruptionLevel::Active,
            Level::TimeSensitive => UNNotificationInterruptionLevel::TimeSensitive,
        });
        if notice.sound {
            content.setSound(Some(&UNNotificationSound::defaultSound()));
        }
        match notice.category {
            Category::NeedsYou => content.setCategoryIdentifier(&NSString::from_str(NEEDS_YOU)),
            Category::Finished => content.setCategoryIdentifier(&NSString::from_str(FINISHED)),
            Category::Plain => {}
        }
        // The same identifier replaces a tab's earlier notification; a nil trigger delivers now.
        let request = UNNotificationRequest::requestWithIdentifier_content_trigger(
            &NSString::from_str(&notice.id),
            &content,
            None,
        );
        let done = RcBlock::new(|error: *mut NSError| {
            if let Some(error) = error_of(error) {
                tracing::warn!("posting a notification failed: {error}");
            }
        });
        self.center.addNotificationRequest_withCompletionHandler(&request, Some(&done));
    }

    pub(crate) fn remove(&self, id: &str) {
        let ids = NSArray::from_retained_slice(&[NSString::from_str(id)]);
        self.center.removePendingNotificationRequestsWithIdentifiers(&ids);
        self.center.removeDeliveredNotificationsWithIdentifiers(&ids);
    }

    /// Ask for permission (macOS prompts only while it hasn't been decided), then read it again.
    pub(crate) fn ask(&self) {
        self.asked.set(true);
        let tx = self.tx.clone();
        let center = self.center.clone();
        let done = RcBlock::new(move |_granted: Bool, error: *mut NSError| {
            if let Some(error) = error_of(error) {
                tracing::warn!("asking for notification permission failed: {error}");
            }
            read_permission(&center, tx.clone());
        });
        self.center.requestAuthorizationWithOptions_completionHandler(
            UNAuthorizationOptions::Alert | UNAuthorizationOptions::Sound,
            &done,
        );
    }

    /// Read the permission; the answer arrives on the channel.
    pub(crate) fn refresh_permission(&self) {
        read_permission(&self.center, self.tx.clone());
    }
}

#[allow(unsafe_code)]
fn read_permission(center: &UNUserNotificationCenter, tx: Sender<FromCenter>) {
    let done = RcBlock::new(move |settings: std::ptr::NonNull<UNNotificationSettings>| {
        // SAFETY: the center hands the handler a valid settings object for the call's duration.
        let settings = unsafe { settings.as_ref() };
        let permission = match settings.authorizationStatus() {
            UNAuthorizationStatus::NotDetermined => Permission::NotAsked,
            UNAuthorizationStatus::Denied => Permission::Denied,
            _ => Permission::Allowed {
                time_sensitive: settings.timeSensitiveSetting() == UNNotificationSetting::Enabled,
            },
        };
        let _ = tx.try_send(FromCenter::Permission(permission));
    });
    center.getNotificationSettingsWithCompletionHandler(&done);
}

#[allow(unsafe_code)]
fn error_of(error: *mut NSError) -> Option<String> {
    // SAFETY: when not null, `error` is a valid `NSError` for the duration of the callback.
    unsafe { error.as_ref() }.map(|error| error.localizedDescription().to_string())
}

fn categories() -> Retained<NSSet<UNNotificationCategory>> {
    let category = |id: &str, category: Category| {
        let buttons: Vec<Retained<UNNotificationAction>> = actions(category)
            .iter()
            .map(|&(action, label)| {
                let (id, label) = (NSString::from_str(action), NSString::from_str(label));
                match action {
                    // Typing a reply needs no window: the text goes to the terminal as it is.
                    REPLY => Retained::into_super(
                        UNTextInputNotificationAction::actionWithIdentifier_title_options_textInputButtonTitle_textInputPlaceholder(
                            &id,
                            &label,
                            UNNotificationActionOptions::empty(),
                            &NSString::from_str("Send"),
                            &NSString::from_str("Reply"),
                        ),
                    ),
                    OPEN | SHOW_CHANGES => UNNotificationAction::actionWithIdentifier_title_options(
                        &id,
                        &label,
                        UNNotificationActionOptions::Foreground,
                    ),
                    _ => UNNotificationAction::actionWithIdentifier_title_options(
                        &id,
                        &label,
                        UNNotificationActionOptions::empty(),
                    ),
                }
            })
            .collect();
        UNNotificationCategory::categoryWithIdentifier_actions_intentIdentifiers_options(
            &NSString::from_str(id),
            &NSArray::from_retained_slice(&buttons),
            &NSArray::new(),
            UNNotificationCategoryOptions::empty(),
        )
    };
    NSSet::from_retained_slice(&[
        category(NEEDS_YOU, Category::NeedsYou),
        category(FINISHED, Category::Finished),
    ])
}

/// Whether this process was signed with the Time Sensitive entitlement (read once). Only a
/// Developer ID build with the provisioning profile has it; everything else delivers "needs you"
/// at Active.
#[allow(unsafe_code)]
pub(crate) fn entitled_to_time_sensitive() -> bool {
    #[link(name = "Security", kind = "framework")]
    unsafe extern "C" {
        fn SecTaskCreateFromSelf(allocator: *const c_void) -> *mut c_void;
        fn SecTaskCopyValueForEntitlement(
            task: *mut c_void,
            entitlement: *const c_void,
            error: *mut *mut c_void,
        ) -> *const c_void;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        static kCFBooleanTrue: *const c_void;
        fn CFRelease(cf: *const c_void);
    }
    static ENTITLED: OnceLock<bool> = OnceLock::new();
    *ENTITLED.get_or_init(|| {
        let name = objc2_core_foundation::CFString::from_str(TIME_SENSITIVE);
        // SAFETY: SecTaskCreateFromSelf returns an owned task (or null), released below. The
        // entitlement name is a valid CFString for the call; the value it returns is owned and
        // released below. kCFBooleanTrue is a constant that is only compared, never released.
        unsafe {
            let task = SecTaskCreateFromSelf(std::ptr::null());
            if task.is_null() {
                return false;
            }
            let key = std::ptr::from_ref(&*name).cast::<c_void>();
            let value = SecTaskCopyValueForEntitlement(task, key, std::ptr::null_mut());
            let entitled = !value.is_null() && value == kCFBooleanTrue;
            if !value.is_null() {
                CFRelease(value);
            }
            CFRelease(task);
            entitled
        }
    })
}

#[allow(unsafe_code)] // `define_class!` declares an Objective-C class; objc2 checks the signatures.
mod delegate {
    use objc2::rc::Retained;
    use objc2::runtime::NSObjectProtocol;
    use objc2::{AnyThread, DefinedClass, define_class, msg_send};
    use objc2_foundation::NSObject;
    use objc2_user_notifications::{
        UNNotification, UNNotificationDefaultActionIdentifier, UNNotificationPresentationOptions,
        UNNotificationResponse, UNTextInputNotificationResponse, UNUserNotificationCenter,
        UNUserNotificationCenterDelegate,
    };
    use smol::channel::Sender;

    use crate::workspace::notifications::{FromCenter, Response};

    pub(super) struct Ivars {
        tx: Sender<FromCenter>,
    }

    define_class!(
        // SAFETY: NSObject has no subclassing requirements, and we don't implement `Drop`.
        #[unsafe(super(NSObject))]
        #[name = "WormholeNotificationDelegate"]
        #[ivars = Ivars]
        pub(super) struct Delegate;

        unsafe impl NSObjectProtocol for Delegate {}

        unsafe impl UNUserNotificationCenterDelegate for Delegate {
            // Possibly on another thread: the channel takes it to the main thread.
            #[unsafe(method(userNotificationCenter:didReceiveNotificationResponse:withCompletionHandler:))]
            fn did_receive(
                &self,
                _center: &UNUserNotificationCenter,
                response: &UNNotificationResponse,
                done: &block2::DynBlock<dyn Fn()>,
            ) {
                let id = response.notification().request().identifier().to_string();
                let action = response.actionIdentifier();
                // SAFETY: a constant NSString the framework exports.
                let default = unsafe { UNNotificationDefaultActionIdentifier };
                let action = (&*action != default).then(|| action.to_string());
                let reply = response
                    .downcast_ref::<UNTextInputNotificationResponse>()
                    .map(|reply| reply.userText().to_string());
                let _ = self.ivars().tx.try_send(FromCenter::Response(Response { id, action, reply }));
                done.call(());
            }

            // Without this macOS keeps banners away while Wormhole is in front; whether one is
            // wanted then is the app's decision (`notify_in_front`), made before it is posted.
            #[unsafe(method(userNotificationCenter:willPresentNotification:withCompletionHandler:))]
            fn will_present(
                &self,
                _center: &UNUserNotificationCenter,
                _notification: &UNNotification,
                done: &block2::DynBlock<dyn Fn(UNNotificationPresentationOptions)>,
            ) {
                done.call((UNNotificationPresentationOptions::Banner
                    | UNNotificationPresentationOptions::List
                    | UNNotificationPresentationOptions::Sound,));
            }
        }
    );

    impl Delegate {
        pub(crate) fn new(tx: Sender<FromCenter>) -> Retained<Self> {
            let this = Self::alloc().set_ivars(Ivars { tx });
            // SAFETY: `-[NSObject init]` is the designated initializer of the superclass.
            unsafe { msg_send![super(this), init] }
        }
    }
}
