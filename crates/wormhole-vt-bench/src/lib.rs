//! Tools for testing and measuring Wormhole's terminal engine.
//!
//! Everything here talks to a terminal engine through one small trait, [`Engine`], so the same
//! conformance cases, byte streams and comparisons run against `wormhole-vt` and, from
//! `tools/vt-compare`, against other terminals' engines: adding an engine is one file in
//! `engines/` and one line in [`engines::for_each_engine`].
//!
//! - [`bench`]: throughput and allocation measurement, what `vtbench` runs;
//! - [`bisim`]: the repaint's bisimulation: an engine and a replica made from its repaint must stay
//!   equal, at every cut of a stream;
//! - [`corpus`]: deterministic generators of realistic byte streams;
//! - [`diff`]: the differential harness (two engines, one stream) and a random stream generator;
//! - [`vtt`]: the `.vtt` conformance format (`crates/wormhole-vt/tests/conformance/README.md`)
//!   and its runner;
//! - [`window_host`]: the app's side of the engine on a fake window (events, DECCOLM, window
//!   operations), shared by the headless suite hosts `esctest-host` and `vttest-host`.
//!
//! This crate depends on the engine, never the other way round: `wormhole-vt` must not know about
//! its tools.

pub mod bench;
pub mod bisim;
pub mod corpus;
pub mod diff;
pub mod engine;
pub mod engines;
pub mod rng;
pub mod vtt;
pub mod window_host;

pub use engine::{Attrs, Cell, CellColor, Cursor, Engine};
