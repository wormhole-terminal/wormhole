//! The checkpoint thread: the only place the host reads a screen for a checkpoint and the only place it
//! writes one. The PTY thread parses and the driver thread serves; neither waits for a disk. A job is "write
//! one now"; the thread reads the terminal (`Source`), seals and writes it (`Sink`), and says what it cost.

use std::io;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::{SystemTime, UNIX_EPOCH};

use wormhole_term::checkpoint::{Key, Snapshot, Store, Written};
use wormhole_term::hosted::Capture;

/// The name of the thread, which the tests look for to see where the work happened.
pub const THREAD_NAME: &str = "wormhole-checkpoint";

/// What a checkpoint reads of the terminal. Called on the checkpoint thread only.
pub trait Source: Send + Sync {
    fn capture(&self) -> Capture;
}

/// Where a checkpoint goes. Called on the checkpoint thread only.
pub trait Sink: Send {
    fn write(&mut self, key: Option<&Key>, snapshot: &Snapshot<'_>) -> io::Result<Written>;
}

/// The real sink: the terminal's directory under `sessions.noindex`, made when the first write comes.
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
    fn write(&mut self, key: Option<&Key>, snapshot: &Snapshot<'_>) -> io::Result<Written> {
        if self.store.is_none() {
            self.store = Some(Store::open(&self.sessions_dir, &self.id)?);
        }
        match &mut self.store {
            Some(store) => store.write(key, snapshot),
            None => Err(io::Error::other("no store")),
        }
    }
}

/// What one job came to.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Done {
    /// The bytes put on disk (0 when nothing was written).
    pub bytes: u64,
    pub wrote: bool,
    pub error: Option<String>,
}

pub struct Job {
    /// How the program ended, if it has: written into the manifest.
    pub ended: Option<String>,
    /// Told when the job is done, for a caller that waits (a flush).
    pub reply: Option<mpsc::Sender<Done>>,
}

/// What the app told the host: the key (none: the app has none) and the shell's key.
#[derive(Clone)]
pub struct Settings {
    pub key: Option<Key>,
    pub shell: String,
}

pub struct Worker {
    jobs: mpsc::Sender<Job>,
    done: mpsc::Receiver<Done>,
}

fn unix_now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

impl Worker {
    pub fn start(source: Box<dyn Source>, mut sink: Box<dyn Sink>, settings: Settings) -> io::Result<Self> {
        let (jobs, rx) = mpsc::channel::<Job>();
        let (done_tx, done) = mpsc::channel::<Done>();
        thread::Builder::new().name(THREAD_NAME.into()).spawn(move || {
            while let Ok(job) = rx.recv() {
                let result = run(&*source, &mut *sink, &settings, job.ended.as_deref());
                if let Some(reply) = job.reply {
                    let _ = reply.send(result.clone());
                }
                if done_tx.send(result).is_err() {
                    return;
                }
            }
        })?;
        Ok(Self { jobs, done })
    }

    /// Ask for a write; returns at once.
    pub fn submit(&self, job: Job) {
        let _ = self.jobs.send(job);
    }

    /// The jobs that finished since the last call.
    pub fn finished(&self) -> Vec<Done> {
        self.done.try_iter().collect()
    }
}

fn run(source: &dyn Source, sink: &mut dyn Sink, settings: &Settings, ended: Option<&str>) -> Done {
    let capture = source.capture();
    // With a key the screen is part of every generation; a moment with none (a full-screen program shows the
    // other screen, or nothing is printed) writes nothing, so the generation before it, which has the screen,
    // stays the newest.
    if settings.key.is_some() && capture.screen.is_none() {
        return Done::default();
    }
    let snapshot = Snapshot {
        time: unix_now(),
        cols: capture.cols,
        rows: capture.rows,
        cwd: capture.cwd.as_deref(),
        shell: Some(settings.shell.as_str()).filter(|s| !s.is_empty()),
        title: capture.title.as_deref(),
        ended,
        screen: capture.screen.as_deref(),
    };
    match sink.write(settings.key.as_ref(), &snapshot) {
        Ok(written) => Done { bytes: written.bytes, wrote: true, error: None },
        Err(err) => {
            tracing::warn!("a checkpoint could not be written: {err}");
            Done { bytes: 0, wrote: false, error: Some(err.to_string()) }
        }
    }
}
