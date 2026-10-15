#![no_main]

//! One payload of any message type straight into the message decoder, skipping the framing, so
//! the fuzzer spends its time on the payload parsers (the first byte is the type id).

use libfuzzer_sys::fuzz_target;
use wormhole_wire::Message;
use wormhole_wire_fuzz::check_round_trip;

fuzz_target!(|data: &[u8]| {
    let [ty, payload @ ..] = data else { return };
    if let Ok(Some(msg)) = Message::decode(*ty, payload) {
        check_round_trip(&msg);
    }
});
