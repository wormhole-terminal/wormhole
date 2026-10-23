//! The driver: the socket, the connections and the clock around a [`Session`]. One thread, one
//! wait (`polling`, kqueue on macOS) for the listener, the connections and the wakes from the
//! terminal's threads; everything the host decides is the session's.
//!
//! Each turn: pump the session, write each connection's queued frames with one `write`, close the
//! connections that are done, then wait for readiness, a wake, or the session's next deadline.
//! When the session has more to send and the socket took everything, the next turn starts at
//! once. Detached and idle, the only timer is the hourly look at the socket file.

use std::io::{self, Read as _, Write as _};
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use polling::{AsRawSource, Event, Events, PollMode, Poller};
use wormhole_term::hosted::meta::{self, Meta};
use wormhole_term::sys::socket::{Stream, peer_user};
use wormhole_term::sys::user::UserId;

use crate::listen::{Listener, TOUCH_EVERY};
use crate::peer::Policy;
use crate::program::Spawner;
use crate::session::{ConnId, Notice, Session};
use crate::sys::ready::ReadyPipe;

/// The listener's key in the poller; a connection's is its id (from 1).
const LISTENER: usize = 0;
/// The most bytes read from one connection in a turn, so one cannot starve the others.
const READ_PER_TURN: usize = 1024 * 1024;
/// How long the host tries to get its last words (`Detached{HostClosing}`) out at the end.
const FLUSH_AT_END: Duration = Duration::from_millis(200);

/// What wakes the driver from another thread: the terminal's stream, its end, a signal.
pub(crate) struct Wakes {
    poller: Arc<Poller>,
    /// A wake is on its way: the next ones until the driver looks cost a load.
    pending: AtomicBool,
    terminate: AtomicBool,
}

impl Wakes {
    pub(crate) fn new(poller: Arc<Poller>) -> Arc<Self> {
        Arc::new(Self { poller, pending: AtomicBool::new(false), terminate: AtomicBool::new(false) })
    }

    pub(crate) fn wake(&self) {
        if !self.pending.swap(true, Ordering::AcqRel) {
            let _ = self.poller.notify();
        }
    }

    /// A signal asked the host to end.
    pub(crate) fn terminate(&self) {
        self.terminate.store(true, Ordering::Release);
        self.wake();
    }
}

/// The files the host owns besides the socket.
pub(crate) struct Files<'a> {
    pub hosts_dir: &'a Path,
    pub meta: Meta,
}

/// Serve until the session is done.
pub(crate) fn serve<S: Spawner>(
    session: &mut Session<S>,
    listener: &mut Listener,
    wakes: &Arc<Wakes>,
    ready: &mut ReadyPipe,
    files: &mut Files<'_>,
    peers: &Policy,
) -> io::Result<()> {
    let poller = wakes.poller.clone();
    watch(&poller, &listener.listener, LISTENER, false)?;
    let mut conns: Vec<(ConnId, Stream, bool)> = Vec::new();
    let mut events = Events::new();
    let mut buf = vec![0u8; 64 * 1024];
    let mut touch_at = Instant::now() + TOUCH_EVERY;
    loop {
        wakes.pending.store(false, Ordering::Release);
        let now = Instant::now();
        if wakes.terminate.swap(false, Ordering::AcqRel) {
            session.terminate(now);
        }
        let pump = session.pump(now);
        for notice in session.take_notices() {
            on_notice(notice, ready, files);
        }
        let wrote_all = write_all(session, &mut conns, &poller);
        if session.is_done() {
            flush_at_end(session, &mut conns);
            for (_, stream, _) in &conns {
                let _ = poller.delete(stream);
            }
            let _ = poller.delete(&listener.listener);
            return Ok(());
        }
        let mut deadline = pump.next.map_or(touch_at, |next| next.min(touch_at));
        if pump.busy && wrote_all {
            deadline = now;
        }
        events.clear();
        let timeout = deadline.saturating_duration_since(Instant::now());
        match poller.wait(&mut events, Some(timeout)) {
            Ok(_) => {}
            Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
            Err(err) => return Err(err),
        }
        let now = Instant::now();
        for event in events.iter() {
            if event.key == LISTENER {
                accept_all(session, listener, peers, &poller, &mut conns, now);
            } else if event.readable {
                let id = ConnId(event.key as u64);
                read_from(session, &mut conns, id, &mut buf, &poller, now);
            }
        }
        if now >= touch_at {
            touch_at = now + TOUCH_EVERY;
            if !listener.is_ours() {
                // About to be replaced: out of the poller first.
                let _ = poller.delete(&listener.listener);
            }
            match listener.refresh() {
                Ok(true) => {
                    watch(&poller, &listener.listener, LISTENER, false)?;
                    files.meta.socket = listener.path.clone();
                    let _ = meta::write(files.hosts_dir, &files.meta);
                }
                Ok(false) => {}
                Err(err) => {
                    tracing::warn!("cannot look after the socket file: {err}");
                    // Still accepting on the old one (registered again if it was taken out).
                    let _ = watch(&poller, &listener.listener, LISTENER, false);
                }
            }
        }
    }
}

