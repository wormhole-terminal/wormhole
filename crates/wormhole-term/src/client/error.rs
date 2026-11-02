//! Why an attach to a running session host failed, as data. The difference matters: a host that is
//! **gone** has nothing to lose a terminal to, while one that is alive but refused, outdated, busy
//! or slow still runs the person's program, and the terminal must keep its name (`AttachError` is
//! what `wormhole-core` classifies into "gone" or "unavailable", with the one check this crate
//! cannot make: whether the process the meta names still runs).

use std::io;

use wormhole_wire::version::Range;
use wormhole_wire::{Incompatible, error_code};

/// Why [`Pty::attach_hosted`](crate::Pty::attach_hosted) did not attach.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AttachError {
    /// Nothing accepted the connection (no socket, a refused connection): the host may be gone, or
    /// only its socket. Whoever can look at its process decides.
    Unreachable(String),
    /// The host is there and will not serve this app: it said so (`PEER_REFUSED`), closed the
    /// connection without a word (a host older than protocol 1.3 does), or the socket is another
    /// user's. The program runs on in the host.
    Refused(String),
    /// The host speaks no protocol major this build does.
    Incompatible { host_min: u16, host_max: u16, host_version: String, ours: Range },
    /// The host has all the connections it allows.
    Busy,
    /// The host took the connection and did not answer in time.
    TimedOut,
    /// Anything else: the socket is another host's, or the stream was not a host's.
    Failed(String),
}

impl AttachError {
    /// A failure to connect to the socket.
    pub(crate) fn from_connect(err: &io::Error) -> Self {
        match err.kind() {
            io::ErrorKind::NotFound | io::ErrorKind::ConnectionRefused => Self::Unreachable(err.to_string()),
            io::ErrorKind::PermissionDenied => Self::Refused(err.to_string()),
            io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => Self::TimedOut,
            _ => Self::Failed(err.to_string()),
        }
    }

    /// A failure to talk to a host that took the connection: it is there.
    pub(crate) fn from_talking(err: &io::Error) -> Self {
        match err.kind() {
            io::ErrorKind::TimedOut | io::ErrorKind::WouldBlock => Self::TimedOut,
            io::ErrorKind::UnexpectedEof | io::ErrorKind::BrokenPipe | io::ErrorKind::ConnectionReset => {
                Self::Refused("the session host closed the connection".into())
            }
            _ => Self::Failed(err.to_string()),
        }
    }

    /// The host's `Error` in answer to `Hello`.
    pub(crate) fn from_error_code(code: u16, text: &str) -> Self {
        match code {
            error_code::BUSY => Self::Busy,
            error_code::PEER_REFUSED => Self::Refused(text.to_owned()),
            _ => Self::Failed(text.to_owned()),
        }
    }

    /// The host's `Incompatible` in answer to `Hello`.
    pub(crate) fn from_incompatible(i: &Incompatible, ours: Range) -> Self {
        Self::Incompatible {
            host_min: i.host_min,
            host_max: i.host_max,
            host_version: format!(
                "{}.{}.{}",
                i.host_version.major, i.host_version.minor, i.host_version.patch
            ),
            ours,
        }
    }
}

impl std::fmt::Display for AttachError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Unreachable(why) => write!(f, "could not reach the session host: {why}"),
            Self::Refused(why) => write!(f, "the session host will not serve this app: {why}"),
            Self::Incompatible { host_min, host_max, host_version, ours } => write!(
                f,
                "the session host speaks protocol {host_min}..{host_max} (version {host_version}), this app {}..{}",
                ours.min, ours.max
            ),
            Self::Busy => f.write_str("the session host has all the connections it allows"),
            Self::TimedOut => f.write_str("the session host did not answer"),
            Self::Failed(why) => write!(f, "the session host's handshake failed: {why}"),
        }
    }
}

impl std::error::Error for AttachError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_failure_to_connect_is_unreachable_only_when_nothing_is_there() {
        let kind = |k: io::ErrorKind| AttachError::from_connect(&io::Error::new(k, "x"));
        assert!(matches!(kind(io::ErrorKind::NotFound), AttachError::Unreachable(_)));
        assert!(matches!(kind(io::ErrorKind::ConnectionRefused), AttachError::Unreachable(_)));
        assert!(matches!(kind(io::ErrorKind::PermissionDenied), AttachError::Refused(_)));
        assert_eq!(kind(io::ErrorKind::TimedOut), AttachError::TimedOut);
        assert!(matches!(kind(io::ErrorKind::Other), AttachError::Failed(_)));
    }

    #[test]
    fn a_host_that_took_the_connection_and_closed_it_is_there_and_refusing() {
        let talk = |k: io::ErrorKind| AttachError::from_talking(&io::Error::new(k, "x"));
        assert!(matches!(talk(io::ErrorKind::UnexpectedEof), AttachError::Refused(_)));
        assert!(matches!(talk(io::ErrorKind::ConnectionReset), AttachError::Refused(_)));
        assert_eq!(talk(io::ErrorKind::TimedOut), AttachError::TimedOut);
    }

    #[test]
    fn the_hosts_error_codes_map_to_their_failures() {
        assert_eq!(AttachError::from_error_code(error_code::BUSY, "full"), AttachError::Busy);
        assert_eq!(
            AttachError::from_error_code(error_code::PEER_REFUSED, "no"),
            AttachError::Refused("no".into())
        );
        assert!(matches!(AttachError::from_error_code(error_code::INVALID, "bad"), AttachError::Failed(_)));
    }
}
