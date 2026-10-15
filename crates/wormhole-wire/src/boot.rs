//! The bootstrap: `Hello`, `Welcome`, `Incompatible`. **Frozen**: the first 64 bytes of a `Hello` and
//! of a `Welcome` (the 8-byte header and 56 bytes of payload) are specified once and never change,
//! so that any version can tell any other what it speaks, even a version that does not exist
//! yet. Growth happens only in the TLV tail after them.
//!
//! `Hello` payload, offsets from the payload's start:
//!
//! ```text
//!  0 "WHSP"   4 proto_min u16   6 proto_max u16   8 proto_minor u16   10 role u8   11 hello_flags u8
//! 12 features u64   20 client_version 3×u16   26 reserved (30 bytes, zero)   56 TLVs
//! ```
//!
//! `Welcome` payload:
//!
//! ```text
//!  0 "WHSP"   4 proto u16   6 proto_minor u16   8 state u8   9 reserved u8   10 host_version 3×u16
//! 16 pid u32   20 session_id 12 bytes   32 features u64   40 started_at_ms u64   48 first_block_id u64   56 TLVs
//! ```

use crate::cursor::{Reader, Seen, Writer};
use crate::error::{EncodeError, WireError};
use crate::types::{AppVersion, Counters, ExitInfo, Features, HostState, Role, Size};

pub const MAGIC: [u8; 4] = *b"WHSP";
/// Payload bytes of the frozen part of `Hello` and `Welcome`.
pub const BOOT_FIXED: usize = 56;

/// `Hello.hello_flags`: take the controller's place from whoever holds it.
const HELLO_FORCE: u8 = 1 << 0;
/// The longest ticket a `Hello` may carry.
pub const MAX_TICKET: usize = 1024;

/// The first message a client sends, on the channel it opens.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Hello<'a> {
    /// The oldest and newest protocol major this client speaks.
    pub proto_min: u16,
    pub proto_max: u16,
    /// The client's minor at `proto_max` (for logs; behavior is gated by `features`).
    pub proto_minor: u16,
    pub role: Role,
    /// With `Role::Controller`: take over from the current controller.
    pub force: bool,
    pub features: Features,
    pub client_version: AppVersion,
    pub client_name: &'a str,
    pub ticket: Option<&'a [u8]>,
}

const H_NAME: u8 = 1;
const H_TICKET: u8 = 2;

impl<'a> Hello<'a> {
    pub(crate) fn encode(&self, w: &mut Writer<'_>) -> Result<(), EncodeError> {
        if self.proto_min == 0 || self.proto_min > self.proto_max {
            return Err(EncodeError::BadValue("Hello.proto range"));
        }
        let start = w.len();
        w.raw(&MAGIC);
        w.u16(self.proto_min);
        w.u16(self.proto_max);
        w.u16(self.proto_minor);
        w.u8(self.role.to_wire());
        w.u8(if self.force { HELLO_FORCE } else { 0 });
        w.u64(self.features.0);
        self.client_version.write(w);
        w.zeros(BOOT_FIXED - (w.len() - start));
        if !self.client_name.is_empty() {
            let at = w.tlv_begin(H_NAME);
            w.raw(self.client_name.as_bytes());
            w.tlv_end(at, "Hello.client_name")?;
        }
        if let Some(ticket) = self.ticket {
            if ticket.len() > MAX_TICKET {
                return Err(EncodeError::DataTooLarge("Hello.ticket"));
            }
            w.tlv_raw(H_TICKET, "Hello.ticket", ticket)?;
        }
        Ok(())
    }

    pub(crate) fn decode(payload: &'a [u8]) -> Result<Self, WireError> {
        const WHAT: &str = "Hello";
        let mut r = Reader::new(payload, WHAT);
        if r.fixed::<4>()? != MAGIC {
            return Err(WireError::BadMagic);
        }
        let proto_min = r.u16()?;
        let proto_max = r.u16()?;
        let proto_minor = r.u16()?;
        let role = Role::from_wire(r.u8()?);
        let flags = r.u8()?;
        let features = Features(r.u64()?);
        let client_version = AppVersion::read(&mut r)?;
        r.skip(BOOT_FIXED - 26)?;
        if proto_min == 0 || proto_min > proto_max {
            return Err(WireError::BadValue(WHAT));
        }
        let (mut client_name, mut ticket, mut seen) = ("", None, Seen::default());
        for tlv in r.tlvs() {
            let (tag, value) = tlv?;
            match tag {
                H_NAME => {
                    seen.once(tag, WHAT)?;
                    client_name = std::str::from_utf8(value).map_err(|_| WireError::BadUtf8(WHAT))?;
                }
                H_TICKET => {
                    seen.once(tag, WHAT)?;
                    if value.len() > MAX_TICKET {
                        return Err(WireError::BadValue(WHAT));
                    }
                    ticket = Some(value);
                }
                _ => {}
            }
        }
        Ok(Self {
            proto_min,
            proto_max,
            proto_minor,
            role,
            force: flags & HELLO_FORCE != 0,
            features,
            client_version,
            client_name,
            ticket,
        })
    }
}

