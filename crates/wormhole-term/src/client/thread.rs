//! A hosted terminal's connection thread: one per terminal, and the only thread that reads or
//! writes its socket. It waits in a poller for the socket and for the `Pty`'s commands (`link`),
//! like a local terminal's PTY thread waits for the master and its inbox.
//!
//! **The stream.** Each `Attach` starts a new one (a new [`StreamOrder`]). `Raw` (the spawning
//! connection) is `Out` from offset 0; `Sync` is `ResizeApplied` (the size the replica takes),
//! the repaint's seeds, the repaint in parts, and then `Out` from the repaint's offset; on its
//! first part the replica is replaced by a fresh engine of that size, seeded, which the repaint's
//! bytes then fill. Frames still on their way from the stream before are dropped until the new
//! one's first frame. `None` (headless: a terminal not on screen) feeds no replica: events, sizes
//! and counters only, and a changed revision tells the app something printed.
//!
//! **Credit.** `Ack` of the stream offset parsed, after every batch and at least every
//! [`ACK_EVERY`]. **Input** goes in frames of at most 64 KiB with at most [`INPUT_WINDOW`] not yet
//! acknowledged. **Pings** when nothing came for [`PING_IDLE`]. `Detached{TooSlow}` or a stream out
//! of order means attach again with a repaint.
//!
//! **The end.** The program's `Exited` is reported as a local terminal's exit; a connection that
//! ends without one, while nobody asked for the end, is a host that died: the exit is reported
//! without a code and `Link::lost` is set first, for [`Pty::host_lost`](crate::Pty::host_lost).

use std::borrow::Cow;
use std::collections::VecDeque;
use std::io::{self, Write as _};
use std::num::NonZeroUsize;
use std::sync::Arc;
use std::time::{Duration, Instant};

use polling::{Event, Events, PollMode, Poller};
use wormhole_vt::repaint::Seeds;
use wormhole_wire::{
    self as wire, Attach, CHANNEL_LOCAL, Decoder, DetachReason, Message, StreamMode, StreamOrder,
};

use super::events::{self, Heard};
use super::link::{Cmd, Link};
use super::replica::{ReplicaOptions, grid_of, wire_colors, wire_scheme, wire_size};
use super::{ACK_EVERY, INPUT_WINDOW, PING_IDLE, WINDOW};
use crate::GridSize;
use crate::engine::OwnFeed;
use crate::events::EventProxy;
use crate::hosted::ATTACH_HISTORY_LINES;
use crate::pty::{Flow, Sink};
use crate::screen::Screen;
use crate::sys::socket::Stream;

/// The socket's key in the poller.
const SOCKET: usize = 1;
/// How long a closed terminal waits for its host to finish after `Close`, past the grace.
const CLOSE_PATIENCE: Duration = Duration::from_secs(20);
/// The most the thread reads in one turn before it writes again: its acks open the host's window
/// while a flood goes on, instead of after it.
const READ_TURN: usize = 256 * 1024;

/// A `Sync` attach whose repaint has not begun: what its first frames said.
#[derive(Clone, Copy, Debug, Default)]
struct Awaiting {
    resize: Option<wire::ResizeApplied>,
    seeds: Option<Seeds>,
}

/// Why the loop ended.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum End {
    /// The host closed the connection (or it broke).
    Eof,
    /// We let go: detached, or a closed terminal's host took too long.
    Done,
}

