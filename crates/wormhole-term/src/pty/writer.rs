//! Input on its way to the program: queued by the app, written to the master whenever it takes
//! it.

use std::borrow::Cow;
use std::collections::VecDeque;
use std::io::{self, Write};

use super::master::Interest;
use super::reader::Core;

/// The most bytes of input kept for a program that does not read it. Past this the oldest pieces
/// are dropped (a program that never reads its terminal, a flood of queries it ignores: every
/// answer of the engine is a piece here, and the queue would grow as fast as the program asks).
const QUEUE_MAX: usize = 1 << 20;

/// Input the master has not taken yet, in order.
#[derive(Default)]
pub(super) struct Input {
    queue: VecDeque<Cow<'static, [u8]>>,
    /// How much of the front is written already; less than its length.
    done: usize,
    /// The bytes of all the pieces in the queue, the written part of the front included.
    queued: usize,
}

impl Input {
    /// Queue `bytes` behind what is waiting. A piece is a whole reply, key or paste: when the
    /// queue is over [`QUEUE_MAX`] the oldest pieces go, whole, so that no reply is ever cut (half
    /// of one reaches the program as typed text). The piece being written, which the program has
    /// part of, and the newest are never dropped.
    pub(super) fn push(&mut self, bytes: Cow<'static, [u8]>) {
        if bytes.is_empty() {
            return;
        }
        self.queued += bytes.len();
        self.queue.push_back(bytes);
        let mut dropped = 0;
        while self.queued > QUEUE_MAX {
            // The front is off limits once the program has some of it; the newest always is.
            let oldest = usize::from(self.done > 0);
            if oldest + 1 >= self.queue.len() {
                break;
            }
            let Some(piece) = self.queue.remove(oldest) else { break };
            self.queued -= piece.len();
            dropped += piece.len();
        }
        if dropped > 0 {
            tracing::debug!("the program does not read its input: dropped {dropped} bytes of it");
        }
    }

    /// What is next to write.
    fn front(&self) -> Option<&[u8]> {
        self.queue.front().map(|bytes| &bytes[self.done..])
    }

    /// `written` bytes of [`front`](Self::front) went out.
    fn advance(&mut self, written: usize) {
        self.done += written;
        if self.queue.front().is_some_and(|bytes| self.done >= bytes.len()) {
            if let Some(bytes) = self.queue.pop_front() {
                self.queued -= bytes.len();
            }
            self.done = 0;
        }
    }

    /// Drop everything (the program is gone).
    pub(super) fn clear(&mut self) {
        self.queue.clear();
        self.done = 0;
        self.queued = 0;
    }

    /// Whether nothing waits.
    pub(super) fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}

impl Core {
    /// Write as much queued input as the master takes; the rest waits for it to have room, which
    /// the poller then reports.
    pub(super) fn write_input(&mut self) {
        let mut file = self.master.file();
        while let Some(chunk) = self.input.front() {
            match file.write(chunk) {
                Ok(0) => {
                    self.input.clear();
                    break;
                }
                Ok(written) => self.input.advance(written),
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => break,
                Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
                Err(err) => {
                    // The program is gone or has closed its terminal.
                    tracing::debug!("cannot write to the PTY: {err}");
                    self.input.clear();
                    break;
                }
            }
        }
        let interest = if self.input.is_empty() { Interest::Output } else { Interest::OutputAndRoom };
        self.master.poll_for(interest);
    }
}

#[cfg(test)]
mod tests {
    use super::{Input, QUEUE_MAX};

    /// Everything the master would be given, front to back.
    fn drain(input: &mut Input) -> Vec<u8> {
        let mut out = Vec::new();
        while let Some(chunk) = input.front() {
            out.extend_from_slice(chunk);
            let len = chunk.len();
            input.advance(len);
        }
        out
    }

    #[test]
    fn input_goes_out_in_order_a_piece_at_a_time() {
        let mut input = Input::default();
        assert!(input.is_empty());
        input.push(b"abc".as_slice().into());
        input.push(Vec::new().into());
        input.push(b"de".as_slice().into());
        assert_eq!(input.front(), Some(b"abc".as_slice()));
        input.advance(2);
        assert_eq!(input.front(), Some(b"c".as_slice()));
        input.advance(1);
        assert_eq!(input.front(), Some(b"de".as_slice()));
        input.advance(2);
        assert!(input.is_empty());
        input.push(b"x".as_slice().into());
        input.clear();
        assert_eq!(input.front(), None);
    }

    #[test]
    fn a_program_that_never_reads_gets_the_newest_pieces_whole_and_the_queue_stays_bounded() {
        let mut input = Input::default();
        // 3,000 replies of 1,000 bytes each, each one byte value so a cut one shows.
        for i in 0..3_000usize {
            input.push(vec![(i % 251) as u8; 1_000].into());
            assert!(input.queued <= QUEUE_MAX, "{} bytes after {i}", input.queued);
        }
        let out = drain(&mut input);
        assert!(out.len() <= QUEUE_MAX && out.len() > QUEUE_MAX - 1_000, "{}", out.len());
        assert!(out.chunks(1_000).all(|piece| piece.iter().all(|&b| b == piece[0])), "a piece was cut");
        assert_eq!(out.last(), Some(&((2_999 % 251) as u8)), "the newest is the last");
        assert_eq!(input.queued, 0);
    }

    #[test]
    fn the_piece_the_program_has_part_of_is_never_dropped() {
        let mut input = Input::default();
        input.push(vec![7u8; 1_000].into());
        input.advance(10);
        for _ in 0..3_000 {
            input.push(vec![9u8; 1_000].into());
        }
        let out = drain(&mut input);
        assert!(out.starts_with(&[7u8; 990]) && out[990] == 9, "the rest of the first, then the newer ones");
        assert!(out.len() <= QUEUE_MAX + 1_000);
    }

    #[test]
    fn one_piece_bigger_than_the_cap_is_kept_whole() {
        // A paste: the app asked for it, and it is the only thing waiting.
        let mut input = Input::default();
        input.push(vec![1u8; 2 * QUEUE_MAX].into());
        assert_eq!(drain(&mut input).len(), 2 * QUEUE_MAX);
        input.push(vec![1u8; 10].into());
        input.push(vec![2u8; 2 * QUEUE_MAX].into());
        assert_eq!(drain(&mut input).len(), 2 * QUEUE_MAX, "it pushes the older piece out, not itself");
    }
}
