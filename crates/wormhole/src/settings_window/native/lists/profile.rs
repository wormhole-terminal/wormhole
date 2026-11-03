//! Profile: the name, where terminals start, the environment, the git identity, the agent logins and deleting
//! the profile Wormhole is in. They live in `profiles.json`, written the way the GPUI page writes them
//! (`profiles.edit` then `save`), when the person is done with a field.

use std::collections::BTreeMap;

use gpui_kit::App;
use serde_json::{Map, Value};
use wormhole_core::config::profile::{AgentLogin, GitIdentity, Profile};
#[allow(
    clippy::wildcard_imports,
    reason = "the keys are generated, one constant per message: naming each would list them twice"
)]
use wormhole_text::keys::*;
use wormhole_text::{text, whole};

use wormhole_core::agents::AgentId;

use super::Entry;
use crate::app_state::{AppState, app, app_mut};
use crate::settings::SettingsStore;
use crate::settings_window::native::rows::row_label;
use wormhole_core::view::sidebar::GroupColor;

/// One edit of the profile, chosen by the field that was left.
/// The agents that have a login folder of their own, in the order the page lists them.
const LOGIN_AGENTS: [(AgentId, &str, &str); 2] = [
    (AgentId::Claude, "Claude", "~/.claude-work"), // text: name (the agent's own name)
    (AgentId::Codex, "Codex", "~/.codex-work"),    // text: name (the agent's own name)
];

/// `NAME=value` lines as an environment. Blank lines and `#` comments are skipped.
fn parse_env(text: &str) -> Result<BTreeMap<String, String>, String> {
    let mut env = BTreeMap::new();
    for line in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
        let Some((name, value)) = line.split_once('=') else {
            return Err(settings_profile_env_not_a_pair(line));
        };
        env.insert(name.trim().to_string(), value.trim().to_string());
    }
    Ok(env)
}

type Change = Box<dyn FnOnce(&mut Profile)>;

const GROUP_FIELDS: &str = "Profile:0";
const GROUP_DELETE: &str = "Profile:1";

fn active(cx: &App) -> Option<Profile> {
    app(cx).profiles.get(app(cx).active_profile()).cloned()
}

/// The profile's name and its avatar's letter, for the sidebar row.
pub(super) fn identity(cx: &App) -> Value {
    let (name, letter) = match active(cx) {
        Some(profile) => (profile.name.clone(), profile.avatar.to_string()),
        None => (String::new(), String::new()),
    };
    serde_json::json!([name, letter])
}

fn field(id: &str, label: &str, value: &str) -> Entry {
    Entry::new(id, "field", label).with("value", value)
}

