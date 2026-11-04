//! The Notifications section: the permission, which events notify, delivery and the Dock.

use super::{entry, group};
use crate::config::settings_index::Group;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;

pub(super) const NOTIFICATIONS: &[Group] = &[
    group(
        SETTINGS_NOTIFICATIONS_GROUP_MACOS,
        &[entry("notify-permission", SETTINGS_NOTIFICATIONS_NOTIFY_PERMISSION, &[])
            .with_keywords(SETTINGS_NOTIFICATIONS_NOTIFY_PERMISSION_KEYWORDS)],
    ),
    group(
        SETTINGS_NOTIFICATIONS_GROUP_NOTIFY_ME_WHEN,
        &[
            entry("notify-needs-you", SETTINGS_NOTIFICATIONS_NOTIFY_NEEDS_YOU, &["notify_needs_you"])
                .with_description(SETTINGS_NOTIFICATIONS_NOTIFY_NEEDS_YOU_DESCRIPTION)
                .with_keywords(SETTINGS_NOTIFICATIONS_NOTIFY_NEEDS_YOU_KEYWORDS),
            entry("notify-agent-done", SETTINGS_NOTIFICATIONS_NOTIFY_AGENT_DONE, &["notify_agent_done"])
                .with_description(SETTINGS_NOTIFICATIONS_NOTIFY_AGENT_DONE_DESCRIPTION)
                .with_keywords(SETTINGS_NOTIFICATIONS_NOTIFY_AGENT_DONE_KEYWORDS),
            entry(
                "notify-command-done",
                SETTINGS_NOTIFICATIONS_NOTIFY_COMMAND_DONE,
                &["notify_command_done", "notify_command_after"],
            )
            .with_description(SETTINGS_NOTIFICATIONS_NOTIFY_COMMAND_DONE_DESCRIPTION)
            .with_keywords(SETTINGS_NOTIFICATIONS_NOTIFY_COMMAND_DONE_KEYWORDS),
            entry(
                "notify-command-failed",
                SETTINGS_NOTIFICATIONS_NOTIFY_COMMAND_FAILED,
                &["notify_command_failed"],
            )
            .with_description(SETTINGS_NOTIFICATIONS_NOTIFY_COMMAND_FAILED_DESCRIPTION)
            .with_keywords(SETTINGS_NOTIFICATIONS_NOTIFY_COMMAND_FAILED_KEYWORDS),
            entry("notify-bell", SETTINGS_NOTIFICATIONS_NOTIFY_BELL, &["notify_bell"])
                .with_description(SETTINGS_NOTIFICATIONS_NOTIFY_BELL_DESCRIPTION)
                .with_keywords(SETTINGS_NOTIFICATIONS_NOTIFY_BELL_KEYWORDS),
        ],
    ),
    group(
        SETTINGS_NOTIFICATIONS_GROUP_DELIVERY,
        &[
            entry("notify-only-unseen", SETTINGS_NOTIFICATIONS_NOTIFY_ONLY_UNSEEN, &["notify_only_unseen"])
                .with_description(SETTINGS_NOTIFICATIONS_NOTIFY_ONLY_UNSEEN_DESCRIPTION)
                .with_keywords(SETTINGS_NOTIFICATIONS_NOTIFY_ONLY_UNSEEN_KEYWORDS),
            entry("notify-in-front", SETTINGS_NOTIFICATIONS_NOTIFY_IN_FRONT, &["notify_in_front"])
                .with_description(SETTINGS_NOTIFICATIONS_NOTIFY_IN_FRONT_DESCRIPTION)
                .with_keywords(SETTINGS_NOTIFICATIONS_NOTIFY_IN_FRONT_KEYWORDS),
            entry("notify-sound", SETTINGS_NOTIFICATIONS_NOTIFY_SOUND, &["notify_sound"])
                .with_keywords(SETTINGS_NOTIFICATIONS_NOTIFY_SOUND_KEYWORDS),
            entry(
                "notify-group-bursts",
                SETTINGS_NOTIFICATIONS_NOTIFY_GROUP_BURSTS,
                &["notify_group_bursts"],
            )
            .with_description(SETTINGS_NOTIFICATIONS_NOTIFY_GROUP_BURSTS_DESCRIPTION)
            .with_keywords(SETTINGS_NOTIFICATIONS_NOTIFY_GROUP_BURSTS_KEYWORDS),
        ],
    ),
    group(
        SETTINGS_NOTIFICATIONS_GROUP_DOCK,
        &[
            entry("dock-badge", SETTINGS_NOTIFICATIONS_DOCK_BADGE, &["dock_badge"])
                .with_description(SETTINGS_NOTIFICATIONS_DOCK_BADGE_DESCRIPTION)
                .with_keywords(SETTINGS_NOTIFICATIONS_DOCK_BADGE_KEYWORDS),
            entry("dock-bounce", SETTINGS_NOTIFICATIONS_DOCK_BOUNCE, &["dock_bounce"])
                .with_description(SETTINGS_NOTIFICATIONS_DOCK_BOUNCE_DESCRIPTION)
                .with_keywords(SETTINGS_NOTIFICATIONS_DOCK_BOUNCE_KEYWORDS),
        ],
    ),
];
