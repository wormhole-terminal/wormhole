//! The queue of replies owed to the program, and the one place that decides which are kept.
//!
//! A reply is a whole control string (`CSI … R`, `DCS … ST`): the program reads it as one thing,
//! and half of one is worse than none (an unterminated string swallows the keys typed after it).
//! So everything is decided **once per whole reply**, when it is complete, and never inside it:
//!
//! - **the cap**: a reply is kept when it fits under the host's limit ([`Limits::replies`], bytes
//!   waiting for the host *and* bytes held behind a picture, together) or when nothing else is
//!   waiting; otherwise it is dropped whole. A reply that is bigger than the cap by itself (an
//!   OSC 52 answer) therefore still gets through an empty queue. The queue never holds more than
//!   the cap, but for the answers to pictures (see below), which are kept;
//! - **the one-byte C1 controls** (S8C1T): the introducers are converted when the reply is
//!   complete, by the mode in force when it was asked, also for a reply that waits behind a picture
//!   and for the picture's own answer;
//! - **the order**: programs read their answers in the order they asked, so while a picture's
//!   answer waits for the host to decode it ([`ReplyQueue::wait`]) every later reply is held behind
//!   it, and let through in order when the answer is settled ([`ReplyQueue::settle`]).
//!
//! A reply made of pieces is composed with [`Terminal::reply_with`], which hands the pieces to a
//! scratch buffer and the queue the whole.
//!
//! [`Limits::replies`]: crate::terminal::Limits::replies
//! [`Terminal::reply_with`]: crate::terminal::Terminal::reply_with

use std::collections::VecDeque;
use std::fmt;
use std::io::Write as _;

use crate::terminal::graphics::{AnswerError, Target};
use crate::terminal::modes::conformance::eight_bit_in_place;
use crate::types::ImageHandle;

/// The most reply bytes kept for a host that does not collect them.
pub(in crate::terminal) const REPLIES_MAX: usize = 1 << 20;
/// The most picture answers waiting at once; past it the oldest is given up on.
const WAITING_MAX: usize = 64;

/// What the terminal's state says about a reply that is being queued.
#[derive(Clone, Copy, Debug)]
pub(in crate::terminal) struct Rules {
    /// [`Limits::replies`](crate::terminal::Limits::replies).
    pub cap: usize,
    /// S8C1T is on: the introducers go as one byte.
    pub eight_bit: bool,
}

/// One piece of the replies in line behind a picture's answer: bytes of one whole reply that are
/// ready to go, or the place of the answer that waits.
#[derive(Debug)]
enum Part {
    Ready(Vec<u8>),
    /// A picture's answer, and whether it goes out in 8-bit form (the mode of the command).
    Waiting(ImageHandle, Target, bool),
}

/// See the module docs.
#[derive(Default)]
pub(in crate::terminal) struct ReplyQueue {
    /// What the host collects.
    ready: Vec<u8>,
    /// Replies behind an answer that waits for the host; empty while none waits.
    parts: VecDeque<Part>,
    /// Bytes in the `Ready` parts.
    held: usize,
    /// A reply being composed in pieces (kept for its capacity).
    scratch: Vec<u8>,
}

impl fmt::Debug for ReplyQueue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ReplyQueue")
            .field("ready", &self.ready.len())
            .field("held", &self.held)
            .field("parts", &self.parts)
            .finish()
    }
}

impl ReplyQueue {
    /// The bytes the host writes to the program.
    pub fn bytes(&self) -> &[u8] {
        &self.ready
    }

    /// How many bytes [`bytes`](Self::bytes) has.
    pub fn len(&self) -> usize {
        self.ready.len()
    }

    /// The host wrote the replies out. What waits behind a picture stays.
    pub fn clear(&mut self) {
        self.ready.clear();
    }

    /// Take the replies, leaving none.
    pub fn take(&mut self) -> Vec<u8> {
        std::mem::take(&mut self.ready)
    }

    /// A reset: no answer waits and nothing is held behind one.
    pub fn forget_held(&mut self) {
        self.parts.clear();
        self.held = 0;
    }

    /// What waits behind a picture, as text for the state dump.
    #[cfg(any(test, feature = "internals"))]
    pub fn describe_held(&self) -> String {
        format!("{:?} held {}", self.parts, self.held)
    }

    /// Bytes waiting for the host and bytes held behind a picture.
    pub fn total(&self) -> usize {
        self.ready.len() + self.held
    }

    /// Whether a reply of `len` bytes fits: under the cap with what waits, or alone.
    fn fits(&self, len: usize, cap: usize) -> bool {
        self.total() + len <= cap || self.total() == 0
    }

    /// One whole reply, formatted from `args`. Written straight into the queue when nothing waits
    /// behind a picture, so a reply allocates nothing.
    pub fn push_fmt(&mut self, rules: Rules, args: fmt::Arguments<'_>) {
        if self.parts.is_empty() {
            let from = self.ready.len();
            let _ = self.ready.write_fmt(args);
            self.settle_ready(from, rules);
        } else {
            let mut bytes = Vec::new();
            let _ = bytes.write_fmt(args);
            self.hold(bytes, rules);
        }
    }

