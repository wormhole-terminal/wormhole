//! The Advanced section: resetting settings and tips.

use super::{entry, group};
use crate::config::settings_index::Group;

pub(super) const ADVANCED: &[Group] = &[
    group(
        "Files",
        &[
            entry("open-config", "Settings file", "config.json: edits apply live", "json config", &[]),
            entry("open-data", "Data folder", "Terminals, notes, themes, logs", "finder database", &[]),
            entry("open-logs", "Logs", "Set WORMHOLE_LOG=debug for more detail", "debug log", &[]),
            entry(
                "reset-settings",
                "Reset all settings",
                "Your terminals, notes and themes are kept",
                "defaults reset restore",
                &[],
            ),
        ],
    ),
    group(
        "Tips",
        &[entry(
            "reset-tips",
            "Closed tips",
            "Bring back the tips closed with “Don't show again”, such as the sidebar's coding agents note",
            "hints tips reset show again dismissed",
            &[],
        )],
    ),
];
