//! Inline images lifted out of the PTY's bytes.
//!
//! kitty's `ESC _ G … ESC \`, Sixel's `ESC P … q … ESC \` and iTerm2's `ESC ] 1337 ; File=… BEL`
//! are taken out of the stream before either parser sees them (alacritty would drop them, and its
//! parser keeps a whole string in memory meanwhile) and replaced by the placeholder cells that say
//! where the picture goes (`images::placeholder`), printed right where the sequence was, so they
//! land at the cursor as the picture would. The pixels go to the terminal's `ImageStore`: Sixel
//! decoded here (small by its cap), the rest on a thread of its own. kitty is answered as its
//! protocol asks, through the same reply path as a terminal query.
//!
//! Everything else passes through unchanged, byte for byte, however a read splits it: a read
//! that ends inside an introducer holds those few bytes back for the next one. Other APC, DCS and
//! OSC strings are never touched. A sequence cut short by CAN or SUB, or by an escape that isn't
//! `ESC \`, is dropped the way the parsers drop it. Caps bound what one sequence may accumulate;
//! past them the rest is skipped and the image is refused.

use std::io;
use std::sync::Arc;

use alacritty_terminal::event::WindowSize;
use base64::Engine;
use base64::engine::general_purpose::GeneralPurpose;
use base64::engine::{DecodePaddingMode, GeneralPurposeConfig};
use memchr::memchr;

use super::super::events::EventProxy;
use super::super::images::{
    self, Format, Limits, MAX_CELLS, MAX_DIMENSION, MAX_PAYLOAD_BYTES, MAX_PIXELS, MAX_SIXEL_BYTES,
    PlacementSpec, iterm, kitty, placeholder_bytes, sixel,
};

/// Most one kitty command may be (the protocol chunks payloads at 4096).
const KITTY_MAX: usize = 1024 * 1024;
/// Most one iTerm2 sequence may be: the payload's base64 and its arguments.
const ITERM_MAX: usize = MAX_PAYLOAD_BYTES / 3 * 4 + 4096;
/// The longest introducer held back until a read says what it starts.
const PREFIX_MAX: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Kitty,
    Sixel,
    ItermFile,
    ItermMultipart,
    ItermPart,
    ItermEnd,
}

impl Kind {
    fn cap(self) -> usize {
        match self {
            Kind::Kitty => KITTY_MAX,
            Kind::Sixel => MAX_SIXEL_BYTES,
            _ => ITERM_MAX,
        }
    }

    /// An OSC ends with BEL too.
    fn is_osc(self) -> bool {
        matches!(self, Kind::ItermFile | Kind::ItermMultipart | Kind::ItermPart | Kind::ItermEnd)
    }
}

/// What an escape at the start of a slice turns out to be.
enum Classified {
    /// Not an image sequence: this many bytes go on as they are.
    Pass(usize),
    /// The slice ends before it says: hold it for the next read.
    Hold,
    /// An image sequence of this kind, this many bytes of introducer.
    Start(Kind, usize),
}

/// What one kitty command did, for the answer.
struct Outcome {
    /// The id the terminal chose for a numbered image.
    assigned: Option<u32>,
    /// What to print in the sequence's place.
    replacement: Option<Vec<u8>>,
    /// Whether to answer now (a chunk in the middle of a transfer is answered at its end).
    reply: bool,
}

pub(super) struct ImageFilter {
    proxy: EventProxy,
    /// An introducer a read ended inside, completed by the next read.
    held: Vec<u8>,
    in_sequence: Option<Kind>,
    /// The sequence's bytes after its introducer, up to the kind's cap.
    seq: Vec<u8>,
    overflow: bool,
    /// The last byte was ESC, which `\` would make the end.
    esc_pending: bool,
    transfers: kitty::Transfers,
    /// iTerm2's multipart image under way: its arguments and its base64 so far.
    multipart: Option<(iterm::FileArgs, Vec<u8>)>,
    /// Output with replacements in it, not yet handed over: `out[taken..]`.
    out: Vec<u8>,
    taken: usize,
}

impl ImageFilter {
    pub(super) fn new(proxy: EventProxy) -> Self {
        Self {
            proxy,
            held: Vec::new(),
            in_sequence: None,
            seq: Vec::new(),
            overflow: false,
            esc_pending: false,
            transfers: kitty::Transfers::default(),
            multipart: None,
            out: Vec::new(),
            taken: 0,
        }
    }

    /// Read through `inner` and hand over what isn't an image sequence, replacements in place.
    pub(super) fn read_from(
        &mut self,
        mut inner: impl FnMut(&mut [u8]) -> io::Result<usize>,
        buf: &mut [u8],
    ) -> io::Result<usize> {
        loop {
            if self.taken < self.out.len() {
                return Ok(self.hand_over(buf));
            }
            let n = match inner(buf) {
                Ok(n) => n,
                // Nothing more for now: held bytes go on unchanged rather than stall (the next
                // read reports the error again).
                Err(_) if !self.held.is_empty() => 0,
                Err(err) => return Err(err),
            };
            if n == 0 {
                if self.held.is_empty() {
                    return Ok(0);
                }
                self.out = std::mem::take(&mut self.held);
                self.taken = 0;
                return Ok(self.hand_over(buf));
            }
            match self.filter(&buf[..n]) {
                None => return Ok(n),
                // All of it was a sequence (or is held): read on.
                Some(out) if out.is_empty() => continue,
                Some(out) => {
                    self.out = out;
                    self.taken = 0;
                    return Ok(self.hand_over(buf));
                }
            }
        }
    }

