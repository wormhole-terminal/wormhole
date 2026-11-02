//! The control messages a client sends to set a terminal up and steer it, and the host's error
//! reply: `Spawn`, `Attach`, `Resize`, colors, limits, options, clipboard answers, hooks.

use crate::cursor::{Reader, Seen, Writer};
use crate::error::{EncodeError, WireError};
use crate::types::{List, Rgb, Size, StreamMode};

/// The most arguments and the most environment entries a `Spawn` may carry.
pub const MAX_SPAWN_ITEMS: usize = 4096;
/// The most palette entries `Colors` may carry (the 256-color palette).
pub const MAX_PALETTE: usize = 256;

/// `Options.flags`.
pub mod option_flags {
    /// Honor the program's window operations (`CSI t`).
    pub const WINDOW_OPS: u32 = 1 << 0;
    /// Answer the screen-checksum request (DECRQCRA).
    pub const CHECKSUM: u32 = 1 << 1;
}

/// Terminal settings the host applies; sent with `Spawn` and changed with `SetOptions`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Options {
    pub scrollback_lines: u32,
    pub flags: u32,
}

impl Options {
    pub(crate) fn read(r: &mut Reader<'_>) -> Result<Self, WireError> {
        Ok(Self { scrollback_lines: r.u32()?, flags: r.u32()? })
    }

    pub(crate) fn write(&self, w: &mut Writer<'_>) {
        w.u32(self.scrollback_lines);
        w.u32(self.flags);
    }
}

/// The colors the host answers a program's color queries with (OSC 4, 10, 11, 12).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Colors<'a> {
    pub fg: Rgb,
    pub bg: Rgb,
    pub cursor: Option<Rgb>,
    /// Palette entries from index 0.
    pub palette: List<'a, Rgb>,
}

impl<'a> Colors<'a> {
    pub(crate) fn write(&self, w: &mut Writer<'_>) -> Result<(), EncodeError> {
        if self.palette.len() > MAX_PALETTE {
            return Err(EncodeError::TooMany("Colors.palette"));
        }
        w.raw(&self.fg);
        w.raw(&self.bg);
        w.bool(self.cursor.is_some());
        w.raw(&self.cursor.unwrap_or([0; 3]));
        w.u16(self.palette.len() as u16);
        for c in self.palette.iter() {
            w.raw(c);
        }
        Ok(())
    }

    pub(crate) fn read(r: &mut Reader<'a>) -> Result<Self, WireError> {
        let fg = r.fixed::<3>()?;
        let bg = r.fixed::<3>()?;
        let has_cursor = r.bool()?;
        let cursor = r.fixed::<3>()?;
        let n = usize::from(r.u16()?);
        if n > MAX_PALETTE {
            return Err(WireError::TooMany("Colors"));
        }
        let mut palette = Vec::with_capacity(n.min(r.remaining() / 3));
        for _ in 0..n {
            palette.push(r.fixed::<3>()?);
        }
        Ok(Self { fg, bg, cursor: has_cursor.then_some(cursor), palette: List::Owned(palette) })
    }
}

/// The first message on a new host's first connection: what to run, and how. The environment is
/// complete (the host adds nothing), and travels here, never on a command line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Spawn<'a> {
    pub size: Size,
    pub options: Options,
    pub program: &'a str,
    pub cwd: &'a str,
    pub args: List<'a, &'a str>,
    pub env: List<'a, (&'a str, &'a str)>,
    pub colors: Option<Colors<'a>>,
    /// Text a previous session of this terminal left (minor 4), which the host draws dimmed before the
    /// program's first byte, with the "restored" marker, as a terminal restarted in the app does. In
    /// chunks of at most [`SEED_CHUNK`] bytes (a TLV's length is 16 bits), each valid UTF-8 on its own
    /// (see [`seed_chunks`]); an older host skips the tag and starts with a blank screen.
    pub seed: List<'a, &'a str>,
    /// The seed is VT with colors and attributes (SGR) and carries its own "restored" divider, as a
    /// checkpoint's screen is (minor 4): the host draws it as it is, filtered to text and SGR, and does not dim
    /// it or add a divider. A host that skips the tag (older) draws the seed as text.
    pub seed_styled: bool,
}

