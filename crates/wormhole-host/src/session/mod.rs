//! The session: everything a host decides, as a state machine with no I/O and no clock of its own.
//!
//! The driver (`serve`, or a test) hands it what happened: a connection came ([`Session::accept`]),
//! bytes arrived ([`Session::received`]), a connection ended ([`Session::closed`]), a signal asked
//! the host to end ([`Session::terminate`]); then [`Session::pump`] moves the terminal's stream to
//! the connections and says when it wants to be called again. What it has for a connection is in
//! [`Session::output`]; the driver writes what the socket takes and reports it with
//! [`Session::wrote`]. Every call takes the time, so a test drives it on a virtual clock.
//!
//! - `handle`: the messages, by phase and role;
//! - `flow`: the stream to one attachment: repaint parts, output under the credit window, the
//!   cut-loose rule, counters;
//! - `conn`: one connection's state.

mod conn;
mod flow;
mod handle;
#[cfg(test)]
mod tests;

use std::time::{Duration, Instant};

use wormhole_term::hosted::{Cursor, Wake};
use wormhole_wire::{self as wire, AppVersion, DetachReason, Features, HostState, Message};

use conn::Conn;
pub use conn::ConnId;

use crate::program::{Program, Spawner};

/// The host's rules, with the design's numbers as defaults (`session-host.md`, sections 6, 7, 9).
#[derive(Clone, Debug)]
pub struct Config {
    /// The 12-character id.
    pub id: String,
    pub host_version: AppVersion,
    /// Milliseconds since the Unix epoch.
    pub started_at_ms: u64,
    pub max_connections: usize,
    /// A connection must say `Hello` within this.
    pub hello_timeout: Duration,
    /// A host nobody asked to `Spawn` within this ends.
    pub spawn_timeout: Duration,
    /// The window of an `Attach` that names none, and the bounds of one that does.
    pub default_window: u32,
    pub min_window: u32,
    pub max_window: u32,
    /// A client that could not be sent its stream for this long is cut loose.
    pub stall_limit: Duration,
    /// Resize requests are applied at most this often; the latest wins.
    pub resize_every: Duration,
    /// `Counters` go out at most this often.
    pub counters_every: Duration,
    /// After the program exited, and nobody is connected, the host stays this long.
    pub ttl_after_exit: Duration,
    /// The grace a SIGTERM gives the program (as closing a local terminal does).
    pub terminate_grace: Duration,
    /// How long, past the grace, the host waits for the hangup to finish before it exits anyway.
    pub end_margin: Duration,
}

impl Config {
    pub fn new(id: String, host_version: AppVersion, started_at_ms: u64) -> Self {
        Self {
            id,
            host_version,
            started_at_ms,
            max_connections: 4,
            hello_timeout: Duration::from_secs(2),
            spawn_timeout: Duration::from_secs(10),
            default_window: 1024 * 1024,
            min_window: 64 * 1024,
            max_window: 8 * 1024 * 1024,
            stall_limit: Duration::from_millis(250),
            resize_every: Duration::from_millis(8),
            counters_every: Duration::from_millis(16),
            ttl_after_exit: Duration::from_secs(7 * 24 * 60 * 60),
            terminate_grace: Duration::from_secs(3),
            end_margin: Duration::from_secs(15),
        }
    }
}

