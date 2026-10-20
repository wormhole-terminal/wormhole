//! Your own sequence: a private OSC the engine does not know, handed to the host as an event.
//!
//! A program asks `OSC 51234 ; ping ST` and the host answers `OSC 51234 ; pong ST`. The host asks
//! the engine for the number when it builds the terminal, reads `Event::Osc` where it reads the
//! other events, and puts its answer into the program's input at the place `replies_before` names,
//! so that the program reads its answers in the order it asked.
//!
//! ```sh
//! cargo run -p wormhole-vt --example custom_osc
//! ```

#![allow(clippy::print_stdout)] // an example: what it prints is what it shows

use wormhole_vt::passthrough::{Passthrough, PassthroughError};
use wormhole_vt::{Event, Terminal};

fn main() -> Result<(), PassthroughError> {
    // 1 KiB is the most payload the host accepts for this number; a longer string is dropped whole.
    // Numbers the engine uses itself (titles, colors, the clipboard, …) are refused here.
    let mut term = Terminal::builder().size(80, 24).passthrough(Passthrough::new().osc(51234, 1024))?.build();

    // The program asks the cursor position (the engine answers), then our question, then asks again.
    term.feed(b"\x1b[6n\x1b]51234;ping\x07\x1b[6n");

    let mut answers = Vec::new(); // (where in the replies, what to put there)
    for event in term.events() {
        if let Event::Osc { number: 51234, payload, terminator, replies_before } = event {
            // The payload is whatever the program printed: check it before it means anything.
            if payload == b"ping" {
                // End the answer the way the question ended, as terminals do.
                answers.push((replies_before, format!("\x1b]51234;pong{}", terminator.as_str())));
            }
        }
    }

    // What the program is owed, in the order it asked: position, pong, position.
    let mut input = term.take_replies();
    for (at, answer) in answers.into_iter().rev() {
        input.splice(at..at, answer.into_bytes());
    }
    println!("written to the program: {:?}", String::from_utf8_lossy(&input));
    Ok(())
}