const S_COLORS: u8 = 1;
const S_SEED: u8 = 2;
const S_SEED_STYLED: u8 = 3;

/// The most bytes of one seed chunk: a TLV's length is 16 bits.
pub const SEED_CHUNK: usize = 60_000;
/// The most seed text a `Spawn` carries (the end of the text is kept): well inside a frame.
pub const SEED_MAX: usize = 512 * 1024;

/// `text` as seed chunks: its last [`SEED_MAX`] bytes (cut at a character, and at a line when one is near),
/// in pieces of at most [`SEED_CHUNK`] bytes that each end on a character.
pub fn seed_chunks(text: &str) -> Vec<&str> {
    let mut text = text;
    if text.len() > SEED_MAX {
        let mut start = text.len() - SEED_MAX;
        while !text.is_char_boundary(start) {
            start += 1;
        }
        // The next line start, when there is one close: the first line is not left cut in half.
        if let Some(newline) = text[start..].find('\n').filter(|n| *n < 4096) {
            start += newline + 1;
        }
        text = &text[start..];
    }
    let mut chunks = Vec::new();
    while !text.is_empty() {
        let mut end = text.len().min(SEED_CHUNK);
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        chunks.push(&text[..end]);
        text = &text[end..];
    }
    chunks
}

impl<'a> Spawn<'a> {
    pub(crate) fn encode(&self, w: &mut Writer<'_>) -> Result<(), EncodeError> {
        if self.args.len() > MAX_SPAWN_ITEMS {
            return Err(EncodeError::TooMany("Spawn.args"));
        }
        if self.env.len() > MAX_SPAWN_ITEMS {
            return Err(EncodeError::TooMany("Spawn.env"));
        }
        self.size.write(w);
        self.options.write(w);
        w.str16("Spawn.program", self.program)?;
        w.str16("Spawn.cwd", self.cwd)?;
        w.u16(self.args.len() as u16);
        for a in self.args.iter() {
            w.str16("Spawn.arg", a)?;
        }
        w.u16(self.env.len() as u16);
        for (k, v) in self.env.iter() {
            w.str16("Spawn.env key", k)?;
            w.str16("Spawn.env value", v)?;
        }
        if let Some(colors) = &self.colors {
            let at = w.tlv_begin(S_COLORS);
            colors.write(w)?;
            w.tlv_end(at, "Spawn.colors")?;
        }
        for chunk in self.seed.iter() {
            w.tlv_raw(S_SEED, "Spawn.seed", chunk.as_bytes())?;
        }
        if self.seed_styled {
            w.tlv_raw(S_SEED_STYLED, "Spawn.seed_styled", &[1])?;
        }
        Ok(())
    }

    pub(crate) fn decode(r: &mut Reader<'a>) -> Result<Self, WireError> {
        const WHAT: &str = "Spawn";
        let size = Size::read(r)?;
        let options = Options::read(r)?;
        let program = r.str16()?;
        let cwd = r.str16()?;
        let n = usize::from(r.u16()?);
        if n > MAX_SPAWN_ITEMS {
            return Err(WireError::TooMany(WHAT));
        }
        // Each string costs at least its two length bytes: a short payload cannot make this big.
        let mut args = Vec::with_capacity(n.min(r.remaining() / 2));
        for _ in 0..n {
            args.push(r.str16()?);
        }
        let n = usize::from(r.u16()?);
        if n > MAX_SPAWN_ITEMS {
            return Err(WireError::TooMany(WHAT));
        }
        let mut env = Vec::with_capacity(n.min(r.remaining() / 4));
        for _ in 0..n {
            env.push((r.str16()?, r.str16()?));
        }
        let (mut colors, mut seen, mut seed, mut seed_styled) = (None, Seen::default(), Vec::new(), false);
        for tlv in r.tlvs() {
            let (tag, value) = tlv?;
            if tag == S_COLORS {
                seen.once(tag, WHAT)?;
                colors = Some(Colors::read(&mut Reader::new(value, WHAT))?);
            } else if tag == S_SEED {
                // Chunks of one text: as many as the frame holds.
                seed.push(std::str::from_utf8(value).map_err(|_| WireError::BadUtf8(WHAT))?);
            } else if tag == S_SEED_STYLED {
                seed_styled = value == [1];
            }
        }
        Ok(Self {
            size,
            options,
            program,
            cwd,
            args: List::Owned(args),
            env: List::Owned(env),
            colors,
            seed: List::Owned(seed),
            seed_styled,
        })
    }
}

