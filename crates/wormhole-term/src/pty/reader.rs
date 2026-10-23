//! The reader thread: one per terminal, doing all of its I/O.
//!
//! It waits for the master to have output, reads it into one buffer and hands it to the parser in
//! slices (`parse`). Between slices, and whenever it wakes, it also looks at what the rest of the
//! app queued for it (input, a resize, the end: `inbox`), writes input to the master whenever the
//! master takes it (`writer`), and notices that the child exited (`exit`). It is the only thread
//! that reads the master, writes it and waits for the child.

use std::io::{self, Read as _};
use std::num::NonZeroUsize;
use std::os::fd::OwnedFd;
use std::process::{Child, ExitStatus};
use std::sync::Arc;
use std::time::{Duration, Instant};

use polling::{Events, Poller};

use super::InputHook;
use super::exit::{self, Watch};
use super::inbox::{Inbox, Msg};
use super::lock::FairLock;
use super::master::{Key, Master};
use super::open;
use super::parse::{Feed, Flow, Sink};
use super::writer::Input;
use crate::events::EventProxy;

/// How much one read can take. macOS hands a read of a pseudo-terminal at most 1 KiB however large
/// the buffer (`tests::read_sizes`; a second read straight after finds the queue empty 99% of
/// the time, so gathering several into one pass saved nothing and cost a syscall each), so most
/// of this is for systems that return more.
const READ_BUF: usize = 64 * 1024;

/// How much output is read after the child exits, at most: what its last writes left in the
/// queue, not what a background job that holds the slave keeps printing.
const DRAIN_MAX: usize = 1024 * 1024;

/// Everything the thread owns but the read buffer and the sink, which are borrowed
/// from the thread alongside it.
pub(super) struct Core {
    pub(super) master: Master,
    pub(super) child: Child,
    /// Our copy of the slave, held until the program has exited and its output is read. On macOS
    /// the system throws away what a program wrote that nobody has read yet when the last
    /// descriptor of its terminal closes, and a short-lived program (`echo`) closes it within
    /// microseconds of writing, before this thread has got to its first read: the screen stays
    /// empty (measured: a read 20 ms late finds nothing 200 times in 200, and everything 200 in
    /// 200 with a copy held open). Elsewhere what was written stays readable. The cost: the
    /// master no longer reports the end of the output, which the exit watch tells instead.
    slave: Option<OwnedFd>,
    pub(super) poller: Arc<Poller>,
    /// Who tells the thread that the child exited.
    exit: Watch,
    inbox: Arc<Inbox>,
    pub(super) input: Input,
    /// Taken out of the inbox, applied in order.
    messages: Vec<Msg>,
    /// The terminal was closed: end the program, giving it this long.
    close: Option<Duration>,
    /// Told how much input the master has taken (a hosted terminal's), and the count: written to
    /// the master or dropped for a program that does not read it (`Input::push`).
    pub(super) input_hook: Option<InputHook>,
    pub(super) input_total: u64,
}

/// The reader thread's state.
pub(super) struct Reader<S, F> {
    buf: Box<[u8]>,
    sink: Sink<S, F>,
    core: Core,
    spin: Spin,
}

/// How long the reader keeps asking an empty master before it sleeps in the poller (see
/// [`Next::ReadAgain`]).
const SPIN_FOR: Duration = Duration::from_micros(50);

/// Where the reader is in its spin after output.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Spin {
    /// Output was just read: the spin starts at the next empty read.
    Armed,
    /// Asking until this moment, then the poller.
    Until(Instant),
}

/// What a wait for the poller found.
#[derive(Default)]
struct Woken {
    /// The master has output.
    output: bool,
    /// The child exited.
    exited: bool,
}

/// What to do after a read.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Next {
    /// Read again before asking the poller: a program that prints a lot has printed more by now,
    /// and a read that finds nothing costs a fraction of a poll (`ttystorm`'s `logflood` read
    /// 144 MB/s polling after every read and 164 reading again first). After output this goes on
    /// for [`SPIN_FOR`] even when the reads find nothing: a flood is a ping-pong with the
    /// writer, which the kernel hands 1 KiB at a time, and an engine that parses a read faster
    /// than the writer refills the queue would sleep and be woken for every one of them (the
    /// engine's colorflood: 38% of the reader's time in `kevent`; with the spin, `perf.sh`
    /// logflood/colorflood/cursorstorm 150/144/154 -> 198/225/203 MB/s). A quiet terminal never
    /// spins: only a read that found output arms it.
    ReadAgain,
    /// Ask the poller when there is more.
    Poll,
}

