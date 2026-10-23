//! Whether macOS lets Wormhole notify, and asking it.

use gpui_kit::*;

#[cfg(any(test, feature = "devctl"))]
use super::route::Route;
use super::{Notifier, notifier};
use crate::platform::notifications::{self as system, Permission};

/// Whether macOS lets Wormhole notify, as last read.
pub(crate) fn permission(cx: &App) -> Permission {
    cx.try_global::<Notifier>().and_then(|n| n.permission).unwrap_or(Permission::OutsideBundle)
}

/// Read the permission again (the answer arrives a moment later).
pub(crate) fn refresh_permission(cx: &mut App) {
    #[cfg(test)]
    {
        notifier(cx).refreshes += 1;
    }
    if let Some(center) = &notifier(cx).center {
        center.refresh_permission();
    }
}

/// "Allow notifications": macOS asks the user (once; after that only System Settings can).
pub(crate) fn ask_permission(cx: &mut App) {
    #[cfg(any(test, feature = "devctl"))]
    {
        let notifier = notifier(cx);
        notifier.asks += 1;
        // No macOS to answer in a test or a test instance, so the answer is yes. (The bundled app
        // never gets here without a center: the route is `Center`, or the permission is not asked.)
        if matches!(notifier.route, Route::Gpui | Route::Log) {
            notifier.permission = Some(Permission::Allowed { time_sensitive: false });
        }
    }
    if let Some(center) = &notifier(cx).center {
        center.ask();
    }
}

/// The welcome's presses that turn a banner on or say Allow: macOS is asked, but only when it has not
/// been asked yet (after that its answer stands, and only System Settings can change it). Returns
/// whether it was asked.
pub(crate) fn ask_permission_if_new(cx: &mut App) -> bool {
    if permission(cx) != Permission::NotAsked {
        return false;
    }
    ask_permission(cx);
    true
}

/// How many times macOS was asked in this run.
#[cfg(any(test, feature = "devctl"))]
pub(crate) fn permission_asks(cx: &App) -> u32 {
    cx.try_global::<Notifier>().map_or(0, |notifier| notifier.asks)
}

/// A test or a test instance pretends macOS has said `permission`.
#[cfg(any(test, feature = "devctl"))]
pub(crate) fn pretend_permission(permission: Permission, cx: &mut App) {
    notifier(cx).permission = Some(permission);
    cx.refresh_windows();
}

/// "Open System Settings": Wormhole's page under Notifications.
pub(crate) fn open_system_settings(cx: &App) {
    system::open_settings(cx);
}

/// How many times the permission was read again.
#[cfg(test)]
pub(crate) fn permission_refreshes(cx: &App) -> u32 {
    cx.try_global::<Notifier>().map_or(0, |n| n.refreshes)
}
