//! Terminals that outlive the app (`repo-docs/docs/session-host.md`), as the GPUI app does with
//! `wormhole-host`: a terminal can start in a session host of its own, let go of it when the app quits
//! (the program keeps running there), and attach to it again at the next launch.
//!
//! What is core's stays core's: the host program's place (`ipc::host_binary`), the scan of the hosts
//! folder and the launch plan (`ipc::hosts`), the attach and what a failed one means
//! (`ipc::hosts::AttachFailure`), the PTY's hosted constructors (`terminal::{spawn_hosted,
//! attach_hosted}`) and the store's `tabs.host_id`. This folder only joins them behind UniFFI:
//!
//! - `desk`: the `HostedTerminals` object (start, attach, the launch plan, quit);
//! - `registry`: which terminal runs in which host, in the profile's database;
//! - `checkpoints`: terminals that come back after the Mac restarts (the sealed screen the host keeps);
//! - `end`: ending a host from its terminal's bar;
//! - `texts`: the screens saved each minute, which a terminal whose host is gone is drawn from;
//! - `slot`: what a `Terminal` knows of its host, and its `detach`, `hosting`, `set_visible`;
//! - `program`, `records`, `error`: the plain values that cross.

mod checkpoints;
mod desk;
mod end;
mod error;
mod program;
mod records;
mod registry;
mod shown;
mod slot;
mod texts;

#[cfg(test)]
mod tests;

pub use desk::{HostedStart, HostedTerminals};
pub use error::{HostUnavailableKind, HostingError};
pub use program::HostingProgram;
pub use records::{LaunchPlan, OrphanHost, SavedTerminal, TerminalHosting};
pub(crate) use slot::HostSlot;
