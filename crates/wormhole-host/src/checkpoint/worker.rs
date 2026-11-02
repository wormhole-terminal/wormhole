//! The checkpoint thread: the only place the host reads a screen for a checkpoint and the only place it
//! writes one. The PTY thread parses and the driver thread serves; neither waits for a disk. There is **one**
//! thread for the life of the host: it is told what to do over a channel (a job to write, a new key, to stop), and
//! a new key never starts a second one.

use std::io;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use wormhole_term::checkpoint::{Key, Snapshot, Store, Written};

/// The name of the thread, which the tests look for to see where the work happened.
pub const THREAD_NAME: &str = "wormhole-checkpoint";

/// What a checkpoint reads of the terminal. Called on the checkpoint thread only.
pub trait Source: Send + Sync {
    /// The primary screen as VT bytes; `None` while a full-screen program shows the other screen, or nothing is
    /// printed (then nothing is written, and the generation before stays the newest).
    fn screen(&self) -> Option<Vec<u8>>;
}

/// Where a checkpoint goes. Called on the checkpoint thread only.
pub trait Sink: Send {
    /// Write a generation; with `only`, drop every generation before it (Clear Buffer).
    fn write(&mut self, key: &Key, snapshot: &Snapshot<'_>, only: bool) -> io::Result<Written>;
}

/// The real sink: the terminal's directory under `sessions.noindex`, made when the first write comes. It is
/// made once: a directory the app removed (the terminal was closed) is not made again.
pub struct DirSink {
    pub sessions_dir: PathBuf,
    pub id: String,
    store: Option<Store>,
}

impl DirSink {
    pub fn new(sessions_dir: PathBuf, id: String) -> Self {
        Self { sessions_dir, id, store: None }
    }
}

impl Sink for DirSink {
    fn write(&mut self, key: &Key, snapshot: &Snapshot<'_>, only: bool) -> io::Result<Written> {
        if self.store.is_none() {
            self.store = Some(Store::open(&self.sessions_dir, &self.id)?);
        }
        let Some(store) = &mut self.store else { return Err(io::Error::other("no store")) };
        if only { store.write_only(key, snapshot) } else { store.write(key, snapshot) }
    }
}

/// What one job came to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Done {
    /// The generation written, if one was.
    pub generation: Option<u64>,
    /// The bytes put on disk (0 when nothing was written).
    pub bytes: u64,
    pub error: Option<String>,
    /// The job asked to be told (`Checkpoint{flush}`): the driver passes it to the app.
    pub ack: bool,
}

/// One write.
pub struct Job {
    /// How the program ended, if it has: written into the manifest.
    pub ended: Option<String>,
    /// Drop every earlier generation (Clear Buffer).
    pub only: bool,
    /// The app asked to be told when this is on disk.
    pub ack: bool,
    /// Told when the job is done, for a caller that waits (a flush).
    pub reply: Option<mpsc::Sender<Done>>,
}

enum Msg {
    Job(Job),
    /// A new key, or none: stop writing (the feature was turned off).
    Key(Option<Key>),
    /// The terminal is being closed: write nothing more, jobs still queued included, and end.
    Close,
}

pub struct Worker {
    msgs: mpsc::Sender<Msg>,
    done: mpsc::Receiver<Done>,
}

fn unix_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

impl Worker {
    /// Start the thread. `wake` is called after each job, from the thread, so the driver looks at what was done.
    pub fn start(
        source: Box<dyn Source>,
        mut sink: Box<dyn Sink>,
        key: Key,
        wake: Box<dyn Fn() + Send>,
    ) -> io::Result<Self> {
        let (msgs, rx) = mpsc::channel::<Msg>();
        let (done_tx, done) = mpsc::channel::<Done>();
        thread::Builder::new().name(THREAD_NAME.into()).spawn(move || {
            let mut key = Some(key);
            while let Ok(msg) = rx.recv() {
                match msg {
                    Msg::Key(new) => key = new,
                    Msg::Close => return,
                    Msg::Job(job) => {
                        let result = match &key {
                            Some(key) => run(&*source, &mut *sink, key, &job),
                            None => Done::default(),
                        };
                        if let Some(reply) = &job.reply {
                            let _ = reply.send(result.clone());
                        }
                        if done_tx.send(result).is_err() {
                            return;
                        }
                        wake();
                    }
                }
            }
        })?;
        Ok(Self { msgs, done })
    }

    /// Ask for a write; returns at once.
    pub fn submit(&self, job: Job) {
        let _ = self.msgs.send(Msg::Job(job));
    }

    /// Use `key` from now on, or (none) write nothing more.
    pub fn set_key(&self, key: Option<Key>) {
        let _ = self.msgs.send(Msg::Key(key));
    }

    /// The terminal is closed: end the thread without writing what is still queued.
    pub fn close(&self) {
        let _ = self.msgs.send(Msg::Close);
    }

    /// The jobs that finished since the last call.
    pub fn finished(&self) -> Vec<Done> {
        self.done.try_iter().collect()
    }
}

fn run(source: &dyn Source, sink: &mut dyn Sink, key: &Key, job: &Job) -> Done {
    // A moment with no primary screen (a full-screen program shows the other one, or nothing is printed) writes
    // nothing, so the generation before it, which has the screen, stays the newest.
    let Some(screen) = source.screen() else { return Done { ack: job.ack, ..Done::default() } };
    let snapshot = Snapshot { time: unix_now(), ended: job.ended.as_deref(), screen: &screen };
    match sink.write(key, &snapshot, job.only) {
        Ok(written) => {
            Done { generation: Some(written.generation), bytes: written.bytes, error: None, ack: job.ack }
        }
        Err(err) => {
            tracing::warn!("a checkpoint could not be written: {err}");
            Done { generation: None, bytes: 0, error: Some(err.to_string()), ack: job.ack }
        }
    }
}
