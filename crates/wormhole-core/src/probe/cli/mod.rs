//! `Wormhole probe new | validate | test`: the Probe tools of the command line
//! (`repo-docs/docs/probes.md`, section 5; `repo-docs/docs/cli.md`).
//!
//! Like `sessions`, none of this goes through the app: it reads and writes a folder, so it works with the
//! app closed and in CI. The functions here return what to print and the exit code and print nothing,
//! so the tests can read both; `ipc::control::probe` prints, and decides whether the feature is on.
//!
//! Everything printed that a Probe or a folder chose (its name, a message that quotes its text, a path
//! it names) goes through `hidden::reveal`: a Probe is hostile input, and a name with an escape sequence
//! or a right-to-left override must not drive the terminal that shows it. The `--json` output keeps the
//! text as it is and writes the same characters as `\uXXXX`, which a parser reads back unchanged.
//!
//! Exit codes, as in `cli.md`: 0 fine, 1 problems (the Probe has errors, a test failed, a folder is
//! not there or is not a Probe), 2 a mistake in the arguments.
//!
//! - `args`: the words after `probe`.
//! - `output`: cleaning and the shape of the output.
//! - `validate_cmd`, `test_cmd`, `new_cmd`: one verb each.
//!
//! Must not use anything above `rules`, `fixture`, `scaffold` and `validate`; it knows no settings.

mod args;
mod new_cmd;
mod output;
mod test_cmd;
mod validate_cmd;

use std::path::{Path, PathBuf};

use args::Command;

/// Where the command runs.
#[derive(Debug, Clone, Copy)]
pub struct Env<'a> {
    /// Relative folders are read against it.
    pub cwd: &'a Path,
    /// `<data dir>/probes`, where installed Probes live: `new` does not scaffold in it.
    pub data_probes: Option<&'a Path>,
    /// What the command is called when someone types it (`wh`), for the usage text.
    pub command: &'a str,
}

/// What a run printed and its exit code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Outcome {
    pub(super) fn ok(stdout: String) -> Self {
        Self { code: 0, stdout, stderr: String::new() }
    }

    pub(super) fn new(code: i32, stdout: String, stderr: String) -> Self {
        Self { code, stdout, stderr }
    }

    /// An error on stderr, as `Wormhole: …`.
    pub(super) fn fail(code: i32, message: &str) -> Self {
        Self { code, stdout: String::new(), stderr: format!("Wormhole: {}\n", output::clean(message)) }
    }
}

/// The usage text of the `probe` verb.
pub fn help(wh: &str) -> String {
    format!(
        "\
  {wh} probe validate [<folder>] [--json]
      Check a Probe's folder (default: here): the manifest, the limits, the patterns. Lists what it
      asks for in plain words: each permission with Click or Touch ID, the dangerous combinations,
      and what each program can do. Exit 0 without errors, 1 with. Works while the app is closed.
  {wh} probe new <id> [--kind rules|command|status|agent] [--dir <parent>]
      A new Probe in <parent>/<id>/ (default: here), a working example of the kind with tests/
      and PROBE.md for whoever works on it. Never replaces a folder.
  {wh} probe test [<folder>] [--json]
      Run the Probe's rules over tests/*.toml: for each case, the spans, the look and the action's
      target. Exit 0 when all pass, 1 when one fails.
  Probes are behind the `probes` feature flag, off in releases.
"
    )
}

/// `probe <args>` (the words after `probe`) in `env`.
pub fn run(args: &[String], env: &Env<'_>) -> Outcome {
    match args::parse(args) {
        Ok(Command::Help) => Outcome::ok(help(env.command)),
        Ok(Command::Validate { dir, json }) => validate_cmd::run(&resolve(env, dir.as_deref()), json),
        Ok(Command::Test { dir, json }) => test_cmd::run(&resolve(env, dir.as_deref()), json),
        Ok(Command::New { id, kind, dir }) => new_cmd::run(&id, kind, &resolve(env, dir.as_deref()), env),
        Err(mistake) => Outcome::new(
            2,
            String::new(),
            format!("Wormhole: {}\n\n{}", output::clean(&mistake), help(env.command)),
        ),
    }
}

/// `dir` against the working folder; the working folder itself without one.
fn resolve(env: &Env<'_>, dir: Option<&str>) -> PathBuf {
    match dir {
        Some(dir) => env.cwd.join(dir),
        None => env.cwd.to_path_buf(),
    }
}

#[cfg(test)]
mod tests;
