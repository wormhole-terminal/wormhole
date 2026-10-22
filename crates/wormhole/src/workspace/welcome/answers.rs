//! The welcome program's requests, answered (`Action::Welcome`). The door is core's `Registry`: a
//! request passes only from its welcome terminal with that terminal's token while the welcome runs, and
//! within the caps. What passes is done here, on the UI thread, outside any window's update, so the
//! window a terminal is in can be updated through `AppState::with_workspace`.
//!
//! Each op asks for one thing and gets exactly that: the look changes only to a theme, colour,
//! ligature setting or font step the page offers; what reaches the person changes only by the switches
//! and sound Settings → Notifications has (macOS is asked for permission only when a press turns a
//! banner on or says Allow, and only if it has not been asked); hooks are written for the one agent
//! named, if it was found on PATH; the example band goes up in the window's sidebar and comes down.

use std::time::Duration;

use wormhole_core::agents::AgentId;
use wormhole_core::agents::integrations::{self, State};
use wormhole_core::config::NotifySound;
use wormhole_core::ipc::control::{
    AgentFact, ErrorCode, HooksFact, Limited, LookFacts, NotifyFacts, PermissionFact, PrimaryFact, Reply,
    Target, ThemeFact, WelcomeOp, WelcomeState,
};
use wormhole_core::onboarding::intro::Handover;
use wormhole_core::theme::fonts::FontSlot;

use super::{Play, Run, remember_first_run};
use crate::platform::notifications::Permission;
use crate::settings::settings;
use crate::settings::update_settings;
use crate::workspace::app::{AppState, app, app_mut};
use crate::workspace::control::{budget, find};
use crate::workspace::notifications;
use crate::workspace::onboarding::{offered_primaries, offered_themes};
use gpui_kit::App;

/// Answer a welcome request. `from` is the tab it says it comes from.
pub(crate) fn handle(token: String, op: WelcomeOp, from: Option<i64>, cx: &mut App) -> Reply {
    if let Err(error) = app(cx).welcomes.gates.authorize(from, &token, &op) {
        return Reply::error(error.code, error.message);
    }
    let Some(tab) = from else { return Reply::error(ErrorCode::Refused, "outside any terminal") };
    tracing::debug!(tab, op = op.name(), "welcome request");
    let limited = match &op {
        WelcomeOp::State => None,
        WelcomeOp::SetLook { .. } => Some(Limited::WelcomeLook),
        WelcomeOp::SetNotify { .. } => Some(Limited::WelcomeNotify),
        WelcomeOp::InstallHooks { .. } => Some(Limited::WelcomeInstall),
        WelcomeOp::Demo { .. } => Some(Limited::WelcomeDemo),
        WelcomeOp::Finish { .. } => Some(Limited::WelcomeFinish),
    };
    if let Some(kind) = limited
        && let Err(reply) = budget(kind, from, cx)
    {
        return reply;
    }
    match op {
        WelcomeOp::State => Reply::ok(state(tab, cx)),
        WelcomeOp::SetLook { theme, primary, ligatures, font_step } => {
            set_look(theme, primary, ligatures, font_step, cx)
        }
        WelcomeOp::SetNotify { needs_you, agent_done, sound, allow } => {
            set_notify(needs_you, agent_done, sound, allow, cx)
        }
        WelcomeOp::InstallHooks { agent } => install(tab, agent, cx),
        WelcomeOp::Demo { show } => demo(tab, show, cx),
        WelcomeOp::Finish { skipped } => finish(tab, skipped, cx),
    }
}

/// Everything the pages are made of.
fn state(tab: i64, cx: &mut App) -> WelcomeState {
    let reduce_motion = cx.reduce_motion();
    let look = look_facts(cx);
    let notify = notify_facts(cx);
    let Some(run) = app(cx).welcomes.run(tab) else {
        // The gate and the run are made and dropped together; a gate without a run is a bug.
        return empty_state(tab);
    };
    WelcomeState {
        wait_ms: wait(run, reduce_motion, cx).as_millis() as u64,
        reduce_motion,
        handover: run.handover.key().into(),
        first_run: run.first_run,
        look,
        notify: Some(notify),
        agents: run.agents.clone(),
        problem: run.problem.clone(),
    }
}