pub(super) fn lists(out: &mut Map<String, Value>, cx: &App) {
    let (mut fields, mut delete) = (Vec::new(), Vec::new());
    // The profiles file could not be read: Wormhole runs on Default until it is put right.
    if let Some(error) = &app(cx).profiles_error {
        fields.push(
            Entry::new("unreadable", "value", &settings_window_profiles_unreadable(error)).into_value(),
        );
    }
    if let Some(profile) = active(cx) {
        let env: Vec<String> = profile.env.iter().map(|(name, value)| format!("{name}={value}")).collect();
        let login = |agent| {
            profile.agent_logins.iter().find(|l| l.agent == agent).map(|l| l.dir.clone()).unwrap_or_default()
        };
        fields.push(field("name", row_label("profile-name"), &profile.name).into_value());
        // The avatar's letter and the profile's colour, beside the name they belong to.
        fields.push(
            field("avatar", &super::super::words::word("Avatar letter"), &profile.avatar.to_string()) // text: a native word, translated by its id
                .into_value(),
        );
        let tokens = &cx.global::<SettingsStore>().tokens;
        let dots: Vec<Value> = GroupColor::ALL
            .iter()
            .map(|color| serde_json::json!([color.key(), color.label(), color.color(tokens).to_hex()]))
            .collect();
        fields.push(
            Entry::new("color", "dots", &super::super::words::word("Colour")) // text: a native word, translated by its id
                .with("dots", dots)
                .with("value", profile.color.key())
                .into_value(),
        );
        fields.push(
            field("start", row_label("profile-start"), &profile.start_dir)
                .with("hint", text(SETTINGS_PROFILE_THE_HOME_FOLDER))
                .into_value(),
        );
        fields.push(
            field("env", row_label("profile-env"), &env.join("\n"))
                .with("hint", text(SETTINGS_PROFILE_NAME_VALUE_ONE_PER_LINE))
                .with("multiline", true)
                .into_value(),
        );
        fields.push(
            field("git-name", text(SETTINGS_PROFILE_NAME_FIELD), &profile.git.name)
                .with("hint", row_label("profile-git"))
                .into_value(),
        );
        fields.push(field("git-email", text(SETTINGS_PROFILE_EMAIL), &profile.git.email).into_value());
        for (agent, name, example) in LOGIN_AGENTS {
            fields.push(
                field(&format!("login-{agent:?}").to_lowercase(), name, &login(agent))
                    .with("hint", format!("{} · {example}", row_label("profile-logins")))
                    .into_value(),
            );
        }
        // The folders that open in this profile: each can be taken out, and one added.
        for (at, folder) in profile.folders.iter().enumerate() {
            fields.push(
                Entry::new(&format!("folder-{at}"), "folder", folder)
                    .with("action", text(SETTINGS_PROFILE_STOP_OPENING_FOLDER))
                    .with("actionId", "remove")
                    .into_value(),
            );
        }
        fields.push(
            // The button says it; a label beside it would say it twice.
            Entry::new("add-folder", "button", "")
                .with("value", text(SETTINGS_PROFILE_ADD_A_FOLDER))
                .with("actionId", "add_folder")
                .into_value(),
        );
        let logins = profile.agent_logins.len();
        fields.push(
            Entry::new("logins-count", "value", row_label("profile-logins"))
                .with(
                    "value",
                    if logins == 0 {
                        text(SETTINGS_PROFILE_VALUE_SHARED).to_string()
                    } else {
                        whole(logins).to_string()
                    },
                )
                .into_value(),
        );
        if !profile.id.is_default() {
            delete.push(
                Entry::new("delete", "danger", row_label("profile-delete"))
                    .with("value", text(SETTINGS_PROFILE_DELETE_PROFILE))
                    .with("confirm", settings_profile_delete_question(&profile.name))
                    .with("action", text(SETTINGS_PROFILE_DELETE_BUTTON))
                    .with("actionId", "delete")
                    .into_value(),
            );
        } else {
            delete.push(
                Entry::new("delete", "value", row_label("profile-delete"))
                    .with("hint", text(SETTINGS_PROFILE_THE_DEFAULT_PROFILE_STAYS_IT_HOLDS))
                    .into_value(),
            );
        }
    }
    // A new profile is made from the page of any, so the button is there even while the active one is unreadable.
    delete.insert(
        0,
        Entry::new("new", "button", "")
            .with("value", super::super::words::word("New profile…")) // text: a native word, translated by its id
            .with("actionId", super::new_profile::OPEN)
            .into_value(),
    );
    out.insert(GROUP_FIELDS.into(), Value::Array(fields));
    out.insert(GROUP_DELETE.into(), Value::Array(delete));
}

