//! Coding agents: the agents on this Mac with their hooks, "Other agents", the programs let in, the skills.
//! The reads are `agents_read::AgentsSnapshot`'s, the writes the integrations' own.

use std::path::Path;

use gpui_kit::App;
use serde_json::{Map, Value, json};
use wormhole_core::agents::integrations::{self, State};
use wormhole_core::agents::skill::{self, State as SkillState};
use wormhole_core::agents::{self, AGENTS, Agent, AgentId, Tile};
use wormhole_core::ipc::access::Capability;
use wormhole_core::ipc::control::doctor::mcp_add_command;
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::{text, whole};

use super::Entry;
use crate::agent_access::{self, Approved, How};
use crate::app_state::app;
use crate::settings_window::native::agents_read::AgentsSnapshot;

const GROUP_AGENTS: &str = "Agents:0"; // text: not shown (a key or page id of the native window, or a log line)
const GROUP_COMMAND_LINE: &str = "Agents:1"; // text: not shown (a key or page id of the native window, or a log line)
const GROUP_ACCESS: &str = "Agents:2"; // text: not shown (a key or page id of the native window, or a log line)
const GROUP_SKILLS: &str = "Agents:3"; // text: not shown (a key or page id of the native window, or a log line)

fn id_of(agent: &Agent) -> String {
    format!("agent-{:?}", agent.id).to_lowercase()
}

fn agent_of(id: &str) -> Option<&'static Agent> {
    AGENTS.iter().find(|agent| id_of(agent) == id)
}

fn tile_of(agent: &Agent) -> String {
    match agent.tile {
        Tile::Black => "#111111".into(),
        Tile::Solid(rgb) | Tile::Gradient(rgb, _) => format!("#{rgb:06x}"),
    }
}

fn icon_of(agent: &Agent) -> &'static str {
    if agent.id == AgentId::Claude { "claude" } else { "prompt" }
}

fn tiled(entry: Entry, agent: &Agent) -> Entry {
    entry.with("tile", tile_of(agent)).with("icon", icon_of(agent))
}

pub(super) fn lists(out: &mut Map<String, Value>, cx: &mut App) {
    let snapshot = super::cache(cx).and_then(|cache| cache.agents.as_ref());
    let (mut agents_list, mut skills_list) = (Vec::new(), Vec::new());
    if let Some(snapshot) = snapshot {
        let home = snapshot.home().to_path_buf();
        for row in snapshot.installed() {
            agents_list.push(agent_entry(row, &home));
            // Each profile's own login of the agent: a second place for the hooks, with its own button.
            for (at, login) in row.logins.iter().enumerate() {
                agents_list.push(login_entry(row.agent, at, login));
            }
        }
        let others = snapshot.others().count();
        if others > 0 {
            agents_list.push(
                Entry::new("others", "value", text(SETTINGS_AGENTS_OTHER))
                    .with("value", whole(others).to_string())
                    .into_value(),
            );
        }
        skills_list = skills(snapshot);
    }
    out.insert(GROUP_AGENTS.into(), Value::Array(agents_list));
    // Under the Command line tool button: where the command is, or whose it is.
    out.insert(
        GROUP_COMMAND_LINE.into(),
        json!([Entry::new("cli-status", "value", &super::super::system::cli_tool_line()).into_value()]),
    );
    out.insert(GROUP_SKILLS.into(), Value::Array(skills_list));
    out.insert(GROUP_ACCESS.into(), Value::Array(programs(cx)));
}

fn agent_entry(row: &crate::settings_window::native::agents_read::AgentRow, home: &Path) -> Value {
    let agent = row.agent;
    let path = row.spec.map(|spec| spec.display_path(home));
    let (hint, detail, value) = match row.hooks() {
        Some(State::Installed) => (SETTINGS_AGENTS_HINT_HOOKS, SETTINGS_AGENTS_INSTALLED_HOOKS_IN, "on"),
        Some(State::Partial) => {
            (SETTINGS_AGENTS_HINT_PARTIAL, SETTINGS_AGENTS_INSTALLED_HOOKS_INCOMPLETE_IN, "repair")
        }
        Some(State::NotInstalled) => {
            (SETTINGS_AGENTS_HINT_OUTPUT, SETTINGS_AGENTS_INSTALLED_STATUS_FROM_OUTPUT_HOOKS_GO, "off")
        }
        None => (SETTINGS_AGENTS_HINT_OUTPUT, SETTINGS_AGENTS_INSTALLED_STATUS_FROM_OUTPUT_NO_HOOK, "none"),
    };
    let more = match &path {
        Some(shown) => format!("{} {shown}", text(detail)),
        None => text(detail).to_string(),
    };
    // An agent without hooks has nothing to add, so it is a plain line.
    let kind = if row.spec.is_some() { "agent" } else { "value" };
    let mut entry = tiled(Entry::new(&id_of(agent), kind, agent.name), agent)
        .with("hint", text(hint))
        .with("info", more)
        .with("value", if value == "none" { "" } else { value });
    if let Some(Err(err)) = &row.state {
        entry = entry.with("note", err.as_str());
    } else if let Some(note) =
        row.spec.and_then(|spec| spec.note).filter(|_| row.hooks() != Some(State::NotInstalled))
    {
        entry = entry.with("note", note);
    }
    entry.into_value()
}

