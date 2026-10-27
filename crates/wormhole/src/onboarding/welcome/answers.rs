//! The welcome program's requests, answered (`Action::Welcome`). The door is core's `Registry`: a
//! request passes only from its welcome terminal with that terminal's token while the welcome runs, and
//! within the caps. What passes is done here, on the UI thread, outside any window's update, so the
//! window a terminal is in can be updated through `AppState::with_workspace`.
//!
//! Each op asks for one thing and gets exactly that: the look changes only to a theme, colour,
//! ligature setting or font step the page offers; what reaches the person changes only by the switches
//! and sound Settings → Notifications has (macOS is asked for permission only when a press turns a
//! banner on or says Allow, and only if it has not been asked); hooks are written for the one agent
//! named, if it was found on PATH; the example band goes up in the window's sidebar and comes down; how
//! much the window shows changes only to the title row, status bar, minimap, lines between the columns,
//! tools and right panel that the walk's answers make (core's `Shown`), and only the parts that differ
//! from what the window shows are written.

use std::time::Duration;

use wormhole_core::agents::AgentId;
use wormhole_core::agents::integrations::{self, State};
use wormhole_core::config::NotifySound;
use wormhole_core::config::presets::{Shown, TitleShown, ToolsShown};
use wormhole_core::import::{Env, Proposed, Source};
use wormhole_core::ipc::control::{
    AgentFact, ErrorCode, HooksFact, ImportDone, Limited, LookFacts, NotifyFacts, PermissionFact,
    PrimaryFact, Reply, ShownDone, ThemeFact, WelcomeOp, WelcomeState,
};
use wormhole_core::onboarding::intro::Handover;
use wormhole_core::theme::fonts::FontSlot;

use super::{ImportedTheme, Play, Run};
use crate::onboarding::{offered_primaries, offered_themes};
use crate::platform::notifications::Permission;
use crate::settings::import as shared;
use crate::settings::update_settings;
use crate::settings::{settings, settings_without_zen};
use gpui_kit::App;

/// Counts one request of a kind against its caller's budget, or says when to try again. It is the
/// control socket's, which every caller's budgets are kept by; it is handed in because the app root
/// that owns them sits above the welcome.
pub(crate) type Budget = fn(Limited, Option<i64>, &mut App) -> Result<(), Reply>;

/// What the welcome asks of the app root, which owns it: the budgets, and the notifications' permission
/// (what macOS has said of banners, and asking it, once). Handed in, not named, so the welcome uses
/// nothing above the window.
#[derive(Clone, Copy)]
pub(crate) struct Services {
    pub budget: Budget,
    pub permission: fn(&App) -> Permission,
    pub ask_permission: fn(&mut App) -> bool,
    /// Whether the right panel of the window terminal `tab` is in is on screen; `None` if no window has
    /// that terminal, or its settings are not the ones an edit reaches (another profile's).
    pub panel: fn(tab: i64, &App) -> Option<bool>,
    /// Opens or closes the right panel of the window terminal `tab` is in.
    pub set_panel: fn(tab: i64, open: bool, &mut App),
    /// Shows or hides the example band in the sidebar of the window terminal `tab` is in; whether there
    /// is such a terminal.
    pub demo: fn(tab: i64, show: bool, &mut App) -> bool,
    /// The welcome in terminal `tab` is over, in whichever window it is: the example band goes and, with
    /// `note`, the sidebar's note for a run that ended on the primer comes up.
    pub ended: fn(tab: i64, note: bool, &mut App),
    /// The first run is over: not shown again.
    pub remember_first_run: fn(&mut App),
    /// Where an `Import` looks for the other terminal's settings: this process's own home and variables
    /// (`Env::from_process`); a test hands in a folder of its own, never the real home.
    pub import_env: fn() -> Option<Env>,
}

