//! What a program in a terminal may do with the clipboard (OSC 52).
//!
//! A program, also one over ssh or a `cat` of a hostile file, can ask the terminal to set the
//! clipboard and to read it back. Reading is the worse half: a password manager's last copy is in
//! it. So the two have a setting each ([`ClipboardRead`], [`ClipboardWrite`]), and a read that is
//! to be asked about goes through a [`ReadGate`] that decides, per terminal, whether to answer, to
//! refuse or to put the question to the person. The gate is plain state; the terminal view draws
//! the question and answers the engine, and `docs/threat-model.md` says why.

mod gate;
mod policy;

pub use gate::{ASK_TIMEOUT, Choice, ReadGate, Request};
pub use policy::{ClipboardRead, ClipboardWrite};
