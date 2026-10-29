//! What a hosted terminal's [`Pty`](crate::Pty) tells its connection's thread, and the handle it
//! keeps for that. One queue, so input, resizes and the end go out in the order they were asked
//! for; whoever sends wakes the thread only when the queue was empty, as the PTY's inbox does.

use std::borrow::Cow;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::Duration;

use polling::Poller;

use crate::GridSize;
use crate::types::{COLOR_COUNT, ColorScheme, Rgb};

/// One thing the `Pty` asks of the thread.
pub(crate) enum Cmd {
    /// Bytes for the program's input.
    Input(Cow<'static, [u8]>),
    /// Ask the host for this size.
    Resize(GridSize),
    /// The theme's colors, for the host's answers to color queries.
    Colors(Box<[Rgb; COLOR_COUNT]>),
    /// The theme is light or dark, for the host's answers and its reports of a change.
    ColorScheme(ColorScheme),
    ClearHistory,
    /// The answer to the host's `ClipboardLoad{ticket}`.
    Clipboard {
        ticket: u32,
        data: Vec<u8>,
    },
    /// On screen (`Sync`, a replica fed) or not (`None`, headless).
    Visible(bool),
    /// Let go of the host, which keeps running.
    Detach,
    /// End the terminal: the host hangs the program up and gives it this long.
    Close(Duration),
}

/// The queue into the thread, and what the thread says back.
pub(crate) struct Link {
    queue: Mutex<Vec<Cmd>>,
    /// The queue is not empty; checked without its lock.
    waiting: AtomicBool,
    poller: Arc<Poller>,
    /// The connection ended without the program's exit: the host is gone.
    lost: AtomicBool,
    /// The thread is over; nothing sent now is read.
    ended: AtomicBool,
}

impl Link {
    pub(crate) fn new(poller: Arc<Poller>) -> Self {
        Self {
            queue: Mutex::new(Vec::new()),
            waiting: AtomicBool::new(false),
            poller,
            lost: AtomicBool::new(false),
            ended: AtomicBool::new(false),
        }
    }

    /// Queue `cmd` for the thread, waking it if it sleeps. Dropped once the thread is over.
    pub(crate) fn send(&self, cmd: Cmd) {
        if self.ended.load(Ordering::Acquire) {
            return;
        }
        let mut queue = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
        let was_empty = queue.is_empty();
        queue.push(cmd);
        self.waiting.store(true, Ordering::Release);
        drop(queue);
        if was_empty {
            // Fails only when the poller is gone, and then so is the thread.
            let _ = self.poller.notify();
        }
    }

    pub(crate) fn has_messages(&self) -> bool {
        self.waiting.load(Ordering::Acquire)
    }

    /// Move everything queued into `into`, in order.
    pub(crate) fn take(&self, into: &mut Vec<Cmd>) {
        let mut queue = self.queue.lock().unwrap_or_else(PoisonError::into_inner);
        self.waiting.store(false, Ordering::Release);
        into.append(&mut queue);
    }

    pub(crate) fn set_lost(&self) {
        self.lost.store(true, Ordering::Release);
    }

    pub(crate) fn lost(&self) -> bool {
        self.lost.load(Ordering::Acquire)
    }

    /// The thread is over: what is queued is dropped, and so is what comes.
    pub(crate) fn end(&self) {
        self.ended.store(true, Ordering::Release);
        self.queue.lock().unwrap_or_else(PoisonError::into_inner).clear();
    }
}

/// A hosted terminal's end of its connection, as the `Pty` holds it.
pub(crate) struct Hosted {
    pub(crate) link: Arc<Link>,
    /// The host's 12-character id.
    pub(crate) id: String,
    /// [`detach`](Self::detach) was called: dropping the terminal leaves the host running.
    detached: AtomicBool,
}

impl Hosted {
    pub(crate) fn new(link: Arc<Link>, id: String) -> Self {
        Self { link, id, detached: AtomicBool::new(false) }
    }

    pub(crate) fn input(&self, bytes: Cow<'static, [u8]>) {
        if !bytes.is_empty() {
            self.link.send(Cmd::Input(bytes));
        }
    }

    pub(crate) fn resize(&self, size: GridSize) {
        self.link.send(Cmd::Resize(size));
    }

    pub(crate) fn set_colors(&self, palette: &[Rgb; COLOR_COUNT]) {
        self.link.send(Cmd::Colors(Box::new(*palette)));
    }

    pub(crate) fn set_color_scheme(&self, scheme: ColorScheme) {
        self.link.send(Cmd::ColorScheme(scheme));
    }

    pub(crate) fn clear_history(&self) {
        self.link.send(Cmd::ClearHistory);
    }

    pub(crate) fn set_visible(&self, visible: bool) {
        self.link.send(Cmd::Visible(visible));
    }

    /// Let go of the host; it keeps the program running. Once.
    pub(crate) fn detach(&self) {
        if !self.detached.swap(true, Ordering::AcqRel) {
            self.link.send(Cmd::Detach);
        }
    }

    pub(crate) fn is_detached(&self) -> bool {
        self.detached.load(Ordering::Acquire)
    }

    /// End the terminal, unless it was detached.
    pub(crate) fn close(&self, grace: Duration) {
        if !self.is_detached() {
            self.link.send(Cmd::Close(grace));
        }
    }
}