    fn hand_over(&mut self, buf: &mut [u8]) -> usize {
        let rest = &self.out[self.taken..];
        let n = rest.len().min(buf.len());
        buf[..n].copy_from_slice(&rest[..n]);
        self.taken += n;
        if self.taken == self.out.len() {
            self.out.clear();
            self.taken = 0;
        }
        n
    }

    /// Run `input` through the filter. `None` when it passes as it is; otherwise what to hand
    /// over instead (possibly nothing yet).
    fn filter(&mut self, input: &[u8]) -> Option<Vec<u8>> {
        let held = std::mem::take(&mut self.held);
        let joined;
        let bytes: &[u8] = if held.is_empty() {
            input
        } else {
            joined = [held.as_slice(), input].concat();
            &joined
        };
        let mut out: Option<Vec<u8>> = (!held.is_empty() || self.in_sequence.is_some()).then(Vec::new);
        // How much of `bytes` is in `out` already (or was consumed by a sequence).
        let mut copied = 0;
        let mut at = 0;
        while at < bytes.len() {
            if let Some(kind) = self.in_sequence {
                at = self.consume(kind, bytes, at, &mut out, &mut copied);
                continue;
            }
            let Some(esc) = memchr(0x1b, &bytes[at..]) else { break };
            let esc = at + esc;
            match classify(&bytes[esc..]) {
                Classified::Pass(n) => at = esc + n,
                Classified::Hold => {
                    out.get_or_insert_with(Vec::new).extend_from_slice(&bytes[copied..esc]);
                    self.held.extend_from_slice(&bytes[esc..]);
                    copied = bytes.len();
                    at = bytes.len();
                }
                Classified::Start(kind, introducer) => {
                    out.get_or_insert_with(Vec::new).extend_from_slice(&bytes[copied..esc]);
                    at = esc + introducer;
                    copied = at;
                    self.in_sequence = Some(kind);
                    self.seq.clear();
                    self.overflow = false;
                    self.esc_pending = false;
                }
            }
        }
        let mut out = out?;
        out.extend_from_slice(&bytes[copied..]);
        Some(out)
    }

    /// Inside a sequence: take its bytes up to its end, from `at`. Returns where to go on;
    /// `copied` follows, since nothing of a sequence goes to the output.
    fn consume(
        &mut self,
        kind: Kind,
        bytes: &[u8],
        mut at: usize,
        out: &mut Option<Vec<u8>>,
        copied: &mut usize,
    ) -> usize {
        while at < bytes.len() {
            let b = bytes[at];
            at += 1;
            if self.esc_pending {
                self.esc_pending = false;
                if b == b'\\' {
                    self.finish(kind, out);
                    *copied = at;
                    return at;
                }
                // An escape that isn't the end cuts the sequence short, as the parsers would
                // cut it; the escape itself goes on to them.
                self.abandon();
                *copied = at - 2;
                return at - 2;
            }
            match b {
                0x1b => self.esc_pending = true,
                0x07 if kind.is_osc() => {
                    self.finish(kind, out);
                    *copied = at;
                    return at;
                }
                0x18 | 0x1a => {
                    self.abandon();
                    *copied = at;
                    return at;
                }
                _ => {
                    if self.seq.len() < kind.cap() {
                        self.seq.push(b);
                    } else {
                        self.overflow = true;
                    }
                }
            }
        }
        *copied = at;
        at
    }

    fn abandon(&mut self) {
        self.in_sequence = None;
        self.seq.clear();
        self.overflow = false;
    }

    /// The sequence ended: act on it, and print its replacement.
    fn finish(&mut self, kind: Kind, out: &mut Option<Vec<u8>>) {
        self.in_sequence = None;
        let seq = std::mem::take(&mut self.seq);
        let overflow = std::mem::take(&mut self.overflow);
        let replacement = match kind {
            Kind::Kitty => self.finish_kitty(&seq, overflow),
            Kind::Sixel => self.finish_sixel(&seq, overflow),
            Kind::ItermFile => {
                let (args, payload) = split_once(&seq, b':');
                let args = iterm::parse_args(&String::from_utf8_lossy(args));
                self.finish_iterm(args, payload, overflow)
            }
            Kind::ItermMultipart => {
                let args = iterm::parse_args(&String::from_utf8_lossy(&seq));
                self.multipart = Some((args, Vec::new()));
                None
            }
            Kind::ItermPart => {
                if let Some((_, data)) = &mut self.multipart {
                    if data.len() + seq.len() <= ITERM_MAX && !overflow {
                        data.extend_from_slice(&seq);
                    } else {
                        tracing::warn!("inline image dropped: iTerm2 parts over {ITERM_MAX} bytes");
                        self.multipart = None;
                    }
                }
                None
            }
            Kind::ItermEnd => match self.multipart.take() {
                Some((args, data)) => self.finish_iterm(args, &data, false),
                None => None,
            },
        };
        if let Some(replacement) = replacement {
            out.get_or_insert_with(Vec::new).extend_from_slice(&replacement);
        }
    }

