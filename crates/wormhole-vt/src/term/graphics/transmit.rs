//! Taking a kitty command in: chunks put together, the picture checked and stored, the answer
//! chosen.
//!
//! Spec: kitty, "Transmitting images" (chunking with `m`, the formats `f=24|32|100`, `o=z`, ids
//! and numbers), "Querying support" (`a=q`) and "Terminal Responses".

use crate::grid::Grid;
use crate::term::{Event, Terminal};
use crate::types::ImageHandle;

use super::apc::{PAYLOAD_MAX, Parsed};
use super::command::{Action, Command, Medium};
use super::images::{IMAGE_BUDGET_BYTES, Image};
use super::reply::{Error, Target};
use super::sniff;
use super::upload::{Compression, ImageError, ImageFormat, ImageUpload};
use super::{QUEUED_BYTES_MAX, Transfer};

/// Most pixels on a side: kitty's own limit.
pub const MAX_DIMENSION: u32 = 10_000;
/// Most pixels one picture may have: 64 MiB of RGBA (a 4K screenshot is 8.3 million).
pub const MAX_PIXELS: u64 = 16 * 1024 * 1024;

/// How a command ended, for the answer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Flow {
    /// Answer now. `assigned` is the id the terminal gave a numbered picture.
    Done { assigned: Option<u32> },
    /// Answer when the host has decoded this picture.
    Waiting { image: ImageHandle, assigned: Option<u32> },
}

/// A transmission that passed the checks.
struct Checked {
    format: ImageFormat,
    width: u32,
    height: u32,
}

impl Terminal {
    /// An APC string that starts with `G` begins.
    pub(in crate::term) fn begin_kitty(&mut self) {
        self.graphics.apc.begin();
    }

    pub(in crate::term) fn put_kitty(&mut self, data: &[u8]) {
        self.graphics.apc.put(data);
    }

    pub(in crate::term) fn cancel_kitty(&mut self) {
        self.graphics.apc.cancel();
    }

    /// The string ended: act on it.
    pub(in crate::term) fn finish_kitty(&mut self) {
        let Parsed { command, error, payload, overflow, bad_base64 } = self.graphics.apc.finish();
        let usable = match (error, overflow, bad_base64) {
            (Some(error), ..) => Err(error),
            (_, true, _) => Err(Error::TooBig),
            (_, _, true) => Err(Error::Invalid("the payload is not base64")),
            _ => Ok(()),
        };
        // A chunk of a transmission under way: only its payload and `m` count.
        if let Some(mut transfer) = self.graphics.transfer.take() {
            let target = Target::of(&transfer.first);
            if let Err(error) = usable {
                return self.answer(target.render(Err(error)));
            }
            if transfer.data.len() + payload.len() > PAYLOAD_MAX {
                return self.answer(target.render(Err(Error::TooBig)));
            }
            transfer.data.extend_from_slice(&payload);
            if command.more {
                self.graphics.transfer = Some(transfer);
            } else {
                self.run_kitty(transfer.first, transfer.data);
            }
            return;
        }
        if let Err(error) = usable {
            return self.answer(Target::of(&command).render(Err(error)));
        }
        let transmits = matches!(command.action, Action::Transmit | Action::TransmitDisplay);
        if command.more && transmits {
            self.graphics.transfer = Some(Transfer { first: command, data: payload });
        } else {
            self.run_kitty(command, payload);
        }
    }

    /// A whole command, its payload put together.
    fn run_kitty(&mut self, command: Command, payload: Vec<u8>) {
        let target = Target::of(&command);
        match self.kitty_action(&command, payload) {
            Ok(Flow::Done { assigned }) => {
                let target = assigned.map_or(target, |id| target.with_id(id));
                self.answer(target.render(Ok(())));
            }
            Ok(Flow::Waiting { image, assigned }) => {
                let target = assigned.map_or(target, |id| target.with_id(id));
                let freed = self.graphics.gate.wait(image, target);
                self.release_replies(freed);
            }
            Err(error) => self.answer(target.render(Err(error))),
        }
    }

