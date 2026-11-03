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

use super::Entry;
use crate::app_state::{AppState, app, app_mut};
use crate::settings_window::registry::row_label;
use crate::settings_window::sections::{LOGIN_AGENTS, parse_env};

/// One edit of the profile, chosen by the field that was left.
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
    if let Some(profile) = active(cx) {
        let env: Vec<String> = profile.env.iter().map(|(name, value)| format!("{name}={value}")).collect();
        let login = |agent| {
            profile.agent_logins.iter().find(|l| l.agent == agent).map(|l| l.dir.clone()).unwrap_or_default()
        };
        fields.push(field("name", row_label("profile-name"), &profile.name).into_value());
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
    out.insert(GROUP_FIELDS.into(), Value::Array(fields));
    out.insert(GROUP_DELETE.into(), Value::Array(delete));
}

pub(super) fn perform(key: &str, action: &str, cx: &mut App) -> bool {
    let mut parts = key.splitn(3, ':');
    let (Some("Profile"), Some(group), Some(id)) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
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
    let Some(value) = action.strip_prefix('=') else { return true };
    let value = value.trim().to_string();
    let change: Result<Change, String> = match id {
        "name" if value.is_empty() => Err("a profile needs a name".into()),
        "name" => Ok(Box::new(move |p| {
            p.name = value.clone();
        })),
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
        Ok(change) => {
            let state = app_mut(cx);
            let result =
                state.profiles.edit(&profile_id, change).and_then(|()| state.profiles.save(&state.root));
            match result {
                // The chip, the avatar and the odd window's edge read the profile.
                Ok(()) => cx.refresh_windows(),
                Err(err) => tracing::warn!("saving the profile: {err}"),
            }
        }
        Err(why) => tracing::warn!("profile: {why}"),
    }
    true
}
