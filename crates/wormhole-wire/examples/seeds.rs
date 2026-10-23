//! Writes the fuzzer's starting corpus: every sample message as a frame for `wire_frame` (after
//! its two control bytes) and as a payload for `wire_message` (after its type id), and all of them
//! as one stream. `scripts/fuzz.sh seeds` runs this.
//!
//!     cargo run -p wormhole-wire --example seeds -- crates/wormhole-wire/fuzz/corpus

#![allow(clippy::disallowed_methods, reason = "a tool that writes the fuzz corpus, not the app")]

use std::fs;
use std::path::PathBuf;

use wormhole_wire::{HEADER_LEN, encode, samples};

fn main() -> std::io::Result<()> {
    let root = PathBuf::from(std::env::args().nth(1).unwrap_or_else(|| "corpus".into()));
    let (frames, messages) = (root.join("wire_frame"), root.join("wire_message"));
    fs::create_dir_all(&frames)?;
    fs::create_dir_all(&messages)?;
    let mut stream = vec![0, 0];
    for (name, msg) in samples::all() {
        let mut bytes = Vec::new();
        if encode(&mut bytes, 1, 0, &msg).is_err() {
            continue;
        }
        let mut seed = vec![0, 0];
        seed.extend_from_slice(&bytes);
        fs::write(frames.join(name), &seed)?;
        // A cut every 3 bytes, to start the fuzzer on the incremental path.
        seed[0] = 2;
        fs::write(frames.join(format!("{name}-cut")), &seed)?;
        stream.extend_from_slice(&bytes);
        let mut payload = vec![msg.ty()];
        payload.extend_from_slice(&bytes[HEADER_LEN..]);
        fs::write(messages.join(name), &payload)?;
    }
    fs::write(frames.join("all-samples"), &stream)?;
    Ok(())
}
