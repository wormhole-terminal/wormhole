//! The Probes that are on, as a shell runs them: the rules the rows are decorated with, the spans that come of
//! them per row, and what the hooks have set. No window, no draw call: the GPUI app and the Swift app's
//! library both read these and only draw. What listens to the event bus for the hooks is `ipc::probe_feed`.
//!
//! - `ruleset`: the compiled rules of every Probe that is on, asked about one logical line.
//! - `decor`: the spans a screen's rows get from them, kept between frames.
//! - `state`: the status items and tinted blocks the hooks set.
//!
//! Must not use anything above `active`.

pub mod decor;
pub mod ruleset;
pub mod state;

pub use decor::{Decor, Inputs, RuleSpan};
pub use ruleset::{Decoration, RuleSet};
pub use state::{Effects, StatusItem};
