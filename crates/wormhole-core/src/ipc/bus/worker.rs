//! The output worker: what terminals print, read off the PTY's path and told to the streams that
//! asked for it (`wh events --output`).
//!
//! Nothing here runs on a PTY reader thread, and nothing the UI does waits for it. The UI notices
//! output where it already hears it (a terminal's coalesced `Output` event) and only *pulses* the
//! worker: a note, under a short lock, that terminal `n` printed. The worker thread, which starts with
//! the first pulse and sleeps while there are none, flushes a terminal at most every
//! [`FLUSH_EVERY`], and no later than that after its first unflushed pulse, by taking a look at its
//! screen (`wormhole_term::ScreenReader::tail`: short holds of the screen's lock, in chunks) and
//! publishing what was printed since the last look as one `output` event, capped. A terminal nobody
//! watches any more has its place (a pin in its grid) given back.

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError, Weak};
use std::time::{Duration, Instant};

use wormhole_term::{ReadScratch, ScreenReader, Tail, TailLimits, WeakScreenReader};

use super::Bus;
use crate::ipc::control::event::{Body, Event, Topic};

/// A terminal is looked at at most this often.
pub(super) const FLUSH_EVERY: Duration = Duration::from_millis(100);
/// What one `output` event may carry.
pub const OUTPUT_LINES_MAX: usize = 500;
pub const OUTPUT_BYTES_MAX: usize = 64 * 1024;

const LIMITS: TailLimits = TailLimits { lines: OUTPUT_LINES_MAX, bytes: OUTPUT_BYTES_MAX };

/// A terminal that printed and is waiting to be looked at.
struct Pending {
    window: i64,
    reader: WeakScreenReader,
    /// When the first pulse since the last look came.
    first: Instant,
}

/// Where a watched terminal's output was last read to.
struct Track {
    window: i64,
    reader: WeakScreenReader,
    tail: Tail,
    last_look: Instant,
}

#[derive(Default)]
struct Work {
    pending: HashMap<i64, Pending>,
    /// By terminal. A track is taken out while it is being looked at, so no lock is held then.
    tracks: HashMap<i64, Track>,
    /// Nobody watches output any more: give every track back.
    reset: bool,
    closed: bool,
}

/// The shared half of the worker: the bus's pulses in, the thread's wake-ups out.
#[derive(Default)]
pub(super) struct Worker {
    work: Mutex<Work>,
    wake: Condvar,
    started: AtomicBool,
}

impl std::fmt::Debug for Worker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Worker")
            .field("started", &self.started.load(Ordering::Relaxed))
            .finish_non_exhaustive()
    }
}

impl Worker {
    fn lock(&self) -> MutexGuard<'_, Work> {
        self.work.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// Terminal `terminal` printed. `reader` makes the handle the first pulse leaves; later pulses
    /// before the look change nothing. Starts the thread on first use.
    pub(super) fn pulse(
        self: &Arc<Self>,
        bus: &Arc<Bus>,
        window: i64,
        terminal: i64,
        reader: impl FnOnce() -> Option<WeakScreenReader>,
    ) {
        {
            let mut work = self.lock();
            if work.closed {
                return;
            }
            if let std::collections::hash_map::Entry::Vacant(slot) = work.pending.entry(terminal) {
                let Some(reader) = reader() else { return };
                slot.insert(Pending { window, reader, first: Instant::now() });
                self.wake.notify_one();
            }
        }
        if !self.started.swap(true, Ordering::AcqRel) {
            let (worker, bus) = (self.clone(), Arc::downgrade(bus));
            let spawned = std::thread::Builder::new()
                .name("wormhole-output".into())
                .stack_size(256 * 1024)
                .spawn(move || run(&worker, &bus));
            if let Err(err) = spawned {
                tracing::warn!("couldn't start the output worker: {err}");
                self.started.store(false, Ordering::Release);
            }
        }
    }

