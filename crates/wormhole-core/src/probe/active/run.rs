//! Starting a hook's `run`: the one place a Probe's program is spawned.
//!
//! What may run was decided before this: the Probe is approved and `process` lists the program (the
//! dispatcher), and the arguments were built under the fixed capture rules (`hooks::plan`). What is left is
//! how it starts, which is the same for every Probe and none of its business:
//!
//! - from the approved copy, re-hashed first (`approval::runnable`), so the script that runs is the script
//!   that was approved; a `./script` is a file inside that copy, resolved through its links;
//! - no shell, stdin closed, a scrubbed environment: only the user's name and temp folder, the variables the
//!   user granted by name, `PATH` of absolute folders and the `WORMHOLE_*` facts of the terminal;
//! - `git` is never started as the Probe wrote it but through [`GitCmd`] (read-only subcommands, no
//!   repository code, no external diff or textconv), and only in a folder the shell itself confirmed;
//! - a deadline and a cap on what it prints, both enforced by the same runner git uses.
//!
//! Blocks until the process ends: call it from a thread of its own, never from a frame.

use std::ffi::OsString;
use std::path::{Component, Path, PathBuf};
use std::process::Output;
use std::time::Duration;

use crate::git::{GitCmd, GitError, Limit, run_bounded};
use crate::platform::process;
use crate::probe::approval;
use crate::probe::events::RUN_PRINT_MAX;
use crate::probe::manifest::Cwd;
use crate::probe::validate::READ_ONLY_GIT;
use crate::storage::paths::DataRoot;

/// What a program is given besides what the Probe granted by name: who the user is and where scratch files
/// go.
const PASSED_ENV: [&str; 4] = ["HOME", "USER", "LOGNAME", "TMPDIR"];

/// The usual homes of the programs a Probe names, after the folder the program itself is in.
const PATH_TAIL: [&str; 6] =
    ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin", "/usr/sbin", "/sbin"];

/// `git` subcommands that take these two options; the others of [`READ_ONLY_GIT`] refuse them.
const DIFFING: [&str; 3] = ["diff", "show", "log"];

/// A program to start for a Probe.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunJob {
    /// The Probe's id.
    pub probe: String,
    /// The program and its arguments, as `hooks::plan` built them.
    pub argv: Vec<String>,
    pub cwd: Cwd,
    /// The terminal's shell-confirmed folder, for `Cwd::Terminal`.
    pub folder: Option<PathBuf>,
    pub timeout: Duration,
    /// Variables the user let this Probe pass on, by name.
    pub env: Vec<String>,
    pub terminal: Option<i64>,
}

/// How a started program ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunEnd {
    Exited {
        code: i32,
        stdout: String,
    },
    TimedOut,
    /// It printed more than [`RUN_PRINT_MAX`] bytes and was killed.
    TooMuchOutput,
    /// It was never started, and why: the Probe is not as approved, there is no folder, no such program.
    NotRun(String),
}

/// Run `job` for the Probe it names, from its approved copy in `root`. Reads the copy and the key (which may
/// prompt) and waits for the process: off the main thread.
pub fn execute(root: &DataRoot, job: &RunJob) -> RunEnd {
    match approval::runnable(root, &job.probe) {
        Ok(copy) => run_in(copy.dir(), job),
        Err(why) => RunEnd::NotRun(why.to_string()),
    }
}

fn run_in(probe_dir: &Path, job: &RunJob) -> RunEnd {
    let folder = match (job.cwd, &job.folder) {
        (Cwd::Probe, _) => probe_dir.to_path_buf(),
        (Cwd::Terminal, Some(folder)) => folder.clone(),
        (Cwd::Terminal, None) => return RunEnd::NotRun("the terminal has no confirmed folder".into()),
    };
    let Some((program, args)) = job.argv.split_first() else { return RunEnd::NotRun("no program".into()) };
    let ended = if program == "git" {
        run_git(&folder, args, job)
    } else {
        run_program(probe_dir, &folder, program, args, job)
    };
    match ended {
        Ok(output) => RunEnd::Exited {
            code: output.status.code().unwrap_or(-1),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        },
        Err(err) => match err.downcast_ref::<GitError>() {
            Some(GitError::TimedOut(_)) => RunEnd::TimedOut,
            Some(GitError::OutputTooLarge { .. }) => RunEnd::TooMuchOutput,
            _ => RunEnd::NotRun(format!("{err:#}")),
        },
    }
}

