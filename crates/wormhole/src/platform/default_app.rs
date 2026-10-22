//! Which app the system runs a kind of file with (as Terminal runs a `.command`), and making
//! Wormhole that app. Which kinds, and what a terminal does with a file it is asked to open, are
//! the app's (`workspace::default_terminal`, core's `default_terminal`).

use super::os::default_app as os;

/// The bundle identifier this build runs under: `None` outside an app bundle (a `cargo run`
/// build), which can't be anyone's default.
pub fn own_bundle() -> Option<String> {
    os::own_bundle()
}

/// The bundle identifier of the app that runs files of `kind` (a content type), if any.
pub fn runner_for(kind: &str) -> Option<String> {
    os::runner_for(kind)
}

/// Make the app with `bundle` the one that runs files of `kind`.
pub fn set_runner_for(kind: &str, bundle: &str) -> anyhow::Result<()> {
    os::set_runner_for(kind, bundle)
}
