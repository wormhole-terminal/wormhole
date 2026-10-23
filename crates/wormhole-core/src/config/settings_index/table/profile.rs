//! The Profile section: a profile's name, start folder, environment, git identity and agent logins, and deleting it.

use super::{entry, group, untitled};
use crate::config::settings_index::Group;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;

pub(super) const PROFILE: &[Group] = &[
    untitled(&[
        entry("profile-name", SETTINGS_PROFILE_NAME, &[]).with_keywords(SETTINGS_PROFILE_NAME_KEYWORDS),
        entry("profile-start", SETTINGS_PROFILE_START, &[]).with_keywords(SETTINGS_PROFILE_START_KEYWORDS),
        entry("profile-env", SETTINGS_PROFILE_ENV, &[])
            .with_description(SETTINGS_PROFILE_ENV_DESCRIPTION)
            .with_keywords(SETTINGS_PROFILE_ENV_KEYWORDS),
        entry("profile-git", SETTINGS_PROFILE_GIT, &[])
            .with_description(SETTINGS_PROFILE_GIT_DESCRIPTION)
            .with_keywords(SETTINGS_PROFILE_GIT_KEYWORDS),
        entry("profile-logins", SETTINGS_PROFILE_LOGINS, &[])
            .with_description(SETTINGS_PROFILE_LOGINS_DESCRIPTION)
            .with_keywords(SETTINGS_PROFILE_LOGINS_KEYWORDS),
    ]),
    group(
        SETTINGS_PROFILE_GROUP_DANGER_ZONE,
        &[entry("profile-delete", SETTINGS_PROFILE_DELETE, &[])
            .with_description(SETTINGS_PROFILE_DELETE_DESCRIPTION)
            .with_keywords(SETTINGS_PROFILE_DELETE_KEYWORDS)],
    ),
];
