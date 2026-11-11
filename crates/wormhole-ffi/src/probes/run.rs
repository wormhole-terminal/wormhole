//! Running the Probes that are on: loading them, listening to the event bus for their hooks, and carrying out what
//! the hooks decided. What the GPUI app does in `app::probes` and `app::probe_effects`, over core's shell-neutral
//! parts (`ipc::probe_feed`, `probe::active`, `probe::runtime`); what only Swift can show arrives as `HookEvent`s.
//!
//! The Probes are read off every thread that matters from their sealed copies (`active::load`) when something may
//! have changed them: the server starting, the settings (the `probes` flag, "Never hide lines"), the user approving or
//! turning one off (`approvals_changed`). One thread (`wormhole-probe-effects`) reads the hook feed, plans each event
//! (pure, under each hook's budget) and carries the effects out. Nothing runs in a frame and no hook can block,
//! rewrite or answer anything a terminal does. With the flag off, or no Probe with a hook on, there is no
//! subscription and the bus is told nothing extra.

use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, PoisonError, Weak};
use std::time::Instant;

use smol::channel::Sender;
use wormhole_core::config::features::FeatureFlag;
use wormhole_core::ipc::probe_feed::{self, Feed, Listener, TabFacts as HookFacts};
use wormhole_core::probe::active::{self, Dispatcher, Effect, RunJob};
use wormhole_core::probe::hidden::{reveal, shown};
use wormhole_core::probe::open_path::PathRef;
use wormhole_core::probe::runtime::{RuleSet, StatusItem};
use wormhole_term::folds::FoldOwner;
use wormhole_text::keys;

use super::state;
use crate::hook_server::{Core, HookEvent};

/// The Probes of one server: the feed that is running, which load is the newest, and what this server has put in the
/// process-wide state (`state`): it takes back only what it put there, so a server that never had Probes on (every test's)
/// leaves the state of one that does alone.
#[derive(Default)]
pub(crate) struct Probes {
    listener: Mutex<Option<Listener>>,
    serial: AtomicU64,
    installed: AtomicBool,
    folds_off: AtomicBool,
}

/// The one server that is running, for the Settings window's approvals to tell (`approvals_changed`).
static SERVER: Mutex<Option<Weak<Core>>> = Mutex::new(None);

/// Read the settings and the Probes again and run what is on. Every call starts a load that replaces an older one.
pub(crate) fn reload(core: &Arc<Core>) {
    *SERVER.lock().unwrap_or_else(PoisonError::into_inner) = Some(Arc::downgrade(core));
    let settings = core.settings();
    let folds_off = !settings.feature_enabled(FeatureFlag::OutputFolds) || settings.never_hide_lines;
    let moved = if folds_off {
        core.probes.folds_off.store(true, Ordering::Relaxed);
        state::set_folds_off(true)
    } else {
        core.probes.folds_off.swap(false, Ordering::Relaxed) && state::set_folds_off(false)
    };
    let serial = core.probes.serial.fetch_add(1, Ordering::AcqRel) + 1;
    if !settings.feature_enabled(FeatureFlag::Probes) {
        let stopped = core.probes.listener.lock().unwrap_or_else(PoisonError::into_inner).take().is_some();
        if core.probes.installed.swap(false, Ordering::Relaxed) {
            state::install(None, Vec::new());
            retain_folds(core, &[]);
        }
        if moved || stopped {
            core.ui.push(HookEvent::ProbesChanged);
        }
        return;
    }
    if moved {
        core.ui.push(HookEvent::ProbesChanged);
    }
    let (weak, root) = (Arc::downgrade(core), core.root.clone());
    let spawned = std::thread::Builder::new().name("wormhole-probes-load".into()).spawn(move || {
        let loaded = active::load(&root);
        if let Some(core) = weak.upgrade().filter(|core| core.probes.serial.load(Ordering::Acquire) == serial)
        {
            start(&core, loaded);
        }
    });
    if let Err(err) = spawned {
        tracing::warn!("the Probes could not be loaded: {err}");
    }
}

/// The user approved or turned off a Probe (`ProbesDesk`): the ones that are on are read again.
pub(crate) fn approvals_changed() {
    let core = SERVER.lock().unwrap_or_else(PoisonError::into_inner).as_ref().and_then(Weak::upgrade);
    if let Some(core) = core {
        reload(&core);
    }
}

fn start(core: &Arc<Core>, loaded: active::Loaded) {
    for (id, why) in &loaded.skipped {
        tracing::warn!(probe = %id, "a Probe that is on could not run: {why}");
    }
    // Nothing is on and this server never had anything on: there is nothing to put in the state or take out of it.
    if loaded.probes.is_empty() && !core.probes.installed.load(Ordering::Relaxed) {
        return;
    }
    // A Probe turned off, or one that lost the `fold` grant, loses its folds now, not at the next fold.
    let folding: Vec<String> =
        loaded.probes.iter().filter(|probe| probe.grants.fold).map(|probe| probe.id.clone()).collect();
    retain_folds(core, &folding);
    let names = loaded.probes.iter().map(|probe| (probe.id.clone(), probe.name.clone())).collect();
    core.probes.installed.store(true, Ordering::Relaxed);
    state::install(Some(RuleSet::new(active::rules_of(&loaded.probes))), names);
    let dispatcher = Dispatcher::new(loaded.probes);
    let listener = listen(core, dispatcher);
    *core.probes.listener.lock().unwrap_or_else(PoisonError::into_inner) = listener;
    core.ui.push(HookEvent::ProbesChanged);
}