fn run_git(folder: &Path, args: &[String], job: &RunJob) -> anyhow::Result<Output> {
    let sub = args.first().map(String::as_str).filter(|sub| READ_ONLY_GIT.contains(sub));
    let sub = sub.ok_or_else(|| anyhow::anyhow!("git runs only {}", READ_ONLY_GIT.join(", ")))?;
    let mut git = GitCmd::new(folder);
    git.arg(sub);
    if DIFFING.contains(&sub) {
        git.args(["--no-ext-diff", "--no-textconv"]);
    }
    git.args(&args[1..]).timeout(job.timeout).max_output(RUN_PRINT_MAX as u64).output()
}

fn run_program(
    probe_dir: &Path,
    folder: &Path,
    program: &str,
    args: &[String],
    job: &RunJob,
) -> anyhow::Result<Output> {
    let path = resolve(probe_dir, program)?;
    let mut command = process::command(&path);
    command.args(args).current_dir(folder).env_clear().envs(environment(
        std::env::vars_os(),
        &path,
        folder,
        job,
    ));
    run_bounded(&mut command, job.timeout, Limit::Stop(RUN_PRINT_MAX as u64))
}

/// The file `program` names: `./script` inside the approved copy (every link followed, and the result must
/// stay inside it), else a bare name found on the user's path.
fn resolve(probe_dir: &Path, program: &str) -> anyhow::Result<PathBuf> {
    let Some(relative) = program.strip_prefix("./") else {
        if program.contains('/') {
            anyhow::bail!("`{program}` is neither a bare name nor a `./` script");
        }
        return crate::agents::find_on_path(program)
            .ok_or_else(|| anyhow::anyhow!("`{program}` is not installed"));
    };
    if Path::new(relative).components().any(|part| !matches!(part, Component::Normal(_))) {
        anyhow::bail!("`{program}` leaves the Probe's folder");
    }
    let inside = std::fs::canonicalize(probe_dir)?;
    let script = std::fs::canonicalize(probe_dir.join(relative))?;
    if !script.starts_with(&inside) {
        anyhow::bail!("`{program}` is a link out of the Probe's folder");
    }
    Ok(script)
}

