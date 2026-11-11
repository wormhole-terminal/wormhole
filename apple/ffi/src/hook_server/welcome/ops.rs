//! The program's requests, answered. Each op asks for one thing and gets exactly that: the look changes only to
//! a theme, colour, ligature setting or font step the page offers; what reaches the person only by the switches
//! and sound Settings → Notifications has (macOS is asked for permission only from a press that turns a banner
//! on, or says Allow, and only while it has not been asked); hooks are written for the one agent named, if it
//! was found; the example band goes up in the window's sidebar and comes down.

use std::path::PathBuf;
use std::sync::Arc;

use wormhole_core::agents::AgentId;
use wormhole_core::agents::integrations::{self, State};
use wormhole_core::config::{Config, NotifySound, Settings};
use wormhole_core::import::{self, Env, Proposed, Source};
use wormhole_core::ipc::control::{
    AgentFact, ErrorCode, HooksFact, ImportDone, NotifyFacts, PermissionFact, Reply, WelcomeOp,
};
use wormhole_core::onboarding::ONBOARDED_KEY;
use wormhole_core::storage::store::Store;
use wormhole_core::theme::fonts::FontSlot;
use wormhole_core::theme::registry::UserThemes;
use wormhole_core::theme::tokens::Appearance;

use super::facts::{self, offered_primaries, offered_themes};
use super::{Core, HookEvent, ImportedTheme};

pub(super) fn answer(core: &Arc<Core>, tab: i64, op: WelcomeOp) -> Reply {
    match op {
        WelcomeOp::State => Reply::ok(state(core, tab)),
        WelcomeOp::SetLook { theme, primary, ligatures, font_step } => {
            set_look(core, tab, theme, primary, ligatures, font_step)
        }
        WelcomeOp::SetNotify { needs_you, agent_done, sound, allow } => {
            set_notify(core, needs_you, agent_done, sound, allow)
        }
        // This app's window does not report how much it shows (the state says so), so the program never sends it.
        WelcomeOp::SetShown { .. } => {
            Reply::error(ErrorCode::Unsupported, "this window can't change how much it shows from here")
        }
        WelcomeOp::InstallHooks { agent } => install(core, tab, agent),
        WelcomeOp::Demo { show } => demo(core, tab, show),
        WelcomeOp::Import { source } => import(core, tab, &source),
        WelcomeOp::Finish { skipped: _ } => finish(core, tab),
    }
}

fn state(core: &Core, tab: i64) -> wormhole_core::ipc::control::WelcomeState {
    let seat = core.seat();
    let settings = core.settings();
    let welcomes = core.welcomes();
    let Some(run) = welcomes.run(tab) else {
        tracing::error!(tab, "a welcome gate without a run");
        return facts::empty_state();
    };
    let notify = NotifyFacts {
        needs_you: settings.notify_kind_on(settings.notify_needs_you),
        agent_done: settings.notify_kind_on(settings.notify_agent_done),
        sound: settings.notify_sound,
        permission: welcomes.permission,
    };
    facts::state(&seat.profile, &settings, run, notify)
}

/// The shown profile's `config.json`, changed through core's settings (clamped and sanitised as every write is).
fn edit(core: &Core, change: impl FnOnce(&mut Settings)) {
    let seat = core.seat();
    Config::load_for(&seat.profile).update(change);
    core.settings_changed(seat.profile.id().as_str());
}

fn set_look(
    core: &Core,
    tab: i64,
    theme: Option<String>,
    primary: Option<String>,
    ligatures: Option<bool>,
    font_step: Option<i8>,
) -> Reply {
    let seat = core.seat();
    let settings = core.settings();
    let imported = core.welcomes().run(tab).and_then(|run| run.imported.clone());
    // A theme on offer is a built-in one or the one an import brought over, which is set by its file.
    let mut theme = theme;
    if let Some(name) = &theme {
        if let Some(imported) = imported.as_ref().filter(|imported| &imported.name == name) {
            theme = Some(imported.value.clone());
        } else if !offered_themes(&settings.theme).iter().any(|offered| &offered.name == name) {
            return Reply::error(ErrorCode::NotFound, format!("no theme {name:?} on offer"));
        }
    }
    if let Some(hex) = &primary
        && !hex.is_empty()
    {
        let base = facts::base_tokens(&seat.profile, &settings.theme);
        let offered = base.as_ref().map(|base| offered_primaries(base).1).unwrap_or_default();
        if !offered.iter().any(|(_, color)| &color.to_hex() == hex) {
            return Reply::error(ErrorCode::NotFound, format!("no primary colour {hex:?} on offer"));
        }
    }
    edit(core, |s| {
        if let Some(name) = theme {
            s.theme = name;
        }
        if let Some(hex) = primary {
            s.choose_primary(hex);
        }
        if let Some(on) = ligatures {
            s.font_ligatures = on;
        }
        if let Some(step) = font_step {
            s.step_slot_size(FontSlot::Terminal, f32::from(step));
        }
    });
    Reply::ok(serde_json::json!({}))
}

fn set_notify(
    core: &Core,
    needs_you: Option<bool>,
    agent_done: Option<bool>,
    sound: Option<NotifySound>,
    allow: bool,
) -> Reply {
    let before = core.settings();
    let (was_needs_you, was_agent_done) =
        (before.notify_kind_on(before.notify_needs_you), before.notify_kind_on(before.notify_agent_done));
    let turns_on =
        (needs_you == Some(true) && !was_needs_you) || (agent_done == Some(true) && !was_agent_done);
    edit(core, |s| {
        if let Some(on) = needs_you {
            s.change_notify_kind(|s| s.notify_needs_you = on);
        }
        if let Some(on) = agent_done {
            s.change_notify_kind(|s| s.notify_agent_done = on);
        }
        if let Some(sound) = sound {
            s.notify_sound = sound;
        }
    });
    if (turns_on || allow) && core.welcomes().permission == PermissionFact::NotAsked {
        core.ui.push(HookEvent::WelcomeAskPermission);
    }
    Reply::ok(serde_json::json!({}))
}

