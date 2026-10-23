//! The Updates section: automatic updates and the channel.

use super::{entry, group};
use crate::config::settings_index::Group;
use wormhole_text::keys::*;

pub(super) const UPDATES: &[Group] = &[group(
    SETTINGS_UPDATES_GROUP_UPDATES,
    &[
        entry("version", SETTINGS_UPDATES_VERSION, &[]).with_keywords(SETTINGS_UPDATES_VERSION_KEYWORDS),
        entry("auto-update", SETTINGS_UPDATES_AUTO_UPDATE, &["auto_update"])
            .with_description(SETTINGS_UPDATES_AUTO_UPDATE_DESCRIPTION)
            .with_keywords(SETTINGS_UPDATES_AUTO_UPDATE_KEYWORDS),
        entry("update-channel", SETTINGS_UPDATES_UPDATE_CHANNEL, &["update_channel"])
            .with_description(SETTINGS_UPDATES_UPDATE_CHANNEL_DESCRIPTION)
            .with_keywords(SETTINGS_UPDATES_UPDATE_CHANNEL_KEYWORDS),
        entry("update-status", SETTINGS_UPDATES_UPDATE_STATUS, &[])
            .with_keywords(SETTINGS_UPDATES_UPDATE_STATUS_KEYWORDS),
    ],
)];