    /// One whole reply that is made already.
    pub fn push_bytes(&mut self, rules: Rules, bytes: &[u8]) {
        if self.parts.is_empty() {
            let from = self.ready.len();
            self.ready.extend_from_slice(bytes);
            self.settle_ready(from, rules);
        } else {
            self.hold(bytes.to_vec(), rules);
        }
    }

    /// The reply just written at `from..` of `ready` is complete: convert it, and keep it or take
    /// it back out whole.
    fn settle_ready(&mut self, from: usize, rules: Rules) {
        if rules.eight_bit {
            eight_bit_in_place(&mut self.ready, from);
        }
        let len = self.ready.len() - from;
        // Seen without the reply itself: it is not yet counted against the cap.
        if !(self.held + from + len <= rules.cap || self.held + from == 0) {
            self.ready.truncate(from);
        }
    }

    /// A whole reply behind the answers that wait.
    fn hold(&mut self, mut bytes: Vec<u8>, rules: Rules) {
        if rules.eight_bit {
            eight_bit_in_place(&mut bytes, 0);
        }
        if !self.fits(bytes.len(), rules.cap) {
            return;
        }
        self.held += bytes.len();
        self.parts.push_back(Part::Ready(bytes));
    }

    /// An empty scratch buffer to compose a reply in pieces in.
    pub fn take_scratch(&mut self) -> Vec<u8> {
        let mut scratch = std::mem::take(&mut self.scratch);
        scratch.clear();
        scratch
    }

    /// Give the scratch buffer back (its capacity is kept for the next reply).
    pub fn put_scratch(&mut self, scratch: Vec<u8>) {
        self.scratch = scratch;
    }

    /// An answer for `image` that waits for the host: replies from here on go behind it. Past the
    /// caps the oldest waiting answer is given up on and sent as an error instead.
    pub fn wait(&mut self, rules: Rules, image: ImageHandle, target: Target) {
        self.parts.push_back(Part::Waiting(image, target, rules.eight_bit));
        let waiting = self.parts.iter().filter(|part| matches!(part, Part::Waiting(..))).count();
        let oldest = self.parts.iter().find_map(|part| match part {
            Part::Waiting(image, ..) => Some(*image),
            Part::Ready(_) => None,
        });
        if let Some(oldest) = oldest.filter(|_| waiting > WAITING_MAX || self.held >= rules.cap) {
            self.settle(oldest, Err(AnswerError::BadData("the picture was not decoded in time")));
        }
    }

    /// The answer for `image` is known: it takes its place in line, and the replies at the front of
    /// the line that are ready go to the host, in order.
    pub fn settle(&mut self, image: ImageHandle, result: Result<(), AnswerError>) {
        let at = self.parts.iter().position(|part| matches!(part, Part::Waiting(h, ..) if *h == image));
        if let Some(at) = at
            && let Some(Part::Waiting(_, target, eight_bit)) = self.parts.remove(at)
        {
            let mut reply = target.render(result).unwrap_or_default();
            if eight_bit {
                eight_bit_in_place(&mut reply, 0);
            }
            self.held += reply.len();
            self.parts.insert(at, Part::Ready(reply));
        }
        // Moved from one count to the other, so the total does not change. The answers rendered
        // here are new, but they are kept whatever the cap says: at most `WAITING_MAX` of them
        // wait, each of a hundred bytes, and a program that asked for one and never gets it stalls.
        while let Some(Part::Ready(_)) = self.parts.front() {
            if let Some(Part::Ready(bytes)) = self.parts.pop_front() {
                self.held = self.held.saturating_sub(bytes.len());
                self.ready.extend_from_slice(&bytes);
            }
        }
    }
}

/// The pieces of a reply being composed; see [`Terminal::reply_with`](crate::terminal::Terminal::reply_with).
pub(in crate::terminal) struct Pieces<'a>(pub(super) &'a mut Vec<u8>);

impl Pieces<'_> {
    /// The next piece, formatted.
    pub fn put(&mut self, args: fmt::Arguments<'_>) {
        let _ = self.0.write_fmt(args);
    }
}

#[cfg(test)]
mod tests {
    use super::{ReplyQueue, Rules, WAITING_MAX};
    use crate::terminal::graphics::{AnswerError, Target};
    use crate::types::ImageHandle;

    const SEVEN: Rules = Rules { cap: 1 << 20, eight_bit: false };

    fn target(id: u32) -> Target {
        Target::default().with_id(id)
    }

    fn text(queue: &ReplyQueue) -> String {
        String::from_utf8_lossy(queue.bytes()).into_owned()
    }

