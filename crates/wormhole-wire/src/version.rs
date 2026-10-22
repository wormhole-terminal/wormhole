//! Version rules. A protocol **major** is a break; a **minor** only adds (messages, TLV tags,
//! event kinds, enum values, feature bits), so a reader of minor N skips what minor N+1 added
//! and nothing else changes. The wire carries majors as ranges (`Hello.proto_min..=proto_max`);
//! the minor travels for logs, and optional behavior is gated by [`crate::Features`].
//!
//! The app speaks the current major and the one before it. A host older than that is "older
//! host, read-only" in the app: it can still be listed and killed, because the admin set
//! (`Hello`, `Welcome`, `Incompatible`, `GetInfo`, `Info`, `Kill`) is frozen across majors.

use crate::boot::{Hello, Incompatible};
use crate::types::AppVersion;

/// The protocol major this build speaks.
pub const PROTO_MAJOR: u16 = 1;
/// The minor of that major: bumped when a message, tag or kind is added.
pub const PROTO_MINOR: u16 = 1;
/// How many majors back a client keeps a codec for (the doc's "last two majors").
pub const MAJORS_BACK: u16 = 1;

/// An inclusive range of protocol majors.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range {
    pub min: u16,
    pub max: u16,
}

/// How a host fits a client.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostFit {
    /// Both speak this major.
    Compatible(u16),
    /// The host is older than the client's oldest major: show it read-only; `Admin` and `Kill` work.
    TooOld,
    /// The host needs a newer client than this one: the app has to be updated to attach.
    TooNew,
}

impl Range {
    pub const fn new(min: u16, max: u16) -> Self {
        Self { min, max }
    }

    /// What this build speaks as a client or host: the current major and the one before.
    pub const fn current() -> Self {
        let back = PROTO_MAJOR.saturating_sub(MAJORS_BACK);
        Self { min: if back == 0 { 1 } else { back }, max: PROTO_MAJOR }
    }

    /// What an `Admin` connection offers: every major, because the admin set never changes.
    pub const fn admin() -> Self {
        Self { min: 1, max: PROTO_MAJOR }
    }

    /// The newest major both ranges contain.
    pub fn agree(self, other: Self) -> Option<u16> {
        let (lo, hi) = (self.min.max(other.min), self.max.min(other.max));
        (lo <= hi).then_some(hi)
    }

    /// Classify a host's range (learned from its `Incompatible`) against this client's.
    pub fn fit(self, host: Self) -> HostFit {
        match self.agree(host) {
            Some(major) => HostFit::Compatible(major),
            None if host.max < self.min => HostFit::TooOld,
            None => HostFit::TooNew,
        }
    }

    /// A host's decision on a `Hello`: the major to speak, or the `Incompatible` to answer with.
    pub fn answer(self, hello: &Hello<'_>, host_version: AppVersion) -> Result<u16, Incompatible> {
        self.agree(Self::new(hello.proto_min, hello.proto_max)).ok_or(Incompatible {
            host_min: self.min,
            host_max: self.max,
            host_version,
        })
    }
}
