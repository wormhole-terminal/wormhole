//! The Coding agents section, and the Project folders and Feature flags sections that hold one setting each.

use super::{entry, group, untitled};
use crate::config::features::FeatureFlag;
use crate::config::settings_index::Group;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;

pub(super) const AGENTS: &[Group] = &[
    untitled(&[
        entry("agents-summary", SETTINGS_AGENTS_SUMMARY, &[]).with_keywords(SETTINGS_AGENTS_SUMMARY_KEYWORDS)
    ]),
    group(
        SETTINGS_AGENTS_GROUP_NOTIFICATIONS,
        &[entry("agents-notifications", SETTINGS_AGENTS_NOTIFICATIONS, &[])
            .with_description(SETTINGS_AGENTS_NOTIFICATIONS_DESCRIPTION)
            .with_keywords(SETTINGS_AGENTS_NOTIFICATIONS_KEYWORDS)],
    ),
    group(
        SETTINGS_AGENTS_GROUP_INSTALLED,
        &[entry("agents-installed", SETTINGS_AGENTS_INSTALLED, &[])
            .with_keywords(SETTINGS_AGENTS_INSTALLED_KEYWORDS)],
    ),
    group(
        SETTINGS_AGENTS_GROUP_OTHER_AGENTS,
        &[entry("agents-other", SETTINGS_AGENTS_OTHER, &[])
            .with_description(SETTINGS_AGENTS_OTHER_DESCRIPTION)
            .with_keywords(SETTINGS_AGENTS_OTHER_KEYWORDS)],
    ),
    group(
        SETTINGS_AGENTS_GROUP_HOW_STATUS_WORKS,
        &[entry("agents-how", SETTINGS_AGENTS_HOW, &[])
            .with_description(SETTINGS_AGENTS_HOW_DESCRIPTION)
            .with_keywords(SETTINGS_AGENTS_HOW_KEYWORDS)],
    ),
    group(
        SETTINGS_AGENTS_GROUP_COMMAND_LINE,
        &[
            // The command's name is on the row itself (`ipc::control::COMMAND_NAME`); the keywords
            // carry the names it has been called, so a search finds the row by either.
            entry("cli-tool", SETTINGS_AGENTS_CLI_TOOL, &[])
                .with_description(SETTINGS_AGENTS_CLI_TOOL_DESCRIPTION)
                .with_keywords(SETTINGS_AGENTS_CLI_TOOL_KEYWORDS),
            // While Automation is on, Agent access below decides who may type and open.
            entry("control-write", SETTINGS_AGENTS_CONTROL_WRITE, &["control_write"])
                .with_needs(|s| !s.feature_enabled(FeatureFlag::Automation))
                .with_description(SETTINGS_AGENTS_CONTROL_WRITE_DESCRIPTION)
                .with_keywords(SETTINGS_AGENTS_CONTROL_WRITE_KEYWORDS),
        ],
    )
    .with_blurb(SETTINGS_AGENTS_GROUP_COMMAND_LINE_BLURB),
    // Who may control Wormhole through the command line, MCP and the socket (`agent-permissions.md`).
    group(
        SETTINGS_AGENTS_GROUP_ACCESS,
        &[
            entry("agent-access", SETTINGS_AGENTS_ACCESS, &["agent_access"])
                .with_flag(FeatureFlag::Automation)
                .with_description(SETTINGS_AGENTS_ACCESS_DESCRIPTION)
                .with_keywords(SETTINGS_AGENTS_ACCESS_KEYWORDS),
            entry("agent-access-programs", SETTINGS_AGENTS_ACCESS_PROGRAMS, &[])
                .with_flag(FeatureFlag::Automation)
                .with_keywords(SETTINGS_AGENTS_ACCESS_PROGRAMS_KEYWORDS),
        ],
    )
    .with_blurb(SETTINGS_AGENTS_GROUP_ACCESS_BLURB),
    // The skill describes the automation verbs, so it comes with the flag.
    group(
        SETTINGS_AGENTS_GROUP_SKILL,
        &[
            entry("agent-skills", SETTINGS_AGENTS_SKILLS, &[])
                .with_flag(FeatureFlag::Automation)
                .with_description(SETTINGS_AGENTS_SKILLS_DESCRIPTION)
                .with_keywords(SETTINGS_AGENTS_SKILLS_KEYWORDS),
            entry("agent-mcp", SETTINGS_AGENTS_SKILL_MCP, &[])
                .with_flag(FeatureFlag::Automation)
                .with_description(SETTINGS_AGENTS_SKILL_MCP_DESCRIPTION)
                .with_keywords(SETTINGS_AGENTS_SKILL_MCP_KEYWORDS),
        ],
    )
    .with_blurb(SETTINGS_AGENTS_GROUP_SKILL_BLURB),
];

pub(super) const PROJECTS: &[Group] = &[untitled(&[entry("projects", SETTINGS_PROJECTS_PROJECTS, &[])
    .with_description(SETTINGS_PROJECTS_PROJECTS_DESCRIPTION)
    .with_keywords(SETTINGS_PROJECTS_PROJECTS_KEYWORDS)])];

pub(super) const FEATURE_FLAGS: &[Group] =
    &[untitled(&[entry("feature-flags", SETTINGS_FEATURE_FLAGS_FEATURE_FLAGS, &["feature_flags"])
        .with_description(SETTINGS_FEATURE_FLAGS_FEATURE_FLAGS_DESCRIPTION)
        .with_keywords(SETTINGS_FEATURE_FLAGS_FEATURE_FLAGS_KEYWORDS)])];
