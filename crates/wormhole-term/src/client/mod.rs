//! The app's side of a hosted terminal (`../repo-docs/docs/session-host.md`, sections 6, 7, 10
//! and 11): starting a session host (`wormhole-host`), and the connection that keeps a [`Pty`]'s
//! screen a replica of the host's, so everything that reads a terminal (snapshots, selection,
//! find, the minimap, text, links, blocks) reads a hosted one unchanged.
//!
//! - `launch`: the host process: `posix_spawn` in a session of its own with a near-empty
//!   environment and the readiness pipe on descriptor 3, and its reaping;
//! - `socket`: connecting (the peer must be this user), the handshake and the spawn, before the
//!   connection's thread takes over;
//! - `link`: what the `Pty` tells that thread, and the handle it keeps;
//! - `thread`: the thread, one per terminal, which does all of the socket's I/O: the stream into
//!   the replica (through the same `Sink`, `FairLock` and engine feed a local terminal's PTY
//!   thread uses), credit, input under its window, resizes, pings, attaching again;
//! - `events`: the host's events as the `PtyEvent`s and shared state a local terminal has;
//! - `replica`: what the replica is made with, and sizes and colors in the wire's terms;
//! - `open`: the constructors, [`Pty::start_hosted`] and [`Pty::attach_hosted`], and what only a
//!   hosted terminal does ([`Pty::detach`], [`Pty::set_visible`], [`Pty::host_lost`]).
//!
//! **The rule: the host decides, the replica displays.** The replica's replies are dropped after
//! every parse and its events too, but its pictures; titles, marks, the exit and the rest come as
//! the host's `Event`s. A resize goes to the host, and the replica takes it when the host's
//! `ResizeApplied` arrives in the stream, never before, so the bytes before it reflow the same on
//! both sides.
//!
//! The constructors block (the host's start, the handshake: at most a few seconds); call them off
//! the UI thread. After that nothing a `Pty` method does waits on the socket.

mod events;
mod launch;
mod link;
mod open;
mod replica;
mod socket;
#[cfg(test)]
mod tests;
mod thread;

use std::path::Path;
use std::time::Duration;

pub(crate) use link::Hosted;
pub use open::HostCommand;

/// The byte window a client gives the host: stream bytes past the last `Ack` it may send.
pub const WINDOW: u32 = 1024 * 1024;
/// A replica acknowledges at least this often, in stream bytes (and after every batch it parsed).
pub const ACK_EVERY: u64 = 64 * 1024;
/// The most input bytes sent and not yet acknowledged by the host's `InputAck`.
pub const INPUT_WINDOW: u64 = 256 * 1024;
/// A connection with nothing received for this long is asked whether the host is there.
pub const PING_IDLE: Duration = Duration::from_secs(2);
/// How long a new host has to listen, and then to say whether its program runs.
pub const START_TIMEOUT: Duration = Duration::from_secs(5);
pub const READY_TIMEOUT: Duration = Duration::from_secs(2);

/// The socket of the host with `id` in `hosts_dir`, as its meta file says (the host decides where
/// it listens; nobody else applies the rule).
pub fn socket_of(hosts_dir: &Path, id: &str) -> anyhow::Result<std::path::PathBuf> {
    let meta = crate::hosted::meta::read(&crate::hosted::paths::meta_path(hosts_dir, id))?;
    anyhow::ensure!(meta.id == id, "the meta file of {id} names another host");
    Ok(meta.socket)
}
