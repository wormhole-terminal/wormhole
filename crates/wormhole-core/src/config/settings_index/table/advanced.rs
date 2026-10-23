//! The Advanced section: resetting settings and tips.

use super::{literal_entry, literal_group};
use crate::config::settings_index::Group;

pub(super) const ADVANCED: &[Group] = &[
    literal_group(
        "Files",
        &[
            literal_entry(
                "open-config",
                "Settings file",
                "config.json: edits apply live",
                "json config",
                &[],
            ),
            literal_entry(
                "open-data",
                "Data folder",
                "Terminals, notes, themes, logs",
                "finder database",
                &[],
            ),
            literal_entry("open-logs", "Logs", "Set WORMHOLE_LOG=debug for more detail", "debug log", &[]),
            literal_entry(
                "reset-settings",
                "Reset all settings",
                "Your terminals, notes and themes are kept",
                "defaults reset restore",
                &[],
            ),
        ],
    ),
    literal_group(
        "Tips",
        &[literal_entry(
            "reset-tips",
            "Closed tips",
            "Bring back the tips closed with “Don't show again”, such as the sidebar's coding agents note",
            "hints tips reset show again dismissed",
            &[],
        )],
    ),
];
