//! Wormhole's engine tools (`crates/wormhole-vt-bench`) pointed at other terminals' engines.
//!
//! `vt-matrix` runs the `.vtt` conformance cases against every engine and shows who agrees with
//! whom; `vt-bench` runs the vtbench corpora against every engine. The engines are used as black
//! boxes through their public APIs only: libghostty-vt's C headers, `wezterm-term`'s and `vt100`'s
//! documented Rust APIs, and alacritty_terminal's.
//!
//! The tests against alacritty (random streams, selection and copying) and the parser benchmark
//! against `vte` (`examples/parse_bench.rs`) live here too: alacritty is not in the app's
//! dependencies any more.

pub mod caps;
pub mod engines;
pub mod matrix;
pub mod render;
