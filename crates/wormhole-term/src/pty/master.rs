//! The master descriptor and what the reader's poller is asked about it.

use std::fs::File;
use std::io;
use std::sync::Arc;

use polling::{Event, PollMode, Poller};

/// Which of the reader's sources a poller event comes from (the event's key).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Key {
    /// The master: output to read, room to write.
    Master,
    /// The child: it exited.
    Exit,
}

impl Key {
    /// The number the poller carries.
    pub(super) fn raw(self) -> usize {
        self as usize
    }

    /// The source a poller's key stands for, if it is one of ours.
    pub(super) fn from_raw(raw: usize) -> Option<Self> {
        [Self::Master, Self::Exit].into_iter().find(|key| key.raw() == raw)
    }
}

/// What the poller is asked to report about the master.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Interest {
    /// Output to read.
    Output,
    /// Output to read and room to write: only while input waits, because a master with room is
    /// its usual state and a level-triggered poller would report it again and again.
    OutputAndRoom,
}

/// Whether the poller still knows the master.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Registration {
    /// Polled for this.
    Polled(Interest),
    /// Not any more: the output ended, which the poller would report again and again.
    Forgotten,
}

/// The master and its registration with the poller.
pub(super) struct Master {
    file: File,
    poller: Arc<Poller>,
    registration: Registration,
}

impl Master {
    /// `file` (the master, non-blocking), polled for output through `poller`.
    pub(super) fn new(file: File, poller: Arc<Poller>) -> io::Result<Self> {
        let mut master = Self { file, poller, registration: Registration::Forgotten };
        master.add()?;
        master.registration = Registration::Polled(Interest::Output);
        Ok(master)
    }

    #[allow(unsafe_code)] // polling's registration contract, kept by `Drop`.
    fn add(&self) -> io::Result<()> {
        // SAFETY: the descriptor is deleted from the poller before it is closed: `forget`, and
        // the drop of this struct, which owns both.
        unsafe { self.poller.add_with_mode(&self.file, Event::readable(Key::Master.raw()), PollMode::Level) }
    }

    /// The descriptor, to read from and write to (`&File` does both).
    pub(super) fn file(&self) -> &File {
        &self.file
    }

    /// Whether the output may still come (it has not ended).
    pub(super) fn is_open(&self) -> bool {
        self.registration != Registration::Forgotten
    }

    /// Ask the poller about `interest`, if that is not what it is asked already.
    pub(super) fn poll_for(&mut self, interest: Interest) {
        if self.registration == Registration::Polled(interest) {
            return;
        }
        if !self.is_open() {
            return;
        }
        let key = Key::Master.raw();
        let event = match interest {
            Interest::Output => Event::readable(key),
            Interest::OutputAndRoom => Event::all(key),
        };
        match self.poller.modify_with_mode(&self.file, event, PollMode::Level) {
            Ok(()) => self.registration = Registration::Polled(interest),
            Err(err) => tracing::debug!("cannot change what the PTY is polled for: {err}"),
        }
    }

    /// Stop polling the master: its output has ended.
    pub(super) fn forget(&mut self) {
        if self.is_open() {
            self.registration = Registration::Forgotten;
            let _ = self.poller.delete(&self.file);
        }
    }
}

impl Drop for Master {
    fn drop(&mut self) {
        self.forget();
    }
}
