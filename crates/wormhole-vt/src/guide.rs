//! The guide: how the engine thinks and how to embed it.
//!
//! The chapters are the files in the crate's `docs/` folder, included here so that their code is
//! compiled and run by `cargo test --doc`. Read them in this order:
//!
//! - [`concepts`]: the model (the terminal as a state machine, cells, modes, replies, events);
//! - [`embedding`]: the loop around a PTY, threads, resizing, history, limits and policy;
//! - [`rendering`]: damage, rows and cells, the cursor, selection, links, pictures and colours;
//! - [`input`](mod@crate::guide::input): keys, mouse, paste and focus, as bytes for the program;
//! - [`passthrough`]: your own OSC, DCS and APC strings, handed over whole;
//! - [`security`]: the threat model, what the engine guarantees and what a host must still do;
//! - [`conformance`]: how the engine is tested and how to check it yourself;
//! - [`tools`]: debugging a stream, benchmarks, profiling and fuzzing.

#[doc = include_str!("../docs/concepts.md")]
pub mod concepts {}

#[doc = include_str!("../docs/embedding.md")]
pub mod embedding {}

#[doc = include_str!("../docs/rendering.md")]
pub mod rendering {}

#[doc = include_str!("../docs/input.md")]
pub mod input {}

#[doc = include_str!("../docs/passthrough.md")]
pub mod passthrough {}

#[doc = include_str!("../docs/security.md")]
pub mod security {}

#[doc = include_str!("../docs/conformance.md")]
pub mod conformance {}

#[doc = include_str!("../docs/tools.md")]
pub mod tools {}
