//! What a hosted terminal's [`Pty`](crate::Pty) tells its connection's thread, and the handle it
//! keeps for that. One queue, so input, resizes and the end go out in the order they were asked
//! for; whoever sends wakes the thread only when the queue was empty, as the PTY's inbox does.

use std::borrow::Cow;
use std::sync::atomic::{AtomicBool, AtomicU8, AtomicU64, Ordering};
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
    /// Start the host's checkpoints with this key (none: the folder and the shell only) and shell key.
    CheckpointKey {
        key: Option<[u8; crate::checkpoint::seal::KEY_LEN]>,
        shell: String,
    },
    /// Write a checkpoint now.
    Checkpoint {
        flush: bool,
    },
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

/// What the stream is asked for, as the connection's thread last attached (`Attach.stream`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkMode {
    /// Events and counters only: the terminal is not on screen.
    Headless,
    /// The program's bytes as they come (the connection that started the host).
    Raw,
    /// Host-synthesized repaints, then the stream: on screen.
    Sync,
}

impl LinkMode {
    fn to_bits(self) -> u8 {
        match self {
            Self::Headless => 0,
            Self::Raw => 1,
            Self::Sync => 2,
        }
    }

    fn from_bits(bits: u8) -> Self {
        match bits {
            1 => Self::Raw,
            2 => Self::Sync,
            _ => Self::Headless,
        }
    }
}

/// How the connection to a host stands, for the Session Hosts window: a copy of counters the
/// thread keeps (relaxed atomics, one add per read or write, so counting costs nothing visible).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LinkStats {
    pub mode: LinkMode,
    /// Bytes read from, and written to, the socket since it connected.
    pub bytes_in: u64,
    pub bytes_out: u64,
    /// Frames decoded from the host, and frames queued for it.
    pub frames_in: u64,
    pub frames_out: u64,
    /// The app let go of the host (it keeps running).
    pub detached: bool,
    /// The connection ended without the program's exit: the host is gone.
    pub lost: bool,
    /// The connection's thread is over.
    pub ended: bool,
    /// How long ago anything last came from the host.
    pub quiet_for: Duration,
    /// How long a ping has gone unanswered, once it has for [`PATIENCE`](super::PATIENCE): the host
    /// is not responding (a stopped or stuck process keeps its socket and says nothing).
    pub not_responding_for: Option<Duration>,
}

/// The counters behind [`LinkStats`].
#[derive(Default)]
struct Traffic {
    bytes_in: AtomicU64,
    bytes_out: AtomicU64,
    frames_in: AtomicU64,
    frames_out: AtomicU64,
    mode: AtomicU8,
    /// Milliseconds after `Link::epoch` of the last frame from the host.
    heard_ms: AtomicU64,
    /// Milliseconds a ping has been unanswered past its patience, plus one; 0 when responsive.
    not_responding_ms: AtomicU64,
}

/// The queue into the thread, and what the thread says back.
pub(crate) struct Link {
    traffic: Traffic,
    /// What `Traffic::heard_ms` counts from.
    epoch: std::time::Instant,
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
            traffic: Traffic::default(),
            epoch: std::time::Instant::now(),
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

    pub(crate) fn count_read(&self, bytes: usize) {
        self.traffic.bytes_in.fetch_add(bytes as u64, Ordering::Relaxed);
    }

    pub(crate) fn count_written(&self, bytes: usize) {
        self.traffic.bytes_out.fetch_add(bytes as u64, Ordering::Relaxed);
    }

    pub(crate) fn count_frame_in(&self) {
        self.traffic.frames_in.fetch_add(1, Ordering::Relaxed);
    }

    pub(crate) fn count_frame_out(&self) {
        self.traffic.frames_out.fetch_add(1, Ordering::Relaxed);
    }

    /// What the thread knows of the host's liveness, for [`LinkStats`].
    pub(crate) fn set_liveness(&self, heard: std::time::Instant, state: super::Responsiveness) {
        let ms = |d: Duration| u64::try_from(d.as_millis()).unwrap_or(u64::MAX);
        self.traffic.heard_ms.store(ms(heard.saturating_duration_since(self.epoch)), Ordering::Relaxed);
        let stuck = match state {
            super::Responsiveness::Responsive => 0,
            super::Responsiveness::NotResponding(for_) => ms(for_).saturating_add(1),
        };
        self.traffic.not_responding_ms.store(stuck, Ordering::Relaxed);
    }

    pub(crate) fn set_mode(&self, mode: LinkMode) {
        self.traffic.mode.store(mode.to_bits(), Ordering::Relaxed);
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
    /// The protocol minor the host said in its `Welcome`: what it understands beyond the first minor.
    minor: u16,
    /// The features the host announced in its `Welcome`: the messages it understands from minor 4 on.
    features: wormhole_wire::Features,
}

impl Hosted {
    /// `minor` is the protocol minor the host said in its `Welcome`.
    pub(crate) fn new(link: Arc<Link>, id: String, minor: u16, features: wormhole_wire::Features) -> Self {
        Self { link, id, detached: AtomicBool::new(false), minor, features }
    }

    /// Whether the host keeps checkpoints when asked.
    pub(crate) fn can_checkpoint(&self) -> bool {
        self.features.contains(wormhole_wire::Features::CHECKPOINT)
    }

    pub(crate) fn checkpoint_key(&self, key: Option<&crate::checkpoint::Key>, shell: &str) {
        self.link.send(Cmd::CheckpointKey { key: key.map(|k| *k.as_bytes()), shell: shell.to_owned() });
    }

    pub(crate) fn checkpoint(&self, flush: bool) {
        self.link.send(Cmd::Checkpoint { flush });
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

    /// The host's protocol minor, as its `Welcome` said it.
    pub(crate) fn minor(&self) -> u16 {
        self.minor
    }

    pub(crate) fn is_detached(&self) -> bool {
        self.detached.load(Ordering::Acquire)
    }

    /// The connection's counters and state, now.
    pub(crate) fn stats(&self) -> LinkStats {
        let traffic = &self.link.traffic;
        LinkStats {
            mode: LinkMode::from_bits(traffic.mode.load(Ordering::Relaxed)),
            bytes_in: traffic.bytes_in.load(Ordering::Relaxed),
            bytes_out: traffic.bytes_out.load(Ordering::Relaxed),
            frames_in: traffic.frames_in.load(Ordering::Relaxed),
            frames_out: traffic.frames_out.load(Ordering::Relaxed),
            detached: self.is_detached(),
            lost: self.link.lost(),
            ended: self.link.ended.load(Ordering::Acquire),
            quiet_for: (self.link.epoch + Duration::from_millis(traffic.heard_ms.load(Ordering::Relaxed)))
                .elapsed(),
            not_responding_for: match traffic.not_responding_ms.load(Ordering::Relaxed) {
                0 => None,
                ms => Some(Duration::from_millis(ms - 1)),
            },
        }
    }

    /// End the terminal, unless it was detached.
    pub(crate) fn close(&self, grace: Duration) {
        if !self.is_detached() {
            self.link.send(Cmd::Close(grace));
        }
    }
}
