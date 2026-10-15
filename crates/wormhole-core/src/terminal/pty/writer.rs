//! Input on its way to the program: queued by the app, written to the master whenever it takes
//! it.

use std::borrow::Cow;
use std::collections::VecDeque;
use std::io::{self, Write};

use super::master::Interest;
use super::reader::Core;

/// Input the master has not taken yet, in order.
#[derive(Default)]
pub(super) struct Input {
    queue: VecDeque<Cow<'static, [u8]>>,
    /// How much of the front is written already; less than its length.
    done: usize,
}

impl Input {
    /// Queue `bytes` behind what is waiting.
    pub(super) fn push(&mut self, bytes: Cow<'static, [u8]>) {
        if !bytes.is_empty() {
            self.queue.push_back(bytes);
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
            self.queue.pop_front();
            self.done = 0;
        }
    }

    /// Drop everything (the program is gone).
    pub(super) fn clear(&mut self) {
        self.queue.clear();
        self.done = 0;
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
    use super::Input;

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
}
