//! Asking a live host who it is, over its socket, as an `Admin` connection: `Hello`, `Welcome`,
//! `GetInfo`, `Info`. The same short exchange `sessions end` opens with, and as harmless: an
//! admin connection takes no part of the terminal, so the app's own controller is not disturbed.
//! The Session Hosts window uses it to tell a host that answers from one that only left files
//! behind, and to read what the meta file cannot say (its state, size, history, revision).

use std::time::{Duration, Instant};

use wormhole_wire::io::{FrameReader, FrameWriter, RecvError};
use wormhole_wire::{AppVersion, CHANNEL_LOCAL, Incompatible, Message, Welcome};

use super::HostMeta;
use super::admin::admin_hello;
use crate::platform::socket::{Stream, peer_is_same_user};

/// What a host said to `GetInfo`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Probed {
    /// The host answered: its `Info`.
    Answered(Welcome),
    /// The host speaks no protocol major this build does (`Incompatible`): the range it does speak
    /// and its version, which is all an old host can say.
    Incompatible { host_min: u16, host_max: u16, version: AppVersion },
    /// Nothing usable: the socket would not connect, the peer is not this user, the host named
    /// another id, hung up or took too long. The text says which.
    Failed(String),
}

/// Ask the host `meta` describes, waiting at most `wait` for all of it.
pub fn probe(meta: &HostMeta, wait: Duration) -> Probed {
    match ask(meta, wait) {
        Ok(probed) => probed,
        Err(why) => Probed::Failed(why),
    }
}

fn ask(meta: &HostMeta, wait: Duration) -> Result<Probed, String> {
    let deadline = Instant::now() + wait;
    let left = || deadline.saturating_duration_since(Instant::now()).max(Duration::from_millis(1));
    let stream = Stream::connect(&meta.socket).map_err(|e| format!("connecting: {e}"))?;
    stream.set_write_timeout(Some(left())).map_err(|e| e.to_string())?;
    stream.set_read_timeout(Some(left())).map_err(|e| e.to_string())?;
    // The socket may sit in a folder others can write to; only our own host gets a word from us.
    if !peer_is_same_user(&stream) {
        return Err("the socket's owner is not this user".into());
    }
    let mut writer = FrameWriter::new(&stream);
    let mut reader = FrameReader::new(&stream);
    writer.send(CHANNEL_LOCAL, 0, &Message::Hello(admin_hello())).map_err(|e| format!("hello: {e}"))?;
    match next(&mut reader, &stream, left())? {
        Reply::Welcome(welcome) if welcome.session_id.as_slice() == meta.id.as_bytes() => {}
        Reply::Welcome(_) => return Err("the socket belongs to another host".into()),
        Reply::Incompatible(inc) => {
            return Ok(Probed::Incompatible {
                host_min: inc.host_min,
                host_max: inc.host_max,
                version: inc.host_version,
            });
        }
        _ => return Err("the host answered something unexpected".into()),
    }
    writer.send(CHANNEL_LOCAL, 0, &Message::GetInfo).map_err(|e| format!("get info: {e}"))?;
    // Anything else in between (a host may say more than asked) is skipped, until the deadline.
    loop {
        match next(&mut reader, &stream, left())? {
            Reply::Info(info) => return Ok(Probed::Answered(info)),
            _ if Instant::now() >= deadline => return Err("no Info in time".into()),
            _ => {}
        }
    }
}

/// The messages of the exchange, copied out of the frame they were read from.
enum Reply {
    Welcome(Welcome),
    Info(Welcome),
    Incompatible(Incompatible),
    Other,
}

/// The next message, within `wait`.
fn next(reader: &mut FrameReader<&Stream>, stream: &Stream, wait: Duration) -> Result<Reply, String> {
    stream.set_read_timeout(Some(wait)).map_err(|e| e.to_string())?;
    match reader.read_frame() {
        Ok(Some(frame)) => Ok(match frame.message {
            Message::Welcome(w) => Reply::Welcome(w),
            Message::Info(w) => Reply::Info(w),
            Message::Incompatible(i) => Reply::Incompatible(i),
            _ => Reply::Other,
        }),
        Ok(None) | Err(RecvError::UnexpectedEof) => Err("the host hung up".into()),
        Err(err) => Err(format!("reading: {err}")),
    }
}

#[cfg(test)]
mod tests;