    #[test]
    fn a_reply_is_kept_or_dropped_whole_by_the_queue_it_meets() {
        let rules = Rules { cap: 10, eight_bit: false };
        let mut queue = ReplyQueue::default();
        queue.push_fmt(rules, format_args!("\x1b[0n"));
        queue.push_fmt(rules, format_args!("\x1b[0n"));
        assert_eq!(queue.len(), 8);
        // Four bytes more would make twelve: not kept, and not cut either.
        queue.push_fmt(rules, format_args!("\x1b[0n"));
        assert_eq!(text(&queue), "\x1b[0n\x1b[0n");
        // A shorter one still fits.
        queue.push_bytes(rules, b"\x1b\\");
        assert_eq!(text(&queue), "\x1b[0n\x1b[0n\x1b\\");
        // The queue emptied, a reply bigger than the cap is the only thing waiting: kept.
        queue.clear();
        queue.push_fmt(rules, format_args!("\x1b]52;c;{}\x07", "A".repeat(100)));
        assert_eq!(queue.len(), 108);
        queue.push_fmt(rules, format_args!("\x1b[0n"));
        assert_eq!(queue.len(), 108, "and nothing joins it");
    }

    #[test]
    fn replies_convert_to_one_byte_controls_by_the_mode_they_were_asked_in() {
        let mut queue = ReplyQueue::default();
        let eight = Rules { eight_bit: true, ..SEVEN };
        queue.push_fmt(eight, format_args!("\x1bP1$r0m\x1b\\"));
        assert_eq!(queue.bytes(), b"\x901$r0m\x9c");
        // Held behind a picture's answer, a reply keeps the form it had when it was asked.
        queue.clear();
        queue.wait(eight, ImageHandle(1), target(1));
        queue.push_fmt(eight, format_args!("\x1b[1;1R"));
        queue.push_fmt(SEVEN, format_args!("\x1b[2;2R"));
        assert!(queue.bytes().is_empty());
        queue.settle(ImageHandle(1), Ok(()));
        // The answer is the picture's, in the form of the command (an APC, 0x9f, in 8 bits).
        assert_eq!(queue.bytes(), b"\x9fGi=1;OK\x9c\x9b1;1R\x1b[2;2R");
    }

    #[test]
    fn later_replies_wait_behind_an_answer_that_waits() {
        let mut queue = ReplyQueue::default();
        queue.wait(SEVEN, ImageHandle(1), target(1));
        queue.push_fmt(SEVEN, format_args!("\x1b[0n"));
        queue.push_fmt(SEVEN, format_args!("\x1b[?62c"));
        queue.wait(SEVEN, ImageHandle(2), target(2));
        queue.push_fmt(SEVEN, format_args!("later"));
        assert!(queue.bytes().is_empty());
        // The second settles first: nothing is free until the first does.
        queue.settle(ImageHandle(2), Ok(()));
        assert!(queue.bytes().is_empty());
        queue.settle(ImageHandle(1), Err(AnswerError::BadData("bad")));
        assert_eq!(text(&queue), "\x1b_Gi=1;ENODATA:bad\x1b\\\x1b[0n\x1b[?62c\x1b_Gi=2;OK\x1b\\later");
        assert_eq!(queue.total(), queue.len());
    }

    #[test]
    fn what_is_released_counts_against_the_cap_like_what_is_held() {
        let rules = Rules { cap: 100, eight_bit: false };
        let mut queue = ReplyQueue::default();
        queue.wait(rules, ImageHandle(1), target(1));
        for _ in 0..100 {
            queue.push_fmt(rules, format_args!("\x1b[0n"));
        }
        assert!(queue.total() <= 100, "held replies are held to the cap too: {}", queue.total());
        queue.settle(ImageHandle(1), Ok(()));
        let answer = b"\x1b_Gi=1;OK\x1b\\";
        assert!(queue.len() <= 100 + answer.len(), "{} bytes released", queue.len());
        assert!(queue.bytes().starts_with(answer), "the picture's answer comes first and is kept");
        assert!(queue.bytes()[answer.len()..].chunks(4).all(|reply| reply == b"\x1b[0n"), "whole replies");
    }

    #[test]
    fn a_host_that_never_answers_cannot_hold_replies_for_ever() {
        let mut queue = ReplyQueue::default();
        for n in 0..(WAITING_MAX as u64 + 40) {
            queue.wait(SEVEN, ImageHandle(n), target(n as u32 + 1));
        }
        assert!(!queue.bytes().is_empty(), "the oldest were given up on");
        let waiting = queue.parts.iter().filter(|part| matches!(part, super::Part::Waiting(..))).count();
        assert!(waiting <= WAITING_MAX, "{waiting}");
        let mut queue = ReplyQueue::default();
        let rules = Rules { cap: 1000, eight_bit: false };
        queue.wait(rules, ImageHandle(1), target(1));
        for _ in 0..1000 {
            queue.push_fmt(rules, format_args!("\x1b[0n"));
        }
        assert!(queue.total() <= 1000, "{}", queue.total());
    }
}
