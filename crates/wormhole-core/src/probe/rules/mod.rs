//! A Probe's text rules, run: the compiled form of `[[contributes.rules]]` and the pure function that
//! finds what they match in one logical line (`repo-docs/docs/probes.md`, section 3.1).
//!
//! This is the matching `probe test` runs over its fixtures and the one the terminal's row builder
//! will call, so a rule behaves in a test as it does on screen. It draws nothing and touches no file:
//! a line of text goes in, spans with their look come out, and an action's target is worked out
//! apart from that ([`target`]), because only a ⌘-click needs it.
//!
//! - `compile`: the rules of a manifest, with the limits `validate` checks them with.
//! - `matching`: [`matches`], with the caps and the overlap rules.
//! - `target`: what an action would open, copy or run for a match.
//!
//! Joining the screen rows of a soft-wrapped line is the caller's: [`matches`] takes the logical line.
//!
//! Must not use anything above `validate`; it reads no file and knows no settings.

mod compile;
mod matching;
mod target;

pub use compile::{CompileError, CompiledRule, CompiledRules};
pub use matching::{MAX_MATCHES_PER_RULE, MAX_SCAN_BYTES, Match, matches};
pub use target::{Target, target};

#[cfg(test)]
mod tests;