/// Answer a welcome request. `from` is the tab it says it comes from; `budget` is checked after the
/// request is authorised, so a request the door refuses does not use any up.
pub(crate) fn handle(
    token: String,
    op: WelcomeOp,
    from: Option<i64>,
    services: Services,
    cx: &mut App,
) -> Reply {
    if let Err(error) = super::welcomes_mut(cx).gates.authorize(from, &token, &op) {
        return Reply::from(error);
    }
    let Some(tab) = from else { return Reply::error(ErrorCode::Refused, "outside any terminal") };
    tracing::debug!(tab, op = op.name(), "welcome request");
    let limited = match &op {
        WelcomeOp::State => None,
        WelcomeOp::SetLook { .. } => Some(Limited::WelcomeLook),
        WelcomeOp::SetNotify { .. } => Some(Limited::WelcomeNotify),
        WelcomeOp::SetShown { .. } => Some(Limited::WelcomeShown),
        WelcomeOp::InstallHooks { .. } => Some(Limited::WelcomeInstall),
        WelcomeOp::Demo { .. } => Some(Limited::WelcomeDemo),
        WelcomeOp::Import { .. } => Some(Limited::WelcomeImport),
        WelcomeOp::Finish { .. } => Some(Limited::WelcomeFinish),
    };
    if let Some(kind) = limited
        && let Err(reply) = (services.budget)(kind, from, cx)
    {
        return reply;
    }
    match op {
        WelcomeOp::State => Reply::ok(state(tab, &services, cx)),
        WelcomeOp::SetLook { theme, primary, ligatures, font_step } => {
            set_look(tab, theme, primary, ligatures, font_step, cx)
        }
        WelcomeOp::SetNotify { needs_you, agent_done, sound, allow } => {
            set_notify(needs_you, agent_done, sound, allow, &services, cx)
        }
        WelcomeOp::SetShown { from, shown } => set_shown(tab, &from, &shown, &services, cx),
        WelcomeOp::InstallHooks { agent } => install(tab, agent, cx),
        WelcomeOp::Demo { show } => demo(tab, show, &services, cx),
        WelcomeOp::Import { source } => import(tab, &source, &services, cx),
        WelcomeOp::Finish { skipped } => finish(tab, skipped, &services, cx),
    }
}

