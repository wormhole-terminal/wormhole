//! The Updates section: automatic updates and the channel.

use super::{literal_entry, literal_group};
use crate::config::settings_index::Group;

pub(super) const UPDATES: &[Group] = &[literal_group(
    "Updates",
    &[
        literal_entry("version", "Version", "", "release build", &[]),
        literal_entry(
            "auto-update",
            "Update automatically",
            "Checks in the background and installs on the next restart",
            "download auto",
            &["auto_update"],
        ),
        literal_entry(
            "update-channel",
            "Channel",
            "Unstable also gets a build of every change once it passes the tests, and shows the \
             feature flags",
            "beta nightly unstable stable prerelease dev builds main feature flags",
            &["update_channel"],
        ),
        literal_entry(
            "update-status",
            "Check for updates",
            "",
            "install restart release notes check skip",
            &[],
        ),
    ],
)];