impl<S: Send + 'static, F: Feed<S>> Reader<S, F> {
    /// The thread for `master` and `child`, parsing into `screen` with `feed`.
    #[allow(clippy::too_many_arguments)] // the thread's whole starting kit, built in one place
    pub(super) fn new(
        master: Master,
        child: Child,
        poller: Arc<Poller>,
        inbox: Arc<Inbox>,
        screen: Arc<FairLock<S>>,
        feed: F,
        proxy: EventProxy,
        input_hook: Option<InputHook>,
    ) -> Self {
        let exit = exit::watch(&poller, &child);
        Self {
            buf: vec![0; READ_BUF].into_boxed_slice(),
            sink: Sink::new(screen, feed, proxy),
            spin: Spin::Armed,
            core: Core {
                master,
                child,
                slave: None,
                poller,
                exit,
                inbox,
                input: Input::default(),
                messages: Vec::new(),
                close: None,
                input_hook,
                input_total: 0,
            },
        }
    }

    /// Keep `slave` open until the program has exited and its output is read, where the system
    /// needs that (see `Core::slave`); anywhere else it is closed at once.
    pub(super) fn holding(mut self, slave: OwnedFd) -> Self {
        self.core.slave = cfg!(target_os = "macos").then_some(slave);
        self
    }

    /// Run until the program is gone (and reported) or the terminal is closed (and the program
    /// ended).
    pub(super) fn run(mut self) {
        let mut events = Events::with_capacity(NonZeroUsize::MIN.saturating_add(3));
        let mut next = Next::Poll;
        // A program that exited before the poller watched it (`/bin/true`) gives it no event.
        let mut exited = self.core.child_gone();
        loop {
            self.core.apply_inbox();
            if let Some(grace) = self.core.close {
                return self.core.hangup(grace, &mut events);
            }
            self.core.write_input();
            self.sink.expire_sync();
            if next == Next::Poll && !exited {
                match self.core.wait(&mut events, self.sink.deadline()) {
                    Ok(woken) => {
                        next = if woken.output { Next::ReadAgain } else { Next::Poll };
                        exited = woken.exited || self.core.unannounced_exit();
                    }
                    Err(err) => {
                        tracing::warn!("waiting for the PTY failed, ending its terminal: {err}");
                        return self.finish();
                    }
                }
            }
            next =
                if next == Next::ReadAgain && self.core.master.is_open() { self.read() } else { Next::Poll };
            if exited && self.core.close.is_none() {
                return self.finish();
            }
        }
    }

    /// One read into the parser.
    fn read(&mut self) -> Next {
        let Self { buf, sink, core, spin } = self;
        match core.master.file().read(buf) {
            Ok(0) => {
                core.output_ended();
                Next::Poll
            }
            Ok(n) => {
                *spin = Spin::Armed;
                sink.parse(&buf[..n], || core.attend());
                Next::ReadAgain
            }
            Err(err) if err.kind() == io::ErrorKind::WouldBlock => {
                let now = Instant::now();
                let until = match *spin {
                    Spin::Armed => now + SPIN_FOR,
                    Spin::Until(until) => until,
                };
                *spin = Spin::Until(until);
                if now < until { Next::ReadAgain } else { Next::Poll }
            }
            Err(err) if err.kind() == io::ErrorKind::Interrupted => Next::ReadAgain,
            Err(err) => {
                // Linux reports the end of the output as EIO.
                tracing::debug!("the PTY's output ended: {err}");
                core.output_ended();
                Next::Poll
            }
        }
    }

    /// The child exited: read what it left, report it, and let go of the terminal.
    fn finish(mut self) {
        let status = self.core.reap();
        let mut drained = 0;
        while self.core.master.is_open() && drained < DRAIN_MAX && self.core.close.is_none() {
            let Self { buf, sink, core, .. } = &mut self;
            match core.master.file().read(buf) {
                Ok(0) => break,
                Ok(n) => {
                    sink.parse(&buf[..n], || core.attend());
                    drained += n;
                }
                Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
                Err(_) => break,
            }
        }
        exit::unwatch(&self.core.poller, &self.core.child);
        // Everything it wrote is read now. An update it began and never ended is shown: nothing
        // runs after this to give up on it.
        self.core.slave = None;
        self.sink.release_sync();
        use std::os::unix::process::ExitStatusExt as _;
        let (code, signal) = status.map_or((None, None), |status| (status.code(), status.signal()));
        self.sink.report_exit(code, signal);
    }
}