    fn grid(&self) -> WindowSize {
        self.proxy.shared().grid_size()
    }

    // ---------------------------------------------------------------- kitty

    fn finish_kitty(&mut self, seq: &[u8], overflow: bool) -> Option<Vec<u8>> {
        let (control, payload) = split_once(seq, b';');
        let control = match kitty::parse(control) {
            Ok(control) => control,
            Err(err) => {
                tracing::info!(?err, "a kitty graphics command wasn't understood");
                return None;
            }
        };
        let result = if overflow {
            self.transfers = kitty::Transfers::default();
            Err(kitty::Error::TooBig)
        } else {
            self.kitty_command(&control, payload)
        };
        match result {
            Ok(outcome) => {
                if outcome.reply {
                    self.reply_kitty(&control, outcome.assigned, Ok(()));
                }
                outcome.replacement
            }
            Err(err) => {
                tracing::info!(action = %control.action, ?err, "a kitty graphics command was refused");
                self.reply_kitty(&control, None, Err(&err));
                None
            }
        }
    }

    fn reply_kitty(
        &self,
        control: &kitty::Control,
        assigned: Option<u32>,
        result: Result<(), &kitty::Error>,
    ) {
        if let Some(bytes) = kitty::response(control, assigned, result) {
            self.proxy.shared().write_reply(bytes);
        }
    }

    fn kitty_command(&mut self, control: &kitty::Control, payload: &[u8]) -> Result<Outcome, kitty::Error> {
        let done = |replacement| Ok(Outcome { assigned: None, replacement, reply: true });
        match control.action {
            'q' => {
                check_transfer(control)?;
                done(None)
            }
            't' | 'T' => {
                if !self.transfers.in_progress() {
                    check_transfer(control)?;
                }
                let Some((first, data)) = self.transfers.feed(control.clone(), payload)? else {
                    return Ok(Outcome { assigned: None, replacement: None, reply: false });
                };
                let format = format_of(&first)?;
                let (width, height) = images::dimensions(format, &data, first.compressed, &Limits::default())
                    .ok_or_else(|| kitty::Error::BadData("the data isn't an image of that format".into()))?;
                check_dimensions(width, height)?;
                let (id, assigned) = {
                    let mut store = self.proxy.shared().images();
                    let (id, assigned) = if first.id != 0 {
                        (first.id, None)
                    } else if first.number != 0 {
                        let id = store.assign_number(first.number);
                        (id, Some(id))
                    } else {
                        (store.fresh_id(), None)
                    };
                    if !store.reserve(id, width, height) {
                        return Err(kitty::Error::TooBig);
                    }
                    (id, assigned)
                };
                self.proxy.decode_image(id, format, data, first.compressed);
                let replacement =
                    if first.action == 'T' { self.place(id, &first, width, height) } else { None };
                Ok(Outcome { assigned, replacement, reply: true })
            }
            'p' => {
                let id = self.existing_id(control)?;
                let (width, height) =
                    self.proxy.shared().images().dimensions(id).ok_or(kitty::Error::NotFound)?;
                done(self.place(id, control, width, height))
            }
            'd' => {
                self.delete(control)?;
                done(None)
            }
            'f' | 'a' => Err(kitty::Error::Unsupported("animation")),
            _ => Err(kitty::Error::Invalid("an action this terminal doesn't know")),
        }
    }

    /// The image a command names, which must exist.
    fn existing_id(&self, control: &kitty::Control) -> Result<u32, kitty::Error> {
        let store = self.proxy.shared().images();
        let id = if control.id != 0 {
            control.id
        } else if control.number != 0 {
            store.id_for_number(control.number).ok_or(kitty::Error::NotFound)?
        } else {
            return Err(kitty::Error::Invalid("no image id"));
        };
        if store.contains(id) { Ok(id) } else { Err(kitty::Error::NotFound) }
    }

    /// Place image `id` as `control` asks: the placement's shape is kept, and the placeholders
    /// are printed unless the program prints them itself (`U=1`).
    fn place(&self, id: u32, control: &kitty::Control, width: u32, height: u32) -> Option<Vec<u8>> {
        let (cols, rows) = cells_for(width, height, control.cols, control.rows, self.grid());
        let source = (control.source_w > 0 && control.source_h > 0).then_some((
            control.source_x,
            control.source_y,
            control.source_w,
            control.source_h,
        ));
        let fit = control.cols > 0 && control.rows > 0;
        self.proxy.shared().images().place(id, control.placement, PlacementSpec { cols, rows, fit, source });
        (!control.unicode_placeholders)
            .then(|| placeholder_bytes(id, control.placement, cols, rows, !control.cursor_stays))
    }