/// The thread's state. The socket's decoder is beside it in [`Client`], so a frame (which borrows
/// the decoder) can be handled with the rest of the state.
pub(crate) struct State {
    link: Arc<Link>,
    proxy: EventProxy,
    /// The size the host last applied, which the replica has.
    size: GridSize,
    replica: Sink<Screen, OwnFeed>,
    options: ReplicaOptions,
    /// Frames to write, and how much of them is written.
    out: Vec<u8>,
    written: usize,
    /// The stream asked for, and what the app wants (on screen or not).
    mode: StreamMode,
    visible: bool,
    order: StreamOrder,
    awaiting: Option<Awaiting>,
    in_repaint: bool,
    /// Stream offset parsed, and acknowledged.
    consumed: u64,
    acked: u64,
    /// Input waiting to go out, how much of its first chunk went, bytes not yet acknowledged, and
    /// the host's count the last `InputAck` gave.
    input: VecDeque<Cow<'static, [u8]>>,
    input_at: usize,
    in_flight: u64,
    input_acked: u64,
    resize: Option<GridSize>,
    next_req: u32,
    revision: Option<u64>,
    exited: bool,
    /// Who ended it, if anyone did: `Close` (until when to wait), `Detach`, the host.
    closing: Option<Instant>,
    detaching: bool,
    host_closing: bool,
    taken_over: bool,
    /// When something last came (or a ping went), and the next ping's number.
    active: Instant,
    ping: u64,
    cmds: Vec<Cmd>,
}

impl State {
    pub(crate) fn new(
        link: Arc<Link>,
        proxy: EventProxy,
        replica: Sink<Screen, OwnFeed>,
        size: GridSize,
        options: ReplicaOptions,
        first: StreamMode,
    ) -> Self {
        let mut state = Self {
            link,
            proxy,
            size,
            replica,
            options,
            out: Vec::new(),
            written: 0,
            mode: StreamMode::None,
            visible: first != StreamMode::None,
            order: StreamOrder::new(),
            awaiting: None,
            in_repaint: false,
            consumed: 0,
            acked: 0,
            input: VecDeque::new(),
            input_at: 0,
            in_flight: 0,
            input_acked: 0,
            resize: None,
            next_req: 1,
            revision: None,
            exited: false,
            closing: None,
            detaching: false,
            host_closing: false,
            taken_over: false,
            active: Instant::now(),
            ping: 1,
            cmds: Vec::new(),
        };
        state.attach(first);
        state
    }

    fn queue(&mut self, msg: &Message<'_>) {
        if let Err(err) = wire::encode(&mut self.out, CHANNEL_LOCAL, 0, msg) {
            tracing::warn!(ty = msg.ty(), "a frame for the session host could not be encoded: {err}");
        }
    }

    /// Start a new stream of `mode`.
    fn attach(&mut self, mode: StreamMode) {
        self.mode = mode;
        self.order = StreamOrder::new();
        self.in_repaint = false;
        self.awaiting = (mode == StreamMode::Sync).then(Awaiting::default);
        let history_lines = ATTACH_HISTORY_LINES as u32;
        self.queue(&Message::Attach(Attach {
            stream: mode,
            history_lines,
            window: WINDOW,
            resume_from: None,
        }));
        tracing::debug!(mode = mode.to_wire(), "attaching to the session host");
    }

    /// The stream is lost (out of order, cut loose): attach again, with a repaint when on screen.
    fn resync(&mut self, why: &str) {
        tracing::info!(consumed = self.consumed, "attaching again: {why}");
        self.attach(if self.visible { StreamMode::Sync } else { StreamMode::None });
    }

    // ---- the Pty's commands ----

    fn apply_commands(&mut self) {
        if !self.link.has_messages() {
            return;
        }
        let mut cmds = std::mem::take(&mut self.cmds);
        self.link.take(&mut cmds);
        for cmd in cmds.drain(..) {
            match cmd {
                Cmd::Input(bytes) if !self.exited && self.closing.is_none() => self.input.push_back(bytes),
                Cmd::Input(_) => {}
                Cmd::Resize(size) => self.resize = Some(size),
                Cmd::Colors(palette) => self.queue(&Message::SetColors(wire_colors(&palette))),
                Cmd::ColorScheme(scheme) => {
                    if let Some(scheme) = wire_scheme(scheme) {
                        self.queue(&Message::SetColorScheme(scheme));
                    }
                }
                Cmd::Clipboard { ticket, data } => {
                    self.queue(&Message::ClipboardReply(wire::ClipboardReply { ticket, data: &data }))
                }
                Cmd::Visible(visible) if visible != self.visible => {
                    self.visible = visible;
                    self.attach(if visible { StreamMode::Sync } else { StreamMode::None });
                }
                Cmd::Visible(_) => {}
                Cmd::Detach => {
                    self.queue(&Message::Detach);
                    self.detaching = true;
                }
                Cmd::Close(grace) => {
                    let grace_ms = grace.as_millis().min(u128::from(u32::MAX)) as u32;
                    self.queue(&Message::Close { grace_ms });
                    self.closing = Some(Instant::now() + grace + CLOSE_PATIENCE);
                    self.input.clear();
                }
            }
        }
        self.cmds = cmds;
    }

