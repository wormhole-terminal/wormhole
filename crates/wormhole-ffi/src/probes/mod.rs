//! Probes in the Swift app: the Probes that are on run here, in Rust, and Swift draws what they say. The same parts
//! the GPUI app runs (`wormhole_core::probe::{active, runtime}`, `ipc::probe_feed`, `wormhole_term`'s rules and
//! folds), hosted by the hook server's core:
//!
//! - `state`: the rules, hook effects and "Never hide lines", process-wide;
//! - `run`: loading, the hook feed, the effects (as `HookEvent`s);
//! - `marks`: a terminal's decorations and fold markers, as the frame's two additive sections;
//! - `terminal`: folds opened, a shortened span revealed, a rule's file found, the guards on folds;
//! - `server`: the status items a hook set, read by the status bar.

mod marks;
mod run;
mod server;
mod state;
mod terminal;

pub(crate) use marks::{Marks, words_of};
pub(crate) use run::{Probes, approvals_changed, reload};
pub use server::ProbeStatusRecord;
pub(crate) use state::rules;
pub use terminal::ResolvedFile;
pub(crate) use terminal::readout;

#[cfg(test)]
pub(crate) use state::test_lock;
