//! Your own sequences: strings the engine does not know, handed to the host as events.
//!
//! A program can send an OSC, DCS or APC string that means nothing to the engine and everything to
//! the host (a status report, a progress bar, a private protocol). The engine drops such a string
//! unless the host asked for it. A [`Passthrough`] is that ask: which numbers and introducers, and
//! how much payload each may carry. The engine then collects a claimed string whole, up to the
//! limit, and queues it as [`Event::Osc`](crate::Event::Osc), [`Event::Dcs`](crate::Event::Dcs)
//! or [`Event::Apc`](crate::Event::Apc) in order with its other events. Nothing of the host runs
//! inside [`Terminal::feed`](crate::Terminal::feed): the host reads the event later, outside the
//! parser, and answers by writing to the program.
//!
//! ```
//! use wormhole_vt::passthrough::Passthrough;
//! use wormhole_vt::{Event, Terminal};
//!
//! let mut term = Terminal::builder()
//!     .passthrough(Passthrough::new().osc(7777, 1024))? // OSC 7777 ; payload, up to 1 KiB
//!     .build();
//! term.feed(b"\x1b]7777;compiling\x07");
//! let events: Vec<Event> = term.events().collect();
//! assert!(matches!(&events[..], [Event::Osc { number: 7777, payload, .. }] if payload == b"compiling"));
//! # Ok::<(), wormhole_vt::passthrough::PassthroughError>(())
//! ```
//!
//! The engine's own sequences cannot be claimed: a built-in stays built-in. OSC numbers the engine
//! acts on (titles, colors, the working folder, hyperlinks, notifications, the clipboard, shell
//! marks, iTerm2's), and the DCS and APC strings it streams into decoders (Sixel, DECRQSS,
//! XTGETTCAP, the DEC state reports, kitty graphics), are refused with a [`PassthroughError`].

use std::fmt;

use crate::parser::{Csi, MAX_INTERMEDIATES};

/// The most payload bytes one claim may ask for (1 MiB). A larger number is clamped to it: a
/// program cannot make the terminal buffer more than this for a claimed string, whatever the host
/// wrote. A built-in string can be larger (a clipboard or a picture); a protocol that needs more
/// takes it in pieces over several sequences.
pub const MAX_BYTES: usize = 1 << 20;

/// The most claims one [`Passthrough`] holds, of all kinds together.
const MAX_CLAIMS: usize = 256;

/// The DCS strings the engine reads itself, streaming their payloads into decoders or tables:
/// (intermediates, final, name).
const RESERVED_DCS: &[(&[u8], u8, &str)] = &[
    (b"$", b'q', "DECRQSS (DCS $ q)"),
    (b"+", b'q', "XTGETTCAP (DCS + q)"),
    (b"", b'q', "Sixel (DCS q)"),
    (b"!", b'u', "DECAUPSS (DCS ! u)"),
    (b"$", b't', "DECRSPS (DCS $ t)"),
    (b"$", b'p', "DECRSTS (DCS $ p)"),
];

/// The APC strings the engine reads itself: (first byte, name).
const RESERVED_APC: &[(u8, &str)] = &[(b'G', "the kitty graphics protocol (APC G)")];

/// Why a [`Passthrough`] was refused by [`Builder::passthrough`](crate::Builder::passthrough) or
/// [`Terminal::set_passthrough`](crate::Terminal::set_passthrough). Nothing is changed when it is.
#[derive(Clone, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum PassthroughError {
    /// More claims than one [`Passthrough`] may hold.
    TooMany,
    /// An OSC number the parser cannot read: it has at most five digits.
    OscNumber(u32),
    /// The engine acts on this OSC number itself.
    ReservedOsc(u32),
    /// A DCS claim that no string can match: an intermediate outside 0x20..=0x2F (or more than
    /// [`MAX_INTERMEDIATES`]), or a final outside 0x40..=0x7E.
    Dcs,
    /// An APC claim that no string can match: the first byte is a control or DEL.
    Apc(u8),
    /// The engine reads this DCS or APC string itself and streams its payload; the name says which.
    Reserved(&'static str),
}

impl fmt::Display for PassthroughError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PassthroughError::TooMany => write!(f, "at most {MAX_CLAIMS} sequences can be passed through"),
            PassthroughError::OscNumber(n) => {
                write!(f, "OSC {n} cannot be claimed: numbers have at most five digits")
            }
            PassthroughError::ReservedOsc(n) => {
                write!(f, "OSC {n} is read by the engine and cannot be claimed")
            }
            PassthroughError::Dcs => f.write_str("the DCS claim matches no string (intermediates or final)"),
            PassthroughError::Apc(byte) => write!(f, "APC cannot start with byte {byte:#04x}"),
            PassthroughError::Reserved(name) => {
                write!(f, "{name} is read by the engine and cannot be claimed")
            }
        }
    }
}

impl std::error::Error for PassthroughError {}

#[derive(Clone, Debug)]
struct DcsClaim {
    intermediates: Vec<u8>,
    final_byte: u8,
    cap: usize,
}

/// The strings to hand to the host, built with chained calls and given to
/// [`Builder::passthrough`](crate::Builder::passthrough). See the [module docs](self).
///
/// A claim that cannot work is remembered and reported where the value is given to the terminal,
/// not here, so the calls chain without `?`.
#[derive(Clone, Debug, Default)]
pub struct Passthrough {
    osc: Vec<(u32, usize)>,
    dcs: Vec<DcsClaim>,
    apc: Vec<(u8, usize)>,
    error: Option<PassthroughError>,
}

