//! The Updates section: automatic updates and the channel.

use super::{entry, group};
use crate::config::settings_index::Group;

pub(super) const UPDATES: &[Group] = &[group(
    "Updates",
    &[
        entry("version", "Version", "", "release build", &[]),
        entry(
            "auto-update",
            "Update automatically",
            "Checks in the background and installs on the next restart",
            "download auto",
            &["auto_update"],
        ),
        entry(
            "update-channel",
            "Channel",
            "Unstable also gets a build of every change once it passes the tests, and shows the \
             feature flags",
            "beta nightly unstable stable prerelease dev builds main feature flags",
            &["update_channel"],
        ),
        entry("update-status", "Check for updates", "", "install restart release notes check skip", &[]),
    ],
)];
