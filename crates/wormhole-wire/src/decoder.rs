//! The incremental decoder: bytes in as they arrive, frames out as they complete.
//!
//! It treats its input as hostile. A header is checked before anything it announces is buffered
//! (a payload length past the type's maximum is fatal, and costs nothing); a frame of a type this
//! version does not know is skipped as it streams past, never buffered; the buffer never holds
//! more than one maximum frame plus one read; and no input makes it panic: every failure is a
//! [`WireError`].
//!
//! Feed it with [`Decoder::fill`] (one `read` straight into its buffer) or [`Decoder::push`], and
//! take frames with [`Decoder::next_frame`] until it returns `None`. A frame borrows the decoder's
//! buffer, so it ends before the next call. In steady state nothing here allocates: the buffer
//! grows to the largest frame seen and stays.

use std::io::{self, Read};

use crate::error::WireError;
use crate::header::{HEADER_LEN, Header, MAX_CONTROL, MAX_PAYLOAD, flags, max_payload};
use crate::message::{Frame, Message};

/// Bytes asked of one `read`: a data frame and its header, so a flood of 64 KiB frames is read a
/// frame per call.
const READ_CHUNK: usize = crate::header::MAX_DATA + HEADER_LEN;
/// A buffer this large that has gone empty is given back; a rare 1 MiB frame must not pin its
/// memory for the connection's life.
const SHRINK_ABOVE: usize = 4 * READ_CHUNK;

#[derive(Clone, Copy, Debug)]
enum State {
    /// Waiting for a header.
    Header,
    /// A known frame: waiting for `len` payload bytes.
    Body(Header),
    /// An unknown frame: dropping `remaining` more bytes as they come.
    Skip { remaining: usize },
}

/// A decoder for one connection's incoming bytes.
#[derive(Debug)]
pub struct Decoder {
    buf: Vec<u8>,
    /// Bytes before `head` are consumed; `head..tail` is waiting; past `tail` is spare.
    head: usize,
    tail: usize,
    state: State,
    /// A frame whose payload is complete at `head`, found by `ready` and not yet taken.
    pending: Option<Header>,
    max_payload: usize,
    fatal: Option<WireError>,
    skipped_frames: u64,
    skipped_bytes: u64,
    frames: u64,
}

impl Default for Decoder {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder {
    pub fn new() -> Self {
        Self {
            buf: Vec::new(),
            head: 0,
            tail: 0,
            state: State::Header,
            pending: None,
            max_payload: MAX_PAYLOAD,
            fatal: None,
            skipped_frames: 0,
            skipped_bytes: 0,
            frames: 0,
        }
    }

    /// Lower the largest payload any frame may have (it can only be at most [`MAX_PAYLOAD`]). A
    /// host sets [`MAX_CONTROL`] until the `Hello` is in, as the doc's 64 KiB handshake bound says,
    /// and raises it after.
    pub fn set_max_payload(&mut self, max: usize) {
        self.max_payload = max.min(MAX_PAYLOAD);
    }

    /// The pre-handshake limit.
    pub fn limit_to_handshake(&mut self) {
        self.set_max_payload(MAX_CONTROL);
    }

    /// Frames of unknown type dropped so far, and their payload bytes.
    pub fn skipped(&self) -> (u64, u64) {
        (self.skipped_frames, self.skipped_bytes)
    }

    /// Frames handed out so far.
    pub fn frames(&self) -> u64 {
        self.frames
    }

    /// Bytes received and not yet part of a returned frame.
    pub fn buffered(&self) -> usize {
        self.tail - self.head
    }

    /// The buffer's capacity, which stays within one maximum frame plus one read.
    pub fn capacity(&self) -> usize {
        self.buf.len()
    }

    /// Between frames with nothing buffered: the only place a stream may end.
    pub fn is_idle(&self) -> bool {
        matches!(self.state, State::Header) && self.head == self.tail && self.pending.is_none()
    }

    /// Whether the stream is lost (a fatal error was returned; every later call returns it again).
    pub fn is_failed(&self) -> bool {
        self.fatal.is_some()
    }

    /// Make room for at least `want` more bytes after `tail`.
    fn make_room(&mut self, want: usize) {
        if self.head == self.tail {
            self.head = 0;
            self.tail = 0;
            if self.buf.len() > SHRINK_ABOVE {
                self.buf = Vec::new();
            }
        }
        if self.buf.len() - self.tail >= want {
            return;
        }
        if self.head > 0 {
            self.buf.copy_within(self.head..self.tail, 0);
            self.tail -= self.head;
            self.head = 0;
        }
        if self.buf.len() - self.tail < want {
            self.buf.resize(self.tail + want, 0);
        }
    }

