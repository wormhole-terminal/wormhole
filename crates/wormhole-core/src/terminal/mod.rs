//! What only core can say about starting a terminal's program. The terminal itself (a program in a
//! PTY and the screen it draws) is `wormhole_term`, its own crate so the session host can link it
//! without the rest of core; the app uses it directly.
//!
//! - `process`: which program a terminal runs and with which environment (it reads `shells`,
//!   `cli_tool` and `ipc`, which is why it is not in the terminal crate);
//! - `spawn`: [`SpawnOptions`] and [`spawn`], which turn a tab's options into the
//!   [`Launch`](wormhole_term::Launch) and [`StartOptions`](wormhole_term::StartOptions) that
//!   `Pty::start` takes.

pub mod cli_tool;
pub mod default_terminal;
mod process;
pub mod shells;
mod spawn;

pub use process::shell_name;
#[cfg(test)]
pub(crate) use process::welcome_command;
pub use spawn::{SpawnOptions, attach_hosted, spawn, spawn_hosted};