fn retain_folds(core: &Core, probes: &[String]) {
    for (_, terminal) in core.registered() {
        terminal.with_pty(|pty| pty.has_folds().then(|| pty.fold_retain_probes(probes)));
    }
}

/// Subscribe to the topics `dispatcher`'s hooks listen to and carry out what they decide.
fn listen(core: &Arc<Core>, mut dispatcher: Dispatcher) -> Option<Listener> {
    let (listener, feed) = probe_feed::listen(&core.bus, &dispatcher)?;
    if listener.output {
        // Where each terminal's output starts from: its cursor now, so what was printed before the Probes came on
        // is not matched as if it were new.
        for (id, terminal) in core.registered() {
            if let Some(reader) = terminal.reader() {
                let window = core.owner_of(id).map_or(core.window, |shared| shared.window_id());
                core.bus.watch_output(window, id, &reader);
            }
        }
    }
    let (weak, results): (Weak<Core>, Sender<Feed>) = (Arc::downgrade(core), listener.sender.clone());
    let spawned = std::thread::Builder::new().name("wormhole-probe-effects".into()).spawn(move || {
        // Ends when the listener is dropped: that closes the channel.
        while let Ok(item) = feed.recv_blocking() {
            let Some(core) = weak.upgrade() else { break };
            let facts = |id: i64| facts_of(&core, id);
            let effects = item.plan(&mut dispatcher, &facts, Instant::now());
            carry_out(&core, effects, &results);
        }
    });
    if let Err(err) = spawned {
        tracing::warn!("the Probes' hooks hear nothing: {err}");
        return None;
    }
    Some(listener)
}

/// What the app knows of terminal `id` that a hook's `match` may test. The model knows no front window, so `front` is
/// "the selected terminal of its window".
fn facts_of(core: &Core, id: i64) -> Option<HookFacts> {
    let shared = core.owner_of(id)?;
    shared.peek_model(|model| {
        let selected = model.terminals_snapshot().terminals.into_iter().find(|t| t.id == id)?;
        let agent = model.facts(id).is_some_and(|facts| facts.agent.is_some());
        Some(HookFacts { front: selected.selected, agent, name: selected.name })
    })
}

/// Terminal `id`'s folder as the git chips have it, which a program's escape sequence alone cannot set: what a
/// hook's `run` starts in.
fn folder_of(core: &Core, id: i64) -> Option<std::path::PathBuf> {
    core.owner_of(id)?.peek_model(|model| model.facts(id).and_then(|facts| facts.cwd.clone()))
}

/// Do what `effects` say. `results` is where a program started here reports its end.
fn carry_out(core: &Arc<Core>, effects: Vec<Effect>, results: &Sender<Feed>) {
    let mut changed = false;
    for effect in effects {
        match effect {
            Effect::Notify { probe, probe_name, title, body, .. } => core.ui.push(HookEvent::ProbeNotice {
                probe,
                subtitle: format!("{} (extension)", reveal(&probe_name)),
                title,
                body,
            }),
            Effect::Status { probe, probe_name, item, text, tone, tooltip, .. } => {
                changed |= state::set_status(StatusItem { probe, probe_name, item, text, tone, tooltip });
            }
            Effect::Highlight { tone, terminal, block, .. } => {
                if let Ok(block) = u64::try_from(block) {
                    state::mark_block(terminal, block, tone);
                    changed = true;
                }
            }
            Effect::Fold { probe, probe_name, terminal, block, label, keep } => {
                // Text a Probe chose, shown on a marker and read by VoiceOver: made visible, cut to fit.
                let owner = FoldOwner { probe, name: shown(&probe_name), label: shown(&label) };
                changed |= fold(core, terminal, block, owner, keep);
            }
            Effect::OpenPath { probe_name, terminal, path, line, .. } => {
                let text = PathRef { path, line, column: None }.text();
                let words = keys::notice_warned_probe_offer(&shown(&probe_name), &shown(&text));
                core.ui.push(HookEvent::ProbeOffer { terminal, words, text });
            }
            Effect::Run { ticket, probe, argv, cwd, timeout, env, terminal } => {
                let folder = terminal.and_then(|id| folder_of(core, id));
                probe_feed::start_run(
                    core.root.clone(),
                    ticket,
                    RunJob { probe, argv, cwd, folder, timeout, env, terminal },
                    results,
                );
            }
            Effect::Paused { probe_name, hook } => {
                tracing::warn!(probe = %probe_name, hook, "a Probe's hook failed too often and was paused");
                core.ui.push(HookEvent::ProbePaused {
                    words: keys::notice_warned_probe_paused(&shown(&probe_name)),
                });
            }
        }
    }
    if changed {
        core.ui.push(HookEvent::ProbesChanged);
    }
}

/// A Probe's hook folds the output of finished block `block` in terminal `id`. The terminal says no to what it may not
/// fold (`wormhole_term::folds`: the live block, what needs the person, a secret being typed, the alternate screen);
/// a refusal is not shown. Whether a fold was made.
fn fold(core: &Core, id: i64, block: i64, owner: FoldOwner, keep: (u32, u32)) -> bool {
    let (Ok(block), Some(terminal)) = (u64::try_from(block), core.terminal(id)) else { return false };
    if state::folds_off() {
        return false;
    }
    match terminal.with_pty(|pty| pty.fold_block_output_by(block, owner, keep)) {
        Some(Ok(_)) => true,
        Some(Err(refused)) => {
            tracing::debug!(?refused, "a Probe's fold was refused");
            false
        }
        None => false,
    }
}
