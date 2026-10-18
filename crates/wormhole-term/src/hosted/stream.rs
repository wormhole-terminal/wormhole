//! The output of the program on its way to the clients: one ring of the bytes the PTY thread
//! parsed (copied only while some client wants them), the resizes applied between slices, and the
//! events, all stamped with the stream offset they belong at; and a [`Cursor`] per client that
//! walks them in order.
//!
//! The PTY thread only ever appends, under one short lock, and never waits for a client. A
//! client that falls more than [`FORWARD_CAP`] bytes behind is not waited for either: its cursor
//! is marked overrun, and its connection cuts it loose (it comes back with a repaint). So the
//! ring is bounded by the cap, whatever the clients do.
//!
//! Lock order (the whole crate's): the screen's lock, then this one; a [`Wake`] takes neither.

use std::collections::VecDeque;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::time::{Duration, Instant};

use wormhole_wire as wire;

use super::event::HostEvent;

/// The most unsent output the ring holds for the slowest client before it is cut loose.
pub const FORWARD_CAP: usize = 4 * 1024 * 1024;
/// The most resizes and events kept for a client that has not taken them.
const MARKS_MAX: usize = 1024;
const EVENTS_MAX: usize = 8192;

/// How the stream wakes whoever serves a client: called on the PTY thread under the stream's lock,
/// so it must be quick and must not take the screen's or the stream's lock (setting a flag and
/// poking a poller is what it is for).
pub type Wake = Arc<dyn Fn() + Send + Sync>;

/// A resize the host applied between two slices.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResizeMark {
    pub offset: u64,
    pub epoch: u32,
    /// The requester's `Resize.req`; 0 when the host resized on its own.
    pub req: u32,
    pub size: wire::Size,
}

/// What a cursor has next.
#[derive(Debug, PartialEq, Eq)]
pub enum Next {
    /// Nothing yet.
    Idle,
    /// The client fell too far behind: stream no more to it.
    Overrun,
    /// Stream bytes `offset..` were appended to the buffer given to [`Cursor::next`].
    Bytes {
        offset: u64,
    },
    Resize(ResizeMark),
    Event {
        offset: u64,
        event: HostEvent,
    },
}

struct Slot {
    id: u64,
    /// Wants the bytes (an attached stream), not only the marks and events (headless).
    bytes: bool,
    /// The next byte to hand out.
    sent: u64,
    next_mark: u64,
    next_event: u64,
    overrun: bool,
    wake: Wake,
    /// With a gap, appends wake the client at most this often (a headless client reads counters).
    gap: Duration,
    woken_at: Option<Instant>,
}

#[derive(Default)]
struct Inner {
    ring: VecDeque<u8>,
    /// The offset of the ring's first byte; the stream is `base + ring.len()` bytes long.
    base: u64,
    marks: VecDeque<ResizeMark>,
    first_mark: u64,
    events: VecDeque<(u64, HostEvent)>,
    first_event: u64,
    slots: Vec<Slot>,
    next_id: u64,
}

impl Inner {
    fn total(&self) -> u64 {
        self.base + self.ring.len() as u64
    }

    /// Drop the bytes nobody needs any more, and mark the clients that needed dropped ones.
    fn trim_bytes(&mut self) {
        let total = self.total();
        let floor = total.saturating_sub(FORWARD_CAP as u64);
        let slowest = self.slots.iter().filter(|s| s.bytes && !s.overrun).map(|s| s.sent).min();
        let keep_from = slowest.map_or(total, |slowest| slowest.max(floor));
        for slot in self.slots.iter_mut().filter(|s| s.bytes && s.sent < keep_from) {
            slot.overrun = true;
        }
        if keep_from > self.base {
            self.ring.drain(..(keep_from - self.base) as usize);
            self.base = keep_from;
        }
    }

    /// Drop the marks and events every client has passed.
    fn trim_aux(&mut self) {
        let end_mark = self.first_mark + self.marks.len() as u64;
        let min_mark = self.slots.iter().map(|s| s.next_mark).min().unwrap_or(end_mark);
        while self.first_mark < min_mark && self.marks.pop_front().is_some() {
            self.first_mark += 1;
        }
        let end_event = self.first_event + self.events.len() as u64;
        let min_event = self.slots.iter().map(|s| s.next_event).min().unwrap_or(end_event);
        while self.first_event < min_event && self.events.pop_front().is_some() {
            self.first_event += 1;
        }
    }

    fn wake_bytes(&mut self) {
        let now = Instant::now();
        for slot in self.slots.iter_mut().filter(|s| s.bytes || !s.gap.is_zero()) {
            if slot.gap.is_zero() || slot.woken_at.is_none_or(|at| now.duration_since(at) >= slot.gap) {
                slot.woken_at = Some(now);
                (slot.wake)();
            }
        }
    }

    fn wake_all(&mut self) {
        for slot in &self.slots {
            (slot.wake)();
        }
    }
}