pub(super) fn perform(key: &str, action: &str, cx: &mut App) -> bool {
    let mut parts = key.splitn(3, ':');
    let (Some("Profile"), Some(group), Some(id)) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    if id == "new" {
        return true;
    }
    let Some(profile_id) = active(cx).map(|p| p.id) else { return true };
    if group == "1" {
        if id == "delete" && action == "delete" && !profile_id.is_default() {
            // After this event: the profile's windows close, this one with them.
            cx.defer(move |cx| {
                if let Err(message) = AppState::delete_profile(&profile_id, cx) {
                    tracing::warn!("deleting a profile: {message}");
                }
            });
        }
        return true;
    }
    if id == "add-folder" {
        add_folder(profile_id, cx);
        return true;
    }
    if let Some(at) = id.strip_prefix("folder-").and_then(|at| at.parse::<usize>().ok()) {
        if action == "remove" {
            save(
                &profile_id,
                Box::new(move |p| {
                    if at < p.folders.len() {
                        drop(p.folders.remove(at))
                    }
                }),
                cx,
            );
        }
        return true;
    }
    let Some(value) = action.strip_prefix('=') else { return true };
    let value = value.trim().to_string();
    let change: Result<Change, String> = match id {
        "name" if value.is_empty() => Err("a profile needs a name".into()),
        "name" => Ok(Box::new(move |p| {
            p.name = value.clone();
        })),
        "avatar" => Ok(Box::new(move |p| {
            p.avatar = value
                .chars()
                .next()
                .map(|c| c.to_ascii_uppercase())
                .unwrap_or_else(|| wormhole_core::config::profile::first_letter(&p.name));
        })),
        "color" => match GroupColor::ALL.into_iter().find(|c| c.key() == value) {
            Some(color) => Ok(Box::new(move |p| p.color = color)),
            None => Err(format!("no profile colour {value}")),
        },
        "start" => Ok(Box::new(move |p| p.start_dir = value.clone())),
        "env" => parse_env(&value).map(|env: BTreeMap<String, String>| -> Box<dyn FnOnce(&mut Profile)> {
            Box::new(move |p| p.env = env)
        }),
        "git-name" => {
            Ok(Box::new(move |p| p.git = GitIdentity { name: value.clone(), email: p.git.email.clone() }))
        }
        "git-email" => {
            Ok(Box::new(move |p| p.git = GitIdentity { name: p.git.name.clone(), email: value.clone() }))
        }
        login => {
            match LOGIN_AGENTS.iter().find(|(agent, ..)| format!("login-{agent:?}").to_lowercase() == login) {
                Some((agent, ..)) => {
                    let agent = *agent;
                    Ok(Box::new(move |p| {
                        p.agent_logins.retain(|l| l.agent != agent);
                        if !value.is_empty() {
                            p.agent_logins.push(AgentLogin { agent, dir: value.clone() });
                        }
                    }))
                }
                None => return true,
            }
        }
    };
    match change {
        Ok(change) => save(&profile_id, change, cx),
        Err(why) => tracing::warn!("profile: {why}"),
    }
    true
}

/// Apply one edit to the profile and write `profiles.json`, as the GPUI page does.
fn save(profile_id: &wormhole_core::storage::paths::ProfileId, change: Change, cx: &mut App) {
    let state = app_mut(cx);
    let result = state.profiles.edit(profile_id, change).and_then(|()| state.profiles.save(&state.root));
    match result {
        // The chip, the avatar and the odd window's edge read the profile.
        Ok(()) => cx.refresh_windows(),
        Err(err) => tracing::warn!("saving the profile: {err}"),
    }
}

/// Ask for a folder and add it to the folders that open in the profile (`~/…` under the home folder).
fn add_folder(profile_id: wormhole_core::storage::paths::ProfileId, cx: &mut App) {
    let paths = cx.prompt_for_paths(gpui_kit::PathPromptOptions {
        files: false,
        directories: true,
        multiple: false,
        prompt: Some(text(SETTINGS_PROFILE_CHOOSE).into()),
    });
    cx.spawn(async move |cx| {
        let Ok(Ok(Some(paths))) = paths.await else { return };
        let Some(path) = paths.into_iter().next() else { return };
        let shown = match crate::settings_window::native::paths::home() {
            Some(home) => wormhole_core::onboarding::tilde(&path, &home),
            None => path.display().to_string(),
        };
        cx.update(|cx| {
            save(
                &profile_id,
                Box::new(move |p| {
                    if !p.folders.contains(&shown) {
                        p.folders.push(shown);
                    }
                }),
                cx,
            );
            crate::settings_window::native::push(cx);
        })
    })
    .detach();
}
