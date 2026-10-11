//! The welcome program's requests, answered (`Action::Welcome`). The door is core's `Registry`: a
//! request passes only from its welcome terminal with that terminal's token while the welcome runs, and
//! within the caps. What passes is done here, on the UI thread, outside any window's update, so the
//! window a terminal is in can be updated through `AppState::with_workspace`.
//!
//! Each op asks for one thing and gets exactly that: the look changes only to a theme, colour,
//! ligature setting or font step the page offers; hooks are written for the one agent named, if it
//! was found on PATH; a folder is trusted only through Touch ID, and only if it is a project root.

use std::path::{Path, PathBuf};
use std::time::Duration;

use wormhole_core::agents::AgentId;
use wormhole_core::agents::integrations::{self, State};
use wormhole_core::fonts::FontSlot;
use wormhole_core::ipc::control::{
    AgentFact, ErrorCode, HooksFact, Limited, LookFacts, PrimaryFact, Reply, Target, ThemeFact, TrustFacts,
    WelcomeOp, WelcomeState,
};
use wormhole_core::project::{self, TrustStatus};

use super::{Play, Run, remember_first_run};
use crate::workspace::control::{budget, find};
use crate::workspace::onboarding::{offered_primaries, offered_themes};
use crate::workspace::settings_page::update_settings;
use crate::workspace::{AppState, app, app_mut};
use gpui_kit::App;

/// Answer a welcome request. `from` is the tab it says it comes from.
pub(crate) fn handle(token: String, op: WelcomeOp, from: Option<i64>, cx: &mut App) -> Reply {
    if let Err(error) = app(cx).welcomes.gates.authorize(from, &token, &op) {
        return Reply::error(error.code, error.message);
    }
    let Some(tab) = from else { return Reply::error(ErrorCode::Refused, "outside any terminal") };
    tracing::debug!(tab, op = op.name(), "welcome request");
    let limited = match &op {
        WelcomeOp::State { .. } => None,
        WelcomeOp::SetLook { .. } => Some(Limited::WelcomeLook),
        WelcomeOp::InstallHooks { .. } => Some(Limited::WelcomeInstall),
        WelcomeOp::Trust { .. } => Some(Limited::WelcomeTrust),
        WelcomeOp::Finish { .. } => Some(Limited::WelcomeFinish),
    };
    if let Some(kind) = limited
        && let Err(reply) = budget(kind, from, cx)
    {
        return reply;
    }
    match op {
        WelcomeOp::State { folder } => Reply::ok(state(tab, folder, cx)),
        WelcomeOp::SetLook { theme, primary, ligatures, font_step } => {
            set_look(theme, primary, ligatures, font_step, cx)
        }
        WelcomeOp::InstallHooks { agent } => install(tab, agent, cx),
        WelcomeOp::Trust { folder } => trust(tab, &folder, cx),
        WelcomeOp::Finish { folder, skipped } => finish(tab, folder, skipped, cx),
    }
}

/// Everything the pages are made of.
fn state(tab: i64, folder: Option<String>, cx: &mut App) -> WelcomeState {
    let reduce_motion = cx.reduce_motion();
    let look = look_facts(cx);
    let trust = trust_facts(folder.as_deref(), cx);
    let Some(run) = app(cx).welcomes.run(tab) else {
        // The gate and the run are made and dropped together; a gate without a run is a bug.
        return empty_state(tab);
    };
    WelcomeState {
        wait_ms: wait(run, reduce_motion, cx).as_millis() as u64,
        reduce_motion,
        handover: run.handover,
        first_run: run.first_run,
        places: run.places.clone(),
        look,
        agents: run.agents.clone(),
        trust,
        problem: run.problem.clone(),
    }
}

/// A state for a terminal with a gate and no run: nothing to show, nothing to wait for.
fn empty_state(tab: i64) -> WelcomeState {
    tracing::error!(tab, "a welcome gate without a run");
    WelcomeState {
        wait_ms: 0,
        reduce_motion: false,
        handover: Default::default(),
        first_run: false,
        places: Vec::new(),
        look: LookFacts {
            themes: Vec::new(),
            theme: String::new(),
            primaries: Vec::new(),
            primary: String::new(),
            font: String::new(),
            font_size: 0.0,
            ligatures: false,
        },
        agents: None,
        trust: None,
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
            light: theme.appearance == wormhole_core::tokens::Appearance::Light,
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

/// The chosen folder's project, when it has one that isn't trusted: what trusting it would enable.
fn trust_facts(folder: Option<&str>, cx: &App) -> Option<TrustFacts> {
    if !crate::workspace::project::project_folders_on(cx) {
        return None;
    }
    // The program names the folder only on the page that shows it.
    let folder = PathBuf::from(folder?);
    let root = project::find_project(&folder)?.root;
    let loaded = crate::workspace::project::load_project_info(&root, cx)?;
    if loaded.status == TrustStatus::Trusted {
        return None;
    }
    Some(TrustFacts {
        folder: root.to_string_lossy().into_owned(),
        name: root.file_name().map(|name| name.to_string_lossy().into_owned()).unwrap_or_default(),
        dir: project::dir_name_at(&root).to_string(),
        commands: loaded.request.as_ref().map_or(0, |request| request.commands.len()),
        asking: app(cx).hub.approving,
    })
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
            s.primary_color = hex;
        }
        if let Some(on) = ligatures {
            s.font_ligatures = on;
        }
        if let Some(step) = font_step {
            FontSlot::Terminal.step_size(s, f32::from(step));
        }
    });
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

/// `Trust`: ask for Touch ID to trust this folder's project. Only a project root, only with the
/// project folders feature on; the answer is in the next state.
fn trust(tab: i64, folder: &str, cx: &mut App) -> Reply {
    if !crate::workspace::project::project_folders_on(cx) {
        return Reply::error(ErrorCode::NotAllowed, "project folders are off");
    }
    let path = Path::new(folder);
    if !path.is_absolute() || !path.is_dir() {
        return Reply::error(ErrorCode::NotFound, format!("no folder at {folder}"));
    }
    let Some(root) = project::find_project(path)
        .map(|found| found.root)
        .filter(|root| root.as_path() == path.canonicalize().unwrap_or_default())
    else {
        return Reply::error(ErrorCode::NotFound, "that folder isn't a project root");
    };
    if let Some(run) = app_mut(cx).welcomes.run_mut(tab) {
        run.problem = None;
    }
    crate::workspace::project::approve_project(&root, cx);
    Reply::ok(serde_json::json!({}))
}

/// `Finish`: the welcome is over. On a first run it is remembered, and the sidebar's note for a run
/// that ended on the primer comes up; a run still playing is cut short.
fn finish(tab: i64, folder: Option<String>, skipped: bool, cx: &mut App) -> Reply {
    tracing::info!(tab, skipped, folder = ?folder, "the welcome finished");
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