    fn kitty_action(&mut self, command: &Command, payload: Vec<u8>) -> Result<Flow, Error> {
        if command.relative {
            return Err(Error::Unsupported("relative placements"));
        }
        match command.action {
            Action::Transmit | Action::TransmitDisplay => self.kitty_transmit(command, payload),
            Action::Put => {
                let slot = self.screen_slot();
                let table = &self.graphics.tables[slot];
                let index = match (command.id, command.number) {
                    (0, 0) => return Err(Error::Invalid("no image id or number")),
                    (0, number) => table.by_number(number),
                    (id, _) => table.by_id(id),
                };
                let index = index.ok_or(Error::NotFound)?;
                // A program that named the picture by number is told which id it has.
                let assigned = (command.id == 0).then(|| table.get(index).map(|image| image.id)).flatten();
                self.kitty_put(slot, index, command)?;
                Ok(Flow::Done { assigned })
            }
            Action::Delete => {
                self.kitty_delete(command)?;
                Ok(Flow::Done { assigned: None })
            }
            Action::Query => {
                self.check_transmission(command, &payload)?;
                Ok(Flow::Done { assigned: None })
            }
            Action::Animation => Err(Error::Unsupported("animation")),
        }
    }

    /// Whether the keys and the bytes make a picture this terminal can show.
    fn check_transmission(&self, command: &Command, data: &[u8]) -> Result<Checked, Error> {
        if command.medium != Medium::Direct {
            return Err(Error::Unsupported("a file or shared memory"));
        }
        let (format, width, height) = match command.format {
            24 | 32 => {
                let (width, height) = (command.width, command.height);
                if width == 0 || height == 0 {
                    return Err(Error::Invalid("raw pixels without a size"));
                }
                check_size(width, height)?;
                let (format, bytes): (_, u64) =
                    if command.format == 24 { (ImageFormat::Rgb, 3) } else { (ImageFormat::Rgba, 4) };
                let expected = u64::from(width) * u64::from(height) * bytes;
                if command.zlib {
                    if data.is_empty() {
                        return Err(Error::BadData("no data"));
                    }
                } else if data.len() as u64 != expected {
                    return Err(Error::BadData("the data is not the size the keys say"));
                }
                (format, width, height)
            }
            100 => {
                if command.zlib {
                    return Err(Error::Invalid("compressed PNG data"));
                }
                if !data.starts_with(b"\x89PNG\r\n\x1a\n") {
                    return Err(Error::BadData("the data is not a PNG"));
                }
                let (width, height) =
                    sniff::dimensions(data).ok_or(Error::BadData("the PNG's header is cut short"))?;
                (ImageFormat::Png, width, height)
            }
            _ => return Err(Error::Invalid("a format this terminal does not read")),
        };
        check_size(width, height)?;
        Ok(Checked { format, width, height })
    }

    fn kitty_transmit(&mut self, command: &Command, payload: Vec<u8>) -> Result<Flow, Error> {
        let Checked { format, width, height } = self.check_transmission(command, &payload)?;
        let compression = if command.zlib { Compression::Zlib } else { Compression::None };
        let (handle, assigned) = self.add_image(command.id, command.number, width, height, payload.len())?;
        self.push_event(Event::ImageData(ImageUpload {
            image: handle,
            format,
            compression,
            width,
            height,
            data: payload,
        }));
        if command.action == Action::TransmitDisplay {
            let slot = self.screen_slot();
            if let Some(index) = self.graphics.tables[slot].find(handle) {
                self.kitty_put(slot, index, command)?;
            }
        }
        let decoded_later = format == ImageFormat::Png || compression == Compression::Zlib;
        Ok(if decoded_later { Flow::Waiting { image: handle, assigned } } else { Flow::Done { assigned } })
    }

    /// Make a picture of this size known on the current screen, replacing the one with its id
    /// and evicting what the budget needs gone. `bytes` is what the upload holds, to see that the
    /// host's queue has room. Returns its handle, and the id given to a picture sent by number.
    pub(super) fn add_image(
        &mut self,
        id: u32,
        number: u32,
        width: u32,
        height: u32,
        bytes: usize,
    ) -> Result<(ImageHandle, Option<u32>), Error> {
        let cost = u64::from(width) * u64::from(height) * 4;
        if cost > IMAGE_BUDGET_BYTES || self.events.len() >= super::super::EVENTS_MAX {
            return Err(Error::TooBig);
        }
        let queued: usize = self
            .events
            .iter()
            .map(|event| match event {
                Event::ImageData(upload) => upload.data.len(),
                _ => 0,
            })
            .sum();
        if queued + bytes > QUEUED_BYTES_MAX {
            return Err(Error::TooBig);
        }
        let slot = self.screen_slot();
        if let Some(old) = self.graphics.tables[slot].by_id(id) {
            self.free_image(slot, old);
        }
        while self.graphics.tables[slot].is_full(cost) {
            let grid = self.grid_of(slot);
            let victim = self.graphics.tables[slot]
                .victim(|image| grid.is_placed(image.handle) || !image.virtuals.is_empty());
            let Some(victim) = victim else { return Err(Error::TooBig) };
            self.free_image(slot, victim);
        }
        let table = &mut self.graphics.tables[slot];
        let (id, assigned) = match (id, number) {
            (0, 0) => (0, None),
            (0, _) => {
                let id = table.free_id();
                (id, Some(id))
            }
            (id, _) => (id, None),
        };
        let handle = self.graphics.next_handle();
        self.graphics.tables[slot].insert(Image {
            handle,
            id,
            number,
            width,
            height,
            used: 0,
            virtuals: Vec::new(),
        });
        Ok((handle, assigned))
    }