/// The environment a program gets: what [`PASSED_ENV`] and the job's granted names pick from `vars`, the
/// `WORMHOLE_*` facts, and a `PATH` of absolute folders only, starting with the one the program is in.
fn environment(
    vars: impl Iterator<Item = (OsString, OsString)>,
    program: &Path,
    folder: &Path,
    job: &RunJob,
) -> Vec<(OsString, OsString)> {
    let wanted = |name: &OsString| {
        PASSED_ENV.iter().any(|kept| name == kept) || job.env.iter().any(|granted| name == granted.as_str())
    };
    let mut env: Vec<(OsString, OsString)> = vars.filter(|(name, _)| wanted(name)).collect();
    let fixed = [("LC_ALL", "C".to_string()), ("WORMHOLE_PROBE", job.probe.clone())];
    env.extend(fixed.into_iter().map(|(name, value)| (name.into(), value.into())));
    env.push(("WORMHOLE_FOLDER".into(), folder.as_os_str().to_os_string()));
    if let Some(terminal) = job.terminal {
        env.push(("WORMHOLE_TERMINAL".into(), terminal.to_string().into()));
    }
    let own = program.parent().filter(|dir| dir.is_absolute()).map(Path::to_path_buf);
    let path = std::env::join_paths(own.into_iter().chain(PATH_TAIL.iter().map(PathBuf::from)));
    env.extend(path.ok().map(|path| ("PATH".into(), path)));
    env
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::TempDir;

    use super::*;

    fn job(argv: &[&str], timeout: Duration) -> RunJob {
        RunJob {
            probe: "acme-tools".into(),
            argv: argv.iter().map(|a| a.to_string()).collect(),
            cwd: Cwd::Probe,
            folder: None,
            timeout,
            env: vec!["ACME_TOKEN".into()],
            terminal: Some(7),
        }
    }

    /// A Probe folder holding `run.sh` with `body`.
    fn folder(body: &str) -> TempDir {
        let dir = TempDir::new().expect("tempdir");
        let script = dir.path().join("run.sh");
        fs::write(&script, format!("#!/bin/sh\n{body}\n")).expect("script");
        fs::set_permissions(&script, crate::platform::fs::permissions(0o755)).expect("chmod");
        dir
    }

    fn ran(dir: &TempDir, argv: &[&str], timeout: Duration) -> RunEnd {
        run_in(dir.path(), &job(argv, timeout))
    }

    const SECONDS: Duration = Duration::from_secs(10);

    #[test]
    fn a_script_gets_its_facts_and_none_of_the_apps_environment() {
        let dir = folder("env");
        let RunEnd::Exited { code: 0, stdout } = ran(&dir, &["./run.sh"], SECONDS) else {
            panic!("a clean run");
        };
        let names: Vec<&str> = stdout.lines().filter_map(|line| line.split('=').next()).collect();
        assert!(names.contains(&"WORMHOLE_PROBE") && names.contains(&"WORMHOLE_TERMINAL"), "{names:?}");
        // `cargo test` runs with CARGO_* and friends set: none of that is the Probe's to see.
        let allowed = ["HOME", "USER", "LOGNAME", "TMPDIR", "PATH", "LC_ALL", "PWD", "SHLVL", "_", "OLDPWD"];
        let extra: Vec<_> =
            names.iter().filter(|n| !allowed.contains(n) && !n.starts_with("WORMHOLE_")).collect();
        assert!(extra.is_empty(), "{extra:?}");
    }

    #[test]
    fn only_the_variables_the_user_named_are_passed_on() {
        let vars = [("ACME_TOKEN", "t"), ("AWS_SECRET", "s"), ("HOME", "/h")]
            .map(|(name, value)| (OsString::from(name), OsString::from(value)));
        let job = job(&["./run.sh"], SECONDS);
        let env = environment(vars.into_iter(), Path::new("/p/run.sh"), Path::new("/f"), &job);
        let names: Vec<_> = env.iter().map(|(name, _)| name.to_string_lossy().into_owned()).collect();
        assert!(names.contains(&"ACME_TOKEN".to_string()) && names.contains(&"HOME".to_string()));
        assert!(!names.contains(&"AWS_SECRET".to_string()));
        let path = env.iter().find(|(name, _)| name == "PATH").expect("a PATH");
        assert!(std::env::split_paths(&path.1).all(|dir| dir.is_absolute()));
    }

    #[test]
    fn a_script_is_only_ever_a_file_inside_the_approved_copy() {
        let dir = folder("echo hi");
        let outside = folder("echo out");
        crate::platform::fs::symlink(outside.path().join("run.sh"), dir.path().join("planted.sh"))
            .expect("link");
        for program in ["./../x", "./a/../../x", "./planted.sh", "/bin/echo", "bin/echo", "./missing.sh"] {
            assert!(matches!(ran(&dir, &[program], SECONDS), RunEnd::NotRun(_)), "{program}");
        }
        assert!(matches!(ran(&dir, &["./run.sh"], SECONDS), RunEnd::Exited { code: 0, .. }));
    }

    #[test]
    fn a_slow_or_noisy_program_is_stopped() {
        let slow = folder("exec sleep 5");
        assert_eq!(ran(&slow, &["./run.sh"], Duration::from_millis(300)), RunEnd::TimedOut);
        let noisy = folder("exec yes");
        assert_eq!(ran(&noisy, &["./run.sh"], SECONDS), RunEnd::TooMuchOutput);
    }

    #[test]
    fn nothing_runs_in_a_folder_the_shell_did_not_confirm_and_git_stays_read_only() {
        let dir = folder("echo hi");
        let mut no_folder = job(&["./run.sh"], SECONDS);
        no_folder.cwd = Cwd::Terminal;
        assert!(matches!(run_in(dir.path(), &no_folder), RunEnd::NotRun(why) if why.contains("confirmed")));
        no_folder.folder = Some(dir.path().to_path_buf());
        assert!(matches!(run_in(dir.path(), &no_folder), RunEnd::Exited { code: 0, .. }));
        let mut commit = job(&["git", "commit", "-m", "x"], SECONDS);
        commit.folder = Some(dir.path().to_path_buf());
        commit.cwd = Cwd::Terminal;
        assert!(matches!(run_in(dir.path(), &commit), RunEnd::NotRun(why) if why.contains("git runs only")));
    }
}
