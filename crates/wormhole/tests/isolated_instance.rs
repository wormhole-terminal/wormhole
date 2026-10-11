//! `scripts/lib.sh`'s `start_isolated`, which every test instance goes through
//! (`scripts/devctl-instance.sh`, the visual check): what it starts gets the instance's own
//! settings and nothing of the Mac's. It runs the function on a stand-in for the app that prints
//! its environment, so it needs only `bash` and `env`, not macOS or a display.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The repository, two levels above this crate.
fn root() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(2).expect("the repository root")
}

/// A folder of this test's own; its name says so, so a failed run leaves nothing nameless behind.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("wormhole-isolated-{}-{name}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("creates the scratch dir");
    dir
}

/// Start the stand-in through `start_isolated` with these extra `VAR=value` pairs, from a parent
/// that carries a tab's variables and a PATH of its own, and return the environment it printed.
fn environment_of(scratch: &Path, extra: &[&str]) -> Vec<(String, String)> {
    use std::os::unix::fs::PermissionsExt as _;
    let app = scratch.join("fake-app");
    std::fs::write(&app, "#!/bin/sh\nenv\n").expect("writes the stand-in");
    std::fs::set_permissions(&app, std::fs::Permissions::from_mode(0o755)).expect("makes it executable");
    let run = scratch.join("run");
    let script = r#"set -euo pipefail; . "$0/scripts/lib.sh"; start_isolated "$1" "$2" "$3" /bin/cat "${@:4}"; wait "$pid""#;
    let output = Command::new("bash")
        .arg("-c")
        .arg(script)
        .arg(root())
        .arg(&app)
        .arg(&run)
        .arg(scratch)
        .args(extra)
        .env("WORMHOLE_SOCKET", "/leaked.sock")
        .env("WORMHOLE_TAB_ID", "7")
        .env("WORMHOLE_BIN", "/leaked/Wormhole")
        .env("PATH", format!("/opt/homebrew/bin:{}", std::env::var("PATH").unwrap_or_default()))
        .output()
        .expect("bash runs");
    assert!(output.status.success(), "start_isolated failed:\n{}", String::from_utf8_lossy(&output.stderr));
    let log = std::fs::read_to_string(run.join("app.log")).expect("the stand-in's output");
    log.lines()
        .filter_map(|line| line.split_once('='))
        .map(|(name, value)| (name.to_string(), value.to_string()))
        .collect()
}

fn value<'a>(env: &'a [(String, String)], name: &str) -> Option<&'a str> {
    env.iter().find(|(n, _)| n == name).map(|(_, v)| v.as_str())
}

#[test]
fn an_isolated_instance_gets_its_own_settings_and_a_fixed_path() {
    let scratch = scratch("settings");
    let env = environment_of(&scratch, &["EXTRA=yes"]);
    let run = scratch.join("run");
    let in_run = |name: &str| run.join(name).display().to_string();

    assert_eq!(value(&env, "PATH"), Some("/usr/bin:/bin:/usr/sbin:/sbin"), "the Finder's PATH");
    assert_eq!(value(&env, "WORMHOLE_AGENT_PATH"), Some(""), "no agent counts as installed");
    assert_eq!(value(&env, "WORMHOLE_DATA_DIR"), Some(in_run("data").as_str()));
    assert_eq!(value(&env, "WORMHOLE_DEVCTL"), Some(in_run("ctl").as_str()));
    assert_eq!(value(&env, "WORMHOLE_AGENT_HOME"), Some(in_run("agent-home").as_str()));
    assert_eq!(
        value(&env, "WORMHOLE_CLI_DIR"),
        Some(in_run("cli-bin").as_str()),
        "Settings -> Command line tool installs into the run dir, never /usr/local/bin"
    );
    let home = run.join("home").canonicalize().expect("the scratch home exists");
    assert_eq!(value(&env, "HOME"), Some(home.display().to_string().as_str()));
    assert_eq!(value(&env, "WORMHOLE_START_DIR"), Some(scratch.display().to_string().as_str()));
    assert_eq!(value(&env, "WORMHOLE_SHELL"), Some("/bin/cat"));
    assert_eq!(value(&env, "WORMHOLE_AUTH"), Some("cancel"));
    assert_eq!(value(&env, "WORMHOLE_NOTIFY"), Some("log"));
    assert_eq!(value(&env, "WORMHOLE_BACKGROUND"), Some("1"));
    assert_eq!(value(&env, "EXTRA"), Some("yes"), "a caller's own pair comes through");
    for tabs_own in ["WORMHOLE_SOCKET", "WORMHOLE_TAB_ID", "WORMHOLE_BIN"] {
        assert_eq!(value(&env, tabs_own), None, "{tabs_own} is the parent tab's, not the instance's");
    }
    std::fs::remove_dir_all(&scratch).ok();
}

/// A scenario's `@env` and `devctl-instance.sh --env` come after the defaults, so they can put
/// a folder with a fake agent on the PATH.
#[test]
fn a_callers_later_pair_wins_over_the_defaults() {
    let scratch = scratch("override");
    let env = environment_of(&scratch, &["PATH=/fake/bin:/usr/bin:/bin", "WORMHOLE_AGENT_PATH=/fake/bin"]);
    assert_eq!(value(&env, "PATH"), Some("/fake/bin:/usr/bin:/bin"));
    assert_eq!(value(&env, "WORMHOLE_AGENT_PATH"), Some("/fake/bin"));
    std::fs::remove_dir_all(&scratch).ok();
}
