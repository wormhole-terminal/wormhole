//! The seam between the Swift UI experiment and the Rust terminal: a [`Terminal`] on the real
//! engine (`wormhole-term`, started through `wormhole-core`), its screen as packed bytes, its
//! events behind one coalesced wake.
//!
//! This crate is the only `unsafe` boundary the experiment has in Rust, and it contains none that
//! is written by hand: the allowance below is for the scaffolding UniFFI's proc macros generate
//! (`extern "C"` entry points taking raw pointers and buffers). Nothing here names the UI.

// UniFFI's generated scaffolding is `unsafe extern "C"` code; no hand-written `unsafe` exists in
// this crate (`apple/scripts/rules.py` checks that).
#![allow(unsafe_code, reason = "UniFFI's generated scaffolding")]

mod error;
mod events;
mod frame;
mod isolation;
mod terminal;

pub use error::FfiError;
pub use events::{TermEvent, TerminalListener};
pub use terminal::{FrameTiming, KeyPress, Terminal};

uniffi::setup_scaffolding!();

/// The packed frame's layout version, for the Swift reader to check against its own.
#[uniffi::export]
pub fn frame_format_version() -> u32 {
    frame::VERSION
}

/// A call that does nothing, for measuring what crossing the seam costs by itself.
#[uniffi::export]
pub fn ping(value: u64) -> u64 {
    value
}
