//! The connection before its thread takes it: connecting (and checking that the peer is this
//! user: a socket outside the hosts folder sits in a folder others may write to), the handshake,
//! and a new host's `Spawn`. Blocking, with deadlines; the decoder goes on to the thread with
//! whatever it read past the last frame.

use std::io::{self, Write as _};
use std::os::fd::AsRawFd;
use std::os::unix::net::UnixStream;
use std::path::Path;
use std::time::Instant;

use wormhole_wire::{
    self as wire, AppVersion, CHANNEL_LOCAL, Decoder, Features, Message, Role, error_code, version,
};

/// A connection whose handshake is done.
pub(crate) struct Joined {
    pub stream: UnixStream,
    /// With whatever it read past the handshake's last frame.
    pub dec: Decoder,
}

/// Connect to `socket`, refusing a peer that is not this user.
pub(super) fn connect(socket: &Path) -> io::Result<UnixStream> {
    let stream = UnixStream::connect(socket)?;
    let peer = peer_uid(&stream)?;
    if peer != euid() {
        tracing::warn!(peer, "a session host's socket is another user's; not using it");
        return Err(io::Error::new(io::ErrorKind::PermissionDenied, "the socket belongs to another user"));
    }
    Ok(stream)
}

/// Say `Hello` as the controller; the host's `Welcome`, which must be for `id`.
pub(super) fn hello(
    stream: &UnixStream,
    dec: &mut Decoder,
    id: &str,
    until: Instant,
) -> io::Result<wire::Welcome> {
    let range = version::Range::current();
    send(
        stream,
        &Message::Hello(wire::Hello {
            proto_min: range.min,
            proto_max: range.max,
            proto_minor: version::PROTO_MINOR,
            role: Role::Controller,
            // The app is a terminal's one controller: a controller still there is a connection of
            // its own the host has not seen close yet (a detach or a crash just before), or an app
            // that is stopped. Waiting for it would refuse the attach.
            force: true,
            features: Features::NONE,
            client_version: client_version(),
            client_name: "Wormhole",
            ticket: None,
        }),
    )?;
    let welcome = recv(stream, dec, until, |msg| match msg {
        Message::Welcome(welcome) => Some(Ok(*welcome)),
        Message::Incompatible(i) => Some(Err(io::Error::other(format!(
            "the session host speaks protocol {}..{} (version {}.{}.{}), this app {}..{}",
            i.host_min,
            i.host_max,
            i.host_version.major,
            i.host_version.minor,
            i.host_version.patch,
            range.min,
            range.max
        )))),
        Message::Error(e) if e.code == error_code::BUSY => {
            Some(Err(io::Error::new(io::ErrorKind::ResourceBusy, e.text.to_owned())))
        }
        Message::Error(e) => Some(Err(io::Error::other(e.text.to_owned()))),
        _ => None,
    })?;
    if welcome.session_id != *id.as_bytes() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "the session host is another terminal's"));
    }
    Ok(welcome)
}

/// Ask a new host to run `spawn`; its `Info` (the program's pid and size), or the host's text for
/// why it could not.
pub(super) fn spawn(
    stream: &UnixStream,
    dec: &mut Decoder,
    spawn: &wire::Spawn<'_>,
    until: Instant,
) -> io::Result<Result<wire::Welcome, String>> {
    send(stream, &Message::Spawn(spawn.clone()))?;
    recv(stream, dec, until, |msg| match msg {
        Message::Info(info) => Some(Ok(Ok(*info))),
        Message::Error(e) if e.ref_type == wire::ty::SPAWN => Some(Ok(Err(e.text.to_owned()))),
        _ => None,
    })
}

/// Write one frame.
pub(super) fn send(stream: &UnixStream, msg: &Message<'_>) -> io::Result<()> {
    let mut buf = Vec::new();
    wire::encode(&mut buf, CHANNEL_LOCAL, 0, msg).map_err(io::Error::other)?;
    (&mut &*stream).write_all(&buf)
}

/// Read frames until `take` makes something of one, by `until`.
fn recv<T>(
    stream: &UnixStream,
    dec: &mut Decoder,
    until: Instant,
    mut take: impl FnMut(&Message<'_>) -> Option<io::Result<T>>,
) -> io::Result<T> {
    loop {
        loop {
            match dec.next_frame() {
                Ok(Some(frame)) => {
                    if let Some(result) = take(&frame.message) {
                        return result;
                    }
                }
                Ok(None) => break,
                Err(err) if err.is_fatal() => return Err(io::Error::new(io::ErrorKind::InvalidData, err)),
                Err(err) => tracing::debug!("a frame from the session host was dropped: {err}"),
            }
        }
        let left = until.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(io::Error::new(io::ErrorKind::TimedOut, "the session host did not answer"));
        }
        stream.set_read_timeout(Some(left))?;
        match dec.fill(&mut &*stream) {
            Ok(0) => {
                return Err(io::Error::new(
                    io::ErrorKind::UnexpectedEof,
                    "the session host closed the connection",
                ));
            }
            Ok(_) => {}
            Err(err) if err.kind() == io::ErrorKind::Interrupted => {}
            Err(err) if matches!(err.kind(), io::ErrorKind::WouldBlock | io::ErrorKind::TimedOut) => {}
            Err(err) => return Err(err),
        }
    }
}

/// This build's version, as `Hello` carries it.
fn client_version() -> AppVersion {
    let mut parts = env!("CARGO_PKG_VERSION").split('.').map(|p| p.parse::<u16>().unwrap_or(0));
    AppVersion {
        major: parts.next().unwrap_or(0),
        minor: parts.next().unwrap_or(0),
        patch: parts.next().unwrap_or(0),
    }
}

#[allow(unsafe_code)] // libc FFI; no arguments.
fn euid() -> u32 {
    // SAFETY: geteuid takes nothing and cannot fail.
    unsafe { libc::geteuid() }
}

/// The effective uid of the process at the other end of a Unix socket.
#[cfg(not(target_os = "linux"))]
#[allow(unsafe_code)] // libc FFI; the descriptor is open for the call, the outputs are locals.
fn peer_uid(socket: &impl AsRawFd) -> io::Result<u32> {
    let (mut uid, mut gid) = (0, 0);
    // SAFETY: getpeereid writes two integers into locals; the descriptor is borrowed and open.
    if unsafe { libc::getpeereid(socket.as_raw_fd(), &mut uid, &mut gid) } == 0 {
        Ok(uid)
    } else {
        Err(io::Error::last_os_error())
    }
}

#[cfg(target_os = "linux")]
#[allow(unsafe_code)] // libc FFI; the credentials go into a local struct of the size given.
fn peer_uid(socket: &impl AsRawFd) -> io::Result<u32> {
    let mut cred = libc::ucred { pid: 0, uid: 0, gid: 0 };
    let mut len = std::mem::size_of::<libc::ucred>() as libc::socklen_t;
    // SAFETY: getsockopt writes at most `len` bytes into the local struct; the descriptor is open.
    let rc = unsafe {
        libc::getsockopt(
            socket.as_raw_fd(),
            libc::SOL_SOCKET,
            libc::SO_PEERCRED,
            (&mut cred as *mut libc::ucred).cast(),
            &mut len,
        )
    };
    if rc == 0 { Ok(cred.uid) } else { Err(io::Error::last_os_error()) }
}