    /// Queue what may go out now: the latest size asked for, and input within its window.
    fn queue_requests(&mut self) {
        if let Some(size) = self.resize.take() {
            let req = self.next_req;
            self.next_req = self.next_req.wrapping_add(1).max(1);
            self.queue(&Message::Resize(wire::Resize { req, size: wire_size(size) }));
        }
        while self.in_flight < INPUT_WINDOW {
            let Some(front) = self.input.front() else { break };
            let rest = &front[self.input_at..];
            let room = (INPUT_WINDOW - self.in_flight) as usize;
            let n = rest.len().min(wire::MAX_DATA).min(room);
            if n > 0 {
                let frame = Message::Input(&rest[..n]);
                if let Err(err) = wire::encode(&mut self.out, CHANNEL_LOCAL, 0, &frame) {
                    tracing::warn!("input for the session host could not be encoded: {err}");
                }
            }
            self.in_flight += n as u64;
            self.input_at += n;
            if self.input_at >= front.len() {
                self.input.pop_front();
                self.input_at = 0;
            }
        }
    }

    // ---- what the host sends ----

    pub(crate) fn handle(&mut self, msg: Message<'_>) {
        match msg {
            Message::Out(out) => self.out_frame(out),
            Message::Repaint(part) => self.repaint(part),
            Message::ResizeApplied(applied) => self.resized(applied),
            Message::Event(event) => self.event(event.kind),
            Message::Backlog(list) => {
                for event in list.iter() {
                    self.event(event.kind);
                }
            }
            Message::Counters(counters) => {
                // Headless: nothing is parsed, so this is how the app hears the program printed.
                if self.mode == StreamMode::None && self.revision != Some(counters.revision) {
                    if self.revision.is_some() {
                        self.proxy.wakeup();
                    }
                    self.revision = Some(counters.revision);
                }
            }
            Message::InputAck(total) => {
                // Cumulative over every client the host ever had: only the growth is ours.
                let grew = total.saturating_sub(self.input_acked);
                self.input_acked = self.input_acked.max(total);
                self.in_flight = self.in_flight.saturating_sub(grew);
            }
            Message::Detached(reason) => self.detached(reason),
            Message::Error(err) => {
                tracing::info!(ref_type = err.ref_type, code = err.code, "the session host refused a request")
            }
            _ => {}
        }
    }

    fn out_frame(&mut self, out: wire::Out<'_>) {
        // The bytes of a stream before the attach that is under way.
        if self.mode == StreamMode::None || self.awaiting.is_some() {
            return;
        }
        if let Err(err) = self.order.out(&out) {
            return self.resync(&format!("output out of order: {err:?}"));
        }
        self.replica.parse(out.bytes, || Flow::Continue);
        self.consumed = out.end();
        if self.consumed - self.acked >= ACK_EVERY {
            self.ack();
        }
    }

    fn repaint(&mut self, part: wire::Repaint<'_>) {
        if part.first {
            let Some(awaiting) = self.awaiting.take() else { return };
            // The new stream begins here: what came before it was the old one's.
            self.order = StreamOrder::new();
            let size = awaiting.resize.map_or(self.size, |a| grid_of(a.size));
            if let Some(applied) = awaiting.resize {
                let _ = self.order.resize_applied(&applied);
                self.size = size;
            }
            let seeds = awaiting.seeds.unwrap_or_default();
            let options = self.options;
            self.replica.update(|screen, feed| {
                let colors = *screen.term.default_colors();
                let mut fresh = options.screen(size);
                fresh.set_default_colors(&colors);
                fresh.term.seed(&seeds);
                *screen = fresh;
                feed.reset();
            });
            self.in_repaint = true;
        } else if !self.in_repaint {
            return;
        }
        if let Err(err) = self.order.repaint(&part) {
            return self.resync(&format!("a repaint out of order: {err:?}"));
        }
        self.replica.parse(part.bytes, || Flow::Continue);
        if part.last {
            self.in_repaint = false;
            // The host counts the window from the repaint's offset.
            self.consumed = part.offset;
            self.acked = part.offset;
        }
    }