    /// How many bytes a read should ask for now: a chunk, or the rest of the frame in progress.
    /// The buffer for a big frame at most doubles per read, so a header that announces a megabyte
    /// reserves nothing until the bytes really arrive.
    fn read_size(&self) -> usize {
        let have = self.tail - self.head;
        match self.state {
            State::Body(h) => {
                let rest = (h.len as usize).saturating_sub(have);
                READ_CHUNK.max(rest.min(2 * have + READ_CHUNK))
            }
            _ => READ_CHUNK,
        }
    }

    /// Append received bytes. Everything given is buffered, so give one read's worth and then
    /// drain with [`Decoder::next_frame`]; for a socket use [`Decoder::fill`], which is bounded by
    /// construction.
    pub fn push(&mut self, data: &[u8]) {
        if self.fatal.is_some() {
            return;
        }
        self.make_room(data.len());
        self.buf[self.tail..self.tail + data.len()].copy_from_slice(data);
        self.tail += data.len();
    }

    /// One `read` from `src` straight into the buffer. Returns the bytes read; 0 is end of stream.
    /// Call it only after `next_frame` has returned `None`, which keeps the buffer bounded.
    pub fn fill<R: Read + ?Sized>(&mut self, src: &mut R) -> io::Result<usize> {
        if self.fatal.is_some() {
            return Ok(0);
        }
        self.make_room(self.read_size());
        let n = src.read(&mut self.buf[self.tail..])?;
        self.tail += n;
        Ok(n)
    }

    /// Advance to the next complete frame, skipping unknown ones. `Ok(true)` when one is waiting.
    fn ready(&mut self) -> Result<bool, WireError> {
        if let Some(e) = self.fatal {
            return Err(e);
        }
        if self.pending.is_some() {
            return Ok(true);
        }
        loop {
            match self.state {
                State::Skip { remaining } => {
                    let n = remaining.min(self.tail - self.head);
                    self.head += n;
                    self.skipped_bytes += n as u64;
                    if n < remaining {
                        self.state = State::Skip { remaining: remaining - n };
                        return Ok(false);
                    }
                    self.state = State::Header;
                    self.skipped_frames += 1;
                }
                State::Header => {
                    if self.tail - self.head < HEADER_LEN {
                        return Ok(false);
                    }
                    let mut raw = [0; HEADER_LEN];
                    raw.copy_from_slice(&self.buf[self.head..self.head + HEADER_LEN]);
                    let h = Header::parse(&raw);
                    self.head += HEADER_LEN;
                    if let Err(e) = self.admit(&h) {
                        self.fatal = Some(e);
                        return Err(e);
                    }
                    self.state = match max_payload(h.ty) {
                        None => State::Skip { remaining: h.len as usize },
                        Some(_) => State::Body(h),
                    };
                }
                State::Body(h) => {
                    if self.tail - self.head < h.len as usize {
                        return Ok(false);
                    }
                    // Not consumed yet: the payload stays at `head` until `take`, so a `fill`
                    // in between cannot move it.
                    self.pending = Some(h);
                    return Ok(true);
                }
            }
        }
    }

    /// Check a header before believing it.
    fn admit(&self, h: &Header) -> Result<(), WireError> {
        let max = max_payload(h.ty).unwrap_or(MAX_PAYLOAD).min(self.max_payload);
        if h.len as usize > max {
            return Err(WireError::FrameTooLarge { ty: h.ty, len: h.len, max: max as u32 });
        }
        if h.flags & flags::COMPRESSED != 0 {
            return Err(WireError::Compressed);
        }
        Ok(())
    }

    /// The next complete frame, if the bytes pushed so far hold one. `Ok(None)` means more bytes
    /// are needed. A non-fatal error (see [`WireError::is_fatal`]) consumed its frame: calling
    /// again reads the next. A fatal one is returned from every later call.
    pub fn next_frame(&mut self) -> Result<Option<Frame<'_>>, WireError> {
        if !self.ready()? {
            return Ok(None);
        }
        self.take()
    }

    /// Whether a complete frame is waiting for [`Decoder::take`]. Skips unknown frames on the way.
    /// This and `take` are `next_frame` in two steps, for callers that must read from a source
    /// between "is one ready" and "give it to me" without holding a borrow.
    pub fn poll(&mut self) -> Result<bool, WireError> {
        self.ready()
    }

    /// The frame `poll` found.
    pub fn take(&mut self) -> Result<Option<Frame<'_>>, WireError> {
        let Some(h) = self.pending.take() else { return Ok(None) };
        self.frames += 1;
        let (start, end) = (self.head, self.head + h.len as usize);
        self.head = end;
        self.state = State::Header;
        let payload = &self.buf[start..end];
        match Message::decode(h.ty, payload)? {
            Some(message) => Ok(Some(Frame { channel: h.channel, flags: h.flags, message })),
            // Unreachable for a type `admit`ted as known; skipping beats panicking.
            None => Ok(None),
        }
    }
}
