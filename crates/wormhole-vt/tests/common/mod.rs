//! Helpers the integration tests and the examples share, each file including what it needs with
//! `#[path]`: a seeded generator ([`rng`]) and a counting allocator ([`alloc`]). The unit tests
//! inside `src/` include `rng.rs` the same way; this folder is not a test target itself.
#![allow(dead_code)] // every includer uses a part

pub mod alloc;
pub mod rng;
pub mod streams;