    fn resized(&mut self, applied: wire::ResizeApplied) {
        if let Some(awaiting) = &mut self.awaiting {
            awaiting.resize = Some(applied);
            return;
        }
        let size = grid_of(applied.size);
        self.size = size;
        // A headless replica is not kept up to date, so it is not resized either.
        if self.mode == StreamMode::None {
            return;
        }
        if let Err(err) = self.order.resize_applied(&applied) {
            return self.resync(&format!("a resize out of order: {err:?}"));
        }
        // The host answers the program, an in-band size report included: the replica's go nowhere.
        self.replica.update(|screen, _| drop(screen.resize(size)));
    }

    fn event(&mut self, kind: wire::EventKind<'_>) {
        match events::deliver(&self.proxy, &self.link, kind) {
            Heard::Done => {}
            Heard::Seeds(seeds) => {
                if let Some(awaiting) = &mut self.awaiting {
                    awaiting.seeds = Some(seeds);
                }
            }
            Heard::Exited => {
                self.exited = true;
                self.input.clear();
            }
        }
    }

    fn detached(&mut self, reason: DetachReason) {
        tracing::info!(reason = reason.to_wire(), "the session host detached this connection");
        match reason {
            DetachReason::TooSlow if self.mode != StreamMode::None => self.resync("too slow for the stream"),
            DetachReason::HostClosing => self.host_closing = true,
            DetachReason::TakenOver => self.taken_over = true,
            _ => {}
        }
    }

    /// Acknowledge what was parsed.
    fn ack(&mut self) {
        if self.mode != StreamMode::None && self.consumed > self.acked {
            self.acked = self.consumed;
            let consumed = self.consumed;
            self.queue(&Message::Ack(consumed));
        }
    }

    /// The connection ended: report the end the app did not hear yet.
    fn ended(&mut self) {
        if self.detaching {
            return;
        }
        let expected = self.exited || self.closing.is_some() || self.host_closing || self.taken_over;
        if !expected {
            tracing::warn!("the session host's connection ended without the program's exit");
            // Before the exit is reported: the app reads it when it hears that.
            self.link.set_lost();
        }
        // Nothing runs after this to give up on an update the host never ended, whether or not the
        // exit was heard first: the connection is the only thing that could have ended it.
        self.replica.release_sync();
        if !self.exited {
            self.exited = true;
            self.proxy.exited(None, None);
        }
    }

    /// Whether a quiet connection is pinged: one on screen, or with input waiting for the host,
    /// is where a stuck host would show. Hidden ones stay quiet, so twenty terminals in the sidebar
    /// do not wake the app and their hosts ten times a second.
    fn pings(&self) -> bool {
        !self.detaching && (self.visible || self.in_flight > 0)
    }

    /// When the loop must wake at the latest.
    fn deadline(&self) -> Option<Instant> {
        let ping = self.pings().then_some(self.active + PING_IDLE);
        [ping, self.replica.deadline(), self.closing].into_iter().flatten().min()
    }

    fn tick(&mut self, now: Instant) {
        self.replica.expire_sync();
        if self.pings() && now >= self.active + PING_IDLE {
            let n = self.ping;
            self.ping += 1;
            self.active = now;
            self.queue(&Message::Ping(n));
        }
    }
}

/// The thread: the socket, its poller and decoder, and the state.
pub(crate) struct Client {
    stream: Stream,
    poller: Arc<Poller>,
    dec: Decoder,
    /// The socket is polled for room to write too (only while frames wait).
    writable: bool,
    /// Writing failed: the host is going; what it still sends is read until the end.
    broken: bool,
    pub(crate) state: State,
}

