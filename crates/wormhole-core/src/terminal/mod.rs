//! What only core can say about starting a terminal's program. The terminal itself (a program in a
//! PTY and the screen it draws) is `wormhole_term`, its own crate so the session host can link it
//! without the rest of core; the app uses it directly.
//!
//! - `process`: [`SpawnOptions`], and which program a terminal runs with which environment (it
//!   reads `shells`, `cli_tool` and `ipc`, which is why it is not in the terminal crate);
//! - `spawn`: [`spawn`] and the hosted variants, which turn a tab's options into the
//!   [`Launch`](wormhole_term::Launch) and [`StartOptions`](wormhole_term::StartOptions) that
//!   `Pty::start` takes;
//! - `shells`: shell profiles, what a new terminal runs;
//! - `cli_tool`: the `wh` command line tool and the PATH every terminal starts with;
//! - `default_terminal`: Wormhole as the Mac's default terminal, and what it does with a file
//!   macOS hands it.
//!
//! Tier 6: it uses `storage`, `config` and `ipc` (the socket a terminal's hooks call). Only
//! `onboarding` is above it.

pub mod cli_tool;
pub mod default_terminal;
mod process;
pub mod shells;
mod spawn;

#[cfg(test)]
pub(crate) use process::welcome_command;
pub use process::{SpawnOptions, shell_name};
pub use spawn::{attach_hosted, spawn, spawn_hosted};
