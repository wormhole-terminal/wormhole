//! The Coding agents section, and the Project folders and Feature flags sections that hold one setting each.

use super::{entry, group};
use crate::config::settings_index::Group;

pub(super) const AGENTS: &[Group] = &[
    group("", &[entry("agents-summary", "Agents found", "", "installed hooks repair path summary", &[])]),
    group(
        "Notifications",
        &[entry(
            "agents-notifications",
            "Notify me when an agent needs you or finishes",
            "These, and notifications about long commands and the bell, have their own section",
            "notifications alerts banner",
            &[],
        )],
    ),
    group(
        "Installed",
        &[entry(
            "agents-installed",
            "Installed agents",
            "",
            "claude code codex gemini copilot cursor opencode aider amp goose qwen hooks status install \
             remove repair",
            &[],
        )],
    ),
    group(
        "Other agents",
        &[entry(
            "agents-other",
            "Other agents",
            "Agents Wormhole recognises that aren't on this Mac's PATH",
            "not installed path",
            &[],
        )],
    ),
    group(
        "How status works",
        &[entry(
            "agents-how",
            "How status works",
            "With status hooks the agent tells Wormhole the moment it starts working, waits for you or \
             finishes; without them the status is read from the terminal output",
            "hooks output heuristic backup",
            &[],
        )],
    ),
    Group {
        title: "Command line",
        blurb: "Scripts and agents can drive Wormhole from the command line: list terminals, read a \
                status and send notifications.",
        aside: "",
        entries: &[
            // The command's name is on the row itself (`ipc::control::COMMAND_NAME`); the keywords
            // carry the names it has been called, so a search finds the row by either.
            entry(
                "cli-tool",
                "Command line tool",
                "The Wormhole command in any terminal",
                "wh wht cli command install uninstall path shell alias symlink bin usr local terminal",
                &[],
            ),
            entry(
                "control-write",
                "Let the command line type and open terminals",
                "send-text types into a terminal and new-terminal opens one, as a paste; never into the \
                 terminal that asks, nor one waiting for your answer",
                "cli command line control api socket send-text new-terminal scripts agents automation \
                 remote type",
                &["control_write"],
            ),
        ],
    },
];

pub(super) const PROJECTS: &[Group] = &[group(
    "",
    &[entry(
        "projects",
        "Project folders",
        "The .wormhole folder of a project: trust, and which of its files you use",
        ".wormhole project folder trust touch id actions prompts layout notes theme settings",
        &[],
    )],
)];

pub(super) const FEATURE_FLAGS: &[Group] = &[group(
    "",
    &[entry(
        "feature-flags",
        "Feature flags",
        "Turn features that are still being built on or off in this build",
        "feature flag preview experimental development dev in progress beta unstable project folders",
        &["feature_flags"],
    )],
)];
