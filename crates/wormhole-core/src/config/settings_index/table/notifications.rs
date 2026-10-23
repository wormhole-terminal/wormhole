//! The Notifications section: the permission, which events notify, delivery and the Dock.

use super::{entry, group};
use crate::config::settings_index::Group;

pub(super) const NOTIFICATIONS: &[Group] = &[
    group(
        "macOS",
        &[entry(
            "notify-permission",
            "Notifications in System Settings",
            "",
            "permission allow system settings send test banner",
            &[],
        )],
    ),
    group(
        "Notify me when",
        &[
            entry(
                "notify-needs-you",
                "A terminal needs you",
                "An agent asks a question or for permission",
                "time sensitive focus question permission notifications",
                &["notify_needs_you"],
            ),
            entry(
                "notify-agent-done",
                "An agent finishes",
                "Its Stop hook, or its output settles. A program that reports it is done or failed counts too",
                "notifications done program report failed error",
                &["notify_agent_done"],
            ),
            entry(
                "notify-command-done",
                "A command finishes",
                "Any foreground command, agent or not, that ran longer than",
                "long build threshold seconds notifications",
                &["notify_command_done", "notify_command_after"],
            ),
            entry(
                "notify-command-failed",
                "A command fails",
                "Non-zero exit code. Needs shell integration (on for zsh, bash and fish)",
                "failed failure error exit code build notifications",
                &["notify_command_failed"],
            ),
            entry(
                "notify-bell",
                "The terminal rings the bell",
                "Terminals without an agent; at most once a minute each",
                "bell notifications",
                &["notify_bell"],
            ),
        ],
    ),
    group(
        "Delivery",
        &[
            entry(
                "notify-only-unseen",
                "Only for terminals I’m not looking at",
                "Off: the terminal in front notifies too",
                "unseen front",
                &["notify_only_unseen"],
            ),
            entry(
                "notify-in-front",
                "While Wormhole is in front",
                "A system banner, or nothing",
                "banner nothing front",
                &["notify_in_front"],
            ),
            entry("notify-sound", "Sound", "", "sound alert", &["notify_sound"]),
            entry(
                "notify-group-bursts",
                "Group bursts",
                "Three or more within 10 s become one summary",
                "summary bursts",
                &["notify_group_bursts"],
            ),
        ],
    ),
    group(
        "Dock",
        &[
            entry("dock-badge", "Badge", "The count on the Dock icon", "dock badge count", &["dock_badge"]),
            entry(
                "dock-bounce",
                "Bounce once when a terminal needs you",
                "Only while Wormhole isn’t the active app; never for a bell",
                "dock bounce",
                &["dock_bounce"],
            ),
        ],
    ),
];