    /// Forget the picture at `index` of the screen `slot`'s table: its placements go and the host
    /// is told to let go of the pixels.
    pub(super) fn free_image(&mut self, slot: usize, index: usize) {
        let image = self.graphics.tables[slot].remove(index);
        let grid = self.grid_of_mut(slot);
        grid.delete_image_placements(image.handle);
        self.push_event(Event::ImageFree(image.handle));
    }

    /// Forget every picture of the screen `slot` (0 main, 1 alternate): the host is told to let
    /// go of the pixels, and the screen's placements go.
    pub(in crate::term) fn forget_screen_images(&mut self, slot: usize) {
        for image in self.graphics.tables[slot].drain() {
            self.push_event(Event::ImageFree(image.handle));
        }
        let grid = self.grid_of_mut(slot);
        grid.clear_placements();
    }

    /// A reset: no pictures, no half-read transmission, no answer waiting.
    pub(in crate::term) fn forget_all_images(&mut self) {
        self.forget_screen_images(0);
        self.forget_screen_images(1);
        self.graphics.transfer = None;
        self.graphics.multipart = None;
        self.graphics.gate.clear();
    }

    /// The grid of the screen `slot` names (0 main, 1 alternate): the one showing is `grid`, the
    /// other one is kept in `other`.
    pub(super) fn grid_of(&self, slot: usize) -> &Grid {
        if slot == self.screen_slot() { &self.grid } else { &self.other }
    }

    pub(super) fn grid_of_mut(&mut self, slot: usize) -> &mut Grid {
        if slot == self.screen_slot() { &mut self.grid } else { &mut self.other }
    }

    /// A cell's size in pixels, as the host gave it.
    pub(super) fn cell_size(&self) -> (u32, u32) {
        (u32::from(self.cell_px.0), u32::from(self.cell_px.1))
    }

    /// Send out an answer (or none).
    pub(in crate::term) fn answer(&mut self, bytes: Option<Vec<u8>>) {
        if let Some(bytes) = bytes {
            self.reply_bytes(&bytes);
        }
    }

    /// Replies the gate let go of go out ahead of anything later.
    fn release_replies(&mut self, parts: Vec<Vec<u8>>) {
        for part in parts {
            self.replies.extend_from_slice(&part);
        }
    }

    /// The host has decoded an uploaded picture, or could not. Every
    /// [`Event::ImageData`] is to be answered with one call, in any
    /// order. On an error the picture and its placements are forgotten, and the program is told
    /// if it was waiting for an answer. Replies that waited behind that answer are free to go: read
    /// [`replies`](Terminal::replies) again after the call.
    pub fn image_decoded(&mut self, image: ImageHandle, result: Result<(), ImageError>) {
        let answer = result.map_err(|error| match error {
            ImageError::BadData => Error::BadData("the data could not be decoded as that format"),
            ImageError::TooBig => Error::TooBig,
            ImageError::Unsupported => Error::Unsupported("that picture format"),
        });
        if answer.is_err() {
            for slot in 0..2 {
                if let Some(index) = self.graphics.tables[slot].find(image) {
                    self.free_image(slot, index);
                }
            }
        }
        let freed = self.graphics.gate.settle(image, answer);
        self.release_replies(freed);
    }
}

/// The size limits every picture meets.
pub(super) fn check_size(width: u32, height: u32) -> Result<(), Error> {
    if width == 0 || height == 0 {
        return Err(Error::BadData("an empty picture"));
    }
    if width > MAX_DIMENSION || height > MAX_DIMENSION || u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(Error::TooBig);
    }
    Ok(())
}
