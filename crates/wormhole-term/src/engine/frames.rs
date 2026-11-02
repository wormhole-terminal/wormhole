//! The frames of animated pictures (kitty's `a=f` and `a=c`): decode their data, build each
//! frame's canvas, keep it in the terminal's [`ImageStore`](crate::images::ImageStore), and
//! tell the engine.
//!
//! The engine keeps no pixels, so what a frame is made of is this module's work, and it has to be
//! done **in the order the engine sent it**: a composition reads a frame whose upload may still
//! be decoding. Pictures decode on threads of their own, in no order; frames go through one worker
//! thread per terminal, started with the first, that does the jobs one after the other. A frame
//! waits for its picture's own decode (a PNG root can take tens of milliseconds) before it uses it.
//!
//! # Nothing is dropped, nothing waits under the lock
//!
//! The engine has already told the program `OK` for a composition and for a frame whose data is raw
//! pixels, so a job must never be lost. The jobs of a read are **collected** while the screen's lock
//! is held (`FrameWorker::frame` and the others) and **sent** once it is released (`flush`, called
//! by the reader after each slice: `Feed::after_slice`). Sending waits when the worker has too much
//! queued (64 jobs, 128 MiB of data), which holds the PTY back until it catches up, as a program
//! writing into a full pipe is. Waiting is only safe outside the lock: the worker takes it to
//! answer the engine.
//!
//! Where a frame ends up is the engine's decision alone: a frame whose data cannot be decoded is
//! still a frame, built from its base canvas without the data, and the program is told
//! (`ENODATA`). A frame that does not fit the store has no canvas, and the one before it is
//! shown in its place.
//!
//! A terminal that draws nothing (a session host's) only decodes the data, to know whether the
//! program's reply is `OK`: it keeps no canvases and does no compositions.

use std::cell::RefCell;
use std::sync::mpsc::{SyncSender, sync_channel};
use std::sync::{Arc, Condvar, Mutex, OnceLock, PoisonError};
use std::time::{Duration, Instant};

use wormhole_vt::image::{
    Compression, FrameBase, FrameCompose, FrameRect, FrameTicket, FrameUpload, ImageError, ImageFormat,
    ImageHandle,
};

use crate::events::{EventProxy, Shared};
use crate::images::compose::{blit, solid};
use crate::images::{DecodeError, Format, Image, Limits, decode, key_of as key};
use crate::pty::FairLock;
use crate::screen::Screen;

/// Most jobs waiting for the worker.
pub(super) const JOBS_MAX: usize = 64;
/// Most bytes of frame data waiting for the worker.
pub(super) const QUEUE_BYTES: usize = 128 * 1024 * 1024;
/// How long a frame waits for its picture to finish decoding before it gives up.
const ROOT_WAIT: Duration = Duration::from_secs(10);

enum Job {
    Frame(FrameUpload),
    Compose(FrameCompose),
    Delete { image: ImageHandle, frame: u32 },
}

impl Job {
    /// The bytes of data the job holds, for the queue's budget.
    fn bytes(&self) -> usize {
        match self {
            Job::Frame(upload) => upload.data.len(),
            Job::Compose(_) | Job::Delete { .. } => 0,
        }
    }
}

/// How many bytes of frame data wait for the worker; a sender waits while there are too many.
#[derive(Default)]
struct Budget {
    used: Mutex<usize>,
    freed: Condvar,
}

impl Budget {
    /// Wait until `bytes` more may wait (always when nothing does: a single frame is never refused
    /// for being large, the engine capped it already).
    fn acquire(&self, bytes: usize) {
        let mut used = self.used.lock().unwrap_or_else(PoisonError::into_inner);
        while *used > 0 && *used + bytes > QUEUE_BYTES {
            used = self.freed.wait(used).unwrap_or_else(PoisonError::into_inner);
        }
        *used += bytes;
    }

    fn release(&self, bytes: usize) {
        *self.used.lock().unwrap_or_else(PoisonError::into_inner) -= bytes;
        self.freed.notify_all();
    }
}

/// A line in the log at most once a second, with how many it stood for: a program that sends
/// thousands of frames that fail must not fill the log.
#[derive(Default)]
struct LogGate {
    last: Mutex<Option<(Instant, usize)>>,
}

impl LogGate {
    fn warn(&self, what: &str) {
        let mut last = self.last.lock().unwrap_or_else(PoisonError::into_inner);
        match *last {
            Some((since, skipped)) if since.elapsed() < Duration::from_secs(1) => {
                *last = Some((since, skipped + 1));
            }
            Some((_, skipped)) => {
                tracing::warn!(skipped, "{what}");
                *last = Some((Instant::now(), 0));
            }
            None => {
                tracing::warn!(skipped = 0usize, "{what}");
                *last = Some((Instant::now(), 0));
            }
        }
    }
}

