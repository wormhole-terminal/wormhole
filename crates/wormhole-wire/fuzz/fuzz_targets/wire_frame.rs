#![no_main]

//! Arbitrary bytes into the stream decoder, as a client or a host would receive them. It must
//! not panic or hold more than its bound; where the stream is cut must not change what comes
//! out; and every message that does come out must encode and decode back to itself.

use libfuzzer_sys::fuzz_target;
use wormhole_wire::{Decoder, MAX_CONTROL};
use wormhole_wire_fuzz::{BOUND, check_round_trip};

/// Run the decoder over `stream` fed in pieces of `piece` bytes; one line per frame or error.
fn run(stream: &[u8], piece: usize, handshake: bool) -> Vec<String> {
    let mut dec = Decoder::new();
    if handshake {
        dec.set_max_payload(MAX_CONTROL);
    }
    let mut out = Vec::new();
    for chunk in stream.chunks(piece) {
        dec.push(chunk);
        loop {
            match dec.next_frame() {
                Ok(Some(f)) => {
                    check_round_trip(&f.message);
                    out.push(format!("{f:?}"));
                }
                Ok(None) => break,
                Err(e) => {
                    out.push(format!("error {e:?}"));
                    if e.is_fatal() {
                        return out;
                    }
                }
            }
        }
        assert!(dec.capacity() <= BOUND, "buffer grew to {}", dec.capacity());
    }
    out
}

fuzz_target!(|data: &[u8]| {
    // Two bytes: how the stream is cut, and whether the handshake limit applies; then the stream.
    let [cut, mode, stream @ ..] = data else { return };
    let handshake = mode & 1 == 1;
    let whole = run(stream, stream.len().max(1), handshake);
    let pieces = run(stream, 1 + usize::from(*cut), handshake);
    assert_eq!(whole, pieces, "cut every {} bytes", 1 + usize::from(*cut));
});
