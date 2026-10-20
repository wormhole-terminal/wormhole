//! Wormhole's terminal engine: bytes from a program in, screen state and events out.
//!
//! No PTY, no threads, no UI and no I/O: `wormhole-core` owns the process and the reader thread,
//! the app only draws. Every escape sequence a program sends is read here, so nothing parses the
//! output a second time.
//!
//! - `parser`: the DEC/ANSI state machine; payloads of OSC, DCS and APC are streamed;
//! - `unicode`: cell widths and grapheme cluster breaks from generated tables;
//! - `types`: the value types the app draws and talks with (cells, colors, positions, modes);
//! - `grid`: the screen and its history: pages of cells with interned styles, scrollback, reflow;
//! - `term`: the terminal state: what the sequences mean (cursor, modes, SGR, OSC, replies) and the
//!   events the host hears;

#![forbid(unsafe_code)]

pub mod grid;
pub mod parser;
pub mod terminal;
pub mod types;
pub mod unicode;
