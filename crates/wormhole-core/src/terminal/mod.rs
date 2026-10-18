//! The terminal layer as the app sees it: `wormhole_term` (a program in a PTY and the screen it
//! draws; its own crate, so the session host can link it without the rest of core) re-exported,
//! plus what only core can say about starting a tab's shell:
//!
//! - `process`: which program a terminal runs and with which environment (it reads `shells`,
//!   `welcome`, `cli_tool` and `ipc`, which is why it is not in the terminal crate);
//! - `spawn`: [`SpawnOptions`] and [`spawn`], which turn a tab's options into the
//!   [`Launch`] and [`StartOptions`] that `Pty::start` takes.

mod process;
mod spawn;

pub use process::shell_name;
pub use spawn::{SpawnOptions, attach_hosted, spawn, spawn_hosted};
pub use wormhole_term::*;