/// The worker of one terminal, started with the first frame.
pub(super) struct FrameWorker {
    proxy: EventProxy,
    screen: Arc<FairLock<Screen>>,
    /// Send the engine's replies to the program (not a replica's).
    replies: bool,
    /// Keep the canvases (a terminal that draws), or only decode the data (a session host).
    keep: bool,
    /// Collected while the screen is locked, sent when it is not.
    pending: RefCell<Vec<Job>>,
    queue: OnceLock<Option<SyncSender<Job>>>,
    budget: Arc<Budget>,
    log: Arc<LogGate>,
}

impl FrameWorker {
    pub(super) fn new(proxy: EventProxy, screen: Arc<FairLock<Screen>>, replies: bool, keep: bool) -> Self {
        Self {
            proxy,
            screen,
            replies,
            keep,
            pending: RefCell::new(Vec::new()),
            queue: OnceLock::new(),
            budget: Arc::default(),
            log: Arc::default(),
        }
    }

    /// A frame's data arrived (the screen is locked, being read): do it in turn, and answer the
    /// engine when done.
    pub(super) fn frame(&self, upload: FrameUpload) {
        self.pending.borrow_mut().push(Job::Frame(upload));
    }

    /// A composition arrived. Nothing waits for its answer: the engine said `OK` already.
    pub(super) fn compose(&self, job: FrameCompose) {
        if self.keep {
            self.pending.borrow_mut().push(Job::Compose(job));
        }
    }

    /// The program deleted a frame. In turn, behind the jobs that came before it.
    pub(super) fn delete(&self, image: ImageHandle, frame: u32) {
        if self.keep {
            self.pending.borrow_mut().push(Job::Delete { image, frame });
        }
    }

    /// The screen's lock is released: send what was collected, in order. Waits while the worker
    /// has too much queued.
    pub(super) fn flush(&self) {
        let jobs = std::mem::take(&mut *self.pending.borrow_mut());
        for job in jobs {
            let bytes = job.bytes();
            self.budget.acquire(bytes);
            let Some(queue) = self.queue.get_or_init(|| self.start()).as_ref() else {
                self.give_up(job, bytes);
                continue;
            };
            if let Err(rejected) = queue.send(job) {
                self.give_up(rejected.0, bytes);
            }
        }
    }

    fn start(&self) -> Option<SyncSender<Job>> {
        let (tx, rx) = sync_channel::<Job>(JOBS_MAX);
        let (proxy, screen, replies, keep) =
            (self.proxy.clone(), self.screen.clone(), self.replies, self.keep);
        let (budget, log) = (self.budget.clone(), self.log.clone());
        let spawned = std::thread::Builder::new().name("frame-worker".into()).spawn(move || {
            while let Ok(job) = rx.recv() {
                let bytes = job.bytes();
                work(&proxy, &screen, replies, keep, &log, job);
                budget.release(bytes);
            }
        });
        match spawned {
            Ok(_) => Some(tx),
            Err(err) => {
                tracing::warn!("animation frames cannot be built: no thread for them: {err}");
                None
            }
        }
    }

    /// A job that could not be sent: a frame is answered with an error, the rest is logged.
    fn give_up(&self, job: Job, bytes: usize) {
        self.budget.release(bytes);
        match job {
            Job::Frame(upload) => {
                answer(&self.proxy, &self.screen, upload.ticket, Err(ImageError::TooBig), self.replies)
            }
            _ => self.log.warn("an animation job was lost: the frame worker is gone"),
        }
    }
}

/// Do one job on the worker's thread.
fn work(proxy: &EventProxy, screen: &FairLock<Screen>, replies: bool, keep: bool, log: &LogGate, job: Job) {
    let shared = proxy.shared();
    match job {
        Job::Frame(upload) => {
            let ticket = upload.ticket;
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                if keep { frame_pixels(shared, &upload) } else { validate(&upload) }
            }))
            .unwrap_or_else(|_| {
                tracing::error!("the animation frame decoder panicked");
                Err(ImageError::BadData)
            });
            if let Err(error) = result {
                log.warn(&format!("an animation frame's data was refused: {error}"));
            }
            answer(proxy, screen, ticket, result, replies);
        }
        Job::Compose(job) => {
            let done =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| compose_pixels(shared, &job)));
            if !matches!(done, Ok(true)) {
                log.warn("an animation composition could not be done");
            }
            proxy.wakeup();
        }
        Job::Delete { image, frame } => {
            shared.images().remove_frame(key(image), (frame as usize).saturating_sub(1));
            proxy.wakeup();
        }
    }
}

