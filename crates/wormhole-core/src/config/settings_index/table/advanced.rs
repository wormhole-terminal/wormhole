//! The Advanced section: resetting settings and tips.

use super::{entry, group};
use crate::config::settings_index::Group;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;

pub(super) const ADVANCED: &[Group] = &[
    group(
        SETTINGS_ADVANCED_GROUP_FILES,
        &[
            entry("open-config", SETTINGS_ADVANCED_OPEN_CONFIG, &[])
                .with_description(SETTINGS_ADVANCED_OPEN_CONFIG_DESCRIPTION)
                .with_keywords(SETTINGS_ADVANCED_OPEN_CONFIG_KEYWORDS),
            entry("open-data", SETTINGS_ADVANCED_OPEN_DATA, &[])
                .with_description(SETTINGS_ADVANCED_OPEN_DATA_DESCRIPTION)
                .with_keywords(SETTINGS_ADVANCED_OPEN_DATA_KEYWORDS),
            entry("open-logs", SETTINGS_ADVANCED_OPEN_LOGS, &[])
                .with_description(SETTINGS_ADVANCED_OPEN_LOGS_DESCRIPTION)
                .with_keywords(SETTINGS_ADVANCED_OPEN_LOGS_KEYWORDS),
            entry("reset-settings", SETTINGS_ADVANCED_RESET_SETTINGS, &[])
                .with_description(SETTINGS_ADVANCED_RESET_SETTINGS_DESCRIPTION)
                .with_visible_hint()
                .with_keywords(SETTINGS_ADVANCED_RESET_SETTINGS_KEYWORDS),
        ],
    ),
    group(
        SETTINGS_ADVANCED_GROUP_TIPS,
        &[entry("reset-tips", SETTINGS_ADVANCED_RESET_TIPS, &[])
            .with_description(SETTINGS_ADVANCED_RESET_TIPS_DESCRIPTION)
            .with_keywords(SETTINGS_ADVANCED_RESET_TIPS_KEYWORDS)],
    ),
    group(
        SETTINGS_ADVANCED_GROUP_PROGRAMS,
        &[entry("skip-scans", SETTINGS_ADVANCED_SKIP_SCANS, &[])
            // Only where the system has a Developer Tools list.
            .with_needs(|_| crate::platform::system::has_developer_tools_list())
            .with_description(SETTINGS_ADVANCED_SKIP_SCANS_DESCRIPTION)
            .with_keywords(SETTINGS_ADVANCED_SKIP_SCANS_KEYWORDS)],
    ),
];