/// The ring, the marks and the events (see the module docs).
#[derive(Default)]
pub struct Stream {
    inner: Mutex<Inner>,
}

impl Stream {
    fn lock(&self) -> MutexGuard<'_, Inner> {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner)
    }

    /// How many bytes the program has written (and the host parsed) so far.
    pub fn total(&self) -> u64 {
        self.lock().total()
    }

    /// The PTY thread parsed `bytes`: they follow the ones before. Copied only if a client wants
    /// them.
    pub fn append(&self, bytes: &[u8]) {
        let mut inner = self.lock();
        if inner.slots.iter().any(|s| s.bytes) {
            inner.ring.extend(bytes);
            inner.trim_bytes();
        } else {
            inner.base += bytes.len() as u64;
        }
        inner.wake_bytes();
    }

    /// A resize was applied at `mark.offset` (the stream's end, under the screen's lock).
    pub fn push_mark(&self, mark: ResizeMark) {
        let mut inner = self.lock();
        if inner.slots.is_empty() {
            return;
        }
        if inner.marks.len() >= MARKS_MAX {
            inner.marks.pop_front();
            inner.first_mark += 1;
            let first = inner.first_mark;
            for slot in inner.slots.iter_mut().filter(|s| s.next_mark < first) {
                slot.overrun = true;
            }
        }
        inner.marks.push_back(mark);
        inner.wake_all();
    }

    /// Something happened at stream `offset` (the end of the slice being parsed, or the stream's
    /// end). Dropped when nobody listens.
    pub fn push_event(&self, offset: u64, event: HostEvent) {
        let mut inner = self.lock();
        if inner.slots.is_empty() {
            return;
        }
        if inner.events.len() >= EVENTS_MAX {
            inner.events.pop_front();
            inner.first_event += 1;
        }
        inner.events.push_back((offset, event));
        inner.wake_all();
    }

    /// Whether any client listens.
    pub fn has_clients(&self) -> bool {
        !self.lock().slots.is_empty()
    }

    /// A cursor that starts at the stream's end, with the marks and events from here on. It wants
    /// the bytes (`bytes`) or only the rest; `gap` limits how often appends wake it (zero: every
    /// one). Call under the screen's lock when the bytes must follow a repaint made there.
    pub fn register(self: &Arc<Self>, bytes: bool, gap: Duration, wake: Wake) -> Cursor {
        let mut inner = self.lock();
        let id = inner.next_id;
        inner.next_id += 1;
        let sent = inner.total();
        let (next_mark, next_event) =
            (inner.first_mark + inner.marks.len() as u64, inner.first_event + inner.events.len() as u64);
        inner.slots.push(Slot {
            id,
            bytes,
            sent,
            next_mark,
            next_event,
            overrun: false,
            wake,
            gap,
            woken_at: None,
        });
        Cursor { stream: self.clone(), id }
    }
}

/// One client's place in the stream. Dropping it lets the stream forget the client.
pub struct Cursor {
    stream: Arc<Stream>,
    id: u64,
}

impl Cursor {
    /// The next thing for this client: up to `max` bytes (appended to `out`), a resize, an
    /// event, or nothing. Resizes and events come after the bytes before their offset and before
    /// the bytes after it; a headless cursor gets them at once.
    pub fn next(&mut self, max: usize, out: &mut Vec<u8>) -> Next {
        let mut inner = self.stream.lock();
        let total = inner.total();
        let Some(at) = inner.slots.iter().position(|s| s.id == self.id) else { return Next::Overrun };
        if inner.slots[at].overrun || inner.slots[at].next_mark < inner.first_mark {
            inner.slots[at].overrun = true;
            return Next::Overrun;
        }
        let (bytes, sent) = (inner.slots[at].bytes, inner.slots[at].sent);
        let mark = inner.marks.get((inner.slots[at].next_mark - inner.first_mark) as usize).copied();
        // Events a slow client never took are gone; it goes on from the oldest kept.
        inner.slots[at].next_event = inner.slots[at].next_event.max(inner.first_event);
        let event = inner
            .events
            .get((inner.slots[at].next_event - inner.first_event) as usize)
            .map(|(offset, event)| (*offset, event.clone()));
        if let Some(mark) = mark.filter(|m| !bytes || m.offset <= sent) {
            inner.slots[at].next_mark += 1;
            inner.trim_aux();
            return Next::Resize(mark);
        }
        if let Some((offset, event)) = event.clone().filter(|(o, _)| !bytes || *o <= sent) {
            inner.slots[at].next_event += 1;
            inner.trim_aux();
            return Next::Event { offset, event };
        }
        if !bytes {
            return Next::Idle;
        }
        let mut end = total.min(sent.saturating_add(max as u64));
        if let Some(mark) = mark {
            end = end.min(mark.offset);
        }
        if let Some((offset, _)) = event {
            end = end.min(offset);
        }
        if end <= sent {
            return Next::Idle;
        }
        let from = (sent - inner.base) as usize;
        let to = (end - inner.base) as usize;
        let (front, back) = inner.ring.as_slices();
        if from < front.len() {
            out.extend_from_slice(&front[from..to.min(front.len())]);
        }
        if to > front.len() {
            out.extend_from_slice(&back[from.saturating_sub(front.len())..to - front.len()]);
        }
        inner.slots[at].sent = end;
        inner.trim_bytes();
        Next::Bytes { offset: sent }
    }

