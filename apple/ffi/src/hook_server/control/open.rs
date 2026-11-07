//! Opening terminals from the control line: `new-terminal` and `split`. The model makes the terminal (its id,
//! its row, its place in the split) and says `Opened`; the Swift side starts the program for it as for any
//! terminal the person opens (`Terminal.startForTab`, then `register_terminal`), so what is queued for Swift
//! here is the model's own `Opened` event. A command the request asks for is typed once that shell is ready.

use std::sync::Arc;
use std::time::{Duration, Instant};

use wormhole_core::ipc::Peer;
use wormhole_core::ipc::access::Access;
use wormhole_core::ipc::control::structure::{launch_check, size_check};
use wormhole_core::ipc::control::{
    Error, ErrorCode, Limited, Opened, Reason, Reply, SplitMade, SplitSpec, writes_off,
};
use wormhole_core::workspace::{Action, Event, Refusal};

use super::joined::open_beside;
use super::worktree;
use super::{automation_off, not_found};
use crate::hook_server::core::Core;

/// How often a new terminal is looked at while its command waits.
const POLL: Duration = Duration::from_millis(50);
/// Output that has stopped for this long is a prompt waiting.
const QUIET: Duration = Duration::from_millis(300);
/// A shell that has printed nothing this long after its PTY came has no prompt to wait for.
const SILENT: Duration = Duration::from_secs(2);
/// A shell whose output never settles is typed at all the same.
const GIVE_UP: Duration = Duration::from_secs(5);
/// A terminal that Swift never started a program for: the wait ends.
const NO_PTY: Duration = Duration::from_secs(30);
/// How long a terminal may be in no window's list before it counts as closed: a window that `new-window` planned
/// has its terminal only once Swift has opened it.
const WINDOW_GRACE: Duration = Duration::from_secs(3);
/// The pause between the pasted command and the Return that runs it: the program reads the paste first.
const SUBMIT_DELAY: Duration = Duration::from_millis(60);

/// `new-terminal`: a terminal in `folder` (or where the model starts one), selected, running `command` in
/// its shell when given.
pub(super) fn new_terminal(
    core: &Arc<Core>,
    folder: Option<String>,
    command: Option<String>,
    worktree: Option<String>,
    window: Option<i64>,
    from: Option<i64>,
    access: Access,
) -> Reply {
    if !access.write {
        return Reply::from(writes_off());
    }
    if let Err(error) = launch_check(folder.as_deref(), command.as_deref()) {
        return error.into();
    }
    if let Some(id) = window.filter(|id| *id != core.window) {
        return Reply::error(ErrorCode::NotFound, format!("no window {id}"));
    }
    if let Err(reply) = core.budget(Limited::NewTerminal, from) {
        return reply;
    }
    // A worktree is never made on a script's say alone: the question is put up in the window instead.
    if let Some(branch) = worktree {
        return worktree::ask(core, folder, &branch, command);
    }
    let opened = open(core, Action::NewTerminal, folder);
    match opened {
        Some((terminal, cwd)) => {
            if let Some(command) = command {
                run_first(core, terminal, command);
            }
            tracing::info!(terminal, ?from, "control: a terminal was opened");
            Reply::ok(Opened {
                window: core.window,
                terminal,
                tab: terminal,
                pane: terminal,
                folder: cwd,
                worktree: None,
            })
        }
        None => Reply::error(ErrorCode::Failed, "the terminal could not be made"),
    }
}

/// `split`: a new terminal beside the target, joined into its split (made if it stood alone), to its right or
/// below it. The selection stays where it was unless the request says.
pub(super) fn split(
    core: &Arc<Core>,
    spec: &SplitSpec,
    from: Option<i64>,
    access: Access,
    peer: &Peer,
) -> Reply {
    if let Some(off) = automation_off(core) {
        return off.into();
    }
    if !access.write {
        return Reply::from(writes_off());
    }
    if let Err(error) =
        size_check(spec.size).and(launch_check(spec.folder.as_deref(), spec.command.as_deref()))
    {
        return error.into();
    }
    let Some(anchor) = core.find(spec.target, peer) else { return not_found(spec.target) };
    if let Err(reply) = core.budget(Limited::NewTerminal, from) {
        return reply;
    }
    let before = core.shared.peek_model(|model| model.selected());
    let Some((terminal, cwd)) = open_beside(core, anchor, spec.direction, spec.folder.clone()) else {
        return Error::new(
            ErrorCode::Refused,
            format!(
                "window {} has no room to split terminal {anchor} {}",
                core.window,
                spec.direction.word()
            ),
        )
        .because(Reason::NoRoom)
        .into();
    };
    // A split takes the selection in the model; the control line leaves it where it was unless asked.
    if !spec.select
        && let Some(before) = before
    {
        let events = core.shared.with_model(|model| model.apply(Action::Select(before)));
        core.shared.post(events);
    }
    if let Some(command) = spec.command.clone() {
        run_first(core, terminal, command);
    }
    let split = core
        .publisher
        .listing()
        .windows
        .iter()
        .flat_map(|w| &w.terminals)
        .find(|t| t.id == terminal)
        .and_then(|t| t.split)
        .unwrap_or_default();
    tracing::info!(terminal, split, ?from, "control: a terminal was split off");
    Reply::ok(SplitMade { window: core.window, split, terminal, folder: cwd, size: None })
}

