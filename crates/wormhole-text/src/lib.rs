//! The words of Wormhole's interface.
//!
//! English is the source: Fluent catalogs under `locales/<language>/<area>.ftl`. A test generates
//! `src/keys/` (and the id list and the file list in `src/generated/`) from the English files
//! (`TEXT_BLESS=1 cargo test -p wormhole-text --test generated`), so a message is a constant the
//! compiler knows ([`text`] takes a [`Key`]), or, when it has variables, a typed function that
//! returns the formatted `String`. Nothing is looked up by a string
//! at run time and nothing is formatted per frame: when a language is set ([`set_language`]) every
//! message without variables is formatted once into that language's table, and [`text`] is an array
//! index.
//!
//! The crate is a leaf: no GPUI, no app state, no I/O, so core and the app (and later the terminal
//! layer) can all use it, and the session host never links it. See `repo-docs/docs/languages.md`.
#![forbid(unsafe_code)]
#![cfg_attr(test, allow(clippy::disallowed_methods, reason = "tests build their own files and processes"))]

mod active;
mod args;
mod catalog;
mod generated;
mod key;
// Written by the generator, which keeps it in the shape it has.
#[rustfmt::skip]
pub mod keys;
mod language;
mod pseudo;

#[cfg(feature = "test-support")]
pub use active::with_language;
pub use active::{is_loaded, language, set_language, text, text_in};
pub use key::Key;
pub use language::{Language, negotiate};

/// A number as a message's whole-number argument (what a plural selects on), however it is stored.
/// Larger than an `i64` is not a thing a person counts; it saturates.
pub fn whole<N: TryInto<i64>>(number: N) -> i64 {
    number.try_into().unwrap_or(i64::MAX)
}
