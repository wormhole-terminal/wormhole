//! The control protocol's Swift-app side (`wormhole_core::ipc::control`): what `Wormhole list`, `status`,
//! `notify`, `send-text`, `new-terminal` and the rest do over the `Workspace` and its terminals (the GPUI
//! app's `app/control/`, which cannot be linked from here). The rules (who may do what, the caps, the
//! budgets) are core's, and so are the replies; this finds the terminals a request names and carries out
//! what passes.
//!
//! Requests come from the socket thread to the server's one dispatch thread (`dispatch`), so a handler
//! never waits for the person (a question goes to the desk and the request waits on its own thread, core's
//! `AskJob`). Some requests are not answered with a line made here: `events` gets a subscription to the bus
//! and the socket thread writes what it yields, `read` and `blocks` hand it a job that copies the screen,
//! `wait` and `run` a job a thread of its own waits out.

mod input;
mod listing;
mod open;
mod read;
mod stream;
mod structure;
mod wait;

use std::sync::Arc;

use wormhole_core::config::features::FeatureFlag;
use wormhole_core::ipc::access::Access;
use wormhole_core::ipc::control::{
    Action, Error, ErrorCode, PROTOCOL_VERSION, Reason, Reply, Request, Target,
};
use wormhole_core::ipc::{Answer, Peer};

use super::core::Core;
use super::gate::{Gate, gate, own_opened, sweep};

/// Answer `request` for the socket thread: one reply line, a reply and the stream that follows it, a job for
/// the socket thread, or a wait for the user (the gate decides whether the program may do this at all).
pub(crate) fn answer(core: &Arc<Core>, request: Request, peer: &Peer) -> Answer {
    if let Some(error) = version_error(&request) {
        return Answer::Reply(error);
    }
    sweep(core);
    match gate(core, &request, peer) {
        Gate::Open(admit) => {
            let answer = dispatch(core, request, admit.access, admit.readable, peer);
            // What a request opened is the program's own from now on (its scope).
            if let (Some(opener), Answer::Reply(reply)) = (admit.opener, &answer) {
                own_opened(core, opener, reply);
            }
            answer
        }
        Gate::Reply(reply) => Answer::Reply(reply),
        Gate::Ask(job) => Answer::Ask(job),
    }
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

/// Carry out `request`, which `access` lets in.
fn dispatch(
    core: &Arc<Core>,
    request: Request,
    access: Access,
    readable: Option<Vec<i64>>,
    peer: &Peer,
) -> Answer {
    let from = request.from_tab();
    match &request.action {
        Action::Events(spec) => return stream::events(core, spec, from, access, readable, peer),
        Action::Read(spec) => return read::read(core, spec, from, access, peer),
        Action::Blocks(spec) => return read::blocks(core, spec, from, access, peer),
        Action::Wait(spec) => return wait::wait(core, spec, from, access, peer),
        Action::Run(spec) => return wait::run(core, spec, from, access, peer),
        _ => {}
    }
    Answer::Reply(handle(core, request, access, peer))
}

fn handle(core: &Arc<Core>, request: Request, access: Access, peer: &Peer) -> Reply {
    let from = request.from_tab();
    tracing::debug!(?from, action = request.action.name(), "control request");
    match request.action {
        Action::List => Reply::ok(core.publisher.listing()),
        Action::Doctor => listing::doctor(core, from, peer),
        Action::Status { target } => listing::status(core, target, from, peer),
        Action::Notify { target, title, body } => listing::notify(core, target, &title, &body, from, peer),
        Action::SendText { target, text, submit } => {
            input::send_text(core, target, &text, submit, from, access, peer)
        }
        Action::SendKeys(spec) => input::send_keys(core, &spec, from, access, peer),
        Action::Mouse(spec) => input::mouse(core, &spec, from, access, peer),
        Action::NewTerminal { folder, command, worktree, window } => {
            open::new_terminal(core, folder, command, worktree, window, from, access)
        }
        Action::NewWindow(spec) => structure::new_window(core, &spec, from, access),
        Action::Split(spec) => open::split(core, &spec, from, access, peer),
        Action::Focus(spec) => structure::focus(core, &spec, from, access, peer),
        Action::Rename(spec) => structure::rename(core, &spec, from, access, peer),
        Action::Close(spec) => structure::close(core, &spec, from, access, peer),
        // Answered by `dispatch`, which can carry a stream or a job for the socket thread.
        Action::Events(_) | Action::Read(_) | Action::Blocks(_) | Action::Wait(_) | Action::Run(_) => {
            Reply::error(ErrorCode::Failed, "this request is carried out by the socket: not answered here")
        }
        // The first-run welcome is the GPUI app's own program; this app's first run is its own window.
        Action::Welcome { .. } => Reply::error(ErrorCode::Unsupported, "this app has no welcome terminal"),
    }
}

/// The answer to an action of the automation API while its flag is off in this build.
fn automation_off(core: &Core) -> Option<Error> {
    (!core.settings().feature_enabled(FeatureFlag::Automation)).then(|| {
        Error::new(ErrorCode::Unsupported, "automation is not in this build yet").because(Reason::FlagOff)
    })
}

fn not_found(target: Target) -> Reply {
    Reply::error(ErrorCode::NotFound, format!("no {} {}", target.kind(), target.id()))
}