/// Do `action`, which makes a terminal: the id and folder it opened (`folder` in the place of the model's
/// choice when given), with the events handed to Swift, which starts its program. `None` when it made none
/// (the room was refused).
fn open(core: &Core, action: Action, folder: Option<String>) -> Option<(i64, Option<String>)> {
    let mut events = core.shared.with_model(|model| model.apply(action));
    let refused = events.iter().any(|event| matches!(event, Event::Refused(Refusal::TooSmall)));
    let opened = events.iter_mut().find_map(|event| match event {
        Event::Opened { id, cwd } => Some((*id, cwd)),
        _ => None,
    });
    let Some((id, cwd)) = opened.filter(|_| !refused) else {
        core.shared.post(events);
        return None;
    };
    if let Some(folder) = folder {
        *cwd = Some(folder.clone().into());
        let set =
            core.shared.with_model(|model| model.update_facts(id, |facts| facts.cwd = Some(folder.into())));
        let shown = cwd.as_ref().map(|path| path.to_string_lossy().into_owned());
        events.extend(set);
        core.shared.post(events);
        core.shared.pump.kick();
        return Some((id, shown));
    }
    let shown = cwd.as_ref().map(|path| path.to_string_lossy().into_owned());
    core.shared.post(events);
    core.shared.pump.kick();
    Some((id, shown))
}

/// Type `command` into terminal `tab` and run it once its shell is ready for it: Swift has to start the program
/// first, and the shell may not have drawn a prompt (see the constants). On a thread of its own, bounded by
/// the new-terminal budget that let the request in.
pub(super) fn run_first(core: &Arc<Core>, tab: i64, command: String) {
    let command = command.trim().to_string();
    if command.is_empty() {
        return;
    }
    let core = core.clone();
    let spawned = std::thread::Builder::new().name("wormhole-hook-first".into()).spawn(move || {
        let started = Instant::now();
        let mut wait = Wait::default();
        loop {
            std::thread::sleep(POLL);
            let Some(terminal) = core.terminal(tab) else {
                if started.elapsed() >= NO_PTY
                    || (started.elapsed() >= WINDOW_GRACE && core.owner_of(tab).is_none())
                {
                    tracing::warn!("a terminal never got a shell: its first command was not typed");
                    return;
                }
                continue;
            };
            let Some(reader) = terminal.reader() else { return };
            if wait.ready(Instant::now(), reader.at_prompt(), reader.revision()) {
                terminal.paste(command.clone());
                std::thread::sleep(SUBMIT_DELAY);
                terminal.write(b"\r".to_vec());
                return;
            }
        }
    });
    if let Err(err) = spawned {
        tracing::warn!("couldn't wait to type a first command: {err}");
    }
}

/// When the first command may be typed, decided from looks at the terminal: the shell's own prompt mark when
/// it has integration, else the output going quiet, and at last a time limit.
#[derive(Default)]
struct Wait {
    /// When the PTY was first seen, the revision then, the latest revision and when it changed.
    seen: Option<(Instant, u64, u64, Instant)>,
}

impl Wait {
    fn ready(&mut self, now: Instant, at_prompt: Option<bool>, revision: u64) -> bool {
        let (since, first, last, changed) = self.seen.get_or_insert((now, revision, revision, now));
        if *last != revision {
            *last = revision;
            *changed = now;
        }
        let heard = revision != *first;
        at_prompt == Some(true)
            || (heard && now.duration_since(*changed) >= QUIET)
            || (!heard && now.duration_since(*since) >= SILENT)
            || now.duration_since(*since) >= GIVE_UP
    }
}
