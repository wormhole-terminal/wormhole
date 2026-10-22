//! The system's own notifications, for the routes GPUI can't serve: the notification centre of the
//! bundled app (interruption levels, a thread per terminal, sound, buttons and a Reply field, none of
//! which GPUI's notifications carry) and a script for a build without a bundle. What is said, when,
//! by which route, and what a click does are the app's (`notifications`); this only posts,
//! takes down and asks.
//!
//! The centre installs its own delegate, replacing the one GPUI sets for its notifications, so the
//! app never posts through GPUI on the centre's route ([`Center`]). Where the system has neither
//! ([`available`] says no), the app uses GPUI's portable notifications instead.

use std::io;

use gpui_kit::App;
use wormhole_core::notify::Level;

use super::os::notifications as os;
use super::spawn::spawn_detached;

/// The notification centre of the bundled app: `None` from `open` where there is none.
pub use super::os::notifications::Center;

/// Which buttons a notification carries.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Category {
    /// Open, Reply…
    NeedsYou,
    /// Open, Show Changes.
    Finished,
    /// No buttons; a click opens.
    Plain,
}

/// One notification, ready for any route.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Notice {
    pub id: String,
    pub thread: String,
    pub title: String,
    pub subtitle: String,
    pub body: String,
    pub level: Level,
    pub sound: bool,
    pub category: Category,
}

/// A click on one of our notifications, or one of its buttons.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Response {
    pub id: String,
    /// `None` for a click on the notification itself.
    pub action: Option<String>,
    /// What was typed into Reply.
    pub reply: Option<String>,
}

/// Whether the system lets Wormhole notify.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Permission {
    Checking,
    /// The system hasn't asked yet; nothing is shown until it has.
    NotAsked,
    /// `time_sensitive`: the user lets Time Sensitive notifications through.
    Allowed {
        time_sensitive: bool,
    },
    /// Off in System Settings.
    Denied,
    /// A build without a bundle: the script, no buttons or grouping.
    OutsideBundle,
}

/// What the notification centre reports back, from any thread.
pub enum FromCenter {
    Response(Response),
    Permission(Permission),
}

/// A button of a notification: what it answers with, its label, and what pressing it does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Button {
    pub id: &'static str,
    pub label: &'static str,
    pub kind: ButtonKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonKind {
    /// Brings the app forward.
    Foreground,
    /// A text field: what is typed arrives as the response's `reply`, the app left where it is.
    Reply,
}

/// The buttons of each category, which the centre registers as it opens.
pub type Buttons = fn(Category) -> &'static [Button];

/// Whether the system has notifications of its own beyond GPUI's: the centre in a bundle, a
/// script without one. Where it hasn't, the app posts through GPUI.
pub fn available() -> bool {
    os::available()
}

/// Whether this process may post Time Sensitive notifications (read once): only a build signed
/// with the entitlement; everything else delivers them at Active.
pub fn entitled_to_time_sensitive() -> bool {
    os::entitled_to_time_sensitive()
}

/// Post `notice` by the script a build without a bundle uses (no buttons, levels or threads).
pub fn post_by_script(notice: &Notice) -> io::Result<()> {
    match os::script(notice) {
        Some(command) => spawn_detached(command),
        None => Err(io::Error::from(io::ErrorKind::Unsupported)),
    }
}

/// Open the system's settings for Wormhole's notifications, where there are any.
pub fn open_settings(cx: &App) {
    os::open_settings(cx);
}
