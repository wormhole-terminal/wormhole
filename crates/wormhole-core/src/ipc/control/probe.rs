//! `Wormhole probe new | validate | test`: the Probe tools (`probe::cli`), run from the command line.
//!
//! Like `sessions`, nothing here goes through the app, so it works with the app closed and in CI. The
//! arguments, the checks and the output are the probe domain's; this prints them, and decides whether the
//! feature is on at all.
//!
//! **The gate.** Probes are behind the `probes` feature flag, off in every release, and these tools are
//! the one part of them a person can reach before the app does anything with a Probe. They write and read
//! only the folder they are given, so there is little to protect, but a release that offers
//! `Wormhole probe` advertises a feature it does not have. So the flag is read from the Default profile's
//! `config.json` without the app: the file is only read (never created or repaired, as
//! `Config::load` would), and a file that does not read counts as no flags set, which is what the app's
//! defaults say (a development build: on; a release: off, whatever the file says unless the update channel is
//! Unstable). Another profile's flags are not looked at; Probes are shared by every profile.

use std::path::Path;

use super::COMMAND_NAME;
use super::cli::{print, print_err};
use crate::config::Settings;
use crate::config::features::FeatureFlag;
use crate::probe::cli::{self, Env, Outcome};
use crate::storage::files::{self, LinkPolicy};
use crate::storage::jsonc;
use crate::storage::paths::DataRoot;

/// The usage text of the `probe` verb.
pub fn help(wh: &str) -> String {
    cli::help(wh)
}

/// `Wormhole probe …`: the exit code (0 fine, 1 problems, 2 a mistake in the arguments).
pub fn run(args: &[String]) -> i32 {
    let root = DataRoot::current();
    let enabled = settings_at(&root.default_profile().config_file()).feature_enabled(FeatureFlag::Probes);
    let cwd = std::env::current_dir().unwrap_or_default();
    let probes = root.probes_dir();
    let env = Env { cwd: &cwd, data_probes: Some(&probes), command: COMMAND_NAME };
    let outcome = run_in(args, &env, enabled);
    print(&outcome.stdout);
    print_err(&outcome.stderr);
    outcome.code
}

/// `probe <args>` in `env`; refused when the feature is not `enabled`.
pub fn run_in(args: &[String], env: &Env<'_>, enabled: bool) -> Outcome {
    if !enabled {
        return Outcome {
            code: 1,
            stdout: String::new(),
            stderr: "Wormhole: Probes are not available in this build yet: they are behind the `probes` feature flag, which is off in releases\n".to_string(),
        };
    }
    cli::run(args, env)
}

/// The settings `path` holds, without creating, repairing or saving anything; the defaults when it is
/// missing or does not read.
fn settings_at(path: &Path) -> Settings {
    files::read_to_string_capped(path, 1024 * 1024, LinkPolicy::Follow)
        .ok()
        .and_then(|text| jsonc::parse::<Settings>(&text).ok())
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn words(args: &[&str]) -> Vec<String> {
        args.iter().map(ToString::to_string).collect()
    }

    #[test]
    fn a_missing_or_unreadable_config_means_no_flags_set() {
        let tmp = TempDir::new().expect("tempdir");
        let missing = tmp.path().join("config.json");
        assert!(settings_at(&missing).feature_flags.is_empty());
        assert!(!missing.exists(), "reading never creates the file");
        fs::write(&missing, "{ not json").expect("write");
        assert!(settings_at(&missing).feature_flags.is_empty());
        assert_eq!(fs::read_to_string(&missing).expect("reads"), "{ not json", "nor repairs it");
    }

    #[test]
    fn the_flag_is_read_from_the_file_with_comments_allowed() {
        let tmp = TempDir::new().expect("tempdir");
        let path = tmp.path().join("config.json");
        fs::write(&path, "{\n // off\n \"feature_flags\": { \"probes\": false }\n}").expect("write");
        assert!(!settings_at(&path).feature_enabled(FeatureFlag::Probes));
        fs::write(&path, "{ \"feature_flags\": { \"probes\": true } }").expect("write");
        assert!(settings_at(&path).feature_enabled(FeatureFlag::Probes));
        fs::write(&path, "{}").expect("write");
        assert!(
            settings_at(&path).feature_enabled(FeatureFlag::Probes),
            "a development build: on unless switched off"
        );
    }

    #[test]
    fn with_the_flag_off_nothing_runs_and_nothing_is_written() {
        let tmp = TempDir::new().expect("tempdir");
        let env = Env { cwd: tmp.path(), data_probes: None, command: "wh" };
        for args in [&["new", "my-probe"][..], &["validate"], &["test"], &["--help"], &[]] {
            let out = run_in(&words(args), &env, false);
            assert_eq!(out.code, 1, "{args:?}");
            assert!(out.stdout.is_empty());
            assert!(out.stderr.contains("`probes` feature flag"), "{}", out.stderr);
        }
        assert_eq!(fs::read_dir(tmp.path()).expect("lists").count(), 0);
        let on = run_in(&words(&["new", "my-probe"]), &env, true);
        assert_eq!(on.code, 0, "{on:?}");
    }

    #[test]
    fn the_verb_is_the_command_lines_and_in_the_usage() {
        assert!(super::super::is_verb("probe"));
        let usage = super::super::help();
        for line in [
            "wh probe validate [<folder>] [--json]",
            "wh probe new <id>",
            "wh probe test [<folder>] [--json]",
        ] {
            assert!(usage.contains(line), "{line}");
        }
    }
}
