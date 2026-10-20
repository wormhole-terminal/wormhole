//! The engine behind the terminal: Wormhole's own (`wormhole_vt`), and the host side of it that
//! runs on the PTY thread. The PTY layer (`pty`) is core's own and knows no engine; this module
//! gives it the screen and the [`Feed`] that parses into it.
//!
//! - `mod` (here): [`OwnFeed`], what the PTY thread drives (the host and a
//!   synchronized update's deadline), and [`spawn`];
//! - `host`: the engine's events as `PtyEvent`s, the shell marks as the command log, replies
//!   written back to the PTY in order;
//! - `images`: decoding the pictures the engine sends and keeping their pixels.
//!
//! The screen itself, [`Screen`], is `terminal::screen`. The engine reads every sequence itself
//! (the shell-integration and notification OSCs, pictures, `clear` erasing the history), so the
//! PTY's bytes go from the master to the parser as they are.

mod host;
mod images;
#[cfg(test)]
mod tests;

use std::sync::Arc;
use std::time::Instant;

use anyhow::Context as _;

use super::events::{EventProxy, Shared};
use super::pty::{self, FairLock, Feed};
use super::screen::{Hold, SYNC_TIMEOUT, Screen};
use super::{GridSize, StartOptions, restore};
use host::Host;
#[cfg(any(test, feature = "test-support"))]
pub(super) use host::mark_of;
use images::Uploads;
pub(crate) use images::key as image_key;

/// The host side of the engine, as the PTY thread drives it (the parser is the terminal's own).
pub(super) struct OwnFeed {
    host: Host,
    /// When the update in progress is given up on (a mirror of the screen's [`Hold`], which the
    /// reader asks for without the screen).
    deadline: Option<Instant>,
}

impl OwnFeed {
    pub(super) fn new(proxy: EventProxy, screen: Arc<FairLock<Screen>>) -> Self {
        let uploads = Uploads::new(proxy.clone(), screen);
        Self { host: Host::new(proxy, uploads), deadline: None }
    }
}

impl Feed<Screen> for OwnFeed {
    fn feed(&mut self, screen: &mut Screen, bytes: &[u8]) {
        screen.term.feed(bytes);
        screen.update_hold(Instant::now());
        self.host.settle(screen);
        self.deadline = match screen.hold {
            Hold::Since(began) => Some(began + SYNC_TIMEOUT),
            Hold::Off | Hold::GaveUp => None,
        };
    }

    fn deadline(&self) -> Option<Instant> {
        self.deadline
    }

    fn expire(&mut self, screen: &mut Screen) {
        screen.hold = Hold::GaveUp;
        self.deadline = None;
    }
}

/// What starting a terminal made.
pub(super) struct Spawned {
    /// The terminal the PTY thread parses into and the UI reads. The UI holds the lock only to
    /// copy or to act, so the PTY thread's parsing never waits for a frame; the lock is fair
    /// (`pty::FairLock`), so the UI never waits more than one slice of parsing either.
    pub screen: Arc<FairLock<Screen>>,
    /// The program's input, size and end.
    pub io: pty::Control,
    /// The program's process id.
    pub pid: u32,
}

/// Start `launch` in a PTY of `size`, with the engine's terminal and parser on the reader thread.
pub(super) fn spawn(
    size: GridSize,
    options: &StartOptions,
    launch: &pty::Launch,
    proxy: EventProxy,
    shared: &Arc<Shared>,
) -> anyhow::Result<Spawned> {
    let mut screen = Screen::new(size, options.scrollback_lines);
    screen.set_screen_checksum(options.allow_screen_checksum);
    screen.set_window_ops(options.allow_window_ops);
    screen.set_title_report(options.allow_title_report);
    // Before the program's first byte: a terminal nobody has drawn yet is queried too.
    if let Some(colors) = options.colors {
        screen.set_default_colors(colors);
    }
    // Replay the previous session's text (dimmed) before the shell starts.
    if let Some(text) = options.restore {
        restore::replay(&mut screen, text);
    }
    let screen = Arc::new(FairLock::new(screen));
    let feed = OwnFeed::new(proxy.clone(), screen.clone());
    let started = pty::start(launch, size.window_size(), screen.clone(), feed, proxy)
        .with_context(|| format!("could not start {}", launch.shell.program))?;
    shared.set_writer(started.control.writer());
    Ok(Spawned { screen, io: started.control, pid: started.pid })
}
