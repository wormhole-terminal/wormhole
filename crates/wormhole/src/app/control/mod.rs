//! The control protocol's app side (`wormhole_core::ipc::control`): what
//! `Wormhole list`, `status`, `notify`, `send-text` and `new-terminal` do here (and the first-run
//! welcome's requests go on to `welcome`). Windows and splits (`new-window`, `split`, `focus`, `rename`,
//! `close`) are `structure`.
//!
//! Requests come from the socket thread through `AppState`'s message task (`app::tick::dispatch_hook`),
//! outside any window's update, so every window can be read and the one a request is about updated
//! through `AppState::with_workspace`. The rules (who may do what, the caps, the budgets) are
//! core's; this module finds the windows and terminals a request names and carries out what
//! passes. A terminal, its one tab and its pane are the same thing now (a row of the sidebar, named
//! by its id), so the protocol's `tab` and `pane` ids are the terminal's.
//!
//! Some requests are not answered with a line made here: `events` (`stream`) gets a subscription to
//! the bus (`crate::automation`) and the socket thread writes what it yields, `read` and `blocks`
//! (`read`) hand the socket thread a job that copies the terminal's screen, so the UI thread never
//! does, and `wait` and `run` (`wait`) hand over a job that a thread of its own waits out. Typing
//! (`input`: `send-text`, `send-keys`, `mouse`) is answered here.

mod doctor;
mod input;
mod read;
mod stream;
mod structure;
mod wait;

use std::time::Instant;

use gpui_kit::*;
use wormhole_core::ipc::control::structure::launch_check;
use wormhole_core::ipc::control::{
    Action, Error, ErrorCode, Limited, Listing, PROTOCOL_VERSION, RateLimits, Reached, Reason, Reply,
    Request, StatusInfo, Summary, Target, WindowInfo, notice_text, writes_off,
};
use wormhole_core::ipc::{Answer, Peer};

use crate::app_state::{AppState, OpenWindow, app};
use crate::notifications;
use crate::settings::settings;

/// Each caller's budgets, app-wide (a tab's requests count the same whichever window they reach).
#[derive(Default)]
struct Budgets(RateLimits);

impl Global for Budgets {}

/// Answer `request` for the socket thread: one reply line, or a reply and the stream that follows it.
pub(crate) fn answer(request: Request, peer: &Peer, cx: &mut App) -> Answer {
    if version_error(&request).is_none() {
        let from = request.from_tab();
        match &request.action {
            Action::Events(spec) => return stream::events(spec, from, peer, cx),
            Action::Read(spec) => return read::read(spec, from, peer, cx),
            Action::Blocks(spec) => return read::blocks(spec, from, peer, cx),
            Action::Wait(spec) => return wait::wait(spec, from, peer, cx),
            Action::Run(spec) => return wait::run(spec, from, peer, cx),
            _ => {}
        }
    }
    Answer::Reply(handle(request, peer, cx))
}

/// The answer to an action of the automation API while its flag is off in this build.
fn automation_off(cx: &App) -> Option<Error> {
    (!settings(cx).feature_enabled(wormhole_core::config::features::FeatureFlag::Automation)).then(|| {
        Error::new(ErrorCode::Unsupported, "automation is not in this build yet").because(Reason::FlagOff)
    })
}

fn version_error(request: &Request) -> Option<Reply> {
    (request.version != PROTOCOL_VERSION).then(|| {
        Reply::error(
            ErrorCode::Version,
            format!(
                "this Wormhole speaks protocol {PROTOCOL_VERSION}; the request is version {}",
                request.version
            ),
        )
    })
}

