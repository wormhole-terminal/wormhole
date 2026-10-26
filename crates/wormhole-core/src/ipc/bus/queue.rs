//! The bounded queue behind each subscriber. A publisher must never wait for a client, so a full
//! queue drops its oldest items instead, and says so: the reader is told how many were lost before
//! the next one it gets.

use std::collections::VecDeque;
use std::sync::{Condvar, Mutex, PoisonError};
use std::time::{Duration, Instant};

use crate::ipc::control::event::CloseReason;

/// What waiting on a queue ends with.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Pop<T> {
    Item(T),
    /// This many were dropped since the last time the reader looked, before whatever comes next.
    Lagged(u64),
    /// The queue was closed, and why.
    Closed(CloseReason),
    /// Nothing came in time.
    Empty,
}

#[derive(Debug)]
struct Inner<T> {
    /// Each item with its weight in bytes.
    items: VecDeque<(T, usize)>,
    bytes: usize,
    dropped: u64,
    closed: Option<CloseReason>,
}

#[derive(Debug)]
pub(super) struct Queue<T> {
    inner: Mutex<Inner<T>>,
    ready: Condvar,
    max_items: usize,
    max_bytes: usize,
}

impl<T> Queue<T> {
    pub(super) fn new(max_items: usize, max_bytes: usize) -> Self {
        Self {
            inner: Mutex::new(Inner { items: VecDeque::new(), bytes: 0, dropped: 0, closed: None }),
            ready: Condvar::new(),
            max_items,
            max_bytes,
        }
    }

    /// Add `item`, which weighs `bytes`, dropping the oldest until it fits (it stays even when it
    /// alone is over the byte cap: the newest is what a reader most needs). Never waits.
    pub(super) fn push(&self, item: T, bytes: usize) {
        let mut inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        if inner.closed.is_some() {
            return;
        }
        while !inner.items.is_empty()
            && (inner.items.len() >= self.max_items || inner.bytes + bytes > self.max_bytes)
        {
            if let Some((_, dropped)) = inner.items.pop_front() {
                inner.bytes -= dropped;
                inner.dropped += 1;
            }
        }
        inner.items.push_back((item, bytes));
        inner.bytes += bytes;
        self.ready.notify_one();
    }

    /// The next thing for the reader, waiting up to `wait` for one: a closed queue first, then what
    /// was lost, then the oldest item kept.
    pub(super) fn pop(&self, wait: Duration) -> Pop<T> {
        let deadline = Instant::now() + wait;
        let mut inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        loop {
            if let Some(reason) = inner.closed {
                return Pop::Closed(reason);
            }
            if inner.dropped > 0 {
                return Pop::Lagged(std::mem::take(&mut inner.dropped));
            }
            if let Some((item, bytes)) = inner.items.pop_front() {
                inner.bytes -= bytes;
                return Pop::Item(item);
            }
            let left = deadline.saturating_duration_since(Instant::now());
            if left.is_zero() {
                return Pop::Empty;
            }
            inner = self.ready.wait_timeout(inner, left).unwrap_or_else(PoisonError::into_inner).0;
        }
    }

    /// End the queue: the reader gets `reason` at once, and nothing more is added.
    pub(super) fn close(&self, reason: CloseReason) {
        let mut inner = self.inner.lock().unwrap_or_else(PoisonError::into_inner);
        inner.closed.get_or_insert(reason);
        inner.items.clear();
        inner.bytes = 0;
        self.ready.notify_all();
    }

    #[cfg(test)]
    pub(super) fn len(&self) -> usize {
        self.inner.lock().unwrap_or_else(PoisonError::into_inner).items.len()
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use super::{Pop, Queue};
    use crate::ipc::control::event::CloseReason;

    const NONE: Duration = Duration::ZERO;

    #[test]
    fn items_come_out_in_the_order_they_went_in() {
        let queue = Queue::new(8, 1024);
        for n in 1..=3 {
            queue.push(n, 1);
        }
        assert_eq!(
            [queue.pop(NONE), queue.pop(NONE), queue.pop(NONE)],
            [Pop::Item(1), Pop::Item(2), Pop::Item(3)]
        );
        assert_eq!(queue.pop(NONE), Pop::Empty);
    }

    #[test]
    fn a_full_queue_drops_its_oldest_and_the_reader_is_told_before_the_next() {
        let queue = Queue::new(3, 1024);
        for n in 1..=5 {
            queue.push(n, 1);
        }
        assert_eq!(queue.len(), 3);
        assert_eq!(queue.pop(NONE), Pop::Lagged(2), "1 and 2 were dropped");
        assert_eq!(
            [queue.pop(NONE), queue.pop(NONE), queue.pop(NONE)],
            [Pop::Item(3), Pop::Item(4), Pop::Item(5)]
        );
        // The count starts again once it has been told.
        assert_eq!(queue.pop(NONE), Pop::Empty);
        queue.push(6, 1);
        assert_eq!(queue.pop(NONE), Pop::Item(6));
    }

    #[test]
    fn the_byte_cap_drops_too_and_the_newest_always_stays() {
        let queue = Queue::new(100, 10);
        queue.push("a", 4);
        queue.push("b", 4);
        queue.push("c", 4);
        assert_eq!(queue.pop(NONE), Pop::Lagged(1));
        assert_eq!([queue.pop(NONE), queue.pop(NONE)], [Pop::Item("b"), Pop::Item("c")]);
        queue.push("small", 1);
        queue.push("huge", 50);
        assert_eq!(queue.pop(NONE), Pop::Lagged(1));
        assert_eq!(queue.pop(NONE), Pop::Item("huge"), "over the cap on its own, and kept");
    }

    #[test]
    fn closing_wakes_a_waiting_reader_and_ends_the_queue() {
        let queue = Arc::new(Queue::new(8, 1024));
        queue.push(1, 1);
        let reader = std::thread::spawn({
            let queue = queue.clone();
            move || {
                let start = Instant::now();
                (queue.pop(Duration::from_secs(5)), queue.pop(Duration::from_secs(5)), start.elapsed())
            }
        });
        std::thread::sleep(Duration::from_millis(50));
        queue.close(CloseReason::NotAllowed);
        let (first, second, took) = reader.join().expect("reader");
        // Either it read the item before the close, or the close came first: never a hang.
        assert!(matches!(first, Pop::Item(1) | Pop::Closed(CloseReason::NotAllowed)), "{first:?}");
        assert_eq!(second, Pop::Closed(CloseReason::NotAllowed));
        assert!(took < Duration::from_secs(2), "woken, not timed out: {took:?}");
        queue.push(2, 1);
        assert_eq!(queue.len(), 0, "a closed queue takes nothing");
    }

    #[test]
    fn an_empty_wait_ends_at_its_deadline() {
        let queue: Queue<u8> = Queue::new(2, 8);
        let start = Instant::now();
        assert_eq!(queue.pop(Duration::from_millis(60)), Pop::Empty);
        assert!(start.elapsed() >= Duration::from_millis(55));
    }

    #[test]
    fn a_push_wakes_a_waiting_reader() {
        let queue = Arc::new(Queue::new(2, 8));
        let reader = std::thread::spawn({
            let queue = queue.clone();
            move || queue.pop(Duration::from_secs(5))
        });
        std::thread::sleep(Duration::from_millis(30));
        queue.push(7, 1);
        assert_eq!(reader.join().expect("reader"), Pop::Item(7));
    }
}
