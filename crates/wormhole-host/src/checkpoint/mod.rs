//! The host's checkpoint of its terminal, for a restart of the Mac (`wormhole_term::checkpoint` is the
//! format; `repo-docs/docs/session-host.md` section 8).
//!
//! - `schedule`: when one is due, on a clock the caller gives;
//! - `worker`: the one thread that reads the terminal and writes the files, so the PTY thread and the driver
//!   thread never touch a disk for it.
//!
//! [`Checkpointer`] is the driver's side. It does nothing until the app has sent `SetCheckpointKey` with a key,
//! so with the feature off no file is made, and an empty key stops it again. The driver calls
//! [`Checkpointer::pump`] each turn; at SIGTERM it calls [`Checkpointer::flush`] *before* the shell is hung up, so
//! what the restart finds is the screen as the person left it.
//!
//! **A closed terminal leaves nothing.** When the controller closes the terminal (or an admin ends the host) the
//! driver calls [`Checkpointer::close`] *before* the shell is hung up: the worker drops what it still had to do, and
//! the program's exit afterwards writes nothing. Only a SIGTERM (a restart) and an exit nobody asked for leave a
//! last generation. The host never removes a checkpoint directory (the app does, and a host cannot tell a
//! restart's end from a closed terminal's by itself); it prunes its own old generations and temporary files.

pub mod schedule;
pub mod worker;

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use wormhole_term::checkpoint::Key;
use wormhole_wire as wire;

use schedule::Schedule;
use worker::Worker;
pub use worker::{DirSink, Done, Job, Sink, Source};

/// How long the host waits for the last checkpoint at SIGTERM: the restart gives it about three seconds
/// before the kill, and the hangup of the shell needs some of those.
pub const FLUSH_WITHIN: Duration = Duration::from_secs(1);

/// What a flush came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Flushed {
    /// The app never asked for checkpoints, asked for no more, or the terminal was closed.
    Off,
    /// Written, this many bytes (0: nothing to write, as with a full-screen program on).
    Done(u64),
    /// The write did not finish in time; it may still, and a half-written generation is never read.
    TimedOut,
    Failed(String),
}

#[derive(PartialEq, Eq)]
enum Phase {
    /// Never asked.
    Idle,
    /// Writing, as the schedule says.
    On,
    /// The app told it to stop (the feature went off): nothing more, until a key comes again.
    Stopped,
    /// The terminal is closed: nothing more, ever.
    Closed,
}

pub struct Checkpointer {
    sessions_dir: PathBuf,
    id: String,
    phase: Phase,
    schedule: Schedule,
    worker: Option<Worker>,
    /// How the program ended, once it has: every later generation says so too.
    ended: Option<String>,
    /// Called by the worker after each job, so the driver looks at what was done.
    wake: std::sync::Arc<dyn Fn() + Send + Sync>,
}

/// `exit:<code>` or `signal:<number>`.
pub fn ended_text(info: &wire::ExitInfo) -> String {
    if info.signal != 0 { format!("signal:{}", info.signal) } else { format!("exit:{}", info.code) }
}

impl Checkpointer {
    pub fn new(sessions_dir: PathBuf, id: String, wake: std::sync::Arc<dyn Fn() + Send + Sync>) -> Self {
        Self {
            sessions_dir,
            id,
            phase: Phase::Idle,
            schedule: Schedule::new(Instant::now()),
            worker: None,
            ended: None,
            wake,
        }
    }

    /// Whether checkpoints are being kept.
    pub fn is_on(&self) -> bool {
        self.phase == Phase::On
    }

    /// The app sent `SetCheckpointKey`: with a key, start (or go on with the new key); without, stop. The one
    /// thread is started the first time and told the rest over its channel.
    pub fn configure(&mut self, source: Box<dyn Source>, key: Option<Key>, now: Instant) {
        let sink = Box::new(DirSink::new(self.sessions_dir.clone(), self.id.clone()));
        self.configure_with(source, sink, key, now);
    }