/// A profile's login of an agent: where its config is and the state of the hooks there, as the agent's own row has
/// them for the default location.
fn login_entry(
    agent: &Agent,
    at: usize,
    login: &crate::settings_window::native::agents_read::LoginRow,
) -> Value {
    let state = login.state.as_ref().ok().copied();
    let (badge, value) = match state {
        Some(State::Installed) => (text(SETTINGS_AGENTS_STATUS_HOOKS_ON), "on"),
        Some(State::Partial) => (text(SETTINGS_AGENTS_HOOKS_INCOMPLETE), "repair"),
        _ => (text(SETTINGS_AGENTS_NO_HOOKS), "off"),
    };
    let mut entry = tiled(
        Entry::new(&login_id(agent, at), "agent", &settings_agents_login_state(&login.profile, badge)),
        agent,
    )
    .with("hint", crate::settings_window::native::paths::with_tilde(&login.dir))
    .with("value", value);
    if let Err(err) = &login.state {
        entry = entry.with("note", err.as_str());
    }
    entry.into_value()
}

fn login_id(agent: &Agent, at: usize) -> String {
    format!("{}-login-{at}", id_of(agent))
}

fn skills(snapshot: &AgentsSnapshot) -> Vec<Value> {
    let mut list = Vec::new();
    for row in
        snapshot.skills.iter().filter(|row| row.found || !matches!(row.state, Ok(SkillState::NotInstalled)))
    {
        let agent = agents::agent(row.target.agent);
        let state = row.state.as_ref().ok().copied();
        let (hint, actions, primary): (String, Vec<(&str, &str)>, Option<usize>) = match state {
            Some(SkillState::NotInstalled) if row.covered.is_some() => (
                format!(
                    "{} {}",
                    text(SETTINGS_AGENTS_SKILL_COVERED),
                    row.covered.as_deref().unwrap_or_default()
                ),
                vec![],
                None,
            ),
            Some(SkillState::Current) => (
                text(SETTINGS_AGENTS_SKILL_HINT_CURRENT).into(),
                vec![("uninstall", text(SETTINGS_AGENTS_UNINSTALL))],
                None,
            ),
            Some(SkillState::Outdated) => (
                text(SETTINGS_AGENTS_SKILL_HINT_OUTDATED).into(),
                vec![
                    ("update", text(SETTINGS_AGENTS_UPDATE)),
                    ("uninstall", text(SETTINGS_AGENTS_UNINSTALL)),
                ],
                Some(0),
            ),
            Some(SkillState::Foreign) => (text(SETTINGS_AGENTS_SKILL_HINT_FOREIGN).into(), vec![], None),
            _ => (
                text(SETTINGS_AGENTS_SKILL_HINT_NONE).into(),
                vec![("install", text(SETTINGS_AGENTS_INSTALL))],
                Some(0),
            ),
        };
        let mut entry =
            tiled(Entry::new(&format!("skill-{}", row.target.word()), "skill", agent.name), agent)
                .with("hint", hint)
                .with(
                    "info",
                    format!(
                        "{} {}",
                        row.covered.as_deref().unwrap_or(&row.shown),
                        text(SETTINGS_AGENTS_SKILLS_DESCRIPTION)
                    ),
                )
                .with("actions", json!(actions.iter().map(|(_, words)| words).collect::<Vec<_>>()))
                .with("actionIds", json!(actions.iter().map(|(id, _)| id).collect::<Vec<_>>()));
        if let Some(primary) = primary {
            entry = entry.with("primary", primary);
        }
        if let Err(err) = &row.state {
            entry = entry.with("note", err.as_str());
        }
        list.push(entry.into_value());
    }
    list.push(
        Entry::new("mcp", "button", text(SETTINGS_AGENTS_COPY_COMMAND))
            .with("hint", mcp_add_command())
            .with("value", text(SETTINGS_AGENTS_COPY_COMMAND))
            .into_value(),
    );
    list
}

const CAPABILITIES: [Capability; 4] = [Capability::Read, Capability::Type, Capability::Run, Capability::Open];

fn capability_word(capability: Capability) -> &'static str {
    text(match capability {
        Capability::Read => SETTINGS_AGENTS_ACCESS_CAP_READ,
        Capability::Type => SETTINGS_AGENTS_ACCESS_CAP_TYPE,
        Capability::Run => SETTINGS_AGENTS_ACCESS_CAP_RUN,
        Capability::Open | Capability::Close => SETTINGS_AGENTS_ACCESS_CAP_OPEN,
    })
}

/// A capability's id on the wire: the name of its check, never its words.
fn capability_id(capability: Capability) -> &'static str {
    match capability {
        Capability::Read => "read",
        Capability::Type => "type",
        Capability::Run => "run",
        Capability::Open | Capability::Close => "open",
    }
}

