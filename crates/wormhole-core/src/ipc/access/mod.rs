//! Who may control Wormhole through the automation API (`agent-permissions.md`): the connection approval.
//!
//! A request that needs more than titles and states is judged on *who* sent it: [`resolve`] walks its
//! process chain to the agent behind it and says how that program is told from another ([`Identity`]),
//! [`Grants`] and the store hold what the user allowed, and [`decide`] turns the mode, the request's
//! [`Need`] and what the program holds into allow, ask or refuse. Questions wait in a [`Queue`]. Nothing
//! here shows a question or touches a process: the app implements [`Processes`] and puts the question on
//! the screen. Nothing a caller says counts, and no request can change what is held here.

mod caps;
mod decide;
mod grants;
mod identity;
mod queue;
mod resolve;

pub use caps::{CapSet, Capability, Need};
pub use decide::{Access, Allowance, Verdict, decide};
pub use grants::{COOL_DOWN, Grants, Held, ProcessRef, Scope};
pub use identity::{Identity, Kind, agent_word};
pub use queue::{Full, Pending, QUEUE_MAX, Queue};
pub use resolve::{Changed, Processes, Program, Resolved, ScriptPin, resolve};
