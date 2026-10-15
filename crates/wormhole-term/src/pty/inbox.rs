//! What the rest of the app tells the reader thread: input for the program, a new size, the end.
//!
//! One queue, so a reply to a program's query goes out after the replies queued before it and
//! before the keys typed after it. Whoever sends wakes the reader only when the queue was empty:
//! the reader empties it in one go, so the messages that follow before it looks are free.

use std::borrow::Cow;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use polling::Poller;

use crate::WindowSize;

/// One thing the app asks of the reader thread.
pub(super) enum Msg {
    /// Bytes for the program's input.
    Input(Cow<'static, [u8]>),
    /// The terminal's new size.
    Resize(WindowSize),
    /// The terminal was closed: end the program (see `hangup`), giving it this long to go.
    Close { grace: Duration },
}

/// The queue of [`Msg`]s into the reader thread, and the way to wake it.
pub(super) struct Inbox {
    queue: Mutex<Vec<Msg>>,
    /// The queue is not empty; checked between slices without taking its lock.
    waiting: AtomicBool,
    poller: Arc<Poller>,
}

impl Inbox {
    /// An empty inbox whose senders wake the thread waiting in `poller`.
    pub(super) fn new(poller: Arc<Poller>) -> Self {
        Self { queue: Mutex::new(Vec::new()), waiting: AtomicBool::new(false), poller }
    }

    /// Queue `msg` for the reader, waking it if it sleeps.
    pub(super) fn send(&self, msg: Msg) {
        let mut queue = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
        let was_empty = queue.is_empty();
        queue.push(msg);
        self.waiting.store(true, Ordering::Release);
        drop(queue);
        if was_empty {
            // Fails only when the poller is gone, and then nobody is left to read the message.
            let _ = self.poller.notify();
        }
    }

    /// Whether there is anything to [`take`](Self::take).
    pub(super) fn has_messages(&self) -> bool {
        self.waiting.load(Ordering::Acquire)
    }

    /// Move everything queued into `into` (appending, in order).
    pub(super) fn take(&self, into: &mut Vec<Msg>) {
        let mut queue = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
        self.waiting.store(false, Ordering::Release);
        into.append(&mut queue);
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use polling::{Events, Poller};

    use super::{Inbox, Msg};

    #[test]
    fn messages_arrive_in_order_and_the_first_wakes_the_poller() {
        let poller = Arc::new(Poller::new().expect("a poller"));
        let inbox = Inbox::new(poller.clone());
        assert!(!inbox.has_messages());
        inbox.send(Msg::Input(b"a".as_slice().into()));
        inbox.send(Msg::Input(b"b".as_slice().into()));
        assert!(inbox.has_messages());
        // The wakeup is there to be collected; one notify was enough for both messages.
        let mut events = Events::new();
        poller.wait(&mut events, Some(std::time::Duration::from_secs(5))).expect("woken");
        let mut got = Vec::new();
        inbox.take(&mut got);
        assert!(!inbox.has_messages());
        let bytes: Vec<&[u8]> = got
            .iter()
            .map(|msg| match msg {
                Msg::Input(bytes) => &bytes[..],
                _ => b"?",
            })
            .collect();
        assert_eq!(bytes, [b"a".as_slice(), b"b".as_slice()]);
    }
}