/// A state for a terminal with a gate and no run: nothing to show, nothing to wait for.
fn empty_state(tab: i64) -> WelcomeState {
    tracing::error!(tab, "a welcome gate without a run");
    WelcomeState {
        wait_ms: 0,
        reduce_motion: false,
        handover: Handover::default().key().into(),
        first_run: false,
        look: LookFacts {
            themes: Vec::new(),
            theme: String::new(),
            primaries: Vec::new(),
            primary: String::new(),
            font: String::new(),
            font_size: 0.0,
            ligatures: false,
        },
        notify: None,
        agents: None,
        problem: None,
    }
}

/// How long the program waits before it draws: the rest of the run's wait for the ship to dock (or the
/// light to lift), if the run is still playing and wasn't cut short.
fn wait(run: &Run, reduce_motion: bool, cx: &App) -> Duration {
    let playing = match run.play {
        Play::Never => None,
        Play::Pending => Some(Duration::ZERO),
        Play::Since(at) => crate::intro::progress(cx).filter(|p| !p.skipped).map(|_| at.elapsed()),
    };
    run.handover.wait(playing, reduce_motion)
}

fn look_facts(cx: &App) -> LookFacts {
    let store = cx.global::<crate::settings::SettingsStore>();
    let s = &store.settings;
    let themes = offered_themes(&s.theme)
        .into_iter()
        .map(|theme| ThemeFact {
            name: theme.name.clone(),
            light: theme.appearance == wormhole_core::theme::tokens::Appearance::Light,
        })
        .collect();
    let (theme_primary, others) = offered_primaries(store.base_tokens());
    let mut primaries = Vec::new();
    if theme_primary.is_some() {
        primaries.push(PrimaryFact { name: "Theme".to_string(), hex: String::new(), ansi: None });
    }
    for (ix, color) in others {
        primaries.push(PrimaryFact {
            name: ansi_name(ix).to_string(),
            hex: color.to_hex(),
            ansi: u8::try_from(ix).ok(),
        });
    }
    LookFacts {
        themes,
        theme: s.theme.clone(),
        primaries,
        primary: s.primary().map(|color| color.to_hex()).unwrap_or_default(),
        font: store.fonts.family(FontSlot::Terminal).to_string(),
        font_size: store.fonts.size(FontSlot::Terminal),
        ligatures: s.font_ligatures,
    }
}

/// A terminal colour by its place in the 16.
fn ansi_name(ix: usize) -> &'static str {
    ["Black", "Red", "Green", "Yellow", "Blue", "Magenta", "Cyan", "White"][ix % 8]
}

/// What reaches the person, as Settings → Notifications has it, and what macOS says of banners.
fn notify_facts(cx: &App) -> NotifyFacts {
    let s = settings(cx);
    NotifyFacts {
        needs_you: s.notify_kind_on(s.notify_needs_you),
        agent_done: s.notify_kind_on(s.notify_agent_done),
        sound: s.notify_sound,
        permission: match notifications::permission(cx) {
            Permission::Checking => PermissionFact::Checking,
            Permission::NotAsked => PermissionFact::NotAsked,
            Permission::Allowed { .. } => PermissionFact::Allowed,
            Permission::Denied => PermissionFact::Denied,
            Permission::OutsideBundle => PermissionFact::Outside,
        },
    }
}

