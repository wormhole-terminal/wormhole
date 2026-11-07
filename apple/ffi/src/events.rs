//! A terminal's events on their way to Swift: one queue, one wake.
//!
//! The engine sends an event per read burst and a flood makes thousands a second; calling Swift for
//! each would be thousands of main-thread hops. So the thread that receives them only queues them
//! (a burst of output is one `Output` in the queue) and calls [`TerminalListener::wake`] once, until
//! Swift drains the queue; the next event after a drain wakes it again. An idle terminal calls
//! nothing and its thread sleeps in the channel.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};

use wormhole_term::{PtyEvent, PtyEvents};

/// What Swift implements to hear that there is something to drain. Called from a Rust thread, with
/// no lock held, at most once between two drains; it must not call back into the terminal's
/// blocking methods, only note the wake and hop to the main queue.
#[uniffi::export(with_foreign)]
pub trait TerminalListener: Send + Sync {
    fn wake(&self);
}

/// Hears every event of a terminal on its event thread, before Swift's queue does: how a
/// `Workspace` learns what a row shows (`workspace_feed`). It must return quickly and may not call
/// into Swift.
pub(crate) trait EventFeed: Send + Sync {
    fn on_event(&self, event: &PtyEvent);
    /// What the person did to the terminal, on the caller's thread (Swift's).
    fn on_touch(&self, touch: Touch);
}

/// What the person did to a terminal: what makes the program print, which is not its own work.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Touch {
    Typed,
    Key { key: String, control: bool },
    Resized,
}

/// What a terminal's program did, as the UI needs to hear it. The rest of `PtyEvent`'s variants
/// (clipboard, marks, notifications, program status, window operations) join as they are needed.
#[derive(Debug, Clone, PartialEq, Eq, uniffi::Enum)]
pub enum TermEvent {
    /// New content: take a frame.
    Output,
    Title {
        title: String,
    },
    ResetTitle,
    Bell,
    /// The program exited, with its code when the system reported one.
    Exited {
        code: Option<i32>,
    },
    /// A program set the clipboard (OSC 52) and `clipboard_write` allows it.
    ClipboardWrite {
        text: String,
    },
    /// A program may read the clipboard: `clipboard_give` the text.
    ClipboardRead,
    /// A program asks to read the clipboard and the person decides: `clipboard_answer`.
    ClipboardAsk,
    /// The question went away unanswered (it timed out).
    ClipboardAskClosed,
    /// The session host stopped answering, or answered again: the bar's words about it change
    /// (`Terminal.host_not_responding`).
    HostResponsiveness,
}

/// Events waiting for a drain, and whether the listener was told.
pub(crate) struct EventQueue {
    pending: Mutex<Vec<TermEvent>>,
    /// The listener was woken and Swift has not drained since.
    woken: AtomicBool,
    listener: Arc<dyn TerminalListener>,
    /// Set once, by the workspace the terminal is registered with.
    feed: OnceLock<Arc<dyn EventFeed>>,
    /// What programs may do with the clipboard (`clipboard.rs`).
    pub(crate) clipboard: crate::clipboard::Clipboard,
    /// Told when the host says a checkpoint it was asked to flush is on disk (`hosting::checkpoints`); set once.
    checkpoint_written: OnceLock<Box<dyn Fn(u64) + Send + Sync>>,
}

impl EventQueue {
    pub(crate) fn new(listener: Arc<dyn TerminalListener>) -> Arc<Self> {
        Arc::new(Self {
            pending: Mutex::new(Vec::new()),
            woken: AtomicBool::new(false),
            listener,
            feed: OnceLock::new(),
            clipboard: Default::default(),
            checkpoint_written: OnceLock::new(),
        })
    }

    /// Call `then` with the generation when the host says its checkpoint is written; the first one wins.
    pub(crate) fn on_checkpoint_written(&self, then: Box<dyn Fn(u64) + Send + Sync>) {
        let _ = self.checkpoint_written.set(then);
    }

    /// Let `feed` hear the events from now on; the first one wins.
    pub(crate) fn set_feed(&self, feed: Arc<dyn EventFeed>) {
        let _ = self.feed.set(feed);
    }

    /// Tell the feed what the person did (a no-op until a workspace feeds this terminal).
    pub(crate) fn touched(&self, touch: Touch) {
        if let Some(feed) = self.feed.get() {
            feed.on_touch(touch);
        }
    }

    pub(crate) fn push_event(&self, event: TermEvent) {
        self.push(event);
    }

    fn push(&self, event: TermEvent) {
        {
            let mut pending = self.pending.lock().unwrap_or_else(PoisonError::into_inner);
            // Output is "something is new", not a count: one is enough until it is drained.
            if event == TermEvent::Output && pending.contains(&TermEvent::Output) {
                return;
            }
            pending.push(event);
        }
        if !self.woken.swap(true, Ordering::AcqRel) {
            self.listener.wake();
        }
    }

    /// Take everything queued. Re-arms the wake first, so an event that arrives while Swift handles
    /// these is not lost: it finds the flag clear and wakes again.
    pub(crate) fn drain(&self) -> Vec<TermEvent> {
        self.woken.store(false, Ordering::Release);
        std::mem::take(&mut *self.pending.lock().unwrap_or_else(PoisonError::into_inner))
    }
}

/// The thread that moves a terminal's events into `queue`, until its channel closes.
pub(crate) fn spawn_receiver(events: PtyEvents, queue: Arc<EventQueue>) -> std::io::Result<()> {
    std::thread::Builder::new().name("wormhole-ffi-events".into()).spawn(move || {
        while let Some(event) = smol::block_on(events.recv()) {
            if let Some(feed) = queue.feed.get() {
                feed.on_event(&event);
            }
            if let (PtyEvent::CheckpointWritten(generation), Some(then)) =
                (&event, queue.checkpoint_written.get())
            {
                then(*generation);
            }
            if let Some(event) = translate(event, &queue) {
                queue.push(event);
            }
        }
    })?;
    Ok(())
}

fn translate(event: PtyEvent, queue: &Arc<EventQueue>) -> Option<TermEvent> {
    Some(match event {
        PtyEvent::ClipboardStore(text) => return queue.clipboard.store(text),
        PtyEvent::ClipboardLoad(format, reply) => return queue.clipboard.load(queue, format, reply),
        PtyEvent::Output => TermEvent::Output,
        PtyEvent::Title(title) => TermEvent::Title { title },
        PtyEvent::ResetTitle => TermEvent::ResetTitle,
        PtyEvent::Bell => TermEvent::Bell,
        PtyEvent::Exited(code) => TermEvent::Exited { code },
        PtyEvent::HostResponsiveness => TermEvent::HostResponsiveness,
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::AtomicUsize;

    use super::*;

    struct Counting(AtomicUsize);

    impl TerminalListener for Counting {
        fn wake(&self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn a_flood_wakes_once_until_it_is_drained() {
        let listener = Arc::new(Counting(AtomicUsize::new(0)));
        let queue = EventQueue::new(listener.clone());
        for _ in 0..1000 {
            queue.push(TermEvent::Output);
        }
        queue.push(TermEvent::Bell);
        assert_eq!(listener.0.load(Ordering::SeqCst), 1);
        assert_eq!(queue.drain(), [TermEvent::Output, TermEvent::Bell], "one Output for the flood");
        queue.push(TermEvent::Output);
        assert_eq!(listener.0.load(Ordering::SeqCst), 2, "the next event after a drain wakes again");
    }
}