    /// `a=d`: what `d` names stops showing; an upper-case `d` frees the image too. This terminal
    /// can't tell which placement is at a cell, so the deletes by position take every placement.
    fn delete(&self, control: &kitty::Control) -> Result<(), kitty::Error> {
        let free = control.delete.is_ascii_uppercase();
        let mut store = self.proxy.shared().images();
        match control.delete.to_ascii_lowercase() {
            'a' | 'c' | 'p' | 'q' | 'x' | 'y' | 'z' | 'r' => {
                store.hide_all();
                if free {
                    store.clear();
                }
            }
            'i' | 'n' => {
                let id = if control.delete.eq_ignore_ascii_case(&'i') {
                    (control.id != 0).then_some(control.id).ok_or(kitty::Error::Invalid("no image id"))?
                } else {
                    store.id_for_number(control.number).ok_or(kitty::Error::NotFound)?
                };
                let placement = (control.placement != 0).then_some(control.placement);
                store.hide(id, placement);
                if free && placement.is_none() {
                    store.remove(id);
                }
            }
            'f' => return Err(kitty::Error::Unsupported("animation")),
            _ => return Err(kitty::Error::Invalid("a delete this terminal doesn't know")),
        }
        Ok(())
    }

    // ---------------------------------------------------------------- Sixel

    fn finish_sixel(&mut self, seq: &[u8], overflow: bool) -> Option<Vec<u8>> {
        if overflow {
            tracing::warn!("inline image dropped: a Sixel over {MAX_SIXEL_BYTES} bytes");
            return None;
        }
        let image = match sixel::decode(seq, &Limits::default()) {
            Ok(image) => image,
            Err(err) => {
                tracing::info!(?err, "a Sixel image wasn't shown");
                return None;
            }
        };
        let (width, height) = (image.width, image.height);
        let (cols, rows) = cells_for(width, height, 0, 0, self.grid());
        let id = {
            let mut store = self.proxy.shared().images();
            let id = store.fresh_id();
            if !store.insert(id, Arc::new(image)) {
                tracing::warn!("inline image dropped: no room for a Sixel of {width}×{height}");
                return None;
            }
            store.place(id, 0, PlacementSpec { cols, rows, fit: false, source: None });
            id
        };
        let mut bytes = placeholder_bytes(id, 0, cols, rows, true);
        // Sixel leaves the text cursor under the picture, at its left edge.
        bytes.extend_from_slice(format!("\n\x1b[{cols}D").as_bytes());
        Some(bytes)
    }

    // ---------------------------------------------------------------- iTerm2

    fn finish_iterm(&mut self, args: iterm::FileArgs, payload: &[u8], overflow: bool) -> Option<Vec<u8>> {
        if !args.inline {
            // A download, not a picture: nothing to show.
            return None;
        }
        if overflow {
            tracing::warn!("inline image dropped: an iTerm2 file over {ITERM_MAX} bytes");
            return None;
        }
        let data = match lenient_base64().decode(payload) {
            Ok(data) => data,
            Err(_) => {
                tracing::info!("an iTerm2 inline image wasn't shown: its data isn't base64");
                return None;
            }
        };
        if data.len() > MAX_PAYLOAD_BYTES {
            tracing::warn!("inline image dropped: an iTerm2 file of {} bytes", data.len());
            return None;
        }
        let Some((width, height)) = images::dimensions(Format::Any, &data, false, &Limits::default()) else {
            tracing::info!("an iTerm2 inline image wasn't shown: not a picture this terminal reads");
            return None;
        };
        if let Err(err) = check_dimensions(width, height) {
            tracing::warn!(width, height, ?err, "inline image dropped");
            return None;
        }
        let grid = self.grid();
        let cols_hint = args.width.cells(f32::from(grid.cell_width), u32::from(grid.num_cols));
        let rows_hint = args.height.cells(f32::from(grid.cell_height), u32::from(grid.num_lines));
        let (cols, rows) = cells_for(width, height, cols_hint.unwrap_or(0), rows_hint.unwrap_or(0), grid);
        let fit = args.preserve_aspect_ratio && cols_hint.is_some() && rows_hint.is_some();
        let id = {
            let mut store = self.proxy.shared().images();
            let id = store.fresh_id();
            if !store.reserve(id, width, height) {
                tracing::warn!("inline image dropped: the store is full");
                return None;
            }
            store.place(id, 0, PlacementSpec { cols, rows, fit, source: None });
            id
        };
        self.proxy.decode_image(id, Format::Any, data, false);
        Some(placeholder_bytes(id, 0, cols, rows, true))
    }
}

