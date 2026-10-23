//! The Profile section: a profile's name, start folder, environment, git identity and agent logins, and deleting it.

use super::{literal_entry, literal_group};
use crate::config::settings_index::Group;

pub(super) const PROFILE: &[Group] = &[
    literal_group(
        "",
        &[
            literal_entry(
                "profile-name",
                "Name and colour",
                "",
                "profile rename avatar letter swatch colour color edge",
                &[],
            ),
            literal_entry(
                "profile-start",
                "Start new terminals in",
                "",
                "folder directory cwd home path open also claim",
                &[],
            ),
            literal_entry(
                "profile-env",
                "Environment",
                "Variables this profile's terminals start with",
                "env variable aws path registry export",
                &[],
            ),
            literal_entry(
                "profile-git",
                "Git identity",
                "Set in its terminals only; Wormhole never writes to a repository.",
                "author committer name email commit user",
                &[],
            ),
            literal_entry(
                "profile-logins",
                "Agent logins",
                "A separate login for each agent.",
                "claude codex account config dir home sign in",
                &[],
            ),
        ],
    ),
    literal_group(
        "Danger zone",
        &[literal_entry(
            "profile-delete",
            "Delete profile",
            "Its terminals, notes, tasks and settings are removed and its windows close.",
            "remove danger",
            &[],
        )],
    ),
];
