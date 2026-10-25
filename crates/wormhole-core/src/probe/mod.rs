//! Probes: extensions that are a folder with a `probe.toml` (`repo-docs/docs/probes.md`).
//!
//! This is the files-only level (E1): reading a Probe's folder under caps, parsing its manifest,
//! fingerprinting it, validating it, finding the Probes in the data folder, approving one (the copy it
//! will run from), and the command line that makes, checks and tests one. Nothing here runs a Probe;
//! "on" is a stored approval that the later slices read. The feature flag
//! [`FeatureFlag::Probes`](crate::config::features::FeatureFlag::Probes) is what they gate on.
//!
//! - `cli`: `Wormhole probe new | validate | test`, as text to print and an exit code.
//! - `manifest`: `probe.toml` as types; unknown keys are warnings at parse.
//! - `read`: a folder, once, through `storage::files` with every cap and link rule.
//! - `fingerprint`: the hash an approval is bound to.
//! - `validate`: findings and the summary of what a Probe would be allowed to do.
//! - `fixture`: the `tests/*.toml` files of `probe test` and the run of their cases.
//! - `scaffold`: the files of a new Probe, for `probe new`.
//! - `rules`: a Probe's text rules compiled and run over a line, as `probe test` and the terminal do.
//! - `scan`: the Probes in `<data dir>/probes/`.
//! - `approval`: what the user approved, the state that gives each Probe, and the copy it runs from.
//!
//! A Probe's folder is hostile input, so everything is read through `storage::files`. This domain
//! uses `storage` and `keys` (a command's shortcut) and the agent registry in `wormhole-term`; it is
//! in the tier of `git` and `notify` so that `ipc` can offer it on the command line.

pub mod approval;
pub mod cli;
pub mod fingerprint;
pub mod fixture;
pub mod hidden;
pub mod manifest;
pub mod read;
pub mod rules;
pub mod scaffold;
pub mod scan;
pub mod validate;

pub use fingerprint::{FINGERPRINT_VERSION, fingerprint};
pub use manifest::{Manifest, ManifestError, ParseWarning, Parsed};
pub use read::{MANIFEST_FILE, ProbeFile, ProbeFolder, ReadError};
pub use scan::{ScanError, Scanned, TooManyEntries, scan, scan_data};
pub use validate::{Finding, Report, Severity, Summary, validate};