fn install(core: &Core, tab: i64, agent: AgentId) -> Reply {
    let found = core
        .welcomes()
        .run(tab)
        .and_then(|run| run.agents.as_ref())
        .is_some_and(|agents| agents.found.iter().any(|f| f.id == agent && f.hooks.installable()));
    let Some(spec) = integrations::spec(agent).filter(|_| found) else {
        return Reply::error(ErrorCode::NotFound, "that agent has no hooks to install here");
    };
    let home = integrations::home();
    let result = spec.install(&home);
    if let Err(err) = &result {
        tracing::warn!("{agent:?} hooks: {err}");
    }
    let hooks = match spec.state(&home) {
        Ok(State::Installed) => HooksFact::Installed,
        Ok(State::Partial) => HooksFact::Partial,
        Ok(State::NotInstalled) | Err(_) => HooksFact::Missing,
    };
    if let Some(run) = core.welcomes().run_mut(tab) {
        if let Some(found) = run
            .agents
            .as_mut()
            .and_then(|agents| agents.found.iter_mut().find(|f: &&mut AgentFact| f.id == agent))
        {
            found.hooks = hooks;
        }
        run.problem = result.as_ref().err().map(|err| format!("Couldn't write the hooks: {err}"));
    }
    match result {
        Ok(()) => Reply::ok(serde_json::json!({})),
        Err(err) => Reply::error(ErrorCode::Failed, err),
    }
}

fn demo(core: &Core, tab: i64, show: bool) -> Reply {
    if core.owner_of(tab).is_none() {
        return Reply::error(ErrorCode::NotFound, "no such terminal");
    }
    core.ui.push(HookEvent::WelcomeDemo { terminal: tab, show });
    Reply::ok(serde_json::json!({}))
}

/// The settings of one other terminal, which the page offered because the program found it: the app reads that
/// terminal's files itself and takes what Settings → Import ticks by default (never a shell command), in one
/// settings update, its palette saved as a theme in the profile.
fn import(core: &Core, tab: i64, source: &str) -> Reply {
    let Some(source) = Source::from_key(source) else {
        return Reply::error(ErrorCode::NotFound, "no such terminal to import from");
    };
    let (home, fonts) = {
        let welcomes = core.welcomes();
        let Some(run) = welcomes.run(tab) else {
            return Reply::error(ErrorCode::NotFound, "no such welcome");
        };
        (run.import_home.clone(), run.fonts.clone())
    };
    let env = match home {
        Some(home) => Some(Env::new(PathBuf::from(home))),
        None => Env::from_process(),
    };
    let Some(mut found) = env.and_then(|env| import::scan_one(&env, source)) else {
        return Reply::error(ErrorCode::NotFound, "that terminal's settings are not there any more");
    };
    found.proposal.resolve_fonts(|name| fonts.family(name.to_string()));
    let chosen: Vec<Proposed> = found.proposal.selected().cloned().collect();
    let seat = core.seat();
    let saved = match import::save_themes(&seat.profile.themes_dir(), &chosen) {
        Ok(saved) => saved,
        Err(why) => return Reply::error(ErrorCode::Failed, why),
    };
    edit(core, |s| import::apply(s, &found.proposal, &chosen, &saved));
    remember_imported_theme(core, tab);
    let left = found.proposal.values.len().saturating_sub(chosen.len()) + found.proposal.skipped.len();
    tracing::info!(tab, source = source.key(), brought = chosen.len(), left, "imported");
    Reply::ok(ImportDone { brought: chosen.len(), left })
}

/// The theme file an import just set (it is not one of the built-ins the list offers), kept on the run so the
/// look page lists it from now on, in use or not.
fn remember_imported_theme(core: &Core, tab: i64) {
    let seat = core.seat();
    let value = core.settings().theme.clone();
    let themes = UserThemes::read(&seat.profile.themes_dir());
    let Some(theme) = themes.by_path(&value) else { return };
    let light = themes.resolve(theme).is_ok_and(|tokens| tokens.appearance == Appearance::Light);
    let imported = ImportedTheme { name: theme.name.clone(), value, light };
    if let Some(run) = core.welcomes().run_mut(tab) {
        run.imported = Some(imported);
    }
}

/// The welcome is over: remembered when it was the first run, the example band goes and the animation, if
/// still playing, is cut short; the token opens nothing from now on.
fn finish(core: &Core, tab: i64) -> Reply {
    tracing::info!(tab, "the welcome finished");
    let first_run = {
        let mut welcomes = core.welcomes();
        let first_run = welcomes.run(tab).is_some_and(|run| run.first_run);
        welcomes.gates.finish(tab);
        first_run
    };
    if first_run {
        remember_first_run(core);
    }
    core.ui.push(HookEvent::WelcomeEnded { terminal: tab });
    Reply::ok(serde_json::json!({}))
}

/// The first run is over: not shown again.
pub(super) fn remember_first_run(core: &Core) {
    if let Err(why) = Store::open_at(core.root.base()).and_then(|store| store.set_meta(ONBOARDED_KEY, "1")) {
        tracing::warn!("remembering the first run: {why:#}");
    }
}
