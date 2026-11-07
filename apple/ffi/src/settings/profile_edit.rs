//! The Profile page's fields: what one profile holds besides its name in the list (colour, avatar, start
//! folder, the folders that open in it, environment, git identity, agent logins) and the one save that
//! writes them back through core's `ProfileList::edit`. The rules (a name's length, a path that is not
//! one, a variable that cannot be set) are core's `Profile::check`; this only turns the text fields
//! into a `Profile` the way the GPUI page does (`profile_form`), so a refused edit says core's sentence.

use std::collections::BTreeMap;

use wormhole_core::agents::AgentId;
use wormhole_core::config::profile::{AgentLogin, GitIdentity, Profile, ProfileList, first_letter};
use wormhole_core::storage::paths::{DataRoot, ProfileId};
use wormhole_core::view::sidebar::GroupColor;
use wormhole_text::keys::{
    settings_profile_env_not_a_pair, settings_profile_start_caption_many,
    settings_profile_start_caption_none, settings_profile_start_caption_one,
};

use super::error::SettingsError;
use crate::isolation;

/// The agents that have a login folder of their own, in the order the page lists them (the GPUI page's
/// `LOGIN_AGENTS`): the agent's own name and the placeholder that shows what a folder looks like.
const LOGIN_AGENTS: [(AgentId, &str, &str, &str); 2] = [
    (AgentId::Claude, "claude", "Claude", "~/.claude-work"),
    (AgentId::Codex, "codex", "Codex", "~/.codex-work"),
];

/// One agent's login folder field.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SettingsLoginField {
    /// The agent's id (`claude`), what a save names it by.
    pub agent: String,
    pub name: String,
    pub placeholder: String,
    /// Empty: the agent keeps the login it has.
    pub dir: String,
}

/// Everything the Profile page edits, as text.
#[derive(Debug, Clone, PartialEq, uniffi::Record)]
pub struct SettingsProfileForm {
    pub id: String,
    pub name: String,
    /// One letter; empty is the name's first.
    pub avatar: String,
    /// A `GroupColor` key (`blue`).
    pub color: String,
    /// `~/…` or absolute; empty is the home folder.
    pub start_dir: String,
    pub folders: Vec<String>,
    /// `NAME=value`, one per line.
    pub env: String,
    pub git_name: String,
    pub git_email: String,
    pub logins: Vec<SettingsLoginField>,
    /// "Folders under ~/work open in Home too." under the start folder's label, in the catalogs' words.
    pub start_caption: String,
}

/// `NAME=value` lines into the environment; blank lines and `#` comments are skipped.
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

fn start_caption(profile: &Profile) -> String {
    let folders = profile.folders.iter().map(String::as_str).collect::<Vec<_>>();
    match folders.as_slice() {
        [] => settings_profile_start_caption_none(&profile.name),
        [one] => settings_profile_start_caption_one(one, &profile.name),
        [head @ .., last] => settings_profile_start_caption_many(&head.join(", "), last, &profile.name),
    }
}

fn form_of(profile: &Profile) -> SettingsProfileForm {
    SettingsProfileForm {
        start_caption: start_caption(profile),
        id: profile.id.to_string(),
        name: profile.name.clone(),
        avatar: profile.avatar.to_string(),
        color: profile.color.key().to_string(),
        start_dir: profile.start_dir.clone(),
        folders: profile.folders.clone(),
        env: profile.env.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("\n"),
        git_name: profile.git.name.clone(),
        git_email: profile.git.email.clone(),
        logins: LOGIN_AGENTS
            .iter()
            .map(|(agent, key, name, placeholder)| SettingsLoginField {
                agent: (*key).to_string(),
                name: (*name).to_string(),
                placeholder: (*placeholder).to_string(),
                dir: profile
                    .agent_logins
                    .iter()
                    .find(|login| login.agent == *agent)
                    .map(|login| login.dir.clone())
                    .unwrap_or_default(),
            })
            .collect(),
    }
}

pub(crate) fn form_in(root: &DataRoot, id: Option<&str>) -> Result<SettingsProfileForm, SettingsError> {
    let (list, _) = ProfileList::load_or_recover(root);
    let profile = match id {
        Some(text) => {
            let id = ProfileId::parse(text).map_err(|e| SettingsError::Profile { reason: e.to_string() })?;
            list.get(&id).ok_or_else(|| SettingsError::Profile { reason: format!("no profile {id}") })?
        }
        None => list.active(),
    };
    Ok(form_of(profile))
}