/// Answer `request` with one reply. Never fails: whatever is wrong with it is said in the reply.
pub(crate) fn handle(request: Request, peer: &Peer, cx: &mut App) -> Reply {
    let from = request.from_tab();
    tracing::debug!(?from, action = request.action.name(), "control request");
    if let Some(error) = version_error(&request) {
        return error;
    }
    match request.action {
        Action::List => Reply::ok(listing(cx)),
        Action::Doctor => doctor::doctor(from, peer, cx),
        Action::Status { target } => status(target, from, cx),
        Action::Notify { target, title, body } => notify(target, &title, &body, from, cx),
        Action::SendText { target, text, submit } => input::send_text(target, &text, submit, from, peer, cx),
        Action::SendKeys(spec) => input::send_keys(&spec, from, peer, cx),
        Action::Mouse(spec) => input::mouse(&spec, from, peer, cx),
        Action::NewTerminal { folder, command, worktree, window } => {
            new_terminal(folder, command, worktree, window, from, cx)
        }
        Action::NewWindow(spec) => structure::new_window(&spec, from, cx),
        Action::Split(spec) => structure::split(&spec, from, cx),
        Action::Focus(spec) => structure::focus(&spec, from, cx),
        Action::Rename(spec) => structure::rename(&spec, from, cx),
        Action::Close(spec) => structure::close(&spec, from, cx),
        // Answered by `answer`, which can carry a stream or a job for the socket thread; a reply
        // alone (devctl's `control`) cannot.
        Action::Events(_) => {
            Reply::error(ErrorCode::Failed, "events are a stream: they are not answered here")
        }
        Action::Read(_) | Action::Blocks(_) => {
            Reply::error(ErrorCode::Failed, "reads are carried out by the socket: they are not answered here")
        }
        Action::Wait(_) | Action::Run(_) => Reply::error(
            ErrorCode::Failed,
            "waits are carried out by a thread of the socket's: they are not answered here",
        ),
        // Not behind `control_write`: the token the app put in the welcome terminal is the key.
        Action::Welcome { token, op } => {
            crate::onboarding::welcome::handle(token, op, from, welcome_services(), cx)
        }
    }
}

/// What the welcome is handed: this socket's budgets, the notifications' permission and the windows to
/// show its example band in, which the first run and the welcome, below the app root, may not name.
fn welcome_services() -> crate::onboarding::welcome::Services {
    crate::onboarding::welcome::Services {
        budget,
        permission: crate::notifications::permission,
        ask_permission: crate::notifications::ask_permission_if_new,
        demo: crate::app::welcome::demo,
        ended: crate::app::welcome::ended,
        remember_first_run: crate::window::remember_first_run,
        import_env: wormhole_core::import::Env::from_process,
    }
}

/// Every window with its terminals, oldest window first.
fn listing(cx: &App) -> Listing {
    let front = AppState::front_window(cx).map(|window| window.id);
    let windows = app(cx)
        .windows
        .iter()
        .filter_map(|window| {
            let (terminals, splits) = window
                .workspace
                .read_with(cx, |ws, cx| (ws.control_terminals(cx), ws.control_splits()))
                .ok()?;
            Some(WindowInfo { id: window.id, front: front == Some(window.id), terminals, splits })
        })
        .collect();
    Listing { windows }
}

/// `status`: the target's terminal, the caller's own without one, the app's summary outside any tab.
fn status(target: Option<Target>, from: Option<i64>, cx: &App) -> Reply {
    let Some(target) = target.or(from.map(Target::Tab)) else {
        let listing = listing(cx);
        let states = listing.windows.iter().flat_map(|w| w.terminals.iter().map(|t| t.status));
        return Reply::ok(Summary::of(listing.windows.len(), states));
    };
    let Some(found) = find(target, cx) else { return not_found(target) };
    let terminal =
        found.window.workspace.read_with(cx, |ws, cx| ws.control_terminal(found.terminal, cx)).ok();
    match terminal.flatten() {
        Some(terminal) => Reply::ok(StatusInfo { window: found.window.id, terminal, tab: found.tab }),
        None => not_found(target),
    }
}

