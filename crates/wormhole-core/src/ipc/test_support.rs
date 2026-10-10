//! What the ipc tests share.

use std::time::Duration;

use super::{Incoming, Message};

/// The next message, or `None` when nothing arrives within `within` (or the channel is closed).
///
/// Every wait for a message in a test goes through this: a bare `smol::block_on(rx.recv())` blocks
/// for good when the code under test sends nothing (the server keeps its sender alive), so a
/// regression hung the whole test run instead of failing one test.
pub(super) fn recv_within(rx: &smol::channel::Receiver<Incoming>, within: Duration) -> Option<Incoming> {
    smol::block_on(smol::future::or(async { rx.recv().await.ok() }, async {
        smol::Timer::after(within).await;
        None
    }))
}

/// [`recv_within`] for the messages that need no answer; a control request in its place is `None`.
pub(super) fn recv_message_within(
    rx: &smol::channel::Receiver<Incoming>,
    within: Duration,
) -> Option<Message> {
    match recv_within(rx, within)? {
        Incoming::Message(message) => Some(message),
        Incoming::Control { .. } => None,
    }
}
