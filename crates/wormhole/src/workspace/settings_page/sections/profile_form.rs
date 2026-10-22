//! Settings → Profile: the fields of the shown profile and how an edit reaches `profiles.json`.
//! Text fields apply when they lose focus or take Return, swatches and the folder list at once;
//! what a field holds that the profile refuses (a variable that can't be set, a path that is
//! not one) is said under the page's title and the profile stays as it was.

use std::collections::BTreeMap;
use std::path::Path;

use gpui_kit::component::input::{InputEvent, InputState, TextareaState};
use gpui_kit::*;
use wormhole_core::agents::AgentId;
use wormhole_core::paths::ProfileId;
use wormhole_core::profile::{AgentLogin, GitIdentity, Profile, first_letter};
use wormhole_core::sidebar::GroupColor;

use crate::workspace::app::{app, app_mut};
use crate::workspace::settings_page::SettingsWindow;

/// The agents that have a login folder of their own, in the order the page lists them.
pub(super) const LOGIN_AGENTS: [(AgentId, &str, &str); 2] =
    [(AgentId::Claude, "Claude", "~/.claude-work"), (AgentId::Codex, "Codex", "~/.codex-work")];

/// The fields of the page, made for one profile and made again when Wormhole is in another.
pub(in crate::workspace::settings_page) struct ProfileForm {
    pub id: ProfileId,
    pub name: Entity<InputState>,
    pub avatar: Entity<InputState>,
    pub start: Entity<InputState>,
    pub git_name: Entity<InputState>,
    pub git_email: Entity<InputState>,
    /// One per entry of [`LOGIN_AGENTS`].
    pub logins: Vec<Entity<InputState>>,
    /// `NAME=value`, one per line.
    pub env: Entity<TextareaState>,
    /// Why the last edit was refused.
    pub error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

/// What the fields hold, read at once.
struct Typed {
    name: String,
    avatar: String,
    start: String,
    git: GitIdentity,
    logins: Vec<(AgentId, String)>,
    env: String,
}

impl Typed {
    /// `NAME=value` lines into the environment. Blank lines and `#` comments are skipped.
    fn parse_env(text: &str) -> Result<BTreeMap<String, String>, String> {
        let mut env = BTreeMap::new();
        for line in text.lines().map(str::trim).filter(|l| !l.is_empty() && !l.starts_with('#')) {
            let Some((name, value)) = line.split_once('=') else {
                return Err(format!("“{line}” is not NAME=value"));
            };
            env.insert(name.trim().to_string(), value.trim().to_string());
        }
        Ok(env)
    }

    fn apply(&self, profile: &mut Profile, env: BTreeMap<String, String>) {
        profile.name = self.name.trim().to_string();
        profile.avatar = self
            .avatar
            .trim()
            .chars()
            .next()
            .map(|c| c.to_ascii_uppercase())
            .unwrap_or_else(|| first_letter(&self.name));
        profile.start_dir = self.start.trim().to_string();
        profile.git = GitIdentity { name: self.git.name.trim().into(), email: self.git.email.trim().into() };
        profile.agent_logins = self
            .logins
            .iter()
            .filter(|(_, dir)| !dir.trim().is_empty())
            .map(|(agent, dir)| AgentLogin { agent: *agent, dir: dir.trim().to_string() })
            .collect();
        profile.env = env;
    }
}

fn input(
    value: &str,
    placeholder: &'static str,
    window: &mut Window,
    cx: &mut Context<SettingsWindow>,
) -> Entity<InputState> {
    let value = value.to_string();
    cx.new(|cx| InputState::new(window, cx).placeholder(placeholder).default_value(value))
}

fn on_input(
    this: &mut SettingsWindow,
    _: &Entity<InputState>,
    event: &InputEvent,
    _: &mut Window,
    cx: &mut Context<SettingsWindow>,
) {
    if matches!(event, InputEvent::Blur | InputEvent::PressEnter { .. }) {
        this.commit_profile(cx);
    }
}

fn on_env(
    this: &mut SettingsWindow,
    _: &Entity<TextareaState>,
    event: &InputEvent,
    _: &mut Window,
    cx: &mut Context<SettingsWindow>,
) {
    if matches!(event, InputEvent::Blur) {
        this.commit_profile(cx);
    }
}

impl ProfileForm {
    pub(in crate::workspace::settings_page) fn new(
        profile: &Profile,
        window: &mut Window,
        cx: &mut Context<SettingsWindow>,
    ) -> Self {
        let login = |agent: AgentId| {
            profile.agent_logins.iter().find(|l| l.agent == agent).map(|l| l.dir.clone()).unwrap_or_default()
        };
        let name = input(&profile.name, "Name", window, cx);
        let avatar = input(&profile.avatar.to_string(), "W", window, cx);
        let start = input(&profile.start_dir, "~ (the home folder)", window, cx);
        let git_name = input(&profile.git.name, "Name", window, cx);
        let git_email = input(&profile.git.email, "Email", window, cx);
        let logins: Vec<_> = LOGIN_AGENTS
            .iter()
            .map(|(agent, _, placeholder)| input(&login(*agent), placeholder, window, cx))
            .collect();
        let env_text = profile.env.iter().map(|(k, v)| format!("{k}={v}")).collect::<Vec<_>>().join("\n");
        let env = cx.new(|cx| {
            TextareaState::new(window, cx)
                .auto_grow(ENV_ROWS.0, ENV_ROWS.1)
                .placeholder("NAME=value, one per line")
                .default_value(env_text)
        });
        let mut subscriptions: Vec<Subscription> = [&name, &avatar, &start, &git_name, &git_email]
            .into_iter()
            .chain(&logins)
            .map(|field| cx.subscribe_in(field, window, on_input))
            .collect();
        subscriptions.push(cx.subscribe_in(&env, window, on_env));
        Self {
            id: profile.id.clone(),
            name,
            avatar,
            start,
            git_name,
            git_email,
            logins,
            env,
            error: None,
            _subscriptions: subscriptions,
        }
    }

