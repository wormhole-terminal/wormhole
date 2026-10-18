//! The pictures the engine hands over: decode them (the engine has no decoder for PNG, JPEG, GIF
//! or zlib), keep the pixels in the terminal's [`ImageStore`] under the picture's handle, tell the
//! engine, and drop them when it says the picture is gone.
//!
//! The engine owns where a picture is (placements in the grid, which scroll and reflow with the
//! text); this owns the pixels, under the budgets `images::store` has always kept (per terminal
//! and in all). A picture is answered to the engine exactly once, by [`Terminal::image_decoded`]:
//! at once for raw RGBA and for a refusal, from a thread of its own for anything that needs a
//! decoder (a PNG of a screenshot takes tens of milliseconds, which the reader must not spend
//! while the UI waits for the screen). The program's reply to a kitty command waits for that, and
//! so do the replies after it (the engine holds them).

use std::sync::Arc;

use wormhole_vt::image::ImageHandle;
use wormhole_vt::image::{Compression, ImageError, ImageFormat, ImageUpload};

use crate::events::{EventProxy, Shared};
use crate::images::{DecodeError, Format, Image, Limits, decode};
use crate::pty::FairLock;
use crate::screen::Screen;

/// The key of a picture's pixels in the store: its handle, which counts up from 1 and is never
/// reused. The store's keys are 32 bits; a terminal that sent four billion pictures would have
/// had the first evicted long ago.
pub(crate) fn key(handle: ImageHandle) -> u32 {
    handle.get() as u32
}

/// What to do with the pictures a terminal's engine sends.
pub(super) struct Uploads {
    proxy: EventProxy,
    screen: Arc<FairLock<Screen>>,
    /// The engine's replies after a decode go to the program; a replica's go nowhere (the host
    /// answered the program from its own decode).
    replies: bool,
}

impl Uploads {
    pub(super) fn new(proxy: EventProxy, screen: Arc<FairLock<Screen>>) -> Self {
        Self { proxy, screen, replies: true }
    }

    pub(super) fn replica(proxy: EventProxy, screen: Arc<FairLock<Screen>>) -> Self {
        Self { replies: false, ..Self::new(proxy, screen) }
    }

    /// A picture arrived (the screen is locked, being read). Answer the engine now or when the
    /// decode is done.
    pub(super) fn begin(&self, screen: &mut Screen, upload: ImageUpload) {
        let handle = upload.image;
        let id = key(handle);
        let shared = self.proxy.shared();
        if !shared.images().reserve(id, upload.width, upload.height) {
            tracing::info!(id, "inline image dropped: no room for it");
            return screen.term.image_decoded(handle, Err(ImageError::TooBig));
        }
        let pixels = u64::from(upload.width) * u64::from(upload.height) * 4;
        // Raw pixels need no decoder: keep them as they came.
        if upload.format == ImageFormat::Rgba
            && upload.compression == Compression::None
            && upload.data.len() as u64 == pixels
        {
            let image = Image { width: upload.width, height: upload.height, rgba: upload.data };
            let kept = shared.images().insert_decoded(id, Arc::new(image));
            return self.answer(screen, handle, kept.then_some(()).ok_or(ImageError::TooBig));
        }
        if !shared.begin_decode() {
            tracing::warn!(
                id,
                "inline image dropped: {} are being decoded already",
                crate::images::DECODES_MAX
            );
            shared.images().remove(id);
            return screen.term.image_decoded(handle, Err(ImageError::TooBig));
        }
        let (proxy, locked, replies) = (self.proxy.clone(), self.screen.clone(), self.replies);
        let spawned = std::thread::Builder::new()
            .name("image-decode".into())
            .spawn(move || settle(&proxy, &locked, handle, replies, || decoded(&upload)));
        if let Err(err) = spawned {
            tracing::warn!(id, "inline image dropped: no thread to decode it on: {err}");
            shared.end_decode();
            shared.images().remove(id);
            screen.term.image_decoded(handle, Err(ImageError::TooBig));
        }
    }

    /// The engine let go of a picture.
    pub(super) fn free(&self, handle: ImageHandle) {
        self.proxy.shared().images().remove(key(handle));
    }