/// Join a terminal: what stream to receive and how far to let it run unacknowledged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Attach {
    pub stream: StreamMode,
    /// How many lines of history the attach repaint includes.
    pub history_lines: u32,
    /// The byte window: the host may send up to `consumed + window` (see `Ack`).
    pub window: u32,
    /// Continue from this stream offset out of the resume ring (remote reconnects).
    pub resume_from: Option<u64>,
}

const A_RESUME: u8 = 1;

impl Attach {
    pub(crate) fn encode(&self, w: &mut Writer<'_>) -> Result<(), EncodeError> {
        w.u8(self.stream.to_wire());
        w.u8(0);
        w.u32(self.history_lines);
        w.u32(self.window);
        if let Some(from) = self.resume_from {
            w.tlv_raw(A_RESUME, "Attach.resume_from", &from.to_le_bytes())?;
        }
        Ok(())
    }

    pub(crate) fn decode(r: &mut Reader<'_>) -> Result<Self, WireError> {
        const WHAT: &str = "Attach";
        let stream = StreamMode::from_wire(r.u8()?);
        r.skip(1)?;
        let (history_lines, window) = (r.u32()?, r.u32()?);
        let (mut resume_from, mut seen) = (None, Seen::default());
        for tlv in r.tlvs() {
            let (tag, value) = tlv?;
            if tag == A_RESUME {
                seen.once(tag, WHAT)?;
                resume_from = Some(Reader::new(value, WHAT).u64()?);
            }
        }
        Ok(Self { stream, history_lines, window, resume_from })
    }
}

/// Ask the host to resize. `req` comes back in `ResizeApplied` so the requester recognizes its own.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Resize {
    pub req: u32,
    pub size: Size,
}

impl Resize {
    pub(crate) fn encode(&self, w: &mut Writer<'_>) {
        w.u32(self.req);
        self.size.write(w);
    }

    pub(crate) fn decode(r: &mut Reader<'_>) -> Result<Self, WireError> {
        Ok(Self { req: r.u32()?, size: Size::read(r)? })
    }
}

/// The history budget the app grants this host, and the resume ring's size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub history_bytes: u64,
    pub ring_bytes: u64,
}

impl Limits {
    pub(crate) fn encode(&self, w: &mut Writer<'_>) {
        w.u64(self.history_bytes);
        w.u64(self.ring_bytes);
    }

    pub(crate) fn decode(r: &mut Reader<'_>) -> Result<Self, WireError> {
        Ok(Self { history_bytes: r.u64()?, ring_bytes: r.u64()? })
    }
}

/// The key a host seals its checkpoint with, and what the manifest says of the terminal that the host cannot
/// know: sent by the app right after the spawn or the attach when "restore after restart" is on (added in
/// minor 4). A host that has never been sent one writes no checkpoint at all.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct CheckpointKey<'a> {
    /// The 32-byte key, or empty: the app has none (the Keychain is unavailable), and the host then writes
    /// the folder and the shell but never a screen.
    pub key: &'a [u8],
    /// The key of the shell the terminal runs (`login`, or a shell profile's key), for the manifest.
    pub shell: &'a str,
}

/// The key must not show in a log line or a test failure.
impl std::fmt::Debug for CheckpointKey<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CheckpointKey")
            .field("key", &if self.key.is_empty() { "none" } else { "<hidden>" })
            .field("shell", &self.shell)
            .finish()
    }
}

/// The longest key sent: a longer one is a different thing, refused.
pub const CHECKPOINT_KEY_LEN: usize = 32;

impl<'a> CheckpointKey<'a> {
    pub(crate) fn encode(&self, w: &mut Writer<'_>) -> Result<(), EncodeError> {
        w.bytes32("SetCheckpointKey", self.key)?;
        w.str16("SetCheckpointKey", self.shell)
    }

    pub(crate) fn decode(r: &mut Reader<'a>) -> Result<Self, WireError> {
        let key = r.bytes32()?;
        if !key.is_empty() && key.len() != CHECKPOINT_KEY_LEN {
            return Err(WireError::BadValue("SetCheckpointKey"));
        }
        Ok(Self { key, shell: r.str16()? })
    }
}