fn cap(max_bytes: usize) -> usize {
    max_bytes.min(MAX_BYTES)
}

impl Passthrough {
    /// Nothing passed through: every string the engine does not know is dropped.
    pub fn new() -> Passthrough {
        Passthrough::default()
    }

    fn fail(&mut self, error: PassthroughError) {
        self.error.get_or_insert(error);
    }

    fn count(&self) -> usize {
        self.osc.len() + self.dcs.len() + self.apc.len()
    }

    /// Operating system commands with this number (`OSC number ; payload`), as
    /// [`Event::Osc`](crate::Event::Osc). `max_bytes` is the most payload (what follows the `;`)
    /// accepted, at most [`MAX_BYTES`]; a longer string is dropped whole and nothing is queued.
    pub fn osc(mut self, number: u32, max_bytes: usize) -> Passthrough {
        if number > 99_999 {
            self.fail(PassthroughError::OscNumber(number));
        } else if crate::terminal::is_builtin_osc(number) {
            self.fail(PassthroughError::ReservedOsc(number));
        } else if self.count() >= MAX_CLAIMS {
            self.fail(PassthroughError::TooMany);
        } else if let Some(claim) = self.osc.iter_mut().find(|c| c.0 == number) {
            claim.1 = cap(max_bytes);
        } else {
            self.osc.push((number, cap(max_bytes)));
        }
        self
    }

    /// Device control strings with these `intermediates` and this `final_byte` and no private
    /// marker (`DCS params intermediates final data ST`), as [`Event::Dcs`](crate::Event::Dcs).
    /// `max_bytes` is the most data accepted, at most [`MAX_BYTES`]; a longer string is dropped
    /// whole.
    ///
    /// The strings the engine reads itself are refused: Sixel (`q`), DECRQSS (`$ q`), XTGETTCAP
    /// (`+ q`), DECAUPSS (`! u`), DECRSPS (`$ t`) and DECRSTS (`$ p`).
    pub fn dcs(mut self, intermediates: &[u8], final_byte: u8, max_bytes: usize) -> Passthrough {
        let shaped = intermediates.len() <= MAX_INTERMEDIATES
            && intermediates.iter().all(|b| (0x20..=0x2F).contains(b))
            && (0x40..=0x7E).contains(&final_byte);
        if !shaped {
            self.fail(PassthroughError::Dcs);
        } else if let Some((_, _, name)) =
            RESERVED_DCS.iter().find(|(inter, fin, _)| *inter == intermediates && *fin == final_byte)
        {
            self.fail(PassthroughError::Reserved(name));
        } else if self.count() >= MAX_CLAIMS {
            self.fail(PassthroughError::TooMany);
        } else if let Some(claim) =
            self.dcs.iter_mut().find(|c| c.final_byte == final_byte && c.intermediates == intermediates)
        {
            claim.cap = cap(max_bytes);
        } else {
            self.dcs.push(DcsClaim {
                intermediates: intermediates.to_vec(),
                final_byte,
                cap: cap(max_bytes),
            });
        }
        self
    }

    /// Application program commands whose string starts with `first_byte` (`APC first_byte … ST`),
    /// as [`Event::Apc`](crate::Event::Apc), the first byte included in the payload. `max_bytes` is
    /// the most the whole string may be, at most [`MAX_BYTES`]; a longer string is dropped whole.
    ///
    /// Kitty's graphics protocol (`G`) is read by the engine and refused.
    pub fn apc(mut self, first_byte: u8, max_bytes: usize) -> Passthrough {
        if first_byte < 0x20 || first_byte == 0x7F {
            self.fail(PassthroughError::Apc(first_byte));
        } else if let Some((_, name)) = RESERVED_APC.iter().find(|(byte, _)| *byte == first_byte) {
            self.fail(PassthroughError::Reserved(name));
        } else if self.count() >= MAX_CLAIMS {
            self.fail(PassthroughError::TooMany);
        } else if let Some(claim) = self.apc.iter_mut().find(|c| c.0 == first_byte) {
            claim.1 = cap(max_bytes);
        } else {
            self.apc.push((first_byte, cap(max_bytes)));
        }
        self
    }

    /// Whether nothing is passed through.
    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    /// Whether this can be given to a terminal.
    pub(crate) fn check(&self) -> Result<(), PassthroughError> {
        self.error.clone().map_or(Ok(()), Err)
    }

    /// The OSC numbers and their limits, for the string collector.
    pub(crate) fn osc_caps(&self) -> &[(u32, usize)] {
        &self.osc
    }

    /// The data limit for a DCS with this header, if passed through.
    pub(crate) fn dcs_cap(&self, header: &Csi) -> Option<usize> {
        if header.marker().is_some() {
            return None;
        }
        self.dcs
            .iter()
            .find(|c| c.final_byte == header.final_byte() && c.intermediates == header.intermediates())
            .map(|c| c.cap)
    }

    /// The string limit for an APC starting with `first`, if passed through.
    pub(crate) fn apc_cap(&self, first: u8) -> Option<usize> {
        self.apc.iter().find(|c| c.0 == first).map(|c| c.1)
    }
}