    /// [`configure`](Self::configure) with the sink given, for the tests.
    pub fn configure_with(
        &mut self,
        source: Box<dyn Source>,
        sink: Box<dyn Sink>,
        key: Option<Key>,
        now: Instant,
    ) {
        if self.phase == Phase::Closed {
            return;
        }
        let Some(key) = key else {
            if let Some(worker) = &self.worker {
                worker.set_key(None);
            }
            self.phase = Phase::Stopped;
            return;
        };
        match &self.worker {
            Some(worker) => worker.set_key(Some(key)),
            None => {
                let wake = self.wake.clone();
                match worker::Worker::start(source, sink, key, Box::new(move || wake())) {
                    Ok(worker) => self.worker = Some(worker),
                    Err(err) => {
                        tracing::warn!("no thread for checkpoints: {err}");
                        return;
                    }
                }
            }
        }
        self.phase = Phase::On;
        // The first checkpoint is for the screen as it is: due once, after the quiet time.
        self.schedule = Schedule::new(now);
        self.schedule.force(now);
    }

    fn working(&mut self) -> Option<(&mut Schedule, &Worker, &Option<String>)> {
        if self.phase != Phase::On {
            return None;
        }
        Some((&mut self.schedule, self.worker.as_ref()?, &self.ended))
    }

    /// Each turn of the driver: `output_total` is what the program has printed so far. Collects what the
    /// worker did, asks for a write when one is due, and says when to look again. The generations the app asked to
    /// hear of (`Checkpoint{flush}`) that landed come back in the second part.
    pub fn pump(&mut self, output_total: u64, now: Instant) -> (Option<Instant>, Vec<u64>) {
        let mut acks = Vec::new();
        if let Some(worker) = &self.worker {
            for done in worker.finished() {
                self.schedule.charge(done.bytes, now);
                if done.ack
                    && let Some(generation) = done.generation
                {
                    acks.push(generation);
                }
            }
        }
        let Some((schedule, worker, ended)) = self.working() else { return (None, acks) };
        schedule.note_output(output_total, now);
        if schedule.due(now) {
            schedule.requested();
            worker.submit(Job { ended: ended.clone(), only: false, ack: false, reply: None });
        }
        (schedule.due_at(now), acks)
    }

    /// The app asked for a checkpoint (`Checkpoint`): with `flush`, always, and the host says when it is on
    /// disk; without, only if something was printed since the last. (The budget is charged like any other write;
    /// only [`flush`](Self::flush), the last write, ignores it.)
    pub fn request(&mut self, flush: bool) {
        let Some((schedule, worker, ended)) = self.working() else { return };
        if !flush && !schedule.is_dirty() {
            return;
        }
        schedule.requested();
        worker.submit(Job { ended: ended.clone(), only: false, ack: flush, reply: None });
    }

    /// The buffer was cleared: a new generation now, and none before it kept.
    pub fn cleared(&mut self) {
        let Some((schedule, worker, ended)) = self.working() else { return };
        schedule.requested();
        worker.submit(Job { ended: ended.clone(), only: true, ack: false, reply: None });
    }

    /// The program ended: the last generation says how, and later ones keep saying it. Returns at once; the
    /// worker writes it.
    pub fn ended(&mut self, info: &wire::ExitInfo) {
        self.ended = Some(ended_text(info));
        let Some((schedule, worker, ended)) = self.working() else { return };
        schedule.requested();
        worker.submit(Job { ended: ended.clone(), only: false, ack: false, reply: None });
    }

    /// Write a checkpoint now and wait for it, at most `within`: the host is about to end. The budget does
    /// not apply.
    pub fn flush(&mut self, within: Duration) -> Flushed {
        let Some((schedule, worker, ended)) = self.working() else { return Flushed::Off };
        let (reply, wait) = mpsc::channel();
        schedule.requested();
        worker.submit(Job { ended: ended.clone(), only: false, ack: false, reply: Some(reply) });
        match wait.recv_timeout(within) {
            Ok(Done { error: Some(err), .. }) => Flushed::Failed(err),
            Ok(done) => Flushed::Done(done.bytes),
            Err(_) => Flushed::TimedOut,
        }
    }

    /// The terminal is being closed on purpose: nothing more is written, not even what is queued or the exit
    /// that follows the hangup.
    pub fn close(&mut self) {
        if let Some(worker) = self.worker.take() {
            worker.close();
        }
        self.phase = Phase::Closed;
    }
}

#[cfg(test)]
mod tests;
