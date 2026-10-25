//! Probes: extensions that are a folder with a `probe.toml` (`repo-docs/docs/probes.md`).
//!
//! This is the first slice of the files-only level (E1): reading a Probe's folder under caps,
//! parsing its manifest, fingerprinting it, validating it, and finding the Probes in the data
//! folder. Nothing here enables, approves or runs a Probe, and nothing in the app calls it yet;
//! the feature flag [`FeatureFlag::Probes`](crate::config::features::FeatureFlag::Probes) is what
//! the later slices gate on.
//!
//! - `manifest`: `probe.toml` as types; unknown keys are warnings at parse.
//! - `read`: a folder, once, through `storage::files` with every cap and link rule.
//! - `fingerprint`: the hash an approval is bound to.
//! - `validate`: findings and the summary of what a Probe would be allowed to do.
//! - `scan`: the Probes in `<data dir>/probes/`.
//!
//! A Probe's folder is hostile input, so everything is read through `storage::files`. This domain
//! uses `storage` and `keys` (a command's shortcut) and the agent registry in `wormhole-term`; it is
//! in the tier of `git` and `notify` so that `ipc` can offer it on the command line.

pub mod fingerprint;
mod hidden;
pub mod manifest;
pub mod read;
pub mod scan;
pub mod validate;

pub use fingerprint::{FINGERPRINT_VERSION, fingerprint};
pub use manifest::{Manifest, ManifestError, ParseWarning, Parsed};
pub use read::{MANIFEST_FILE, ProbeFile, ProbeFolder, ReadError};
pub use scan::{ScanError, Scanned, scan, scan_data};
pub use validate::{Finding, Report, Severity, Summary, validate};