/// Tell the engine how the frame went (exactly once), and the program what it then says.
fn answer(
    proxy: &EventProxy,
    screen: &FairLock<Screen>,
    ticket: FrameTicket,
    result: Result<(), ImageError>,
    replies: bool,
) {
    // The screen's lock last, and never together with the store's.
    let mut screen = screen.lock();
    screen.term.frame_decoded(ticket, result);
    if !screen.term.replies().is_empty() {
        if replies {
            proxy.shared().reply(screen.term.replies().to_vec());
        }
        screen.term.clear_replies();
    }
    drop(screen);
    // The new frame shows (and the plan may have changed): bumps the revision as text does.
    proxy.wakeup();
}

/// The picture's size once its own decode is done; `None` when it is gone or never finishes.
fn root_size(shared: &Shared, id: u32) -> Option<(u32, u32)> {
    let started = Instant::now();
    loop {
        {
            let store = shared.images();
            let size = store.dimensions(id)?;
            if !store.is_pending(id) {
                return Some(size);
            }
        }
        if started.elapsed() > ROOT_WAIT {
            return None;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// A copy of frame `frame` (counted from 0) of the picture, or a blank canvas of its size when
/// that frame has no pixels.
fn canvas_of(shared: &Shared, id: u32, frame: usize, (width, height): (u32, u32)) -> Image {
    let found = shared.images().frame(id, frame);
    found.map_or_else(|| solid(width, height, 0), |image| (*image).clone())
}

/// Build the frame an upload describes and keep it. The frame is kept whatever became of its data
/// (as the base canvas alone, when the data could not be decoded or does not fit); the result is
/// what the program is told.
fn frame_pixels(shared: &Shared, upload: &FrameUpload) -> Result<(), ImageError> {
    let id = key(upload.image);
    let size = root_size(shared, id).ok_or(ImageError::BadData)?;
    let rect = upload.rect;
    let frame = (upload.frame as usize).checked_sub(1).ok_or(ImageError::BadData)?;
    let mut canvas = match upload.base {
        FrameBase::Color(color) => solid(size.0, size.1, color),
        FrameBase::Frame(n) => canvas_of(shared, id, (n as usize).saturating_sub(1), size),
        FrameBase::Itself => canvas_of(shared, id, frame, size),
        // A base a newer engine adds is nothing this knows; the frame starts blank.
        _ => solid(size.0, size.1, 0),
    };
    let data = decode_data(upload.format, upload.compression, (rect.width, rect.height), &upload.data);
    let put = data.and_then(|data| {
        let whole = FrameRect { x: 0, y: 0, width: rect.width, height: rect.height };
        blit(&mut canvas, &data, whole, (rect.x, rect.y), upload.blend)
            .then_some(())
            .ok_or(ImageError::BadData)
    });
    let kept = shared.images().set_frame(id, frame, Arc::new(canvas));
    match (put, kept) {
        (Err(error), _) => Err(error),
        (Ok(()), true) => Ok(()),
        (Ok(()), false) => Err(ImageError::TooBig),
    }
}

/// Whether the data of a frame decodes, for a terminal that keeps nothing.
fn validate(upload: &FrameUpload) -> Result<(), ImageError> {
    let rect = upload.rect;
    decode_data(upload.format, upload.compression, (rect.width, rect.height), &upload.data).map(drop)
}

/// Do a composition. Whether it was done. The source is read where it is, the destination is
/// copied (it is what changes).
fn compose_pixels(shared: &Shared, job: &FrameCompose) -> bool {
    let id = key(job.image);
    let Some(size) = root_size(shared, id) else { return false };
    let (source, destination) = (job.source as usize - 1, job.destination as usize - 1);
    let from = shared.images().frame(id, source);
    let blank;
    let from: &Image = match &from {
        Some(frame) => frame,
        None => {
            blank = solid(size.0, size.1, 0);
            &blank
        }
    };
    let mut canvas = canvas_of(shared, id, destination, size);
    if !blit(&mut canvas, from, job.from, (job.to.x, job.to.y), job.blend) {
        return false;
    }
    shared.images().set_frame(id, destination, Arc::new(canvas))
}

/// The data of a frame as pixels, exactly the size the engine measured.
pub(super) fn decode_data(
    format: ImageFormat,
    compression: Compression,
    (width, height): (u32, u32),
    data: &[u8],
) -> Result<Image, ImageError> {
    let format = match format {
        ImageFormat::Rgb => Format::Rgb { width, height },
        ImageFormat::Rgba => Format::Rgba { width, height },
        ImageFormat::Png => Format::Png,
        ImageFormat::File => Format::Any,
        // A format a newer engine adds is one this decoder does not read.
        _ => return Err(ImageError::Unsupported),
    };
    match decode(format, data, compression == Compression::Zlib, &Limits::default()) {
        Ok(image) if (image.width, image.height) == (width, height) => Ok(image),
        Ok(_) => Err(ImageError::BadData),
        Err(DecodeError::TooBig) => Err(ImageError::TooBig),
        Err(DecodeError::Invalid(_)) => Err(ImageError::BadData),
    }
}
