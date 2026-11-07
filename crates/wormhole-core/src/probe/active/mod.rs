//! The Probes that are on, ready to run: what the terminal's rows and the hook delivery use
//! (`repo-docs/docs/probes.md` section 3 and `research/probes-hooks-and-fold.md` section 3).
//!
//! - `load`: the approved Probes read from their sealed copies, rules and hooks compiled, with what the
//!   user granted worked out once.
//! - `dispatch`: one event over every Probe's hooks, under budgets, as [`Effect`]s for the app to carry out.
//!
//! Only what the user approved runs, and only as far as they granted it: a rule's link needs the host in
//! `open` to be granted, a notification needs `notify`, and text the terminals hold reaches a hook only
//! with `terminal = ["read"]`. `load` reads files and `run` starts the one kind of process a Probe may; the
//! rest is data in and data out: no thread, no clock of its own. Must not use anything above `approval`.

mod dispatch;
mod load;
mod run;

pub use dispatch::{Dispatcher, Effect, Limits, RunTicket};
pub use load::{ActiveProbe, Grants, Loaded, load, rules_of};
pub use run::{RunEnd, RunJob, execute};