/// The answer to `ClipboardLoad{ticket}`; an empty `data` is "nothing to give".
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ClipboardReply<'a> {
    pub ticket: u32,
    pub data: &'a [u8],
}

impl<'a> ClipboardReply<'a> {
    pub(crate) fn encode(&self, w: &mut Writer<'_>) -> Result<(), EncodeError> {
        w.u32(self.ticket);
        w.bytes32("ClipboardReply", self.data)
    }

    pub(crate) fn decode(r: &mut Reader<'a>) -> Result<Self, WireError> {
        Ok(Self { ticket: r.u32()?, data: r.bytes32()? })
    }
}

/// An agent hook event delivered to the host's socket when the app's is not there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hook<'a> {
    pub name: &'a str,
    pub payload: &'a [u8],
}

impl<'a> Hook<'a> {
    pub(crate) fn encode(&self, w: &mut Writer<'_>) -> Result<(), EncodeError> {
        w.str16("Hook.name", self.name)?;
        w.bytes32("Hook.payload", self.payload)
    }

    pub(crate) fn decode(r: &mut Reader<'a>) -> Result<Self, WireError> {
        Ok(Self { name: r.str16()?, payload: r.bytes32()? })
    }
}

/// Error codes of [`ErrorMsg`]. Open-ended: a code this version does not know is still a refusal.
pub mod error_code {
    /// The message needs the controller role.
    pub const NOT_CONTROLLER: u16 = 1;
    /// The message does not fit the terminal's state (a second `Spawn`, `Attach` before `Hello`).
    pub const BAD_STATE: u16 = 2;
    /// The program could not be started.
    pub const SPAWN_FAILED: u16 = 3;
    /// A feature that was not negotiated.
    pub const UNSUPPORTED: u16 = 4;
    /// The host has all the connections it allows.
    pub const BUSY: u16 = 5;
    /// A field out of range.
    pub const INVALID: u16 = 6;
    /// The host will not serve this peer (its code signature is not the app's, or the peer is another
    /// user's): sent once, as the host closes the connection, so a client can tell a host that refuses it
    /// from one that is gone (minor 3; an older host closes without a word).
    pub const PEER_REFUSED: u16 = 7;
}

/// The host refused or failed a request, which `ref_type` names (a frame type id).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ErrorMsg<'a> {
    pub ref_type: u8,
    pub code: u16,
    pub text: &'a str,
}

impl<'a> ErrorMsg<'a> {
    pub(crate) fn encode(&self, w: &mut Writer<'_>) -> Result<(), EncodeError> {
        w.u8(self.ref_type);
        w.u16(self.code);
        w.str16("Error.text", self.text)
    }

    pub(crate) fn decode(r: &mut Reader<'a>) -> Result<Self, WireError> {
        Ok(Self { ref_type: r.u8()?, code: r.u16()?, text: r.str16()? })
    }
}

#[cfg(test)]
mod seed_tests {
    use super::*;

    #[test]
    fn short_text_is_one_chunk_and_nothing_is_empty() {
        assert_eq!(seed_chunks("abc"), ["abc"]);
        assert!(seed_chunks("").is_empty());
    }

    #[test]
    fn long_text_is_cut_into_chunks_that_hold_whole_characters_and_join_back() {
        // Two-byte characters across the chunk size.
        let text: String = std::iter::repeat_n('\u{e9}', SEED_CHUNK).collect();
        let chunks = seed_chunks(&text);
        assert!(chunks.len() >= 2);
        assert!(chunks.iter().all(|c| c.len() <= SEED_CHUNK));
        assert_eq!(chunks.concat(), text);
    }

    #[test]
    fn text_past_the_cap_keeps_its_end_and_starts_on_a_line() {
        let mut text = String::new();
        let mut i = 0;
        while text.len() < SEED_MAX + 10_000 {
            text.push_str(&format!("line {i}\n"));
            i += 1;
        }
        let kept = seed_chunks(&text).concat();
        assert!(kept.len() <= SEED_MAX);
        assert!(text.ends_with(&kept));
        assert!(kept.starts_with("line "), "starts on a line: {:?}", &kept[..20]);
    }
}