impl Client {
    pub(crate) fn new(stream: Stream, dec: Decoder, poller: Arc<Poller>, state: State) -> io::Result<Self> {
        stream.set_nonblocking(true)?;
        stream.set_read_timeout(None)?;
        let client = Self { stream, poller, dec, writable: false, broken: false, state };
        client.register()?;
        Ok(client)
    }

    #[allow(unsafe_code)] // polling's registration contract, kept by `Drop`.
    fn register(&self) -> io::Result<()> {
        // SAFETY: the socket is deleted from the poller before it is closed: in the drop of this
        // struct, which owns both.
        unsafe { self.poller.add_with_mode(&self.stream, Event::readable(SOCKET), PollMode::Level) }
    }

    fn want_writable(&mut self, want: bool) {
        if want == self.writable {
            return;
        }
        let event = if want { Event::all(SOCKET) } else { Event::readable(SOCKET) };
        match self.poller.modify_with_mode(&self.stream, event, PollMode::Level) {
            Ok(()) => self.writable = want,
            Err(err) => tracing::debug!("cannot change what the host's socket is polled for: {err}"),
        }
    }

    /// Run until the connection is over.
    pub(crate) fn run(mut self) {
        let mut events = Events::with_capacity(NonZeroUsize::MIN.saturating_add(1));
        let end = loop {
            self.state.apply_commands();
            self.state.queue_requests();
            self.flush();
            if self.state.detaching && self.state.out.is_empty() {
                break End::Done;
            }
            if self.state.closing.is_some_and(|until| Instant::now() >= until) {
                tracing::warn!("the session host did not end in time after Close");
                break End::Done;
            }
            let timeout = self.state.deadline().map(|at| at.saturating_duration_since(Instant::now()));
            events.clear();
            if let Err(err) = self.poller.wait(&mut events, timeout)
                && err.kind() != io::ErrorKind::Interrupted
            {
                tracing::warn!("waiting for the session host failed: {err}");
                break End::Eof;
            }
            if self.read() == Some(End::Eof) {
                break End::Eof;
            }
            self.state.tick(Instant::now());
        };
        if end == End::Eof || self.state.closing.is_some() {
            self.state.ended();
        }
        self.state.link.end();
    }

    /// Read and handle what the socket has; `Some(Eof)` when the connection is over.
    fn read(&mut self) -> Option<End> {
        let mut read = 0;
        loop {
            loop {
                match self.dec.next_frame() {
                    Ok(Some(frame)) => {
                        self.state.active = Instant::now();
                        self.state.handle(frame.message);
                    }
                    Ok(None) => break,
                    Err(err) if err.is_fatal() => {
                        tracing::warn!("the session host's stream is broken: {err}");
                        return Some(End::Eof);
                    }
                    Err(err) => tracing::debug!("a frame from the session host was dropped: {err}"),
                }
            }
            // A batch is parsed: tell the host, so its window opens.
            self.state.ack();
            // Input waiting goes out between batches, and acks within a flood.
            if self.state.link.has_messages() || read >= READ_TURN {
                return None;
            }
            match self.dec.fill(&mut &self.stream) {
                Ok(0) => return Some(End::Eof),
                Ok(n) => read += n,
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => return None,
                Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
                Err(err) => {
                    tracing::info!("reading from the session host failed: {err}");
                    return Some(End::Eof);
                }
            }
        }
    }

    /// Write what is queued, as far as the socket takes it.
    fn flush(&mut self) {
        let state = &mut self.state;
        if self.broken {
            state.out.clear();
            state.written = 0;
        }
        while state.written < state.out.len() {
            match (&self.stream).write(&state.out[state.written..]) {
                Ok(0) => {
                    self.broken = true;
                    break;
                }
                Ok(n) => state.written += n,
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => break,
                Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
                Err(err) => {
                    tracing::info!("writing to the session host failed: {err}");
                    self.broken = true;
                    break;
                }
            }
        }
        if self.broken || state.written >= state.out.len() {
            state.out.clear();
            state.written = 0;
        }
        let pending = !self.state.out.is_empty();
        self.want_writable(pending);
    }
}

impl Drop for Client {
    fn drop(&mut self) {
        let _ = self.poller.delete(&self.stream);
    }
}