impl Core {
    /// Block until the master has output, the child exited, someone wrote to the inbox, or the
    /// parser's `deadline` passed.
    fn wait(&mut self, events: &mut Events, deadline: Option<Instant>) -> io::Result<Woken> {
        let mut timeout = deadline.map(|at| at.saturating_duration_since(Instant::now()));
        if self.exit == Watch::ByPolling {
            timeout = Some(timeout.map_or(exit::POLL_EVERY, |left| left.min(exit::POLL_EVERY)));
        }
        events.clear();
        match self.poller.wait(events, timeout) {
            Ok(_) => {}
            Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
            Err(err) => return Err(err),
        }
        let mut woken = Woken::default();
        for event in events.iter() {
            match Key::from_raw(event.key) {
                Some(Key::Master) => woken.output |= event.readable,
                Some(Key::Exit) => woken.exited = true,
                None => {}
            }
        }
        Ok(woken)
    }

    /// What to do between slices: attend to what the app queued, and whether to go on.
    fn attend(&mut self) -> Flow {
        if !self.inbox.has_messages() {
            return Flow::Continue;
        }
        self.apply_inbox();
        if self.close.is_some() {
            return Flow::Stop;
        }
        self.write_input();
        Flow::Continue
    }

    /// Do what the inbox holds: queue input, resize, note that the terminal was closed.
    pub(super) fn apply_inbox(&mut self) {
        if !self.inbox.has_messages() {
            return;
        }
        let mut messages = std::mem::take(&mut self.messages);
        self.inbox.take(&mut messages);
        let before = self.input_total;
        for msg in messages.drain(..) {
            match msg {
                // Nobody reads what a program that closed its terminal would be sent.
                Msg::Input(bytes) if self.master.is_open() => {
                    self.input_total += self.input.push(bytes) as u64;
                }
                Msg::Input(_) => {}
                Msg::Resize(size) => {
                    if let Err(err) = open::set_size(self.master.file(), size) {
                        tracing::debug!("cannot resize the PTY: {err}");
                    }
                }
                Msg::Close { grace } => self.close = Some(grace),
            }
        }
        self.messages = messages;
        // Dropped input counts as taken: a hosted terminal's client counts the bytes it may still
        // send by the acknowledgements, and would otherwise lose that room for good.
        if self.input_total != before
            && let Some(hook) = &self.input_hook
        {
            hook(self.input_total);
        }
    }

    /// No more output will come; the child may still run (a program can close its standard
    /// streams), and its exit is what ends the terminal.
    fn output_ended(&mut self) {
        self.master.forget();
        self.input.clear();
    }

    /// Whether the child is gone already (or cannot be asked about).
    fn child_gone(&mut self) -> bool {
        self.child.try_wait().map_or(true, |status| status.is_some())
    }

    /// Whether the child has exited while the poller could not say so.
    fn unannounced_exit(&mut self) -> bool {
        self.exit == Watch::ByPolling && self.child_gone()
    }

    /// The child's exit status, reaping it. The poller's report can come a moment before the
    /// system lets `waitpid` have the status.
    fn reap(&mut self) -> Option<ExitStatus> {
        for _ in 0..50 {
            match self.child.try_wait() {
                Ok(Some(status)) => return Some(status),
                Ok(None) => std::thread::sleep(Duration::from_millis(1)),
                Err(_) => return None,
            }
        }
        None
    }
}
