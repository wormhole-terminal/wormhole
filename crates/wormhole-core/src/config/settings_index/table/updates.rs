//! The Updates section: automatic updates and the channel, and the privacy of an update check.

use super::{entry, group};
use crate::config::settings_index::Group;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;

pub(super) const UPDATES: &[Group] = &[
    group(
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
    ),
    // Why update checks are counted, the switch, and exactly what a check sends.
    group(
        SETTINGS_UPDATES_GROUP_PRIVACY,
        &[
            entry("anonymous-count", SETTINGS_UPDATES_ANONYMOUS_COUNT, &["anonymous_count"])
                .with_description(SETTINGS_UPDATES_ANONYMOUS_COUNT_DESCRIPTION)
                .with_keywords(SETTINGS_UPDATES_ANONYMOUS_COUNT_KEYWORDS),
            entry("count-details", SETTINGS_UPDATES_COUNT_DETAILS, &[])
                .with_keywords(SETTINGS_UPDATES_COUNT_DETAILS_KEYWORDS),
        ],
    )
    .with_blurb(SETTINGS_UPDATES_GROUP_PRIVACY_BLURB),
];
