//! "Send test": a real banner through the route real ones take, and the answer for Settings to show.
//!
//! A test that does nothing and says nothing is worse than none, so the press never ends silent:
//! with macOS not asked yet it asks first (and sends once the answer is yes); when macOS has the
//! notifications off it says so instead of posting a banner the system would drop; and the centre's
//! refusal of the request comes back as the row's words.

use gpui_kit::*;
use wormhole_core::notify::{self, Kind};

use super::permission::{ask_permission, refresh_permission};
use super::{Notifier, TEST_ID, notifier, permission, post, time_sensitive};
use crate::app_state::AppState;
use crate::platform::notifications::{Category, Notice, Permission};
use crate::settings::settings;

/// Where the last "Send test" stands.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) enum Test {
    /// Not pressed in this run.
    #[default]
    Idle,
    /// Waiting for macOS: `asked` says its question is up (an answer to it ends the wait), otherwise
    /// its permission is still being read.
    Waiting { asked: bool },
    /// Handed to the route; macOS can still hold it back (Focus, the alert style) and says nothing then.
    Sent,
    /// Off in System Settings: nothing was posted.
    Denied,
    /// macOS was asked and never allowed it (no question appeared, or it was dismissed).
    Unanswered,
    /// The route refused the notification; the reason as it gave it.
    Refused(String),
}

/// How the last test stands, for the Settings row.
pub(crate) fn state(cx: &App) -> Test {
    cx.try_global::<Notifier>().map(|n| n.test.clone()).unwrap_or_default()
}

fn set(test: Test, cx: &mut App) {
    if notifier(cx).test != test {
        notifier(cx).test = test;
        cx.refresh_windows();
    }
}

/// "Send test": a banner as "needs input" would be, about the terminal in front.
pub(crate) fn send_test(cx: &mut App) {
    match permission(cx) {
        Permission::Allowed { .. } | Permission::OutsideBundle => deliver(cx),
        Permission::Denied => set(Test::Denied, cx),
        Permission::NotAsked => {
            set(Test::Waiting { asked: true }, cx);
            ask_permission(cx);
            // A route with no macOS to ask answers at once; the centre's answer comes to `permission_arrived`.
            if matches!(permission(cx), Permission::Allowed { .. }) {
                deliver(cx);
            }
        }
        Permission::Checking => {
            set(Test::Waiting { asked: false }, cx);
            refresh_permission(cx);
        }
    }
}

/// macOS said `permission`, in answer to our question when `to_ask`, else as it was read. Only a
/// press that waits for it cares.
pub(super) fn permission_arrived(permission: Permission, to_ask: bool, cx: &mut App) {
    let Test::Waiting { asked } = notifier(cx).test else { return };
    match permission {
        Permission::Allowed { .. } | Permission::OutsideBundle => deliver(cx),
        Permission::Denied => set(Test::Denied, cx),
        // The question is up while the permission is polled: not an answer yet.
        Permission::NotAsked if asked && !to_ask => {}
        Permission::NotAsked if asked => set(Test::Unanswered, cx),
        Permission::NotAsked => {
            set(Test::Waiting { asked: true }, cx);
            ask_permission(cx);
        }
        Permission::Checking => {}
    }
}

/// The centre's answer to a posted request (from its completion handler).
pub(super) fn posted(id: &str, error: Option<String>, cx: &mut App) {
    if id == TEST_ID
        && let Some(error) = error
    {
        set(Test::Refused(error), cx);
    }
}

fn deliver(cx: &mut App) {
    let name = AppState::front_window(cx)
        .and_then(|front| front.workspace.read_with(cx, |ws, _| ws.active_session_name()).ok())
        .unwrap_or_else(|| "Terminal".into());
    let level = notify::interruption_level(Kind::NeedsYou, time_sensitive(cx));
    let sound = notify::sound(Kind::NeedsYou, level, settings(cx).notify_sound);
    let failed = post(
        Notice {
            id: TEST_ID.to_string(),
            thread: TEST_ID.to_string(),
            title: format!("{name} needs input"),
            subtitle: "Wormhole · test".to_string(),
            body: "This is how a terminal that needs input is announced.".to_string(),
            level,
            sound,
            category: Category::Plain,
        },
        cx,
    );
    set(failed.map_or(Test::Sent, Test::Refused), cx);
}