/// What the driver must act on besides bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Notice {
    /// The program runs: say `ok <pid>` on the readiness pipe and put it in the meta file.
    Spawned { pid: u32 },
    /// It could not be started, or nobody asked in time: say `err <text>`; the host ends.
    SpawnFailed(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Life {
    /// No program yet; the host ends at `deadline` without one.
    Unspawned {
        deadline: Instant,
    },
    Running,
    /// Asked to end: the hangup runs, and the host goes when it is over or at `deadline`.
    Ending {
        deadline: Instant,
    },
    /// Over: the driver flushes what it can, removes the files and exits.
    Done,
}

pub struct Session<S: Spawner> {
    config: Config,
    spawner: S,
    program: Option<S::Program>,
    /// The cursor from the stream's first byte, kept for the connection that spawned until it
    /// attaches (`Raw` takes it) or goes.
    spawn_cursor: Option<(ConnId, Cursor)>,
    conns: Vec<Conn>,
    next_id: u64,
    controller: Option<ConnId>,
    wake: Wake,
    life: Life,
    exited_at: Option<Instant>,
    resize: Option<wire::Resize>,
    resized_at: Option<Instant>,
    notices: Vec<Notice>,
    /// Scratch for the stream's bytes on their way into a frame.
    scratch: Vec<u8>,
}

impl<S: Spawner> Session<S> {
    /// A session that waits for its `Spawn`. `wake` is how the terminal's threads wake the driver.
    pub fn new(config: Config, spawner: S, wake: Wake, now: Instant) -> Self {
        let deadline = now + config.spawn_timeout;
        Self {
            config,
            spawner,
            program: None,
            spawn_cursor: None,
            conns: Vec::new(),
            next_id: 1,
            controller: None,
            wake,
            life: Life::Unspawned { deadline },
            exited_at: None,
            resize: None,
            resized_at: None,
            notices: Vec::new(),
            scratch: Vec::new(),
        }
    }

    /// A new connection, which passed the peer check. `Err` holds what to write to it before
    /// closing it: the host has all the connections it allows, or is ending.
    pub fn accept(&mut self, now: Instant) -> Result<ConnId, Vec<u8>> {
        if self.conns.len() >= self.config.max_connections || matches!(self.life, Life::Done) {
            let mut refusal = Vec::new();
            let busy =
                wire::ErrorMsg { ref_type: 0, code: wire::error_code::BUSY, text: "too many connections" };
            let _ = wire::encode(&mut refusal, wire::CHANNEL_LOCAL, 0, &Message::Error(busy));
            return Err(refusal);
        }
        let id = ConnId(self.next_id);
        self.next_id += 1;
        self.conns.push(Conn::new(id, now + self.config.hello_timeout));
        tracing::info!(conn = id.0, open = self.conns.len(), "connection accepted");
        Ok(id)
    }

    /// Bytes from connection `id`.
    pub fn received(&mut self, id: ConnId, bytes: &[u8], now: Instant) {
        let Some(at) = self.index(id) else { return };
        self.conns[at].dec.push(bytes);
        self.handle_frames(id, now);
    }

    /// Connection `id` ended (the peer closed it, or the driver did): it is forgotten.
    pub fn closed(&mut self, id: ConnId) {
        let Some(at) = self.index(id) else { return };
        let conn = self.conns.remove(at);
        self.drop_spawn_cursor(id);
        if self.controller == Some(id) {
            self.controller = None;
            if let Some(program) = &self.program {
                program.set_controller(false);
            }
        }
        tracing::info!(conn = id.0, open = self.conns.len(), unwritten = conn.pending(), "connection closed");
    }

    /// What is queued for connection `id`.
    pub fn output(&self, id: ConnId) -> &[u8] {
        self.index(id).map_or(&[], |at| self.conns[at].output())
    }

    /// `n` bytes of [`output`](Self::output) were written to connection `id`.
    pub fn wrote(&mut self, id: ConnId, n: usize) {
        if let Some(at) = self.index(id) {
            self.conns[at].wrote(n);
        }
    }

    /// Whether the driver should close connection `id` now (after writing what it can).
    pub fn should_close(&self, id: ConnId) -> bool {
        self.index(id).is_none_or(|at| {
            let conn = &self.conns[at];
            conn.dead || (conn.closing && conn.pending() == 0)
        })
    }

    /// The connections, oldest first.
    pub fn connections(&self) -> impl Iterator<Item = ConnId> + '_ {
        self.conns.iter().map(|c| c.id)
    }

    /// SIGTERM: the same as a controller's `Close` with the default grace.
    pub fn terminate(&mut self, now: Instant) {
        tracing::info!("asked to end by a signal");
        self.end(self.config.terminate_grace, now);
    }

    /// The host is over: the driver flushes, removes its files and exits.
    pub fn is_done(&self) -> bool {
        self.life == Life::Done
    }

    /// What happened since the last call that the driver acts on.
    pub fn take_notices(&mut self) -> Vec<Notice> {
        std::mem::take(&mut self.notices)
    }

    /// The program, once spawned.
    pub fn program(&self) -> Option<&S::Program> {
        self.program.as_ref()
    }

    /// Do what is due: resizes, the program's end, the streams, the timeouts. Returns when to be
    /// called again at the latest (`None`: only when woken), and whether some connection has more
    /// to send right now (call again as soon as its socket takes more).
    pub fn pump(&mut self, now: Instant) -> Pump {
        let mut next = Deadline::default();
        self.pump_life(now, &mut next);
        self.pump_resize(now, &mut next);
        if let Some(program) = &self.program {
            next.at(program.clipboard_expire(now));
        }
        let mut busy = false;
        for at in 0..self.conns.len() {
            if let Phase::Hello { deadline } = self.conns[at].phase {
                if now >= deadline {
                    tracing::info!(conn = self.conns[at].id.0, "no Hello in time");
                    self.conns[at].dead = true;
                }
                next.at(Some(deadline));
                continue;
            }
            busy |= self.pump_conn(at, now, &mut next);
        }
        Pump { next: next.0, busy }
    }

    fn pump_life(&mut self, now: Instant, next: &mut Deadline) {
        match self.life {
            Life::Unspawned { deadline } => {
                if now >= deadline {
                    tracing::info!("no Spawn in time, ending");
                    self.notices.push(Notice::SpawnFailed("no Spawn arrived in time".into()));
                    self.finish();
                } else {
                    next.at(Some(deadline));
                }
            }
            Life::Running => {
                let exited = self.program.as_ref().is_some_and(|p| p.exit().is_some() || p.finished());
                if exited && self.exited_at.is_none() {
                    tracing::info!("the program exited; keeping its screen");
                    self.exited_at = Some(now);
                }
                if let Some(at) = self.exited_at {
                    let ttl = at + self.config.ttl_after_exit;
                    if now >= ttl && self.conns.is_empty() {
                        tracing::info!("exited long ago and nobody came, ending");
                        self.finish();
                    } else if self.conns.is_empty() {
                        next.at(Some(ttl));
                    }
                }
            }
            Life::Ending { deadline } => {
                if self.program.as_ref().is_none_or(|p| p.finished()) || now >= deadline {
                    self.finish();
                } else {
                    next.at(Some(deadline));
                }
            }
            Life::Done => {}
        }
    }

    fn pump_resize(&mut self, now: Instant, next: &mut Deadline) {
        let Some(resize) = self.resize else { return };
        let due = self.resized_at.map_or(now, |at| at + self.config.resize_every);
        if now < due {
            next.at(Some(due));
            return;
        }
        if let Some(program) = &mut self.program {
            program.resize(resize.req, resize.size);
            self.resized_at = Some(now);
        }
        self.resize = None;
    }

    /// End the session: hang the program up (unless it is gone) and go once that is over.
    fn end(&mut self, grace: Duration, now: Instant) {
        if matches!(self.life, Life::Ending { .. } | Life::Done) {
            return;
        }
        match &mut self.program {
            Some(program) if !program.finished() => {
                program.close(grace);
                self.life = Life::Ending { deadline: now + grace + self.config.end_margin };
            }
            _ => self.finish(),
        }
    }

    /// Over: every connection is told, and closed once that is written.
    fn finish(&mut self) {
        if self.life == Life::Done {
            return;
        }
        self.life = Life::Done;
        for conn in &mut self.conns {
            if conn.role().is_some() {
                conn.send(&Message::Detached(DetachReason::HostClosing));
            }
            conn.closing = true;
        }
    }

    fn index(&self, id: ConnId) -> Option<usize> {
        self.conns.iter().position(|c| c.id == id)
    }

    /// What `Welcome` and `Info` say about this host now.
    fn welcome(&self, proto: u16) -> wire::Welcome {
        let program = self.program.as_ref();
        let exit = program.and_then(|p| p.exit());
        let mut session_id = [b'0'; 12];
        for (slot, byte) in session_id.iter_mut().zip(self.config.id.bytes()) {
            *slot = byte;
        }
        wire::Welcome {
            proto,
            proto_minor: wire::version::PROTO_MINOR,
            state: if exit.is_some() { HostState::Exited } else { HostState::Running },
            host_version: self.config.host_version,
            pid: program.map_or(0, |p| p.pid()),
            session_id,
            features: Features::NONE,
            started_at_ms: self.config.started_at_ms,
            first_block_id: program.map_or(0, |p| p.first_block_id()),
            size: program.map_or(wire::Size { cols: 0, rows: 0, cell_w: 0, cell_h: 0 }, |p| p.size()),
            counters: program.map_or(EMPTY_COUNTERS, |p| p.counters()),
            exit,
            token: None,
        }
    }
}

use conn::Phase;

const EMPTY_COUNTERS: wire::Counters = wire::Counters {
    epoch: 0,
    history_lines: 0,
    trimmed_total: 0,
    screen_rows: 0,
    alt_screen: false,
    revision: 0,
};

/// What [`Session::pump`] says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pump {
    /// Call again by then at the latest; `None` when nothing is due but what wakes the driver.
    pub next: Option<Instant>,
    /// A connection has more to send now.
    pub busy: bool,
}

/// The earliest of several deadlines.
#[derive(Default)]
struct Deadline(Option<Instant>);

impl Deadline {
    fn at(&mut self, at: Option<Instant>) {
        if let Some(at) = at {
            self.0 = Some(self.0.map_or(at, |now| now.min(at)));
        }
    }
}
