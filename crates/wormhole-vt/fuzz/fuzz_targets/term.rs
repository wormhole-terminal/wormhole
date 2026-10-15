#![no_main]

//! A terminal fed a script (see the crate's docs): arbitrary bytes at any size, with resizes,
//! scrolling, selections, searches, pins and snapshot reads between them, and the terminal's
//! invariants checked after every step.

use libfuzzer_sys::fuzz_target;
use wormhole_vt_fuzz::{Bytes, Run};

fuzz_target!(|data: &[u8]| {
    Run::script(&mut Bytes(data));
});
