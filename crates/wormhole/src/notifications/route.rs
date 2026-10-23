//! Which way a notification goes out: the system's centre, the platform's script, GPUI's own, devctl's log or
//! nowhere, and how a run decides.

use wormhole_core::update::current_app_bundle;

use crate::platform::notifications::{self as system};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) enum Route {
    /// The bundled app: the system's notification centre.
    Center,
    /// No bundle: the platform's script.
    Script,
    /// GPUI's notifications: tests, and a system without notifications of its own.
    Gpui,
    /// devctl: kept in [`Notifier::log`], never posted.
    Log,
    #[default]
    Off,
}

pub(super) fn route() -> Route {
    #[cfg(feature = "devctl")]
    let logs = logs(std::env::var("WORMHOLE_NOTIFY").ok().as_deref(), crate::background::on());
    #[cfg(not(feature = "devctl"))]
    let logs = false;
    route_for(cfg!(test), logs, system::available(), current_app_bundle().is_some())
}

/// The route for a run: a test is GPUI's test platform, devctl's log wins next, and a system with
/// notifications of its own posts through them (the centre in a bundle, the script without one).
/// GPUI's notifications are only for the rest: its macOS centre installs a delegate of its own,
/// which would take the bundled centre's replies away.
pub(super) fn route_for(test: bool, logs: bool, available: bool, bundled: bool) -> Route {
    match () {
        _ if test => Route::Gpui,
        _ if logs => Route::Log,
        _ if !available => Route::Gpui,
        _ if bundled => Route::Center,
        _ => Route::Script,
    }
}

/// The route once the centre has been opened (or not): a bundle whose centre can't be had falls
/// back to the script, never to GPUI.
pub(super) fn settled(route: Route, center_opened: bool) -> Route {
    if route == Route::Center && !center_opened { Route::Script } else { route }
}

/// devctl builds: whether notifications go to the log (`notify-log`) and nowhere else.
/// `WORMHOLE_NOTIFY=log` asks for it, and background mode does unless `WORMHOLE_NOTIFY`
/// names another route.
#[cfg_attr(not(feature = "devctl"), allow(dead_code))]
pub(super) fn logs(notify: Option<&str>, background: bool) -> bool {
    match notify {
        Some(route) => route == "log",
        None => background,
    }
}
