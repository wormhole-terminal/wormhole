use std::os::unix::net::{UnixListener, UnixStream};

/// A connection to a session host or a control socket.
pub type Stream = UnixStream;
/// A socket that accepts [`Stream`]s.
pub type Listener = UnixListener;
