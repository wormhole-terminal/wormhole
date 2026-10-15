//! The PTY layer: a pseudo-terminal, the program on it, and the thread that moves bytes between
//! them and the screen.
//!
//! - `open`: the master and slave and the window size in cells and pixels; `discipline`: the
//!   slave's line discipline (`IUTF8`, `stty sane`);
//! - `spawn`: the program in a new session with the slave as its controlling terminal;
//! - `reader`: the thread. It polls the master (`master`), reads, parses into the screen in
//!   slices under [`FairLock`] (`parse`), writes input (`writer`), applies resizes, notices that
//!   the child exited (`exit`) and, when the terminal is closed, ends the program (`hangup`);
//! - `inbox`: what the rest of the app tells the thread; `lock`: the fair lock the UI shares the
//!   screen with it.
//!
//! Nothing here knows the engine. The thread is given the screen and a [`Feed`] that parses into
//! it; core's `engine` supplies both.

mod discipline;
#[cfg(test)]
mod engine_tests;
mod exit;
mod hangup;
mod inbox;
mod lock;
mod master;
mod open;
mod parse;
mod reader;
mod spawn;
#[cfg(test)]
mod tests;
mod writer;

use std::borrow::Cow;
use std::io;
use std::sync::Arc;
use std::time::Duration;

use polling::Poller;

use super::WindowSize;
use super::events::EventProxy;
use inbox::{Inbox, Msg};
use master::Master;
use reader::Reader;

pub(in crate::terminal) use hangup::HANGUP_GRACE;
pub(in crate::terminal) use lock::FairLock;
pub(in crate::terminal) use parse::Feed;
pub(in crate::terminal) use spawn::{Launch, Shell};

/// A running terminal's input, size and end, for the app's side. The thread outlives it when the
/// terminal is closed, until the program is gone.
pub(in crate::terminal) struct Control {
    inbox: Arc<Inbox>,
}

/// What starting a terminal made.
pub(in crate::terminal) struct Started {
    pub control: Control,
    /// The program's process id, which leads its session and its process group.
    pub pid: u32,
}

impl Control {
    /// Write to the program's input. Never waits: the thread writes when the master takes it.
    pub(in crate::terminal) fn input(&self, bytes: Cow<'static, [u8]>) {
        self.inbox.send(Msg::Input(bytes));
    }

    /// Tell the program's terminal its new size (the screen is resized by the caller).
    pub(in crate::terminal) fn resize(&self, size: WindowSize) {
        self.inbox.send(Msg::Resize(size));
    }

    /// The terminal was closed: end the program (see `hangup`), giving it `grace` to go before
    /// it is killed. Returns at once. Not for a program that has exited already: its pid may be
    /// someone else's, and its thread is gone.
    pub(in crate::terminal) fn close(&self, grace: Duration) {
        self.inbox.send(Msg::Close { grace });
    }

    /// A way to write to the program's input from anywhere (query replies).
    pub(in crate::terminal) fn writer(&self) -> Box<dyn Fn(Vec<u8>) + Send + Sync> {
        let inbox = self.inbox.clone();
        Box::new(move |bytes| inbox.send(Msg::Input(bytes.into())))
    }
}

/// Start `launch` on a new pseudo-terminal of `size` and a thread that parses its output with
/// `feed` into `screen`.
pub(in crate::terminal) fn start<S: Send + 'static, F: Feed<S>>(
    launch: &Launch,
    size: WindowSize,
    screen: Arc<FairLock<S>>,
    feed: F,
    proxy: EventProxy,
) -> io::Result<Started> {
    let open::Pair { master, slave } = open::open(size)?;
    let poller = Arc::new(Poller::new()?);
    let master = Master::new(master, poller.clone())?;
    let inbox = Arc::new(Inbox::new(poller.clone()));
    let child = spawn::spawn(launch, slave)?;
    let pid = child.id();
    let reader = Reader::new(master, child, poller, inbox.clone(), screen, feed, proxy);
    // When this fails the reader is dropped with the master, which hangs the terminal up on the
    // program.
    std::thread::Builder::new().name("PTY reader".into()).spawn(move || reader.run())?;
    Ok(Started { control: Control { inbox }, pid })
}
