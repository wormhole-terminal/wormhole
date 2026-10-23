//! What a click or a button on a notification does, and taking one down.

use gpui_kit::*;
use wormhole_core::notify::{self};

use super::route::Route;
use super::{REPLY, SHOW_CHANGES, TEST_ID, notifier};
use crate::app_state::AppState;
use crate::platform::notifications::Response;
use crate::window::window_lifecycle::open_tab;

use wormhole_core::config::TabId;

/// Take down a notification by its id, if it is up.
pub(super) fn take_down(id: &str, cx: &mut App) {
    let notifier = notifier(cx);
    if !notifier.up.remove(id) {
        return;
    }
    match notifier.route {
        Route::Center => {
            if let Some(center) = &notifier.center {
                center.remove(id);
            }
        }
        Route::Gpui => cx.dismiss_system_notification(id),
        Route::Log => notifier.log.retain(|notice| notice.id != id),
        _ => {}
    }
}

/// Take down `tab`'s notification, if there is one.
pub(crate) fn dismiss(tab: i64, cx: &mut App) {
    take_down(&notify::notice_id(tab), cx);
}

/// A tab's notification tag.
#[cfg(test)]
pub(crate) fn tag(tab: i64) -> SharedString {
    notify::notice_id(tab).into()
}

/// A click or an action on one of our notifications (from the platform, outside any window's
/// update): the window, terminal and tab come forward; Reply types into the tab and presses
/// Return without bringing anything forward.
pub(crate) fn respond(response: Response, cx: &mut App) {
    tracing::debug!(?response, "notification response");
    take_down(&response.id, cx);
    let Some(tab) = notify::notice_tab(&response.id) else {
        // The summary or the test banner: Wormhole comes forward.
        if response.id == notify::SUMMARY_ID || response.id == TEST_ID {
            crate::background::activate(cx);
            if let Some(front) = AppState::front_window(cx) {
                let _ =
                    front.handle.update(cx, |_, window, cx| crate::background::activate_window(window, cx));
            }
        }
        return;
    };
    match response.action.as_deref() {
        Some(REPLY) => {
            if let Some(text) = response.reply {
                reply(tab, &text, cx);
            }
        }
        Some(SHOW_CHANGES) => {
            crate::background::activate(cx);
            open_tab(tab, cx);
            if let Some(owner) = AppState::find_window(cx, |ws, _| ws.find_session(tab).is_some()) {
                AppState::with_workspace(&owner, cx, |ws, _, cx| ws.show_tab(TabId::Changes, cx));
            }
        }
        _ => {
            crate::background::activate(cx);
            open_tab(tab, cx);
        }
    }
}

/// Type `text` into `tab`'s terminal and press Return (Reply, from a notification or the quick
/// panel), in whichever window it is.
pub(crate) fn reply(tab: i64, text: &str, cx: &mut App) {
    let Some(owner) = AppState::find_window(cx, |ws, _| ws.find_session(tab).is_some()) else { return };
    AppState::with_workspace(&owner, cx, |ws, _, cx| ws.type_reply(tab, text, cx));
}
