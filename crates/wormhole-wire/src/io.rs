//! The transport seam: the codec over any `Read` and `Write`. A Unix socket now, the stdio of
//! `ssh host wormhole-host --stdio` later; nothing in the codec knows which. A readiness wake-up
//! (poll, kqueue, a thread per side) is the caller's: [`Decoder::fill`] and [`FrameWriter::flush`]
//! surface `WouldBlock` as the I/O error it is.

use std::fmt;
use std::io::{self, Read, Write};

use crate::decoder::Decoder;
use crate::error::{EncodeError, WireError};
use crate::message::{Frame, Message, encode};

/// Why a frame could not be received.
#[derive(Debug)]
pub enum RecvError {
    /// The bytes were wrong; see [`WireError::is_fatal`].
    Wire(WireError),
    Io(io::Error),
    /// The stream ended inside a frame.
    UnexpectedEof,
}

impl fmt::Display for RecvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Wire(e) => e.fmt(f),
            Self::Io(e) => e.fmt(f),
            Self::UnexpectedEof => f.write_str("connection ended inside a frame"),
        }
    }
}

impl std::error::Error for RecvError {}

impl From<WireError> for RecvError {
    fn from(e: WireError) -> Self {
        Self::Wire(e)
    }
}

/// Reads frames from a byte source.
pub struct FrameReader<R> {
    src: R,
    dec: Decoder,
}

impl<R: Read> FrameReader<R> {
    pub fn new(src: R) -> Self {
        Self { src, dec: Decoder::new() }
    }

    pub fn decoder(&mut self) -> &mut Decoder {
        &mut self.dec
    }

    pub fn get_ref(&self) -> &R {
        &self.src
    }

    pub fn into_inner(self) -> R {
        self.src
    }

    /// Block until a frame is complete. `Ok(None)` is the peer closing cleanly between frames. A
    /// non-fatal `Wire` error consumed its frame, so reading on is allowed.
    pub fn read_frame(&mut self) -> Result<Option<Frame<'_>>, RecvError> {
        loop {
            if self.dec.poll()? {
                return match self.dec.take()? {
                    Some(frame) => Ok(Some(frame)),
                    None => Err(RecvError::Wire(WireError::BadValue("frame"))),
                };
            }
            match self.dec.fill(&mut self.src) {
                Ok(0) if self.dec.is_idle() => return Ok(None),
                Ok(0) => return Err(RecvError::UnexpectedEof),
                Ok(_) => {}
                Err(e) if e.kind() == io::ErrorKind::Interrupted => {}
                Err(e) => return Err(RecvError::Io(e)),
            }
        }
    }
}

/// Why a frame could not be sent.
#[derive(Debug)]
pub enum SendError {
    Encode(EncodeError),
    Io(io::Error),
}

impl fmt::Display for SendError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Encode(e) => e.fmt(f),
            Self::Io(e) => e.fmt(f),
        }
    }
}

impl std::error::Error for SendError {}

impl From<EncodeError> for SendError {
    fn from(e: EncodeError) -> Self {
        Self::Encode(e)
    }
}

/// Writes frames to a byte sink through one reusable buffer.
pub struct FrameWriter<W> {
    dst: W,
    buf: Vec<u8>,
}

impl<W: Write> FrameWriter<W> {
    pub fn new(dst: W) -> Self {
        Self { dst, buf: Vec::new() }
    }

    pub fn get_ref(&self) -> &W {
        &self.dst
    }

    pub fn into_inner(self) -> W {
        self.dst
    }

    /// Add a frame to the batch without writing it. A host turn that has several frames to send
    /// queues them all and flushes once, which is one `write` for the turn.
    pub fn queue(&mut self, channel: u16, flags: u8, msg: &Message<'_>) -> Result<(), EncodeError> {
        encode(&mut self.buf, channel, flags, msg)
    }

    /// Bytes queued and not yet written.
    pub fn queued(&self) -> usize {
        self.buf.len()
    }

    /// Write everything queued. The queue is emptied even on failure: a broken connection is
    /// closed, not resumed mid-frame.
    pub fn flush(&mut self) -> io::Result<()> {
        let result = self.dst.write_all(&self.buf);
        self.buf.clear();
        result
    }

    /// Queue one frame and write the batch.
    pub fn send(&mut self, channel: u16, flags: u8, msg: &Message<'_>) -> Result<(), SendError> {
        self.queue(channel, flags, msg)?;
        self.flush().map_err(SendError::Io)
    }
}