/// `SetLook`: a theme, colour, ligature switch or font step, if it is one the page offers.
fn set_look(
    theme: Option<String>,
    primary: Option<String>,
    ligatures: Option<bool>,
    font_step: Option<i8>,
    cx: &mut App,
) -> Reply {
    let store = cx.global::<crate::settings::SettingsStore>();
    if let Some(name) = &theme
        && !offered_themes(&store.settings.theme).iter().any(|offered| &offered.name == name)
    {
        return Reply::error(ErrorCode::NotFound, format!("no theme {name:?} on offer"));
    }
    if let Some(hex) = &primary {
        let (_, others) = offered_primaries(store.base_tokens());
        if !hex.is_empty() && !others.iter().any(|(_, color)| &color.to_hex() == hex) {
            return Reply::error(ErrorCode::NotFound, format!("no primary colour {hex:?} on offer"));
        }
    }
    update_settings(cx, |s| {
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

/// `SetNotify`: the two switches and the sound Settings → Notifications has, nothing else. A press that
/// turns a banner on (a switch going from off to on), or says Allow, makes macOS ask for permission
/// if it has not been asked; no other request, and no page merely showing, ever does.
fn set_notify(
    needs_you: Option<bool>,
    agent_done: Option<bool>,
    sound: Option<NotifySound>,
    allow: bool,
    cx: &mut App,
) -> Reply {
    let before = settings(cx);
    let (was_needs_you, was_agent_done) =
        (before.notify_kind_on(before.notify_needs_you), before.notify_kind_on(before.notify_agent_done));
    let turns_on =
        (needs_you == Some(true) && !was_needs_you) || (agent_done == Some(true) && !was_agent_done);
    update_settings(cx, |s| {
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
    if turns_on || allow {
        notifications::ask_permission_if_new(cx);
    }
    Reply::ok(serde_json::json!({}))
}

/// `InstallHooks`: the hooks of this one agent, if it was found on PATH. Nothing else is touched.
fn install(tab: i64, agent: AgentId, cx: &mut App) -> Reply {
    let found = app(cx)
        .welcomes
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
    let state = match spec.state(&home) {
        Ok(State::Installed) => HooksFact::Installed,
        Ok(State::Partial) => HooksFact::Partial,
        Ok(State::NotInstalled) | Err(_) => HooksFact::Missing,
    };
    if let Some(run) = app_mut(cx).welcomes.run_mut(tab) {
        if let Some(found) = run
            .agents
            .as_mut()
            .and_then(|agents| agents.found.iter_mut().find(|f: &&mut AgentFact| f.id == agent))
        {
            found.hooks = state;
        }
        run.problem = result.as_ref().err().map(|err| format!("Couldn't write the hooks: {err}"));
    }
    match result {
        Ok(()) => Reply::ok(serde_json::json!({})),
        Err(err) => Reply::error(ErrorCode::Failed, err),
    }
}

/// `Demo`: the example band in the sidebar of the window the welcome's terminal is in. It is the
/// window's own, drawn for as long as the welcome runs and the program asks (`Workspace::set_welcome_demo`).
fn demo(tab: i64, show: bool, cx: &mut App) -> Reply {
    let Some(found) = find(Target::Tab(tab), cx) else {
        return Reply::error(ErrorCode::NotFound, "no such terminal");
    };
    AppState::with_workspace(&found.window, cx, |ws, _, cx| ws.set_welcome_demo(tab, show, cx));
    Reply::ok(serde_json::json!({}))
}

/// `Finish`: the welcome is over. On a first run it is remembered, and the sidebar's note for a run
/// that ended on the primer comes up; the example band goes; a run still playing is cut short.
fn finish(tab: i64, skipped: bool, cx: &mut App) -> Reply {
    tracing::info!(tab, skipped, "the welcome finished");
    let (first_run, primer) = match app(cx).welcomes.run(tab) {
        Some(run) => (run.first_run, run.agents.as_ref().is_some_and(|agents| agents.found.is_empty())),
        None => (false, false),
    };
    app_mut(cx).welcomes.gates.finish(tab);
    if first_run {
        remember_first_run(cx);
    }
    if let Some(found) = find(Target::Tab(tab), cx) {
        AppState::with_workspace(&found.window, cx, |ws, _, cx| {
            ws.welcome_ended(tab, first_run && primer && !skipped, cx)
        });
    }
    crate::intro::skip(cx);
    Reply::ok(serde_json::json!({}))
}
