//! A running program's code signature, read by [`wormhole_term::sys::codesign`], for the question of
//! who is asking to control Wormhole (`ipc::access`).

pub use wormhole_term::sys::codesign::{Signature, SignerKind, of_pid};