fn on_notice(notice: Notice, ready: &mut ReadyPipe, files: &mut Files<'_>) {
    match notice {
        Notice::Spawned { pid } => {
            files.meta.child_pid = Some(pid);
            if let Err(err) = meta::write(files.hosts_dir, &files.meta) {
                tracing::warn!("cannot write the meta file: {err}");
            }
            ready.say(&format!("ok {pid}"));
        }
        Notice::SpawnFailed(text) => ready.say(&format!("err {text}")),
    }
}

/// Register `source` with the poller (readable, and writable when asked), level-triggered.
#[allow(unsafe_code)] // polling's registration contract.
fn watch(poller: &Poller, source: impl AsRawSource, key: usize, writable: bool) -> io::Result<()> {
    let interest = if writable { Event::all(key) } else { Event::readable(key) };
    // SAFETY: every registered source is deleted from the poller before it is dropped
    // (`forget`, and the listener lives as long as the loop).
    unsafe { poller.add_with_mode(source, interest, PollMode::Level) }
}

fn forget(
    poller: &Poller,
    conns: &mut Vec<(ConnId, Stream, bool)>,
    session: &mut Session<impl Spawner>,
    id: ConnId,
) {
    if let Some(at) = conns.iter().position(|(c, ..)| *c == id) {
        let (_, stream, _) = conns.remove(at);
        let _ = poller.delete(&stream);
    }
    session.closed(id);
}

fn accept_all<S: Spawner>(
    session: &mut Session<S>,
    listener: &Listener,
    peers: &Policy,
    poller: &Poller,
    conns: &mut Vec<(ConnId, Stream, bool)>,
    now: Instant,
) {
    loop {
        let stream = match listener.listener.accept() {
            Ok((stream, _)) => stream,
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => return,
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) => {
                tracing::warn!("accept failed: {err}");
                return;
            }
        };
        // Same user, always; then the signature, when the host is signed (see `peer`).
        match peer_user(&stream) {
            Ok(peer) if peer == UserId::current() => {}
            refused => {
                let peer = refused.map_or_else(|err| format!("unreadable ({err})"), |peer| peer.to_string());
                tracing::warn!(peer_uid = peer, "refused a connection from another user");
                continue;
            }
        }
        if !peers.allows(&stream) {
            continue;
        }
        if stream.set_nonblocking(true).is_err() {
            continue;
        }
        match session.accept(now) {
            Ok(id) => {
                if watch(poller, &stream, id.0 as usize, false).is_err() {
                    session.closed(id);
                    continue;
                }
                // The third field: whether the poller watches it for writability too.
                conns.push((id, stream, false));
            }
            Err(refusal) => {
                let mut stream = stream;
                let _ = stream.write(&refusal);
            }
        }
    }
}

fn read_from<S: Spawner>(
    session: &mut Session<S>,
    conns: &mut Vec<(ConnId, Stream, bool)>,
    id: ConnId,
    buf: &mut [u8],
    poller: &Poller,
    now: Instant,
) {
    let Some(at) = conns.iter().position(|(c, ..)| *c == id) else { return };
    let mut total = 0;
    while total < READ_PER_TURN {
        match conns[at].1.read(buf) {
            Ok(0) => return forget(poller, conns, session, id),
            Ok(n) => {
                total += n;
                session.received(id, &buf[..n], now);
            }
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => return,
            Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
            Err(_) => return forget(poller, conns, session, id),
        }
    }
}

/// Write what each connection has queued (one `write` each), close the ones that are done, and
/// watch for writability where something is left. Whether every queue was written out.
fn write_all<S: Spawner>(
    session: &mut Session<S>,
    conns: &mut Vec<(ConnId, Stream, bool)>,
    poller: &Poller,
) -> bool {
    let mut all = true;
    let ids: Vec<ConnId> = conns.iter().map(|(id, ..)| *id).collect();
    for id in ids {
        let Some(at) = conns.iter().position(|(c, ..)| *c == id) else { continue };
        let out = session.output(id);
        if !out.is_empty() {
            match conns[at].1.write(out) {
                Ok(n) => session.wrote(id, n),
                Err(err) if matches!(err.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::Interrupted) => {}
                Err(_) => {
                    forget(poller, conns, session, id);
                    continue;
                }
            }
        }
        if session.should_close(id) {
            forget(poller, conns, session, id);
            continue;
        }
        let left = !session.output(id).is_empty();
        all &= !left;
        // Only when it changes: a `kevent` per connection per turn would double the syscalls of a
        // flood, whose turns are otherwise one `write` each.
        let (_, stream, watching_writes) = &mut conns[at];
        if *watching_writes != left {
            *watching_writes = left;
            let interest = if left { Event::all(id.0 as usize) } else { Event::readable(id.0 as usize) };
            let _ = poller.modify_with_mode(&*stream, interest, PollMode::Level);
        }
    }
    all
}

/// The host is ending: give the connections a moment to take their last frames.
fn flush_at_end<S: Spawner>(session: &mut Session<S>, conns: &mut [(ConnId, Stream, bool)]) {
    let until = Instant::now() + FLUSH_AT_END;
    for (id, stream, _) in conns.iter_mut() {
        let _ = stream.set_nonblocking(false);
        let _ = stream.set_write_timeout(Some(
            until.saturating_duration_since(Instant::now()).max(Duration::from_millis(1)),
        ));
        let out = session.output(*id).to_vec();
        let _ = stream.write_all(&out);
    }
}
