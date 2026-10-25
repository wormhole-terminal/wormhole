//! The `probe` command line, end to end on real folders in temp dirs.

mod args;
mod new;
mod test_verb;
mod validate;

use std::fs;
use std::path::{Path, PathBuf};

use super::{Env, Outcome, run};

/// `probe <args>` run in `cwd`, with no data folder.
pub(super) fn go(cwd: &Path, args: &[&str]) -> Outcome {
    let words: Vec<String> = args.iter().map(ToString::to_string).collect();
    run(&words, &Env { cwd, data_probes: None, command: "wh" })
}

/// A fixture folder of this crate: `tests/probes/<group>/<name>`.
pub(super) fn fixture_dir(group: &str, name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/probes").join(group).join(name)
}

pub(super) const HEAD: &str = "[probe]\nid = \"tiny\"\nname = \"Tiny\"\ndescription = \"d\"\nauthor = \"a\"\n\
                               version = \"0.1.0\"\nwormhole = \">=1.3\"\napi = 1\n";

/// A Probe of `HEAD` and `rest` in a new folder of `parent`, with `files` beside it.
pub(super) fn probe_in(parent: &Path, folder: &str, rest: &str, files: &[(&str, &str)]) -> PathBuf {
    let root = parent.join(folder);
    fs::create_dir_all(&root).expect("mkdir");
    fs::write(root.join("probe.toml"), format!("{HEAD}{rest}")).expect("write");
    for (path, body) in files {
        let at = root.join(path);
        fs::create_dir_all(at.parent().expect("parent")).expect("mkdir");
        fs::write(at, body).expect("write");
    }
    root
}

/// The JSON a run printed, as a value.
pub(super) fn json(outcome: &Outcome) -> serde_json::Value {
    assert_eq!(outcome.stdout.lines().count(), 1, "one line of JSON: {}", outcome.stdout);
    serde_json::from_str(&outcome.stdout).unwrap_or_else(|err| panic!("{err}: {}", outcome.stdout))
}
