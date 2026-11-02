//! The host's checkpoint of its terminal, for a restart of the Mac (`wormhole_term::checkpoint` is the
//! format; `repo-docs/docs/session-host.md` section 8).
//!
//! - `schedule`: when one is due, on a clock the caller gives;
//! - `worker`: the thread that reads the terminal and writes the files, so the PTY thread and the driver
//!   thread never touch a disk for it.
//!
//! [`Checkpointer`] is the driver's side. It does nothing until the app has sent `SetCheckpointKey`, so with
//! the feature off no file is made. The driver calls [`Checkpointer::pump`] each turn; at SIGTERM it calls
//! [`Checkpointer::flush`] *before* the shell is hung up, so what the restart finds is the screen as the person
//! left it. The host never deletes a checkpoint: a restart ends the shell at the same moment as the host, and
//! only the app knows a terminal was closed on purpose.

pub mod schedule;
pub mod worker;

use std::path::PathBuf;
use std::sync::mpsc;
use std::time::{Duration, Instant};

use wormhole_term::checkpoint::Key;
use wormhole_wire as wire;

use schedule::Schedule;
use worker::Worker;
pub use worker::{DirSink, Done, Job, Settings, Sink, Source};

/// How long the host waits for the last checkpoint at SIGTERM: the restart gives it about three seconds
/// before the kill, and the hangup of the shell needs some of those.
pub const FLUSH_WITHIN: Duration = Duration::from_secs(1);

/// What a flush came to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Flushed {
    /// The app never asked for checkpoints (or the worker could not start).
    Off,
    /// Written, this many bytes (0: nothing to write, as with a full-screen program on).
    Done(u64),
    /// The write did not finish in time; it may still, and a half-written generation is never read.
    TimedOut,
    Failed(String),
}

struct Active {
    schedule: Schedule,
    worker: Worker,
    /// How the program ended, once it has: every later generation says so too.
    ended: Option<String>,
}

pub struct Checkpointer {
    sessions_dir: PathBuf,
    id: String,
    active: Option<Active>,
}

/// `exit:<code>` or `signal:<number>`.
pub fn ended_text(info: &wire::ExitInfo) -> String {
    if info.signal != 0 { format!("signal:{}", info.signal) } else { format!("exit:{}", info.code) }
}

impl Checkpointer {
    pub fn new(sessions_dir: PathBuf, id: String) -> Self {
        Self { sessions_dir, id, active: None }
    }

    /// Whether the app asked for checkpoints.
    pub fn is_on(&self) -> bool {
        self.active.is_some()
    }

    /// The app sent `SetCheckpointKey`: start (or restart with the new key) the thread that writes.
    /// `key` is `None` when the app has none, which makes the checkpoint a manifest only.
    pub fn configure(&mut self, source: Box<dyn Source>, key: Option<Key>, shell: String, now: Instant) {
        let sink = Box::new(DirSink::new(self.sessions_dir.clone(), self.id.clone()));
        self.configure_with(source, sink, key, shell, now);
    }

    /// [`configure`](Self::configure) with the sink given, for the tests.
    pub fn configure_with(
        &mut self,
        source: Box<dyn Source>,
        sink: Box<dyn Sink>,
        key: Option<Key>,
        shell: String,
        now: Instant,
    ) {
        let ended = self.active.take().and_then(|a| a.ended);
        match Worker::start(source, sink, Settings { key, shell }) {
            Ok(worker) => {
                let mut schedule = Schedule::new(now);
                // The first checkpoint is for the folder and the shell as they are: due once, soon.
                schedule.force(now);
                self.active = Some(Active { schedule, worker, ended });
            }
            Err(err) => tracing::warn!("no thread for checkpoints: {err}"),
        }
    }

    /// Each turn of the driver: `output_total` is what the program has printed so far. Collects what the
    /// worker did, asks for a write when one is due, and says when to look again.
    pub fn pump(&mut self, output_total: u64, now: Instant) -> Option<Instant> {
        let active = self.active.as_mut()?;
        for done in active.worker.finished() {
            active.schedule.charge(done.bytes, now);
        }
        active.schedule.note_output(output_total, now);
        if active.schedule.due(now) {
            active.schedule.requested();
            active.worker.submit(Job { ended: active.ended.clone(), reply: None });
        }
        active.schedule.due_at(now)
    }

    /// The app asked for a checkpoint (`Checkpoint`): at once, whatever the quiet rule says. (The budget is
    /// charged for it like any other; only [`flush`](Self::flush), the last write, ignores it.)
    pub fn request(&mut self) {
        let Some(active) = self.active.as_mut() else { return };
        active.schedule.requested();
        active.worker.submit(Job { ended: active.ended.clone(), reply: None });
    }

    /// The program ended: the last generation says how, and later ones keep saying it. Waits up to `within`.
    pub fn ended(&mut self, info: &wire::ExitInfo, within: Duration) -> Flushed {
        if let Some(active) = self.active.as_mut() {
            active.ended = Some(ended_text(info));
        }
        self.flush(within)
    }

    /// Write a checkpoint now and wait for it, at most `within`: the host is about to end. The budget does
    /// not apply.
    pub fn flush(&mut self, within: Duration) -> Flushed {
        let Some(active) = self.active.as_mut() else { return Flushed::Off };
        let (reply, wait) = mpsc::channel();
        active.schedule.requested();
        active.worker.submit(Job { ended: active.ended.clone(), reply: Some(reply) });
        match wait.recv_timeout(within) {
            Ok(Done { error: Some(err), .. }) => Flushed::Failed(err),
            Ok(done) => Flushed::Done(done.bytes),
            Err(_) => Flushed::TimedOut,
        }
    }
}

#[cfg(test)]
mod tests;