/// What a host is: its answer to `Hello`, and the body of `Info`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Welcome {
    /// The protocol major both sides now speak.
    pub proto: u16,
    pub proto_minor: u16,
    pub state: HostState,
    pub host_version: AppVersion,
    /// The pid of the terminal's program (libproc reads it from the app).
    pub pid: u32,
    /// The 12-character random id, ASCII.
    pub session_id: [u8; 12],
    pub features: Features,
    /// Milliseconds since the Unix epoch.
    pub started_at_ms: u64,
    /// The oldest block id the host's command log still holds.
    pub first_block_id: u64,
    pub size: Size,
    pub counters: Counters,
    /// Set when `state` is `Exited`.
    pub exit: Option<ExitInfo>,
    /// The per-terminal secret (security, section 9); sent to controllers only.
    pub token: Option<[u8; 32]>,
}

const W_SIZE: u8 = 1;
const W_COUNTERS: u8 = 2;
const W_EXIT: u8 = 3;
const W_TOKEN: u8 = 4;

impl Welcome {
    pub(crate) fn encode(&self, w: &mut Writer<'_>) -> Result<(), EncodeError> {
        let start = w.len();
        w.raw(&MAGIC);
        w.u16(self.proto);
        w.u16(self.proto_minor);
        w.u8(self.state.to_wire());
        w.u8(0);
        self.host_version.write(w);
        w.u32(self.pid);
        w.raw(&self.session_id);
        w.u64(self.features.0);
        w.u64(self.started_at_ms);
        w.u64(self.first_block_id);
        debug_assert_eq!(w.len() - start, BOOT_FIXED);
        let at = w.tlv_begin(W_SIZE);
        self.size.write(w);
        w.tlv_end(at, "Welcome.size")?;
        let at = w.tlv_begin(W_COUNTERS);
        self.counters.write(w);
        w.tlv_end(at, "Welcome.counters")?;
        if let Some(exit) = &self.exit {
            let at = w.tlv_begin(W_EXIT);
            exit.write(w);
            w.tlv_end(at, "Welcome.exit")?;
        }
        if let Some(token) = &self.token {
            w.tlv_raw(W_TOKEN, "Welcome.token", token)?;
        }
        Ok(())
    }

    pub(crate) fn decode(payload: &[u8], what: &'static str) -> Result<Self, WireError> {
        let mut r = Reader::new(payload, what);
        if r.fixed::<4>()? != MAGIC {
            return Err(WireError::BadMagic);
        }
        let proto = r.u16()?;
        let proto_minor = r.u16()?;
        let state = HostState::from_wire(r.u8()?);
        r.skip(1)?;
        let host_version = AppVersion::read(&mut r)?;
        let pid = r.u32()?;
        let session_id = r.fixed::<12>()?;
        let features = Features(r.u64()?);
        let started_at_ms = r.u64()?;
        let first_block_id = r.u64()?;
        let (mut size, mut counters, mut exit, mut token) = (None, None, None, None);
        let mut seen = Seen::default();
        for tlv in r.tlvs() {
            let (tag, value) = tlv?;
            let mut v = Reader::new(value, what);
            match tag {
                W_SIZE => {
                    seen.once(tag, what)?;
                    size = Some(Size::read(&mut v)?);
                }
                W_COUNTERS => {
                    seen.once(tag, what)?;
                    counters = Some(Counters::read(&mut v)?);
                }
                W_EXIT => {
                    seen.once(tag, what)?;
                    exit = Some(ExitInfo::read(&mut v)?);
                }
                W_TOKEN => {
                    seen.once(tag, what)?;
                    token = Some(v.fixed::<32>()?);
                }
                _ => {}
            }
        }
        Ok(Self {
            proto,
            proto_minor,
            state,
            host_version,
            pid,
            session_id,
            features,
            started_at_ms,
            first_block_id,
            size: size.ok_or(WireError::MissingField(what))?,
            counters: counters.ok_or(WireError::MissingField(what))?,
            exit,
            token,
        })
    }
}

/// The answer to a `Hello` with no protocol major in common.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Incompatible {
    pub host_min: u16,
    pub host_max: u16,
    pub host_version: AppVersion,
}

impl Incompatible {
    pub(crate) fn encode(&self, w: &mut Writer<'_>) {
        w.raw(&MAGIC);
        w.u16(self.host_min);
        w.u16(self.host_max);
        self.host_version.write(w);
    }

    pub(crate) fn decode(payload: &[u8]) -> Result<Self, WireError> {
        let mut r = Reader::new(payload, "Incompatible");
        if r.fixed::<4>()? != MAGIC {
            return Err(WireError::BadMagic);
        }
        Ok(Self { host_min: r.u16()?, host_max: r.u16()?, host_version: AppVersion::read(&mut r)? })
    }
}
