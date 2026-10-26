//! The Motion section: one choice of animation for each thing that moves. Each writes a key inside
//! `config.json`'s `motion` object (`motion.insert`, …), so its changed mark and reset are its own.

use super::{entry, group};
use crate::config::settings_index::Group;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;

pub(super) const MOTION: &[Group] = &[
    group(
        SETTINGS_MOTION_GROUP_TERMINALS,
        &[
            entry("motion-insert", SETTINGS_MOTION_INSERT, &["motion.insert"])
                .with_description(SETTINGS_MOTION_INSERT_DESCRIPTION)
                .with_keywords(SETTINGS_MOTION_INSERT_KEYWORDS),
            entry("motion-fold", SETTINGS_MOTION_FOLD, &["motion.fold"])
                .with_description(SETTINGS_MOTION_FOLD_DESCRIPTION)
                .with_keywords(SETTINGS_MOTION_FOLD_KEYWORDS),
            entry("motion-status", SETTINGS_MOTION_STATUS, &["motion.status"])
                .with_description(SETTINGS_MOTION_STATUS_DESCRIPTION)
                .with_keywords(SETTINGS_MOTION_STATUS_KEYWORDS),
        ],
    ),
    group(
        SETTINGS_MOTION_GROUP_WINDOW,
        &[entry("motion-column", SETTINGS_MOTION_COLUMN, &["motion.column"])
            .with_description(SETTINGS_MOTION_COLUMN_DESCRIPTION)
            .with_keywords(SETTINGS_MOTION_COLUMN_KEYWORDS)],
    ),
];