/// What an escape at `rest[0]` starts (see [`Classified`]).
fn classify(rest: &[u8]) -> Classified {
    match rest.get(1) {
        None => Classified::Hold,
        Some(b'_') => match rest.get(2) {
            None => Classified::Hold,
            Some(b'G') => Classified::Start(Kind::Kitty, 3),
            Some(_) => Classified::Pass(2),
        },
        Some(b'P') => {
            // DCS: numeric parameters, then the final byte; `q` is Sixel.
            let mut i = 2;
            while i < rest.len() && i < PREFIX_MAX {
                match rest[i] {
                    b'0'..=b'9' | b';' => i += 1,
                    b'q' => return Classified::Start(Kind::Sixel, i + 1),
                    _ => return Classified::Pass(i),
                }
            }
            if i >= PREFIX_MAX { Classified::Pass(i) } else { Classified::Hold }
        }
        Some(b']') => {
            const NUMBER: &[u8] = b"1337;";
            let body = &rest[2..];
            let seen = body.len().min(NUMBER.len());
            if body[..seen] != NUMBER[..seen] {
                return Classified::Pass(2);
            }
            if body.len() < NUMBER.len() {
                return Classified::Hold;
            }
            let after = &body[NUMBER.len()..];
            const FORMS: [(&[u8], Kind); 4] = [
                (b"MultipartFile=", Kind::ItermMultipart),
                (b"FilePart=", Kind::ItermPart),
                (b"FileEnd", Kind::ItermEnd),
                (b"File=", Kind::ItermFile),
            ];
            for (name, kind) in FORMS {
                if after.starts_with(name) {
                    return Classified::Start(kind, 2 + NUMBER.len() + name.len());
                }
                if after.len() < name.len() && name.starts_with(after) {
                    return Classified::Hold;
                }
            }
            Classified::Pass(2)
        }
        Some(_) => Classified::Pass(1),
    }
}

fn split_once(bytes: &[u8], at: u8) -> (&[u8], &[u8]) {
    match memchr(at, bytes) {
        Some(i) => (&bytes[..i], &bytes[i + 1..]),
        None => (bytes, &[]),
    }
}

fn lenient_base64() -> GeneralPurpose {
    GeneralPurpose::new(
        &base64::alphabet::STANDARD,
        GeneralPurposeConfig::new().with_decode_padding_mode(DecodePaddingMode::Indifferent),
    )
}

/// Whether a transfer is one this terminal takes: direct data in a format it reads, sized when
/// raw. Files and shared memory are refused alike (`EBADF`), as the protocol has it.
fn check_transfer(control: &kitty::Control) -> Result<(), kitty::Error> {
    if control.transmission != 'd' {
        return Err(kitty::Error::Unsupported("a file or shared memory"));
    }
    format_of(control).map(|_| ())
}

fn format_of(control: &kitty::Control) -> Result<Format, kitty::Error> {
    let sized = |make: fn(u32, u32) -> Format| {
        if control.width == 0 || control.height == 0 {
            Err(kitty::Error::Invalid("raw pixels without a size"))
        } else {
            Ok(make(control.width, control.height))
        }
    };
    match control.format {
        24 => sized(|width, height| Format::Rgb { width, height }),
        32 => sized(|width, height| Format::Rgba { width, height }),
        100 => Ok(Format::Png),
        _ => Err(kitty::Error::Invalid("a format this terminal doesn't read")),
    }
}

fn check_dimensions(width: u32, height: u32) -> Result<(), kitty::Error> {
    if width == 0 || height == 0 {
        return Err(kitty::Error::BadData("an empty image".into()));
    }
    if width > MAX_DIMENSION || height > MAX_DIMENSION || u64::from(width) * u64::from(height) > MAX_PIXELS {
        return Err(kitty::Error::TooBig);
    }
    Ok(())
}

