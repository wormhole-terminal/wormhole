//! The thread that answers the socket: one at a time, in the order the requests came, so a handler sees the
//! workspace as the one before it left it. It never waits for a person (a question goes to the desk) and
//! never panics out: a request whose handler did is answered with a failure and the thread goes on.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;

use wormhole_core::ipc::control::{ErrorCode, Reply};
use wormhole_core::ipc::{Answer, Incoming};

use super::core::Core;
use super::{control, messages};

/// Serve `requests` until the channel closes (`HookServer::stop`).
pub(super) fn spawn(core: Arc<Core>, requests: smol::channel::Receiver<Incoming>) -> std::io::Result<()> {
    std::thread::Builder::new().name("wormhole-hook-dispatch".into()).spawn(move || {
        while let Ok(incoming) = smol::block_on(requests.recv()) {
            handle(&core, incoming);
        }
    })?;
    Ok(())
}

fn handle(core: &Arc<Core>, incoming: Incoming) {
    match incoming {
        Incoming::Message(message) => {
            if catch_unwind(AssertUnwindSafe(|| messages::apply(core, message))).is_err() {
                tracing::error!("a hook message made the handler panic");
            }
        }
        Incoming::Control { request, peer, reply } => {
            let answer = catch_unwind(AssertUnwindSafe(|| control::answer(core, request, &peer)))
                .unwrap_or_else(|_| {
                    tracing::error!("a control request made the handler panic");
                    Answer::Reply(Reply::error(
                        ErrorCode::Failed,
                        "Wormhole failed while handling this request",
                    ))
                });
            // The connection waits on its thread for this; a receiver that gave up is no error.
            let _ = reply.send(answer);
        }
    }
}
