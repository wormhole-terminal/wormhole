//! What the two targets share: the checks a decoded message must pass.

use wormhole_wire::{Decoder, EncodeError, EventKind, Message, encode};

/// The most memory a decoder may hold: one maximum frame, its header and a read or two.
pub const BOUND: usize =
    wormhole_wire::MAX_PAYLOAD + 3 * (wormhole_wire::MAX_DATA + wormhole_wire::HEADER_LEN);

/// A message the decoder accepted must encode again (it is what a relay does) and decode to the same
/// message, unless it holds an event kind this version cannot write back, which is the only
/// message the encoder may refuse.
pub fn check_round_trip(msg: &Message<'_>) {
    let mut buf = Vec::new();
    match encode(&mut buf, 1, 0, msg) {
        Ok(()) => {
            let mut dec = Decoder::new();
            dec.push(&buf);
            let again = dec.next_frame().expect("a message we wrote must read").expect("a frame");
            assert_eq!(&again.message, msg, "the second reading differs");
            assert!(dec.next_frame().expect("no error").is_none());
        }
        Err(EncodeError::BadValue("Event kind")) => {
            let unknown_event = |k: &EventKind<'_>| matches!(k, EventKind::Unknown(_));
            let explained = match msg {
                Message::Event(e) => unknown_event(&e.kind),
                Message::Backlog(events) => events.iter().any(|e| unknown_event(&e.kind)),
                _ => false,
            };
            assert!(explained, "refused to encode a message it decoded: {msg:?}");
        }
        Err(e) => panic!("refused to encode a message it decoded ({e}): {msg:?}"),
    }
}