/// Writes `form` into its profile; `Ok(false)` when it is what the profile already is.
pub(crate) fn save_in(root: &DataRoot, form: &SettingsProfileForm) -> Result<bool, SettingsError> {
    let refused = |reason: String| SettingsError::Profile { reason };
    let id = ProfileId::parse(&form.id).map_err(|e| refused(e.to_string()))?;
    let env = parse_env(&form.env).map_err(refused)?;
    let mut list = ProfileList::load(root).map_err(|e| refused(e.to_string()))?;
    let before = list.get(&id).cloned().ok_or_else(|| refused(format!("no profile {id}")))?;
    list.edit(&id, |profile| {
        profile.name = form.name.trim().to_string();
        profile.avatar = form
            .avatar
            .trim()
            .chars()
            .next()
            .map(|c| c.to_ascii_uppercase())
            .unwrap_or_else(|| first_letter(&form.name));
        if let Some(color) = GroupColor::from_key(&form.color) {
            profile.color = color;
        }
        profile.start_dir = form.start_dir.trim().to_string();
        profile.folders = form.folders.iter().map(|f| f.trim().to_string()).collect();
        profile.git = GitIdentity { name: form.git_name.trim().into(), email: form.git_email.trim().into() };
        profile.agent_logins = form
            .logins
            .iter()
            .filter(|login| !login.dir.trim().is_empty())
            .filter_map(|login| {
                let agent = LOGIN_AGENTS.iter().find(|(_, key, ..)| *key == login.agent)?.0;
                Some(AgentLogin { agent, dir: login.dir.trim().to_string() })
            })
            .collect();
        profile.env = env;
    })
    .map_err(|e| refused(e.to_string()))?;
    if list.get(&id) == Some(&before) {
        return Ok(false);
    }
    list.save(root).map_err(|e| refused(e.to_string()))?;
    Ok(true)
}

/// The fields of a profile (the one Wormhole is in for `None`).
#[uniffi::export]
pub fn settings_profile_form(id: Option<String>) -> Result<SettingsProfileForm, SettingsError> {
    isolation::require()?;
    form_in(&DataRoot::current(), id.as_deref())
}

/// Saves the fields of a profile. Core refuses what a profile cannot hold, with a sentence for the page
/// to show under its title, and the profile stays as it was. `true`: something changed.
#[uniffi::export]
pub fn settings_profile_save(form: SettingsProfileForm) -> Result<bool, SettingsError> {
    isolation::require()?;
    save_in(&DataRoot::current(), &form)
}

#[cfg(test)]
mod tests {
    use wormhole_core::storage::paths::DataRoot;

    use super::{form_in, save_in};

    #[test]
    fn the_fields_round_trip_and_core_refuses_what_a_profile_cannot_hold() {
        let dir = tempfile::tempdir().expect("temp data dir");
        let root = DataRoot::new(dir.path());
        let mut form = form_in(&root, None).expect("default");
        assert_eq!((form.name.as_str(), form.color.as_str(), form.logins.len()), ("Default", "blue", 2));

        form.name = "Home".into();
        form.avatar = "h".into();
        form.color = "green".into();
        form.start_dir = "~/work".into();
        form.folders = vec!["~/clients".into()];
        form.env = "# note\nFOO = bar\n".into();
        form.git_name = "Ada".into();
        form.logins[0].dir = "~/.claude-work".into();
        assert!(save_in(&root, &form).expect("saved"));
        assert!(!save_in(&root, &form).expect("same again"), "an unchanged form writes nothing");

        let back = form_in(&root, None).expect("read back");
        assert_eq!((back.name.as_str(), back.avatar.as_str(), back.color.as_str()), ("Home", "H", "green"));
        assert_eq!((back.start_dir.as_str(), back.env.as_str()), ("~/work", "FOO=bar"));
        assert_eq!(
            (back.folders.clone(), back.logins[0].dir.as_str()),
            (vec!["~/clients".to_string()], "~/.claude-work")
        );

        form.env = "no pair here".into();
        assert!(save_in(&root, &form).expect_err("refused").to_string().contains("no pair here"));
        form.env = String::new();
        form.start_dir = "relative/path".into();
        assert!(save_in(&root, &form).is_err(), "core's own rule: a start folder is a full path");
        assert_eq!(form_in(&root, None).expect("kept").name, "Home");
    }
}