/// Everything the pages are made of.
fn state(tab: i64, services: &Services, cx: &mut App) -> WelcomeState {
    let reduce_motion = cx.reduce_motion();
    let Some(run) = super::run_of(tab, cx) else {
        // The gate and the run are made and dropped together; a gate without a run is a bug.
        return empty_state(tab);
    };
    let look = look_facts(run.imported_theme.as_ref(), cx);
    let notify = notify_facts(services, cx);
    WelcomeState {
        wait_ms: wait(run, reduce_motion, cx).as_millis() as u64,
        reduce_motion,
        handover: run.handover.key().into(),
        first_run: run.first_run,
        look,
        notify: Some(notify),
        agents: run.agents.clone(),
        shown: (services.panel)(tab, cx).map(|panel_open| Shown::read(settings_without_zen(cx), panel_open)),
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
        shown: None,
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

fn look_facts(imported: Option<&ImportedTheme>, cx: &App) -> LookFacts {
    let store = cx.global::<crate::settings::SettingsStore>();
    let s = &store.settings;
    let mut themes: Vec<ThemeFact> = offered_themes(&s.theme)
        .into_iter()
        .map(|theme| ThemeFact {
            name: theme.name.clone(),
            light: theme.appearance == wormhole_core::theme::tokens::Appearance::Light,
        })
        .collect();
    // The theme an import brought over stays on the list, in use or not.
    let mut in_use = s.theme.clone();
    if let Some(imported) = imported {
        themes.push(ThemeFact { name: imported.name.clone(), light: imported.light });
        if s.theme == imported.value {
            in_use.clone_from(&imported.name);
        }
    }
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
        theme: in_use,
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
fn notify_facts(services: &Services, cx: &App) -> NotifyFacts {
    let s = settings(cx);
    NotifyFacts {
        needs_you: s.notify_kind_on(s.notify_needs_you),
        agent_done: s.notify_kind_on(s.notify_agent_done),
        sound: s.notify_sound,
        permission: match (services.permission)(cx) {
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
    tab: i64,
    theme: Option<String>,
    primary: Option<String>,
    ligatures: Option<bool>,
    font_step: Option<i8>,
    cx: &mut App,
) -> Reply {
    let store = cx.global::<crate::settings::SettingsStore>();
    let imported = super::run_of(tab, cx).and_then(|run| run.imported_theme.clone());
    // A theme on offer is a built-in one or the one an import brought over, which is set by its file.
    let mut theme = theme;
    if let Some(name) = &theme {
        let imported_choice = imported.as_ref().filter(|imported| &imported.name == name);
        if let Some(imported) = imported_choice {
            theme = Some(imported.value.clone());
        } else if !offered_themes(&store.settings.theme).iter().any(|offered| &offered.name == name) {
            return Reply::error(ErrorCode::NotFound, format!("no theme {name:?} on offer"));
        }
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
    services: &Services,
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
        let _ = (services.ask_permission)(cx);
    }
    Reply::ok(serde_json::json!({}))
}

/// `SetShown`: how much the window shows, from what the walk showed before the answer (`from`) to what
/// it shows with it. Only the parts where the two differ are written, so what the person changed or
/// arranged meanwhile is left as it is, and a title row or a set of tools of the person's own is
/// accepted only as the one `from` already had (the walk never offers to make one). Every setting but
/// the right panel is the app's; the panel is that window's, and is answered with where it ended up (a
/// window too narrow for it keeps it hidden). A window of a profile whose settings an edit does not
/// reach (`edits_in`) is refused rather than written to the shown profile's.
fn set_shown(tab: i64, from: &Shown, shown: &Shown, services: &Services, cx: &mut App) -> Reply {
    let own_title = matches!(shown.title, TitleShown::Own(_)) && shown.title != from.title;
    let own_tools = matches!(shown.tools, ToolsShown::Own(_)) && shown.tools != from.tools;
    if own_title || own_tools {
        return Reply::error(ErrorCode::BadRequest, "an arrangement of one's own can only stay as it is");
    }
    if (services.panel)(tab, cx).is_none() {
        return Reply::error(ErrorCode::NotFound, "no such terminal, or its settings can't be changed here");
    }
    if (Shown { panel_open: from.panel_open, ..shown.clone() }) != *from {
        update_settings(cx, |s| shown.apply_changes(from, s));
    }
    if shown.panel_open != from.panel_open {
        (services.set_panel)(tab, shown.panel_open, cx);
    }
    let panel_open = (services.panel)(tab, cx).unwrap_or(shown.panel_open);
    Reply::ok(ShownDone { panel_open })
}

/// `InstallHooks`: the hooks of this one agent, if it was found on PATH. Nothing else is touched.
fn install(tab: i64, agent: AgentId, cx: &mut App) -> Reply {
    let found = super::run_of(tab, cx)
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
    if let Some(run) = super::run_of_mut(tab, cx) {
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
/// window's own, drawn for as long as the welcome runs and the program asks.
fn demo(tab: i64, show: bool, services: &Services, cx: &mut App) -> Reply {
    if !(services.demo)(tab, show, cx) {
        return Reply::error(ErrorCode::NotFound, "no such terminal");
    }
    Reply::ok(serde_json::json!({}))
}

/// `Import`: the settings of one other terminal, which the page offered because the program found it.
/// The request names only which terminal: the app reads that terminal's files itself, under its own
/// `Env`, and takes what Settings → Import ticks by default (never a shell command), in one settings
/// update, with its palette saved as a theme in the active profile. The files are read on this thread,
/// once, for a press (a few small capped files and the system's font lookup).
fn import(tab: i64, source: &str, services: &Services, cx: &mut App) -> Reply {
    let Some(source) = Source::from_key(source) else {
        return Reply::error(ErrorCode::NotFound, "no such terminal to import from");
    };
    let Some(found) = (services.import_env)().and_then(|env| shared::scan_in(&env, source)) else {
        return Reply::error(ErrorCode::NotFound, "that terminal's settings are not there any more");
    };
    let chosen: Vec<Proposed> = found.proposal.selected().cloned().collect();
    match shared::apply(cx, &found, &chosen) {
        Ok(done) => {
            remember_imported_theme(tab, cx);
            tracing::info!(tab, source = source.key(), brought = done.brought, left = done.left, "imported");
            Reply::ok(ImportDone { brought: done.brought, left: done.left })
        }
        Err(why) => {
            tracing::warn!(tab, source = source.key(), "import failed: {why}");
            Reply::error(ErrorCode::Failed, why)
        }
    }
}

/// The theme file an import just set (it is not one of the built-ins the list offers), kept on the
/// run so the look page lists it from now on, in use or not.
fn remember_imported_theme(tab: i64, cx: &mut App) {
    let store = cx.global::<crate::settings::SettingsStore>();
    let value = store.settings.theme.clone();
    let themes =
        wormhole_core::theme::registry::UserThemes::read(&crate::active_profile::paths().themes_dir());
    let Some(theme) = themes.themes.iter().find(|theme| theme.path.to_string_lossy() == value.as_str())
    else {
        return;
    };
    let imported = ImportedTheme {
        name: theme.name.clone(),
        value,
        light: store.base_tokens().appearance == wormhole_core::theme::tokens::Appearance::Light,
    };
    if let Some(run) = super::run_of_mut(tab, cx) {
        run.imported_theme = Some(imported);
    }
}

/// `Finish`: the welcome is over. On a first run it is remembered, and the sidebar's note for a run
/// that ended on the primer comes up; the example band goes; a run still playing is cut short.
fn finish(tab: i64, skipped: bool, services: &Services, cx: &mut App) -> Reply {
    tracing::info!(tab, skipped, "the welcome finished");
    let (first_run, primer) = match super::run_of(tab, cx) {
        Some(run) => (run.first_run, run.agents.as_ref().is_some_and(|agents| agents.found.is_empty())),
        None => (false, false),
    };
    super::welcomes_mut(cx).gates.finish(tab);
    if first_run {
        (services.remember_first_run)(cx);
    }
    (services.ended)(tab, first_run && primer && !skipped, cx);
    crate::intro::skip(cx);
    Reply::ok(serde_json::json!({}))
}
