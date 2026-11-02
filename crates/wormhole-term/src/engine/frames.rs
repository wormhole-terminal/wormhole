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
//! Limits, because the data is hostile: at most [`JOBS_MAX`] jobs and [`QUEUE_BYTES`] bytes of
//! frame data wait for the worker (past it a frame is refused with `EFBIG`, and a composition is
//! dropped with a log line: the program was already told `OK`); a frame is decoded under the same
//! limits as a picture; the canvases count in the store's budgets, and a frame that does not
//! fit is refused (`EFBIG`). A frame that fails stays in the engine's count when it is not the last,
//! and here it has no pixels, so it shows nothing; later frames built on it start from a blank
//! canvas.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{SyncSender, TrySendError, sync_channel};
use std::sync::{Arc, OnceLock};
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
    Clear(ImageHandle),
}

/// The worker of one terminal, started with the first frame.
pub(super) struct FrameWorker {
    proxy: EventProxy,
    screen: Arc<FairLock<Screen>>,
    /// Send the engine's replies to the program (not a replica's).
    replies: bool,
    queue: OnceLock<Option<SyncSender<Job>>>,
    /// Bytes of frame data in the queue.
    queued: Arc<AtomicUsize>,
}

impl FrameWorker {
    pub(super) fn new(proxy: EventProxy, screen: Arc<FairLock<Screen>>, replies: bool) -> Self {
        Self { proxy, screen, replies, queue: OnceLock::new(), queued: Arc::new(AtomicUsize::new(0)) }
    }

    /// A frame's data arrived (the screen is locked, being read): do it in turn, and answer the
    /// engine when done.
    pub(super) fn frame(&self, screen: &mut Screen, upload: FrameUpload) {
        let ticket = upload.ticket;
        let bytes = upload.data.len();
        if self.queued.load(Ordering::Acquire) + bytes > QUEUE_BYTES {
            tracing::info!(frame = upload.frame, "animation frame dropped: too much waits to be decoded");
            return screen.term.frame_decoded(ticket, Err(ImageError::TooBig));
        }
        self.queued.fetch_add(bytes, Ordering::AcqRel);
        if !self.send(Job::Frame(upload)) {
            self.queued.fetch_sub(bytes, Ordering::AcqRel);
            screen.term.frame_decoded(ticket, Err(ImageError::TooBig));
        }
    }

    /// A composition arrived. Nothing waits for its answer: the engine said `OK` already.
    pub(super) fn compose(&self, job: FrameCompose) {
        if !self.send(Job::Compose(job)) {
            tracing::warn!("animation composition dropped: too many wait for the frame worker");
        }
    }

    /// The program deleted the frames of a picture.
    pub(super) fn clear(&self, image: ImageHandle) {
        if !self.send(Job::Clear(image)) {
            // Not a thing to lose: do it now. The pixels it would free are the store's.
            self.proxy.shared().images().clear_frames(key(image));
        }
    }

    fn send(&self, job: Job) -> bool {
        let queue = self.queue.get_or_init(|| self.start());
        let Some(queue) = queue else { return false };
        match queue.try_send(job) {
            Ok(()) => true,
            Err(TrySendError::Full(_) | TrySendError::Disconnected(_)) => false,
        }
    }

    fn start(&self) -> Option<SyncSender<Job>> {
        let (tx, rx) = sync_channel::<Job>(JOBS_MAX);
        let (proxy, screen, replies, queued) =
            (self.proxy.clone(), self.screen.clone(), self.replies, self.queued.clone());
        let spawned = std::thread::Builder::new().name("frame-worker".into()).spawn(move || {
            while let Ok(job) = rx.recv() {
                let shared = proxy.shared();
                match job {
                    Job::Frame(upload) => {
                        let (ticket, bytes) = (upload.ticket, upload.data.len());
                        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            frame_pixels(shared, &upload)
                        }))
                        .unwrap_or_else(|_| {
                            tracing::error!("the animation frame decoder panicked");
                            Err(ImageError::BadData)
                        });
                        queued.fetch_sub(bytes, Ordering::AcqRel);
                        answer(&proxy, &screen, ticket, result, replies);
                    }
                    Job::Compose(job) => {
                        let done = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                            compose_pixels(shared, &job)
                        }));
                        if !matches!(done, Ok(true)) {
                            tracing::warn!("animation composition could not be done");
                        }
                        proxy.wakeup();
                    }
                    Job::Clear(image) => {
                        shared.images().clear_frames(key(image));
                        proxy.wakeup();
                    }
                }
            }
        });
        match spawned {
            Ok(_) => Some(tx),
            Err(err) => {
                tracing::warn!("animation frames dropped: no thread to build them on: {err}");
                None
            }
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
    if let Err(error) = result {
        tracing::warn!(?error, "animation frame dropped");
    }
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

/// Build the frame an upload describes and keep it.
fn frame_pixels(shared: &Shared, upload: &FrameUpload) -> Result<(), ImageError> {
    let id = key(upload.image);
    let size = root_size(shared, id).ok_or(ImageError::BadData)?;
    let rect = upload.rect;
    let data = decode_data(upload.format, upload.compression, (rect.width, rect.height), &upload.data)?;
    let frame = (upload.frame as usize).checked_sub(1).ok_or(ImageError::BadData)?;
    let mut canvas = match upload.base {
        FrameBase::Color(color) => solid(size.0, size.1, color),
        FrameBase::Frame(n) => canvas_of(shared, id, (n as usize).saturating_sub(1), size),
        FrameBase::Itself => canvas_of(shared, id, frame, size),
        // A base a newer engine adds is nothing this knows; the frame starts blank.
        _ => solid(size.0, size.1, 0),
    };
    let whole = FrameRect { x: 0, y: 0, width: rect.width, height: rect.height };
    if !blit(&mut canvas, &data, whole, (rect.x, rect.y), upload.blend) {
        return Err(ImageError::BadData);
    }
    let kept = shared.images().set_frame(id, frame, Arc::new(canvas));
    if kept { Ok(()) } else { Err(ImageError::TooBig) }
}

/// Do a composition. Whether it was done.
fn compose_pixels(shared: &Shared, job: &FrameCompose) -> bool {
    let id = key(job.image);
    let Some(size) = root_size(shared, id) else { return false };
    let (source, destination) = (job.source as usize - 1, job.destination as usize - 1);
    let from = canvas_of(shared, id, source, size);
    let mut canvas = canvas_of(shared, id, destination, size);
    if !blit(&mut canvas, &from, job.from, (job.to.x, job.to.y), job.blend) {
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
