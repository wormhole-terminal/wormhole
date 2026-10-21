//! Properties that must hold for every input, checked on generated streams. Each case has a seed
//! and prints it with the stream that broke it, so a failure reproduces: call the helper (for
//! instance `split_at_every_byte(seed, 1, len, mix)`) with that seed, or copy the printed stream
//! into a `.vtt` case.
//!
//! - **Split invariance**: a stream fed whole, cut at any one byte, or cut into random pieces leaves
//!   the terminal in the same state (`dump_state`, which holds every cell, style, link and mode),
//!   with the same replies and events. A host reads the PTY in whatever pieces the kernel gives it.
//! - **Reflow loses nothing**: resizing there and back, through any sizes, keeps every character of
//!   the history and the screen, soft-wrapped lines rewrapped and never split or merged wrongly.
//! - **Replies are well-formed and carry no program text**: everything the terminal writes back is a
//!   sequence the parser reads as one (no stray controls, no C1), and what a request carried
//!   (a DECRQSS string, a colour query's extra parameters, a title when reports are off) is not in it.
//!
//! The memory bounds are in `invariants_memory.rs`, which needs an allocator of its own.
//!
//! The default run is a few seconds; `cargo test -p wormhole-vt --test invariants -- --ignored`
//! runs ten times the seeds and longer streams.

#[path = "../common/rng.rs"]
mod rng;

mod generator;
mod reflow;
mod replies;
mod split;