    fn answer(&self, screen: &mut Screen, handle: ImageHandle, outcome: Result<(), ImageError>) {
        if outcome.is_err() {
            self.proxy.shared().images().remove(key(handle));
        }
        screen.term.image_decoded(handle, outcome);
    }
}

/// Releases the decode's place when the thread ends however it ends.
struct DecodeSlot<'a>(&'a Shared);

impl Drop for DecodeSlot<'_> {
    fn drop(&mut self) {
        self.0.end_decode();
    }
}

/// Answers the engine for a picture exactly once, however the thread that decodes it ends: the
/// engine holds every reply behind a picture's until then, so a thread that died without
/// answering would leave the program's later replies (a cursor report, say) waiting for good.
struct Answer<'a> {
    proxy: &'a EventProxy,
    screen: &'a FairLock<Screen>,
    handle: ImageHandle,
    outcome: Option<Result<(), ImageError>>,
    /// Send the engine's replies to the program (not a replica's).
    replies: bool,
}

impl Drop for Answer<'_> {
    fn drop(&mut self) {
        let outcome = self.outcome.take().unwrap_or(Err(ImageError::BadData));
        let shared = self.proxy.shared();
        if let Err(error) = outcome {
            tracing::warn!(id = key(self.handle), ?error, "inline image dropped");
            shared.images().remove(key(self.handle));
        }
        // The screen's lock last, and never together with the store's.
        let mut screen = self.screen.lock();
        screen.term.image_decoded(self.handle, outcome);
        if !screen.term.replies().is_empty() {
            if self.replies {
                shared.reply(screen.term.replies().to_vec());
            }
            screen.term.clear_replies();
        }
        drop(screen);
        // Bumps the revision and sends `Output`, as printed text does: the picture shows.
        self.proxy.wakeup();
    }
}

/// [`settle`] as a local terminal's decode thread does it.
#[cfg(test)]
pub(super) fn settle_decode(
    proxy: &EventProxy,
    screen: &FairLock<Screen>,
    handle: ImageHandle,
    decode: impl FnOnce() -> Result<Image, ImageError>,
) {
    settle(proxy, screen, handle, true, decode);
}

/// The decode thread's work: run `decode`, keep the pixels and answer the engine (and, when
/// `replies`, the program with what the engine then says). A decoder that panics on hostile data
/// is a refusal like any other bad data.
fn settle(
    proxy: &EventProxy,
    screen: &FairLock<Screen>,
    handle: ImageHandle,
    replies: bool,
    decode: impl FnOnce() -> Result<Image, ImageError>,
) {
    let shared = proxy.shared();
    let id = key(handle);
    // Dropped in reverse: the place is free again before the engine hears.
    let mut answer = Answer { proxy, screen, handle, outcome: None, replies };
    let _slot = DecodeSlot(shared);
    let started = std::time::Instant::now();
    let decoded = std::panic::catch_unwind(std::panic::AssertUnwindSafe(decode)).unwrap_or_else(|_| {
        tracing::error!(id, "the image decoder panicked");
        Err(ImageError::BadData)
    });
    let outcome = decoded.and_then(|image| {
        let kept = shared.images().insert_decoded(id, Arc::new(image));
        kept.then_some(()).ok_or(ImageError::TooBig)
    });
    if outcome.is_ok() {
        tracing::debug!(id, took = ?started.elapsed(), "inline image decoded");
    }
    answer.outcome = Some(outcome);
}

/// The upload as pixels, within the limits, and exactly the size the engine measured.
fn decoded(upload: &ImageUpload) -> Result<Image, ImageError> {
    let (width, height) = (upload.width, upload.height);
    let format = match upload.format {
        ImageFormat::Rgb => Format::Rgb { width, height },
        ImageFormat::Rgba => Format::Rgba { width, height },
        ImageFormat::Png => Format::Png,
        ImageFormat::File => Format::Any,
        // A format a newer engine adds is one this decoder does not read.
        _ => return Err(ImageError::Unsupported),
    };
    let compressed = upload.compression == Compression::Zlib;
    match decode(format, &upload.data, compressed, &Limits::default()) {
        Ok(image) if (image.width, image.height) == (width, height) => Ok(image),
        Ok(_) => Err(ImageError::BadData),
        Err(DecodeError::TooBig) => Err(ImageError::TooBig),
        Err(DecodeError::Invalid(_)) => Err(ImageError::BadData),
    }
}
