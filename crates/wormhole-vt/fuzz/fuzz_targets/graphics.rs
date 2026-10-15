#![no_main]

//! The picture protocols: kitty's APC commands, Sixel in a DCS, iTerm2's OSC 1337 files, framed
//! around arbitrary bytes. The script is frames separated by `0xFF`; the first byte of a frame
//! says how it is wrapped. The host answers every picture (decoded, failed, too big) and the
//! terminal's invariants (picture budgets included) are checked after each frame.

use libfuzzer_sys::fuzz_target;
use wormhole_vt_fuzz::{Bytes, Run, base64};

/// A frame, wrapped as the protocol says. `raw` frames are sent as they are, `b64` ones have the
/// bytes after the first `;` (kitty) or `:` (iTerm2) encoded, so that real pictures get through.
fn wrap(frame: &[u8]) -> Vec<u8> {
    let Some((&kind, body)) = frame.split_first() else { return Vec::new() };
    let mut out = Vec::new();
    match kind % 8 {
        0 => {
            out.extend_from_slice(b"\x1b_G");
            out.extend_from_slice(body);
            out.extend_from_slice(b"\x1b\\");
        }
        1 => {
            out.extend_from_slice(b"\x1b_G");
            if let Some(at) = body.iter().position(|&b| b == b';') {
                out.extend_from_slice(&body[..=at]);
                out.extend_from_slice(base64::encode(&body[at + 1..]).as_bytes());
            } else {
                out.extend_from_slice(body);
            }
            out.extend_from_slice(b"\x1b\\");
        }
        2 => {
            out.extend_from_slice(b"\x1bP");
            out.extend_from_slice(body);
            out.extend_from_slice(b"\x1b\\");
        }
        3 => {
            out.extend_from_slice(b"\x1bPq");
            out.extend_from_slice(body);
            out.extend_from_slice(b"\x1b\\");
        }
        4 => {
            out.extend_from_slice(b"\x1b]1337;");
            out.extend_from_slice(body);
            out.push(7);
        }
        5 => {
            out.extend_from_slice(b"\x1b]1337;File=");
            if let Some(at) = body.iter().position(|&b| b == b':') {
                out.extend_from_slice(&body[..=at]);
                out.extend_from_slice(base64::encode(&body[at + 1..]).as_bytes());
            } else {
                out.extend_from_slice(body);
            }
            out.extend_from_slice(b"\x1b\\");
        }
        // Text and movement around the pictures: they scroll, erase and overwrite.
        6 => out.extend_from_slice(body),
        _ => out.extend_from_slice(b"\x1b[2J\x1b[H\x1b[3J"),
    }
    out
}

fuzz_target!(|data: &[u8]| {
    let mut bytes = Bytes(data);
    let mut run = Run::new(&mut bytes);
    for frame in bytes.frames().into_iter().take(64) {
        match frame.split_first() {
            Some((&kind, rest)) if kind % 8 == 7 && !rest.is_empty() => run.call_with(rest),
            _ => run.feed(&wrap(&frame)),
        }
        run.check("after a frame");
    }
});