    /// Start watching `terminal` from where its cursor is now, so the first output after a
    /// subscription is told. A terminal first heard of by a pulse is watched from the look after it.
    pub(super) fn track(&self, window: i64, terminal: i64, reader: &ScreenReader) {
        let mut tail = Tail::default();
        // The first look only places the pin: nothing printed before it is told.
        let _ = reader.tail(&mut tail, LIMITS, &mut ReadScratch::default());
        let track =
            Track { window, reader: reader.downgrade(), tail, last_look: Instant::now() - FLUSH_EVERY };
        if let Some(old) = self.lock().tracks.insert(terminal, track) {
            release(old);
        }
    }

    /// Nobody asks for output any more.
    pub(super) fn reset(&self) {
        let mut work = self.lock();
        work.pending.clear();
        work.reset = true;
        self.wake.notify_one();
    }

    /// The bus is gone.
    pub(super) fn close(&self) {
        let mut work = self.lock();
        work.closed = true;
        self.wake.notify_one();
    }

    /// Wait until a terminal is due for a look (or everything is to be given back), and take the due
    /// ones out of the pending set. `None`: the bus is gone.
    fn next_due(&self) -> Option<Round> {
        let mut work = self.lock();
        loop {
            if work.closed {
                return None;
            }
            let now = Instant::now();
            let mut round = Round { reset: std::mem::take(&mut work.reset), due: Vec::new() };
            let mut soonest: Option<Instant> = None;
            let Work { pending, tracks, .. } = &mut *work;
            pending.retain(|terminal, pending| {
                let due_at = tracks
                    .get(terminal)
                    .map_or(pending.first, |track| pending.first.max(track.last_look + FLUSH_EVERY));
                if due_at <= now {
                    round.due.push((*terminal, pending.window, pending.reader.clone()));
                    false
                } else {
                    soonest = Some(soonest.map_or(due_at, |at| at.min(due_at)));
                    true
                }
            });
            if round.reset || !round.due.is_empty() {
                return Some(round);
            }
            work = match soonest {
                Some(at) => {
                    self.wake
                        .wait_timeout(work, at.saturating_duration_since(now))
                        .unwrap_or_else(PoisonError::into_inner)
                        .0
                }
                None => self.wake.wait(work).unwrap_or_else(PoisonError::into_inner),
            };
        }
    }
}

/// What the worker does after a wake-up.
struct Round {
    reset: bool,
    due: Vec<(i64, i64, WeakScreenReader)>,
}

/// Give a track's pin back to its grid, if the terminal is still there.
fn release(mut track: Track) {
    if let Some(reader) = track.reader.upgrade() {
        reader.release(&mut track.tail);
    }
}

/// The thread: look at each due terminal and tell what it printed.
fn run(worker: &Worker, bus: &Weak<Bus>) {
    let mut scratch = ReadScratch::default();
    while let Some(round) = worker.next_due() {
        let Some(bus) = bus.upgrade() else { return };
        if round.reset {
            let tracks: Vec<Track> = worker.lock().tracks.drain().map(|(_, track)| track).collect();
            tracks.into_iter().for_each(release);
        }
        for (terminal, window, weak) in round.due {
            let track = worker.lock().tracks.remove(&terminal);
            let Some(reader) = weak.upgrade() else { continue };
            if !bus.output_wanted(window, terminal) {
                track.into_iter().for_each(release);
                continue;
            }
            let mut track = track.unwrap_or_else(|| Track {
                window,
                reader: weak.clone(),
                tail: Tail::default(),
                last_look: Instant::now() - FLUSH_EVERY,
            });
            let read = reader.tail(&mut track.tail, LIMITS, &mut scratch);
            track.last_look = Instant::now();
            track.window = window;
            if !read.is_empty() {
                bus.publish(Topic::Output, || Event {
                    window: Some(window),
                    terminal: Some(terminal),
                    body: Body::Output {
                        screen: if read.alt_screen { "alt" } else { "main" }.into(),
                        lines: read.lines,
                        partial: read.partial,
                        skipped: read.skipped as u64,
                        lost: read.lost,
                    },
                });
            }
            let mut work = worker.lock();
            if !work.reset && !work.closed {
                work.tracks.insert(terminal, track);
            } else {
                drop(work);
                release(track);
            }
        }
        // A terminal that closed has nothing to give back and nothing to be looked at.
        worker.lock().tracks.retain(|_, track| track.reader.upgrade().is_some());
    }
}