/// The cells a `width` × `height` picture takes, as the program asked (`cols`, `rows`: 0 for
/// "from the picture", one of them keeping the aspect ratio) in a grid of `grid`'s cells. A
/// picture sized by itself is scaled to the grid's width when wider; one the program sized is
/// clipped there (its rows stay). Never more than the placeholders can count.
fn cells_for(width: u32, height: u32, cols: u32, rows: u32, grid: WindowSize) -> (u16, u16) {
    let cell_w = f32::from(grid.cell_width).max(1.0);
    let cell_h = f32::from(grid.cell_height).max(1.0);
    let (width, height) = (width.max(1) as f32, height.max(1) as f32);
    let ceil = |v: f32| v.ceil().max(1.0) as u32;
    let auto = cols == 0 && rows == 0;
    let (mut cols, mut rows) = match (cols, rows) {
        (0, 0) => (ceil(width / cell_w), ceil(height / cell_h)),
        (c, 0) => (c, ceil(c as f32 * cell_w * height / (width * cell_h))),
        (0, r) => (ceil(r as f32 * cell_h * width / (height * cell_w)), r),
        (c, r) => (c, r),
    };
    let max_cols = u32::from(grid.num_cols).max(1);
    if cols > max_cols {
        if auto {
            rows = ceil(rows as f32 * max_cols as f32 / cols as f32);
        }
        cols = max_cols;
    }
    (cols.clamp(1, MAX_CELLS) as u16, rows.clamp(1, MAX_CELLS) as u16)
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;
    use std::io::{self, Read};
    use std::sync::Arc;
    use std::time::{Duration, Instant};

    use alacritty_terminal::event::{VoidListener, WindowSize};
    use alacritty_terminal::grid::Dimensions;
    use alacritty_terminal::index::{Column, Line};
    use alacritty_terminal::term::{Config, Term};
    use alacritty_terminal::vte::ansi::{Processor, StdSyncHandler};
    use base64::Engine;

    use super::{ImageFilter, cells_for};
    use crate::terminal::GridSize;
    use crate::terminal::events::EventProxy;
    use crate::terminal::images::{self, PLACEHOLDER, placeholder_at};

    const GRID: WindowSize = WindowSize { num_lines: 6, num_cols: 20, cell_width: 8, cell_height: 16 };

    /// Hands out fixed chunks, then reports `WouldBlock` like a drained PTY.
    struct Chunks(VecDeque<Vec<u8>>);

    impl Read for Chunks {
        fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
            let Some(mut chunk) = self.0.pop_front() else {
                return Err(io::ErrorKind::WouldBlock.into());
            };
            let n = chunk.len().min(buf.len());
            buf[..n].copy_from_slice(&chunk[..n]);
            if n < chunk.len() {
                self.0.push_front(chunk.split_off(n));
            }
            Ok(n)
        }
    }

    struct Run {
        proxy: EventProxy,
        _events: crate::terminal::PtyEvents,
        out: Vec<u8>,
    }

    /// Everything the filter hands over for these chunks, read `buf_len` bytes at a time.
    fn filter(chunks: &[&[u8]], buf_len: usize) -> Run {
        let (proxy, events) = EventProxy::new();
        proxy.shared().set_size(GRID);
        let mut filter = ImageFilter::new(proxy.clone());
        let mut inner = Chunks(chunks.iter().filter(|c| !c.is_empty()).map(|c| c.to_vec()).collect());
        let mut out = Vec::new();
        let mut buf = vec![0u8; buf_len];
        loop {
            match filter.read_from(|b| inner.read(b), &mut buf) {
                Ok(0) => break,
                Ok(n) => out.extend_from_slice(&buf[..n]),
                Err(err) if err.kind() == io::ErrorKind::WouldBlock => break,
                Err(err) => panic!("read failed: {err}"),
            }
        }
        Run { proxy, _events: events, out }
    }

    fn shown(bytes: &[u8]) -> Term<VoidListener> {
        let size = GridSize { cols: 20, rows: 6, cell_width: 8.0, cell_height: 16.0 };
        let mut term = Term::new(Config { scrolling_history: 100, ..Config::default() }, &size, VoidListener);
        Processor::<StdSyncHandler>::new().advance(&mut term, bytes);
        term
    }

    fn screen_text(term: &Term<VoidListener>) -> String {
        (0..term.screen_lines())
            .map(|line| {
                (0..term.columns())
                    .map(|col| {
                        let c = term.grid()[Line(line as i32)][Column(col)].c;
                        if c == PLACEHOLDER { '#' } else { c }
                    })
                    .collect::<String>()
                    .trim_end()
                    .to_string()
            })
            .collect::<Vec<_>>()
            .join("\n")
            .trim_end()
            .to_string()
    }

    /// (col, image, row, col-in-image) of the placeholders on `line`.
    fn placeholders(term: &Term<VoidListener>, line: i32) -> Vec<(usize, u32, u16, u16)> {
        (0..term.columns())
            .filter_map(|col| {
                let p = placeholder_at(&term.grid()[Line(line)][Column(col)])?;
                Some((col, p.image, p.row, p.col))
            })
            .collect()
    }

    /// Wait for the decode thread to put image `id` in the store.
    fn decoded(run: &Run, id: u32) -> Arc<images::Image> {
        let deadline = Instant::now() + Duration::from_secs(5);
        loop {
            if let Some(image) = run.proxy.shared().images().get(id) {
                return image;
            }
            assert!(Instant::now() < deadline, "image {id} never decoded");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    fn b64(bytes: &[u8]) -> String {
        base64::engine::general_purpose::STANDARD.encode(bytes)
    }

    #[test]
    fn everything_else_passes_unchanged_however_reads_split_it() {
        let input = b"plain \x1b[32mtext\x1b[0m \x1b]0;title\x07 \x1b_Xother apc\x1b\\ \x1bP$qm\x1b\\ \x1b]1338;x\x07 \x1b]133;A\x07$ \x1b\x1b_";
        for buf_len in [1, 7, 64, 4096] {
            assert_eq!(filter(&[input], buf_len).out, input, "buffer {buf_len}");
        }
        let chunks: Vec<&[u8]> = input.chunks(3).collect();
        assert_eq!(filter(&chunks, 64).out, input);
        // Bytes held back at the end of a read go on when the stream pauses.
        assert_eq!(filter(&[b"a\x1b]13"], 64).out, b"a\x1b]13");
        assert_eq!(filter(&[b"a\x1b_"], 64).out, b"a\x1b_");
    }

    #[test]
    fn a_kitty_image_is_placed_at_the_cursor_and_decoded_off_the_thread() {
        // 10×20 RGB pixels, sent in one piece, displayed at once.
        let pixels = vec![200u8; 10 * 20 * 3];
        let command = format!("\x1b_Ga=T,f=24,s=10,v=20,i=5;{}\x1b\\", b64(&pixels));
        let input = format!("$ icat\r\n{command}\r\n$ ");
        for split in [0, 5, 13, 40, input.len() / 2, input.len() - 1] {
            let (a, b) = input.as_bytes().split_at(split);
            let run = filter(&[a, b], 64);
            let term = shown(&run.out);
            // Two cells wide (10 px over 8 px cells), two tall (20 over 16).
            assert_eq!(screen_text(&term), "$ icat\n##\n##\n$", "split at {split}");
            assert_eq!(placeholders(&term, 1), [(0, 5, 0, 0), (1, 5, 0, 1)]);
            assert_eq!(placeholders(&term, 2), [(0, 5, 1, 0), (1, 5, 1, 1)]);
            let store = run.proxy.shared().images();
            assert_eq!(store.dimensions(5), Some((10, 20)));
            assert_eq!(store.placement(5, 0).map(|p| (p.cols, p.rows, p.fit)), Some((2, 2, false)));
            drop(store);
            let image = decoded(&run, 5);
            assert_eq!((image.width, image.height, image.rgba[0]), (10, 20, 200));
        }
    }

    #[test]
    fn chunks_and_a_later_placement_with_a_size() {
        let png = images::decode::tests::png(16, 8, [1, 2, 3, 255]);
        let data = b64(&png);
        let (a, b) = data.split_at(data.len() / 2);
        // Transmitted in two chunks, then placed twice: once as it is, once in 1×1 fitted.
        let input = format!(
            "\x1b_Ga=t,f=100,i=9,m=1;{a}\x1b\\\x1b_Gm=0;{b}\x1b\\x\x1b_Ga=p,i=9\x1b\\\r\n\x1b_Ga=p,i=9,p=2,c=1,r=1\x1b\\"
        );
        let run = filter(&[input.as_bytes()], 32);
        let term = shown(&run.out);
        assert_eq!(screen_text(&term), "x##\n#");
        assert_eq!(placeholders(&term, 0), [(1, 9, 0, 0), (2, 9, 0, 1)]);
        assert_eq!(placeholders(&term, 1), [(0, 9, 0, 0)]);
        let store = run.proxy.shared().images();
        assert_eq!(store.placement(9, 0).map(|p| (p.cols, p.rows)), Some((2, 1)));
        assert_eq!(store.placement(9, 2).map(|p| (p.cols, p.rows, p.fit)), Some((1, 1, true)));
        drop(store);
        assert_eq!(decoded(&run, 9).rgba[..4], [1, 2, 3, 255]);
    }

    #[test]
    fn deletes_hide_placements_and_free_images() {
        let pixels = [0u8; 3];
        let image = |i: u32| format!("\x1b_Ga=T,f=24,s=1,v=1,i={i},p=1;{}\x1b\\", b64(&pixels));
        let input = format!("{}{}\x1b_Ga=d,d=i,i=1,p=1\x1b\\\x1b_Ga=d,d=I,i=2\x1b\\", image(1), image(2));
        let run = filter(&[input.as_bytes()], 64);
        let store = run.proxy.shared().images();
        assert!(!store.is_shown(1, 1) && store.contains(1));
        assert!(!store.contains(2), "freed");
        drop(store);
        let input = format!("{}\x1b_Ga=d,d=A\x1b\\", image(3));
        let run = filter(&[input.as_bytes()], 64);
        assert!(run.proxy.shared().images().is_empty());
    }

    #[test]
    fn what_kitty_refuses_prints_nothing_and_the_stream_goes_on() {
        let pixels = [0u8; 3];
        let refused = [
            format!("\x1b_Ga=T,f=24,s=1,v=1,t=f,i=1;{}\x1b\\", b64(b"/etc/passwd")),
            format!("\x1b_Ga=T,f=24,i=2;{}\x1b\\", b64(&pixels)),
            format!("\x1b_Ga=T,f=24,s=100000,v=100000,i=3;{}\x1b\\", b64(&pixels)),
            format!("\x1b_Ga=T,f=100,i=4;{}\x1b\\", b64(b"not a png")),
            "\x1b_Ga=p,i=77\x1b\\".to_string(),
            "\x1b_Ga=f,i=1\x1b\\".to_string(),
            "\x1b_Gnot control\x1b\\".to_string(),
            format!("\x1b_Ga=T,f=24,s=1,v=1,i=6;{}\x18after\x1b\\", b64(&pixels)),
        ];
        for sequence in refused {
            let input = format!("a{sequence}b");
            let run = filter(&[input.as_bytes()], 64);
            let text = screen_text(&shown(&run.out));
            // After CAN the rest prints; the lone `ESC \` that follows is a complete escape to
            // alacritty, which shows nothing of it.
            assert!(text == "ab" || text == "aafterb", "{sequence:?} left {text:?}");
            assert!(run.proxy.shared().images().is_empty(), "{sequence:?}");
        }
        // Over the cap: skipped to its end, the rest of the output intact.
        let mut huge = b"\x1b_Ga=T,f=24,s=1,v=1,i=8;".to_vec();
        huge.extend(std::iter::repeat_n(b'A', super::KITTY_MAX + 10));
        huge.extend_from_slice(b"\x1b\\tail");
        let run = filter(&[&huge], 4096);
        assert_eq!(run.out, b"tail");
    }

    #[test]
    fn a_sixel_is_decoded_here_and_the_cursor_ends_under_it() {
        // 9 px wide, two bands (12 px): two cells by one.
        let input = b"a\x1bP0;1q#1;2;100;0;0!9~-!9~\x1b\\z";
        let run = filter(&[input], 16);
        let term = shown(&run.out);
        // The cursor is under the picture's left edge, where it began.
        assert_eq!(screen_text(&term), "a##\n z");
        let store = run.proxy.shared().images();
        assert_eq!(store.len(), 1);
        let id = placeholders(&term, 0)[0].1;
        drop(store);
        let image = run.proxy.shared().images().get(id).expect("decoded in place");
        assert_eq!((image.width, image.height), (9, 12));
        assert_eq!(&image.rgba[..4], &[255, 0, 0, 255]);
        // Cut short, or empty: nothing shows.
        assert_eq!(screen_text(&shown(&filter(&[b"a\x1bPq#1~\x18b"], 16).out)), "ab");
        assert_eq!(screen_text(&shown(&filter(&[b"a\x1bPq\x1b\\b"], 16).out)), "ab");
    }

    #[test]
    fn iterm2_files_show_inline_sized_as_asked_and_in_parts() {
        let png = images::decode::tests::png(24, 32, [9, 9, 9, 255]);
        let data = b64(&png);
        let input = format!("\x1b]1337;File=name=eC5wbmc=;inline=1;width=4:{data}\x07.");
        let run = filter(&[input.as_bytes()], 64);
        let term = shown(&run.out);
        // Four cells wide as asked, the height by the aspect ratio: 32 px per 24 px over 8×16
        // cells. What follows goes right of the last row, as after a kitty image.
        assert_eq!(screen_text(&term), "####\n####\n####.");
        let id = placeholders(&term, 0)[0].1;
        assert_eq!(
            run.proxy.shared().images().placement(id, 0).map(|p| (p.cols, p.rows, p.fit)),
            Some((4, 3, false))
        );
        assert_eq!(decoded(&run, id).width, 24);
        // Not inline: a download, nothing to show. Not a picture: nothing either.
        assert_eq!(
            screen_text(&shown(
                &filter(&[format!("a\x1b]1337;File=inline=0:{data}\x07b").as_bytes()], 64).out
            )),
            "ab"
        );
        assert_eq!(screen_text(&shown(&filter(&[b"a\x1b]1337;File=inline=1:bm90\x1b\\b"], 64).out)), "ab");
        // In parts, with both sizes given: fitted inside them.
        let (a, b) = data.split_at(7);
        let parts = format!(
            "\x1b]1337;MultipartFile=inline=1;width=2;height=1\x07\x1b]1337;FilePart={a}\x07\x1b]1337;FilePart={b}\x07\x1b]1337;FileEnd\x07"
        );
        let run = filter(&[parts.as_bytes()], 8);
        let term = shown(&run.out);
        assert_eq!(screen_text(&term), "##");
        let id = placeholders(&term, 0)[0].1;
        assert_eq!(
            run.proxy.shared().images().placement(id, 0).map(|p| (p.cols, p.rows, p.fit)),
            Some((2, 1, true))
        );
    }

    #[test]
    fn cells_follow_the_picture_the_request_and_the_grid() {
        assert_eq!(cells_for(10, 20, 0, 0, GRID), (2, 2));
        assert_eq!(cells_for(16, 16, 4, 0, GRID), (4, 2), "rows by the aspect ratio");
        assert_eq!(cells_for(16, 16, 0, 1, GRID), (2, 1));
        assert_eq!(cells_for(16, 16, 3, 5, GRID), (3, 5), "as asked");
        // Wider than the grid: a picture sized by itself scales down, one sized by the program
        // is clipped to the grid's width.
        assert_eq!(cells_for(400, 160, 0, 0, GRID), (20, 4));
        assert_eq!(cells_for(400, 160, 50, 10, GRID), (20, 10));
        assert_eq!(cells_for(1, 1, 0, 0, GRID), (1, 1));
        assert_eq!(cells_for(100_000, 100_000, 0, 0, GRID), (20, 10), "a square stays one");
        assert_eq!(cells_for(10, 100_000, 0, 0, GRID), (2, 297), "never more rows than the marks count");
    }
}
