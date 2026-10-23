//! The sockets a session host and the control commands are reached on, and who is at the other end.

use std::io;

use super::user::UserId;

pub use super::unix::socket::{Listener, Stream};

#[cfg(target_os = "linux")]
use super::linux::socket as os;
#[cfg(target_os = "macos")]
use super::macos::socket as os;

/// The user at the other end of `stream`, as the system recorded it when it connected.
pub fn peer_user(stream: &Stream) -> io::Result<UserId> {
    os::peer_uid(stream).map(UserId::from_raw)
}

/// Whether the peer of `stream` is the user this process acts as. A peer whose id can't be read is
/// not: a server answers only what it can attribute.
pub fn peer_is_same_user(stream: &Stream) -> bool {
    peer_user(stream).is_ok_and(|peer| peer == UserId::current())
}