fn how_long(how: How) -> &'static str {
    text(match how {
        How::Session => SETTINGS_AGENTS_ACCESS_SESSION,
        How::UntilQuit => SETTINGS_AGENTS_ACCESS_UNTIL_QUIT,
        How::Always => SETTINGS_AGENTS_ACCESS_ALWAYS,
    })
}

fn programs(cx: &mut App) -> Vec<Value> {
    agent_access::approved(cx)
        .iter()
        .enumerate()
        .map(|(ix, row): (usize, &Approved)| {
            let mut entry = Entry::new(&format!("program-{ix}"), "skill", &row.label)
                .with("hint", how_long(row.how))
                .with("tile", "#111111")
                .with("icon", "prompt")
                .with("actions", json!([text(SETTINGS_AGENTS_ACCESS_REVOKE)]))
                .with("actionIds", json!(["revoke"]))
                .with("checks", json!(CAPABILITIES.map(capability_word)))
                .with("checkIds", json!(CAPABILITIES.map(capability_id)))
                .with(
                    "checksOn",
                    json!(
                        CAPABILITIES
                            .iter()
                            .filter(|c| row.caps.contains(**c))
                            .map(|c| capability_id(*c))
                            .collect::<Vec<_>>()
                    ),
                );
            if !row.location.is_empty() {
                entry = entry.with("info", row.location.as_str());
            }
            entry.into_value()
        })
        .collect()
}

pub(super) fn load(cx: &mut App) {
    let profiles = app(cx).profiles.profiles().to_vec();
    cx.spawn(async move |cx| {
        let snapshot = cx
            .background_executor()
            .spawn(async move { AgentsSnapshot::load(&integrations::home(), &profiles) })
            .await;
        cx.update(|cx| {
            if cx.has_global::<crate::settings_window::native::Native>() {
                super::cache_mut(cx).agents = Some(snapshot);
                crate::settings_window::native::push(cx);
            }
        })
    })
    .detach();
}

pub(super) fn perform(key: &str, action: &str, cx: &mut App) -> bool {
    let mut parts = key.splitn(3, ':');
    let (Some(page), Some(group), Some(id)) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    let group = &key[..page.len() + 1 + group.len()];
    match group {
        GROUP_AGENTS => {
            hooks(id, action, cx);
            true
        }
        GROUP_SKILLS => {
            if id == "mcp" {
                crate::platform::clipboard::write(mcp_add_command(), cx);
            } else {
                skill_action(id, action, cx);
            }
            true
        }
        GROUP_ACCESS => {
            program(id, action, cx);
            true
        }
        _ => false,
    }
}

fn hooks(id: &str, action: &str, cx: &mut App) {
    // "agent-<id>" is the default location; "agent-<id>-login-<n>" the n-th profile's login folder.
    let (base, login) = match id.split_once("-login-") {
        Some((base, at)) => (base, at.parse::<usize>().ok()),
        None => (id, None),
    };
    let Some(agent) = agent_of(base) else { return };
    let Some(spec) = integrations::spec(agent.id) else { return };
    let remove = action == "remove_hooks";
    let result = match login {
        None if remove => spec.uninstall(&integrations::home()),
        None => spec.install(&integrations::home()),
        Some(at) => {
            let dir = super::cache(cx)
                .and_then(|cache| cache.agents.as_ref())
                .and_then(|snapshot| snapshot.installed().find(|row| row.agent.id == agent.id))
                .and_then(|row| row.logins.get(at))
                .map(|login| login.dir.clone());
            let Some(dir) = dir else { return };
            if remove { spec.uninstall_in(&dir) } else { spec.install_in(&dir) }
        }
    };
    if let Err(err) = result {
        tracing::warn!("{} hooks: {err}", agent.name);
    }
    load(cx);
}

fn skill_action(id: &str, action: &str, cx: &mut App) {
    let Some(target) = skill::TARGETS.iter().find(|t| format!("skill-{}", t.word()) == id) else { return };
    let home = integrations::home();
    let result = if action == "uninstall" {
        skill::uninstall(target, &home).map(drop)
    } else {
        skill::install(target, &home).map(drop)
    };
    if let Err(err) = result {
        tracing::warn!("{} skill: {err}", agents::agent(target.agent).name);
    }
    load(cx);
}

fn program(id: &str, action: &str, cx: &mut App) {
    let Some(row) = id
        .strip_prefix("program-")
        .and_then(|ix| ix.parse::<usize>().ok())
        .and_then(|ix| agent_access::approved(cx).get(ix).cloned())
    else {
        return;
    };
    if action == "revoke" {
        agent_access::revoke(row.which, cx);
        return;
    }
    // "<capability> on|off"
    let Some((word, on)) = action.rsplit_once(' ') else { return };
    if let Some(capability) = CAPABILITIES.into_iter().find(|c| capability_id(*c) == word) {
        agent_access::set_capability(row.which, capability, on == "on", cx);
    }
}