/// `notify`: a notification about the target's tab (the caller's own without one), in the
/// caller's words, delivered by the notification rules like a program's own.
fn notify(target: Option<Target>, title: &str, body: &str, from: Option<i64>, cx: &mut App) -> Reply {
    let Some(words) = notice_text(title, body) else {
        return Reply::error(ErrorCode::BadRequest, "nothing to say once the text is cleaned");
    };
    let Some(target) = target.or(from.map(Target::Tab)) else {
        return Reply::error(
            ErrorCode::BadRequest,
            "name a terminal (--terminal, --tab or --pane); inside one of Wormhole's terminals its own is the default",
        );
    };
    let Some(found) = find(target, cx) else { return not_found(target) };
    if let Err(reply) = budget(Limited::Notify, from, cx) {
        return reply;
    }
    let tab = found.tab;
    let told = AppState::with_workspace(&found.window, cx, |ws, _, cx| ws.control_notice(tab, words, cx));
    if told != Some(true) {
        return Reply::error(ErrorCode::Failed, format!("tab {tab} could not be reached"));
    }
    // At once rather than on the next tick; the window's update is over, so it can be read.
    notifications::sweep(cx);
    Reply::ok(Reached { tab })
}

/// `new-terminal`: a terminal in `folder` (or where the window's current one is), in `window`
/// (or the front one), running `command` in its shell when given.
fn new_terminal(
    folder: Option<String>,
    command: Option<String>,
    worktree: Option<String>,
    window: Option<i64>,
    from: Option<i64>,
    cx: &mut App,
) -> Reply {
    if !settings(cx).control_write {
        let error = writes_off();
        return Reply::from(error);
    }
    if let Err(error) = launch_check(folder.as_deref(), command.as_deref()) {
        return error.into();
    }
    let target = match window {
        Some(id) => app(cx).windows.iter().find(|w| w.id == id).cloned(),
        None => AppState::front_window(cx),
    };
    let Some(target) = target else {
        return match window {
            Some(id) => Reply::error(ErrorCode::NotFound, format!("no window {id}")),
            None => Reply::error(ErrorCode::Failed, "no window is open"),
        };
    };
    if let Err(reply) = budget(Limited::NewTerminal, from, cx) {
        return reply;
    }
    // A worktree is never made on a script's say alone: the dialog asks the user, in the window.
    if let Some(branch) = worktree {
        let asked = AppState::with_workspace(&target, cx, |ws, window, cx| {
            crate::features::git::worktree::api::control_ask(ws, folder, &branch, command, window, cx)
        });
        return match asked {
            Some(Ok(asked)) => Reply::ok(asked),
            Some(Err(error)) => Reply::from(error),
            None => Reply::error(ErrorCode::Failed, "the window could not be reached"),
        };
    }
    let opened = AppState::with_workspace(&target, cx, |ws, window, cx| {
        ws.control_new_terminal(folder, command.as_deref(), window, cx)
    });
    match opened {
        Some(opened) => {
            tracing::info!(terminal = opened.terminal, ?from, "control: a terminal was opened");
            Reply::ok(opened)
        }
        None => Reply::error(ErrorCode::Failed, "the window could not be reached"),
    }
}

/// Count a `kind` request of `from` against its budget, or say when to try again.
fn budget(kind: Limited, from: Option<i64>, cx: &mut App) -> Result<(), Reply> {
    let now = Instant::now();
    let budgets = &mut cx.default_global::<Budgets>().0;
    if budgets.allow(kind, from, now) {
        return Ok(());
    }
    let wait = budgets.retry_after(kind, from, now).as_secs().max(1);
    Err(Error::new(
        ErrorCode::RateLimited,
        format!("too many requests of this kind lately; try again in {wait}s"),
    )
    .retry_after(wait)
    .into())
}

fn not_found(target: Target) -> Reply {
    Reply::error(ErrorCode::NotFound, format!("no {} {}", target.kind(), target.id()))
}

/// Where a target is: its window, its terminal (session) and the tab the request means.
struct Found {
    window: OpenWindow,
    terminal: i64,
    tab: i64,
}

fn find(target: Target, cx: &App) -> Option<Found> {
    app(cx).windows.iter().find_map(|window| {
        let (terminal, tab) =
            window.workspace.read_with(cx, |ws, _| ws.control_find(target)).ok().flatten()?;
        Some(Found { window: window.clone(), terminal, tab })
    })
}
