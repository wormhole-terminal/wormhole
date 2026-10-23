//! The right panel's Usage tab: each coding agent's plan usage limits, as its status line last
//! reported them (`wormhole_core::agents::usage`), plus Gemini's latest model-response tokens.
//!
//! The reports are the account's, so the state is one entity for the whole app ([`state`]) that every
//! window watches; there is nothing per terminal or per window. `def` is the entry in the feature
//! table (the tab, shown while an agent is on screen, and its menu line), `view` the drawing.

mod def;
mod state;
#[cfg(test)]
pub(crate) mod tests;
mod view;

pub(crate) use def::DEF;
pub(crate) use state::{install, record, reports};
