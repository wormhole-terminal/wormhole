//! The Profile section: a profile's name, start folder, environment, git identity and agent logins, and deleting it.

use super::{entry, group};
use crate::config::settings_index::Group;

pub(super) const PROFILE: &[Group] = &[
    group(
        "",
        &[
            entry(
                "profile-name",
                "Name and colour",
                "",
                "profile rename avatar letter swatch colour color edge",
                &[],
            ),
            entry(
                "profile-start",
                "Start new terminals in",
                "",
                "folder directory cwd home path open also claim",
                &[],
            ),
            entry(
                "profile-env",
                "Environment",
                "Variables this profile's terminals start with",
                "env variable aws path registry export",
                &[],
            ),
            entry(
                "profile-git",
                "Git identity",
                "Set in its terminals only; Wormhole never writes to a repository.",
                "author committer name email commit user",
                &[],
            ),
            entry(
                "profile-logins",
                "Agent logins",
                "A separate login for each agent.",
                "claude codex account config dir home sign in",
                &[],
            ),
        ],
    ),
    group(
        "Danger zone",
        &[entry(
            "profile-delete",
            "Delete profile",
            "Its terminals, notes, tasks and settings are removed and its windows close.",
            "remove danger",
            &[],
        )],
    ),
];
