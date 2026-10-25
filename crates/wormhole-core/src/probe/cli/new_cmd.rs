//! `probe new`: a folder with a working example of a kind of Probe.

use std::fs;
use std::path::Path;

use super::text::{self, Out, count};
use super::{Env, Outcome};
use crate::probe::scaffold::{Kind, ScaffoldError, create};

pub(super) fn run(id: &str, kind: Kind, parent: &Path, env: &Env<'_>) -> Outcome {
    if in_probes_folder(parent, env.data_probes) {
        return Outcome::fail(
            2,
            "that is the folder installed Probes live in; make the Probe somewhere else, \
             and install it from there once it is done",
        );
    }
    match create(parent, id, kind) {
        Ok(made) => {
            let mut out = Out::default();
            out.line(&format!(
                "created {} ({} Probe, {})",
                text::path(&made.root),
                kind.word(),
                count(made.files.len(), "file", "files")
            ));
            for file in &made.files {
                out.line(&format!("  {file}"));
            }
            out.blank();
            out.line(&format!(
                "next: cd {id} && {wh} probe validate && {wh} probe test    (PROBE.md says what to do with it)",
                wh = env.command
            ));
            Outcome::ok(out.finish())
        }
        // The id and the folder are what was typed.
        Err(err @ (ScaffoldError::BadId(_) | ScaffoldError::NoParent(_))) => {
            Outcome::fail(2, &err.to_string())
        }
        Err(err) => Outcome::fail(1, &err.to_string()),
    }
}

/// Whether `parent` is the folder of installed Probes or inside it. A Probe made there would skip
/// the copy that is approved; the real paths are compared, so a link or `..` does not get around it.
fn in_probes_folder(parent: &Path, probes: Option<&Path>) -> bool {
    let Some(probes) = probes else { return false };
    match (fs::canonicalize(parent), fs::canonicalize(probes)) {
        (Ok(parent), Ok(probes)) => parent.starts_with(probes),
        _ => false,
    }
}