    fn typed(&self, cx: &App) -> Typed {
        let text = |field: &Entity<InputState>| field.read(cx).value().to_string();
        Typed {
            name: text(&self.name),
            avatar: text(&self.avatar),
            start: text(&self.start),
            git: GitIdentity { name: text(&self.git_name), email: text(&self.git_email) },
            logins: LOGIN_AGENTS.iter().zip(&self.logins).map(|((agent, ..), f)| (*agent, text(f))).collect(),
            env: self.env.read(cx).value().to_string(),
        }
    }
}

/// How many lines the environment field shows: from two up to six, then it scrolls.
const ENV_ROWS: (usize, usize) = (2, 6);

impl SettingsWindow {
    /// Make the fields for the profile Wormhole is in, when they are for another.
    pub(in crate::workspace::settings_page) fn sync_profile_form(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let shown = app(cx).active_profile().clone();
        if self.profile_form.as_ref().is_some_and(|form| form.id == shown) {
            return;
        }
        let Some(profile) = app(cx).profiles.get(&shown).cloned() else { return };
        self.profile_form = Some(ProfileForm::new(&profile, window, cx));
        self.confirm_delete_profile = false;
    }

    /// Write what the fields hold into the profile, when it differs from what it has.
    pub(in crate::workspace::settings_page) fn commit_profile(&mut self, cx: &mut Context<Self>) {
        let Some(form) = self.profile_form.as_ref() else { return };
        let (id, typed) = (form.id.clone(), form.typed(cx));
        let outcome = Typed::parse_env(&typed.env).and_then(|env| {
            let state = app_mut(cx);
            let Some(before) = state.profiles.get(&id).cloned() else { return Ok(false) };
            let mut after = before.clone();
            typed.apply(&mut after, env);
            if after == before {
                return Ok(false);
            }
            state.profiles.edit(&id, |p| *p = after).map_err(|err| err.to_string())?;
            state.profiles.save(&state.root).map_err(|err| err.to_string())?;
            Ok(true)
        });
        self.finish_profile_edit(outcome, cx);
    }

    /// An edit that doesn't come from a text field: a swatch, the folder list.
    pub(in crate::workspace::settings_page) fn edit_profile(
        &mut self,
        change: impl FnOnce(&mut Profile),
        cx: &mut Context<Self>,
    ) {
        let Some(id) = self.profile_form.as_ref().map(|form| form.id.clone()) else { return };
        let state = app_mut(cx);
        let outcome = state
            .profiles
            .edit(&id, change)
            .and_then(|()| state.profiles.save(&state.root))
            .map(|()| true)
            .map_err(|err| err.to_string());
        self.finish_profile_edit(outcome, cx);
    }

    fn finish_profile_edit(&mut self, outcome: Result<bool, String>, cx: &mut Context<Self>) {
        if let Some(form) = self.profile_form.as_mut() {
            form.error = outcome.as_ref().err().map(|message| message.clone().into());
        }
        if outcome == Ok(true) {
            // The chip, the avatar and the odd window's edge read the profile.
            cx.refresh_windows();
        }
        cx.notify();
    }

    pub(in crate::workspace::settings_page) fn set_profile_color(
        &mut self,
        color: GroupColor,
        cx: &mut Context<Self>,
    ) {
        self.edit_profile(|p| p.color = color, cx);
    }

    /// A folder picked for a field (`Pick::Start`, `Pick::Login`) or for the list that opens in
    /// the profile (`Pick::Folder`), as `~/…` when it is under the home folder.
    pub(in crate::workspace::settings_page) fn pick_profile_folder(
        &mut self,
        pick: Pick,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let paths = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Choose".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = paths.await else { return };
            let Some(path) = paths.into_iter().next() else { return };
            let text = tilde(&path);
            this.update_in(cx, |this, window, cx| this.picked_profile_folder(pick, text, window, cx)).ok();
        })
        .detach();
    }

    fn picked_profile_folder(
        &mut self,
        pick: Pick,
        text: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let field = match (pick, self.profile_form.as_ref()) {
            (Pick::Folder, _) => {
                self.edit_profile(
                    |p| {
                        if !p.folders.contains(&text) {
                            p.folders.push(text);
                        }
                    },
                    cx,
                );
                return;
            }
            (Pick::Start, Some(form)) => form.start.clone(),
            (Pick::Login(ix), Some(form)) => match form.logins.get(ix) {
                Some(field) => field.clone(),
                None => return,
            },
            (_, None) => return,
        };
        field.update(cx, |field, cx| field.set_value(text, window, cx));
        self.commit_profile(cx);
    }
}

/// What a folder picked in the page is for.
#[derive(Clone, Copy)]
pub(in crate::workspace::settings_page) enum Pick {
    Start,
    Folder,
    /// The login folder of this entry of [`LOGIN_AGENTS`].
    Login(usize),
}

/// `path` as `~/…` under the home folder, else in full.
fn tilde(path: &Path) -> String {
    match crate::workspace::session::home_dir() {
        Some(home) => wormhole_core::onboarding::tilde(path, home),
        None => path.display().to_string(),
    }
}
