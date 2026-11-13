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
use wormhole_core::terminal::default_terminal;
use wormhole_core::workspace::{Action, Event, Refusal};

use super::joined::{open_beside, zone_of};
use super::worktree;
use super::{automation_off, not_found};
use crate::hook_server::core::Core;
use crate::workspace::Shared;

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
    // The window asked for; else the front one (the GPUI app's rule, `cli.md`), whichever terminal the request came from.
    let target = match window {
        Some(id) => core.share_of_window(id),
        None => Some(core.front_share()),
    };
    let Some(target) = target else {
        return Reply::error(ErrorCode::NotFound, format!("no window {}", window.unwrap_or_default()));
    };
    if let Err(reply) = core.budget(Limited::NewTerminal, from) {
        return reply;
    }
    // A worktree is never made on a script's say alone: the question is put up in the window instead.
    if let Some(branch) = worktree {
        return worktree::ask(core, folder, &branch, command);
    }
    let opened = open(&target, Action::NewTerminal, folder);
    match opened {
        Some((terminal, cwd)) => {
            if let Some(command) = command {
                run_first(core, terminal, command);
            }
            tracing::info!(terminal, ?from, "control: a terminal was opened");
            Reply::ok(Opened {
                window: target.window_id(),
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

/// Something macOS asked Wormhole to open (a double-clicked `.command`, Open With, a folder dropped on the Dock icon,
/// `open -a Wormhole <folder>`): a terminal for each `file://` URL in window `window` (else the first), a folder as it
/// is and a script run in its folder (core's `default_terminal`). The person opened it themselves, so it needs no
/// access and takes none of the control line's budget. How many terminals it made.
pub(crate) fn open_from_system(core: &Arc<Core>, urls: &[String], window: Option<i64>) -> u32 {
    let target = window.and_then(|id| core.share_of_window(id)).unwrap_or_else(|| core.shared.clone());
    let mut made = 0;
    for url in urls {
        let Some(path) = default_terminal::file_path(url) else {
            tracing::warn!(url, "asked to open something that isn't a local file");
            continue;
        };
        let Some(opened) =
            path.metadata().ok().and_then(|meta| default_terminal::Opened::of(&path, meta.is_dir()))
        else {
            tracing::warn!(path = %path.display(), "asked to open a file that can't be read");
            continue;
        };
        let folder = opened.folder().to_string_lossy().into_owned();
        if let Err(error) = launch_check(Some(&folder), opened.line()) {
            tracing::warn!(path = %path.display(), ?error, "asked to open something a terminal can't start in");
            continue;
        }
        let Some((terminal, _)) = open(&target, Action::NewTerminal, Some(folder)) else {
            tracing::warn!(path = %path.display(), "no room for a terminal to open it in");
            continue;
        };
        if let Some(line) = opened.line() {
            run_first(core, terminal, line.to_string());
        }
        tracing::info!(terminal, script = opened.line().is_some(), "opened from the system");
        made += 1;
    }
    made
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
    let Some(owner) = core.owner_of(anchor) else { return not_found(spec.target) };
    // No room is a mistake that costs nothing: said before the budget is spent.
    if !owner.peek_model(|model| model.split_fits(anchor, zone_of(spec.direction))) {
        return no_room(&owner, anchor, spec.direction);
    }
    if let Err(reply) = core.budget(Limited::NewTerminal, from) {
        return reply;
    }
    let before = owner.peek_model(|model| model.selected());
    let Some((terminal, cwd)) = open_beside(&owner, anchor, spec.direction, spec.folder.clone()) else {
        return no_room(&owner, anchor, spec.direction);
    };
    // A split takes the selection in the model; the control line leaves it where it was unless asked.
    if !spec.select
        && let Some(before) = before
    {
        let events = owner.with_model(|model| model.apply(Action::Select(before)));
        owner.post(events);
    }
    if let Some(command) = spec.command.clone() {
        run_first(core, terminal, command);
    }
    // `--size`: the new pane's share of the split, held by the room as a divider's drag is; the answer is what it got.
    let size = spec.size.and_then(|percent| {
        let (applied, events) = owner.with_model(|model| model.set_split_share(terminal, anchor, percent));
        owner.post(events);
        applied
    });
    let split = core
        .listing()
        .windows
        .iter()
        .flat_map(|w| &w.terminals)
        .find(|t| t.id == terminal)
        .and_then(|t| t.split)
        .unwrap_or_default();
    tracing::info!(terminal, split, ?from, "control: a terminal was split off");
    Reply::ok(SplitMade { window: owner.window_id(), split, terminal, folder: cwd, size })
}

/// The refusal of a split that would leave a pane under the minimum size.
fn no_room(owner: &Shared, anchor: i64, side: wormhole_core::ipc::control::Side) -> Reply {
    Error::new(
        ErrorCode::Refused,
        format!("window {} has no room to split terminal {anchor} {}", owner.window_id(), side.word()),
    )
    .because(Reason::NoRoom)
    .into()
}

/// Do `action`, which makes a terminal: the id and folder it opened (`folder` in the place of the model's
/// choice when given), with the events handed to Swift, which starts its program. `None` when it made none
/// (the room was refused).
fn open(shared: &Shared, action: Action, folder: Option<String>) -> Option<(i64, Option<String>)> {
    let mut events = shared.with_model(|model| model.apply(action));
    let refused = events.iter().any(|event| matches!(event, Event::Refused(Refusal::TooSmall)));
    let opened = events.iter_mut().find_map(|event| match event {
        Event::Opened { id, cwd } => Some((*id, cwd)),
        _ => None,
    });
    let Some((id, cwd)) = opened.filter(|_| !refused) else {
        shared.post(events);
        return None;
    };
    if let Some(folder) = folder {
        *cwd = Some(folder.clone().into());
        let set = shared.with_model(|model| model.update_facts(id, |facts| facts.cwd = Some(folder.into())));
        let shown = cwd.as_ref().map(|path| path.to_string_lossy().into_owned());
        events.extend(set);
        shared.post(events);
        shared.pump.kick();
        return Some((id, shown));
    }
    let shown = cwd.as_ref().map(|path| path.to_string_lossy().into_owned());
    shared.post(events);
    shared.pump.kick();
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
