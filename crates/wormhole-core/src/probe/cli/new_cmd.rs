//! `probe new`: a folder with a working example of a kind of Probe.

use std::fs;
use std::path::Path;

use super::output::{self, Out, count};
use super::{Env, Outcome};
use crate::probe::scaffold::{Kind, ScaffoldError, create};

pub(super) fn run(id: &str, kind: Kind, parent: &Path, env: &Env<'_>) -> Outcome {
    if in_probes_folder(parent, id, env.data_probes) {
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
                output::path(&made.root),
                kind.word(),
                count(made.files.len(), "file", "files")
            ));
            for file in &made.files {
                out.line(&format!("  {file}"));
            }
            out.blank();
            out.line(&format!(
                "next: cd {} && {wh} probe validate && {wh} probe test    (PROBE.md says what to do with it)",
                shell_word(&way_to(env.cwd, &made.root)),
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

/// Where `target` is from `cwd`: relative when it is inside it, the whole path otherwise.
fn way_to(cwd: &Path, target: &Path) -> String {
    target.strip_prefix(cwd).unwrap_or(target).to_string_lossy().into_owned()
}

/// `word` as one word for a shell: as it is when that is safe, in single quotes when not. (The
/// output is cleaned after, so a control character in it is written out, not passed on.)
fn shell_word(word: &str) -> String {
    let plain = !word.is_empty()
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '_' | '-' | '+' | '@' | ':'));
    if plain { word.to_string() } else { format!("'{}'", word.replace('\'', "'\\''")) }
}

/// Whether a Probe `id` made in `parent` would be, or be inside, the folder of installed Probes. One
/// made there would skip the copy that is approved. The real paths are compared, so a link or `..` does
/// not get around it, and the folder need not be there yet: it is then where `probes` would be in its
/// real parent.
fn in_probes_folder(parent: &Path, id: &str, probes: Option<&Path>) -> bool {
    let Some(probes) = probes else { return false };
    let Ok(parent) = fs::canonicalize(parent) else { return false };
    let probes = fs::canonicalize(probes).ok().or_else(|| {
        let data = fs::canonicalize(probes.parent()?).ok()?;
        Some(data.join(probes.file_name()?))
    });
    probes.is_some_and(|probes| parent.starts_with(&probes) || parent.join(id) == probes)
}