    /// How many bytes of the stream this client has not been handed.
    pub fn unsent(&self) -> u64 {
        let inner = self.stream.lock();
        inner.slots.iter().find(|s| s.id == self.id).map_or(0, |s| inner.total().saturating_sub(s.sent))
    }

    /// The stream offset of the next byte to hand out.
    pub fn sent(&self) -> u64 {
        let inner = self.stream.lock();
        inner.slots.iter().find(|s| s.id == self.id).map_or(0, |s| s.sent)
    }
}

impl Drop for Cursor {
    fn drop(&mut self) {
        let mut inner = self.stream.lock();
        inner.slots.retain(|s| s.id != self.id);
        inner.trim_bytes();
        inner.trim_aux();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quiet() -> Wake {
        Arc::new(|| {})
    }

    fn size() -> wire::Size {
        wire::Size { cols: 80, rows: 24, cell_w: 128, cell_h: 256 }
    }

    fn bytes_of(cursor: &mut Cursor, max: usize) -> Vec<u8> {
        let mut out = Vec::new();
        let mut all = Vec::new();
        while let Next::Bytes { .. } = cursor.next(max, &mut out) {
            all.append(&mut out);
        }
        all
    }

    #[test]
    fn bytes_are_kept_only_for_a_client_that_wants_them() {
        let stream = Arc::new(Stream::default());
        stream.append(b"nobody listens");
        assert_eq!(stream.total(), 14);
        let mut cursor = stream.register(true, Duration::ZERO, quiet());
        assert_eq!(cursor.next(100, &mut Vec::new()), Next::Idle, "starts at the end");
        stream.append(b"hello ");
        stream.append(b"world");
        assert_eq!(bytes_of(&mut cursor, 4), b"hello world");
        assert_eq!(cursor.sent(), 25);
        drop(cursor);
        stream.append(b"x");
        assert_eq!(stream.lock().ring.len(), 0, "the ring empties with its last reader");
        assert_eq!(stream.total(), 26);
    }

    #[test]
    fn a_resize_comes_between_exactly_the_bytes_it_was_applied_between() {
        let stream = Arc::new(Stream::default());
        let mut cursor = stream.register(true, Duration::ZERO, quiet());
        stream.append(b"before");
        stream.push_mark(ResizeMark { offset: 6, epoch: 1, req: 7, size: size() });
        stream.append(b"after");
        let mut out = Vec::new();
        assert_eq!(cursor.next(100, &mut out), Next::Bytes { offset: 0 });
        assert_eq!(out, b"before");
        assert!(matches!(cursor.next(100, &mut out), Next::Resize(m) if m.req == 7 && m.offset == 6));
        out.clear();
        assert_eq!(cursor.next(100, &mut out), Next::Bytes { offset: 6 });
        assert_eq!(out, b"after");
    }

    #[test]
    fn an_event_waits_for_the_bytes_before_it_and_a_headless_client_gets_it_at_once() {
        let stream = Arc::new(Stream::default());
        let mut raw = stream.register(true, Duration::ZERO, quiet());
        let mut headless = stream.register(false, Duration::ZERO, quiet());
        stream.push_event(5, HostEvent::Bell);
        stream.append(b"12345");
        assert!(matches!(headless.next(100, &mut Vec::new()), Next::Event { offset: 5, .. }));
        let mut out = Vec::new();
        assert_eq!(raw.next(100, &mut out), Next::Bytes { offset: 0 });
        assert!(matches!(raw.next(100, &mut out), Next::Event { offset: 5, .. }));
        assert_eq!(headless.next(100, &mut out), Next::Idle, "a headless client never gets bytes");
    }

    #[test]
    fn a_client_too_far_behind_is_overrun_and_the_ring_stays_bounded() {
        let stream = Arc::new(Stream::default());
        let mut slow = stream.register(true, Duration::ZERO, quiet());
        let chunk = vec![b'x'; 1024 * 1024];
        for _ in 0..6 {
            stream.append(&chunk);
        }
        assert!(stream.lock().ring.len() <= FORWARD_CAP);
        assert_eq!(slow.next(100, &mut Vec::new()), Next::Overrun);
    }

    #[test]
    fn marks_and_events_are_not_kept_for_nobody() {
        let stream = Arc::new(Stream::default());
        stream.push_event(0, HostEvent::Bell);
        stream.push_mark(ResizeMark { offset: 0, epoch: 0, req: 0, size: size() });
        let inner = stream.lock();
        assert!(inner.events.is_empty() && inner.marks.is_empty());
    }
}
