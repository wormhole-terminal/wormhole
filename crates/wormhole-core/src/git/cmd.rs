//! The one way this crate runs git.
//!
//! Wormhole runs git in every session's directory without being asked (status every few seconds,
//! including sessions in the background and ones restored at startup), and that directory may be a
//! repository someone else made. So every command is built here, with the same protections:
//!
//! - **No code from the repository.** `core.fsmonitor` can name a program that `status` and
//!   `diff` then run, so it is forced off; `diff` also gets `--no-ext-diff`/`--no-textconv` from
//!   its callers. Clean/smudge filters (`filter.<name>.clean`) cannot be switched off without
//!   changing what git reports, so they remain the one way a repository's config can run
//!   something, as they do in the user's own `git status` and shell prompt.
//! - **No option injection.** Anything a user or a repository can name (a branch, a commit) goes
//!   through [`GitCmd::revs`], after `--end-of-options`: a branch called `--output=/some/file`
//!   is legal in git and would otherwise make `git log` write that file.
//! - **No pathspec magic.** `GIT_LITERAL_PATHSPECS=1`: `a[1].txt` is that file, not a glob that
//!   also matches `a1.txt`. Every path we pass comes from git's own output, so literal is right.
//! - **The repository we mean.** The variables git uses to pick a repository or an index are
//!   removed, so being launched from a hook or a `git rebase --exec` cannot redirect `-C <dir>`.
//! - **No locks, no prompts, no pager.** `GIT_OPTIONAL_LOCKS=0` and `diff.autoRefreshIndex=false`
//!   keep a background refresh from taking the index lock away from an agent that is committing.
//! - **Bounded.** A deadline (git on a stalled network volume otherwise never returns, and the
//!   panel that waits for it never updates) and an optional cap on the output.

use std::ffi::{OsStr, OsString};
use std::io;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Output, Stdio};
use std::sync::mpsc;
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};

/// Long enough for `git status` in a very large repository on a cold cache (with fsmonitor
/// off, as we run it), short enough that a stuck command does not hold a panel hostage.
pub(crate) const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

/// How long to wait for a killed git to be reaped. A process stuck in the kernel (a dead NFS
/// mount) ignores SIGKILL until its I/O returns; we give up on it rather than hang with it.
const REAP_GRACE: Duration = Duration::from_secs(1);

/// What git (and a shell that was started by git) uses to pick a repository, an index or an
/// object store. Launched from a hook or `git rebase --exec`, we would inherit them, and they
/// beat `-C <dir>`: every session would show the status of the one repository. (The list of
/// `git rev-parse --local-env-vars`, minus the variables that only carry the user's own
/// configuration, which are theirs to set, plus `GIT_NAMESPACE`.)
const SCRUBBED_ENV: [&str; 13] = [
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_OBJECT_DIRECTORY",
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_IMPLICIT_WORK_TREE",
    "GIT_GRAFT_FILE",
    "GIT_INDEX_FILE",
    "GIT_NO_REPLACE_OBJECTS",
    "GIT_REPLACE_REF_BASE",
    "GIT_PREFIX",
    "GIT_SHALLOW_FILE",
    "GIT_COMMON_DIR",
    "GIT_NAMESPACE",
];

/// What the Xcode Command Line Tools stub says instead of running git. macOS ships
/// `/usr/bin/git` as a stub that runs the real git from the tools; without them it prints one of
/// these and exits. The second also asks the system to show the "install the command line
/// developer tools" dialog, so a caller that gets [`GitError::GitMissing`] must not keep polling.
const XCODE_STUB: [&str; 3] = [
    // The tools were installed and are gone (often after a macOS upgrade).
    "xcrun: error: invalid active developer path",
    // They were never installed: "xcode-select: note: No developer tools were found, requesting
    // install."
    "xcode-select: note: no developer tools were found",
    // The selected developer directory has no git.
    "xcrun: error: unable to find utility \"git\"",
];

/// How git begins to say that it found no repository in the folder or any of its parents
/// (`… (or any of the parent directories)`, or `… (or any parent up to mount point …)`). Not
/// the shorter `not a git repository: <path>`: that is a `.git` file pointing at a git
/// directory that is gone (a removed worktree), which is a broken repository, not none.
const NOT_A_REPO: &str = "fatal: not a git repository (or any";

/// Why a git command gave no usable output. Reachable from the `anyhow::Error` a git function
/// returns with `err.downcast_ref::<GitError>()`, or [`GitError::of`].
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GitError {
    /// git looked in the folder and its parents and found no repository.
    NotARepo,
    /// There is no git to run: none on the `PATH`, or only the stub macOS ships until the Xcode
    /// Command Line Tools are installed.
    GitMissing,
    /// git ran and failed; what it said on stderr.
    Failed(String),
    /// git did not finish in time and was killed.
    TimedOut(Duration),
    /// git printed more than the caller allows. It was killed at once, unless the caller asked
    /// how much it prints ([`GitCmd::measure_up_to`]): then `printed` is all of it.
    OutputTooLarge { limit: u64, printed: Option<u64> },
}

impl GitError {
    /// What became of a git function that returned `err`: its [`GitError`], or `Failed` with the
    /// error's text when it was not git that failed (no thread for a pipe, say).
    pub fn of(err: &anyhow::Error) -> Self {
        err.downcast_ref::<Self>().cloned().unwrap_or_else(|| Self::Failed(format!("{err:#}")))
    }
}

impl std::fmt::Display for GitError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotARepo => f.write_str("not a git repository"),
            Self::GitMissing => f.write_str("git is not installed"),
            Self::Failed(stderr) => f.write_str(stderr),
            Self::TimedOut(after) => write!(f, "git did not finish within {}s", after.as_secs_f32()),
            Self::OutputTooLarge { limit, .. } => write!(f, "git printed more than {limit} bytes"),
        }
    }
}

impl std::error::Error for GitError {}

/// Why git, which ran, failed, from what it printed on stderr (in English: every command runs
/// with `LC_ALL=C`). Anything that is neither a missing git nor a missing repository is
/// `Failed` with git's own words: dubious ownership, a corrupt index, a bad revision.
pub(crate) fn classify(status: ExitStatus, stderr: &str) -> GitError {
    let stderr = stderr.trim();
    let lower = stderr.to_ascii_lowercase();
    if XCODE_STUB.iter().any(|stub| lower.contains(&stub.to_ascii_lowercase())) {
        GitError::GitMissing
    } else if stderr.lines().any(|line| line.starts_with(NOT_A_REPO)) {
        GitError::NotARepo
    } else if stderr.is_empty() {
        GitError::Failed(format!("git exited with {status}"))
    } else {
        GitError::Failed(stderr.to_string())
    }
}

/// Why git could not be started: not found means there is no git at all (or not where
/// `WORMHOLE_GIT_BIN` says); anything else (not executable, no processes left) is a failure.
fn unstarted(err: io::Error) -> anyhow::Error {
    if err.kind() == io::ErrorKind::NotFound {
        GitError::GitMissing.into()
    } else {
        anyhow::Error::new(err).context("running git")
    }
}

/// A read-only git invocation in one directory. Chain `arg`s, then `revs`, then `paths`
/// (that is the order git wants them in), then `run` or `output`.
pub(crate) struct GitCmd {
    command: Command,
    timeout: Duration,
    max_output: Option<u64>,
    /// With `max_output`: read on past it, without keeping the bytes, up to this many.
    measure_to: Option<u64>,
}

impl GitCmd {
    pub(crate) fn new(dir: &Path) -> Self {
        Self::with_program(program(), dir)
    }

    /// [`GitCmd::new`] running `program` instead of git: a stand-in that fails the way git can.
    pub(crate) fn with_program(program: impl AsRef<OsStr>, dir: &Path) -> Self {
        let mut command = Command::new(program);
        command
            .arg("--no-pager")
            .arg("-C")
            .arg(dir)
            .args(["-c", "core.fsmonitor=false"])
            // `git diff` refreshes the index after comparing and, whatever GIT_OPTIONAL_LOCKS
            // says, writes it when it found stale entries: it then holds `index.lock`, and an
            // agent's `git commit` in that instant fails. (Seen in 4 of 80 runs on a busy
            // machine; with this off, in none.) The comparison itself is unaffected.
            .args(["-c", "diff.autoRefreshIndex=false"])
            .env("GIT_OPTIONAL_LOCKS", "0")
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("GIT_LITERAL_PATHSPECS", "1")
            .env("LC_ALL", "C");
        for var in SCRUBBED_ENV {
            command.env_remove(var);
        }
        Self { command, timeout: DEFAULT_TIMEOUT, max_output: None, measure_to: None }
    }

    /// `git diff` that never runs a program named by the repository: no external diff driver
    /// and no textconv filter (both come from `.git/config` and `.gitattributes`).
    pub(crate) fn diff(dir: &Path) -> Self {
        let mut cmd = Self::new(dir);
        cmd.args(["diff", "--no-ext-diff", "--no-textconv"]);
        cmd
    }

    pub(crate) fn arg(&mut self, arg: impl AsRef<OsStr>) -> &mut Self {
        self.command.arg(arg);
        self
    }

    pub(crate) fn args<I, S>(&mut self, args: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.command.args(args);
        self
    }

    /// Revisions, refs or `rev:path` names that did not come from this crate: placed after
    /// `--end-of-options`, so none can be read as an option. Call once, after the options.
    pub(crate) fn revs<I, S>(&mut self, revs: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.command.arg("--end-of-options").args(revs);
        self
    }

    /// Paths, after `--` (which is added even when there are none: it also keeps a revision
    /// from being taken for a file of the same name). Call last.
    pub(crate) fn paths<I, S>(&mut self, paths: I) -> &mut Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.command.arg("--").args(paths);
        self
    }

    // Used by the tests, and by callers that want to give up sooner than the default.
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn timeout(&mut self, timeout: Duration) -> &mut Self {
        self.timeout = timeout;
        self
    }

    /// Kill git and fail with [`GitError::OutputTooLarge`] once it has printed more than this.
    pub(crate) fn max_output(&mut self, bytes: u64) -> &mut Self {
        self.max_output = Some(bytes);
        self
    }

    /// With [`GitCmd::max_output`]: past the cap, read on to the end without keeping what git
    /// prints (up to `stop` bytes in all), so the [`GitError::OutputTooLarge`] says how much that
    /// was. git works out a diff before it prints any of it, so the rest costs little to read.
    pub(crate) fn measure_up_to(&mut self, stop: u64) -> &mut Self {
        self.measure_to = Some(stop);
        self
    }

    /// Run it and return what it printed; a non-zero exit is a [`GitError`] saying why.
    pub(crate) fn run(&mut self) -> Result<Vec<u8>> {
        let output = self.output()?;
        if !output.status.success() {
            return Err(classify(output.status, &String::from_utf8_lossy(&output.stderr)).into());
        }
        Ok(output.stdout)
    }

    /// Run it and return everything, whatever the exit status (`diff --no-index` exits 1 when
    /// the files differ). Timeouts and oversized output are still errors.
    pub(crate) fn output(&mut self) -> Result<Output> {
        let limit = match (self.max_output, self.measure_to) {
            (Some(keep), Some(stop)) => Limit::Count { keep, stop: stop.max(keep) },
            (Some(cap), None) => Limit::Stop(cap),
            (None, _) => Limit::None,
        };
        run_bounded(&mut self.command, self.timeout, limit)
    }

    #[cfg(test)]
    pub(crate) fn command(&self) -> &Command {
        &self.command
    }
}

/// The git [`GitCmd::new`] runs: `git` from the `PATH`. Debug builds run `WORMHOLE_GIT_BIN`
/// instead when it is set, so a devctl session can show the panels without git
/// (`/nonexistent`) or with one that fails (a script); release builds never read it.
fn program() -> OsString {
    #[cfg(test)]
    if let Some(program) = PROGRAM.with_borrow(Clone::clone) {
        return program;
    }
    cfg!(debug_assertions)
        .then(|| std::env::var_os("WORMHOLE_GIT_BIN"))
        .flatten()
        .unwrap_or_else(|| "git".into())
}

// Processes started on this thread, so a test can say how many git calls something costs.
#[cfg(test)]
thread_local! {
    pub(crate) static SPAWNS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

// What `GitCmd::new` runs on this thread instead of git (see `testing::with_git`): a test's own
// stand-in, without an environment variable that every other test would see.
#[cfg(test)]
thread_local! {
    pub(crate) static PROGRAM: std::cell::RefCell<Option<OsString>> = const { std::cell::RefCell::new(None) };
}

/// What a reader thread found: the bytes kept, how many arrived in all, and whether it stopped
/// reading because of the cap.
type Drained = io::Result<(Vec<u8>, u64, bool)>;

/// What to do with a pipe's contents.
#[derive(Clone, Copy)]
pub(super) enum Limit {
    /// Keep it all.
    None,
    /// Stop reading, and say so, once more than this many bytes have arrived.
    Stop(u64),
    /// Keep this many bytes and read (and drop) the rest, so the writer is never blocked.
    Keep(usize),
    /// Keep it while it is at most `keep` bytes, then only count it; stop once more than
    /// `stop` bytes have arrived.
    Count { keep: u64, stop: u64 },
}

impl Limit {
    /// The most output the caller takes; more is [`GitError::OutputTooLarge`].
    fn cap(self) -> Option<u64> {
        match self {
            Self::Stop(cap) | Self::Count { keep: cap, .. } => Some(cap),
            Self::None | Self::Keep(_) => None,
        }
    }
}

/// `Command::output` with a deadline and a cap. Both pipes are drained on their own threads
/// (a full pipe would stall git while we wait for it), and the waiting happens on a channel, so
/// a command that finishes quickly costs no polling.
pub(super) fn run_bounded(command: &mut Command, timeout: Duration, limit: Limit) -> Result<Output> {
    command.stdin(Stdio::null()).stdout(Stdio::piped()).stderr(Stdio::piped());
    #[cfg(test)]
    SPAWNS.with(|spawns| spawns.set(spawns.get() + 1));
    let mut child = command.spawn().map_err(unstarted)?;
    let deadline = Instant::now() + timeout;

    let (stdout, stderr) = match (child.stdout.take(), child.stderr.take()) {
        (Some(stdout), Some(stderr)) => (stdout, stderr),
        _ => {
            stop(&mut child);
            anyhow::bail!("git's output pipes are missing");
        }
    };
    let out_rx = reader(stdout, limit);
    let err_rx = reader(stderr, Limit::Keep(1 << 20));

    let drained = match out_rx.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        Ok(drained) => drained,
        Err(mpsc::RecvTimeoutError::Timeout) => {
            stop(&mut child);
            return Err(GitError::TimedOut(timeout).into());
        }
        Err(mpsc::RecvTimeoutError::Disconnected) => {
            stop(&mut child);
            anyhow::bail!("could not start a thread to read git's output");
        }
    };
    let (stdout, printed, stopped) = match drained {
        Ok(drained) => drained,
        Err(err) => {
            stop(&mut child);
            return Err(err).context("reading git's output");
        }
    };
    if let Some(cap) = limit.cap()
        && printed > cap
    {
        stop(&mut child);
        let printed = (!stopped).then_some(printed);
        return Err(GitError::OutputTooLarge { limit: cap, printed }.into());
    }

    // Its output is complete; it is about to exit, so poll shortly rather than block.
    let mut nap = Duration::from_micros(100);
    let status = loop {
        match child.try_wait().context("waiting for git")? {
            Some(status) => break status,
            None if Instant::now() >= deadline => {
                stop(&mut child);
                return Err(GitError::TimedOut(timeout).into());
            }
            None => {
                thread::sleep(nap);
                nap = (nap * 2).min(Duration::from_millis(5));
            }
        }
    };
    // The pipe closed with the process, unless something it started still holds it: then what
    // was said so far will do.
    let stderr = match err_rx.recv_timeout(Duration::from_millis(200)) {
        Ok(Ok((bytes, ..))) => bytes,
        _ => Vec::new(),
    };
    Ok(Output { status, stdout, stderr })
}

/// Read `source` on a thread until its end (or until `limit` says stop).
fn reader(mut source: impl io::Read + Send + 'static, limit: Limit) -> mpsc::Receiver<Drained> {
    let (tx, rx) = mpsc::channel();
    // If no thread can be had the closure, and with it `tx`, is dropped: the receiver then sees
    // a closed channel, which the caller reports.
    let _ = thread::Builder::new().name("git-output".into()).spawn(move || {
        let _ = tx.send(drain(&mut source, limit));
    });
    rx
}

fn drain(source: &mut impl io::Read, limit: Limit) -> Drained {
    let mut bytes = Vec::new();
    let mut total = 0u64;
    let mut chunk = [0u8; 64 * 1024];
    loop {
        let n = match source.read(&mut chunk) {
            Ok(0) => return Ok((bytes, total, false)),
            Ok(n) => n,
            Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
            Err(err) => return Err(err),
        };
        total += n as u64;
        match limit {
            Limit::None => bytes.extend_from_slice(&chunk[..n]),
            Limit::Stop(cap) => {
                bytes.extend_from_slice(&chunk[..n]);
                if total > cap {
                    return Ok((bytes, total, true));
                }
            }
            Limit::Keep(keep) => {
                let room = keep.saturating_sub(bytes.len()).min(n);
                bytes.extend_from_slice(&chunk[..room]);
            }
            Limit::Count { keep, stop } => {
                if total <= keep {
                    bytes.extend_from_slice(&chunk[..n]);
                } else if !bytes.is_empty() {
                    // Too much to keep: from here on it is only counted.
                    bytes = Vec::new();
                }
                if total > stop {
                    return Ok((bytes, total, true));
                }
            }
        }
    }
}

/// Kill and reap, without waiting on a process that cannot be killed.
fn stop(child: &mut Child) {
    let _ = child.kill();
    let give_up = Instant::now() + REAP_GRACE;
    while Instant::now() < give_up {
        if !matches!(child.try_wait(), Ok(None)) {
            return;
        }
        thread::sleep(Duration::from_millis(2));
    }
    tracing::warn!("git (pid {}) did not exit after being killed; leaving it", child.id());
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::process::Command;
    use std::time::{Duration, Instant};

    use super::{GitCmd, GitError, Limit, SCRUBBED_ENV, classify, run_bounded};

    fn args(cmd: &GitCmd) -> Vec<String> {
        cmd.command().get_args().map(|a| a.to_string_lossy().into_owned()).collect()
    }

    fn env_of(cmd: &GitCmd, key: &str) -> Option<Option<OsString>> {
        cmd.command().get_envs().find(|(k, _)| *k == key).map(|(_, v)| v.map(|v| v.to_os_string()))
    }

    #[test]
    fn every_command_is_hardened() {
        let cmd = GitCmd::new(std::path::Path::new("/some/repo"));
        let args = args(&cmd);
        assert_eq!(
            args[..7],
            [
                "--no-pager",
                "-C",
                "/some/repo",
                "-c",
                "core.fsmonitor=false",
                "-c",
                "diff.autoRefreshIndex=false"
            ]
        );
        for (key, value) in
            [("GIT_OPTIONAL_LOCKS", "0"), ("GIT_TERMINAL_PROMPT", "0"), ("GIT_LITERAL_PATHSPECS", "1")]
        {
            assert_eq!(env_of(&cmd, key), Some(Some(OsString::from(value))), "{key}");
        }
        // Removed, not just unset: `Some(None)` is `env_remove`.
        for key in SCRUBBED_ENV {
            assert_eq!(env_of(&cmd, key), Some(None), "{key} must be scrubbed");
        }
        for key in ["GIT_DIR", "GIT_WORK_TREE", "GIT_INDEX_FILE", "GIT_OBJECT_DIRECTORY", "GIT_COMMON_DIR"] {
            assert!(SCRUBBED_ENV.contains(&key), "{key}");
        }
        // The user's own configuration (safe.directory through the environment, say) stays.
        assert_eq!(env_of(&cmd, "GIT_CONFIG_COUNT"), None);
    }

    #[test]
    fn revs_follow_end_of_options_and_paths_follow_the_separator() {
        let mut cmd = GitCmd::new(std::path::Path::new("/r"));
        cmd.arg("diff").arg("-M").revs(["--output=/x", "HEAD"]).paths(["a[1].txt"]);
        let args = args(&cmd);
        assert_eq!(args[7..], ["diff", "-M", "--end-of-options", "--output=/x", "HEAD", "--", "a[1].txt"]);
        // `--` is there even with no paths.
        let mut cmd = GitCmd::new(std::path::Path::new("/r"));
        cmd.arg("log").revs(["main"]).paths::<_, &str>([]);
        assert_eq!(args_tail(&cmd), ["log", "--end-of-options", "main", "--"]);
    }

    fn args_tail(cmd: &GitCmd) -> Vec<String> {
        args(cmd)[7..].to_vec()
    }

    fn kill_probe() -> (std::path::PathBuf, Command) {
        // Writes its pid, then becomes `sleep`, so the pid is the process we expect to be killed.
        let pidfile =
            std::env::temp_dir().join(format!("hd-git-pid-{}-{:?}", std::process::id(), Instant::now()));
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg(format!("echo $$ > '{}'; exec sleep 30", pidfile.display()));
        (pidfile, cmd)
    }

    fn alive(pid: &str) -> bool {
        Command::new("kill").args(["-0", pid]).output().is_ok_and(|o| o.status.success())
    }

    #[test]
    fn a_command_that_does_not_finish_is_killed_at_the_deadline() {
        let (pidfile, mut cmd) = kill_probe();
        let started = Instant::now();
        let err = run_bounded(&mut cmd, Duration::from_millis(300), Limit::None).expect_err("times out");
        assert!(matches!(err.downcast_ref::<GitError>(), Some(GitError::TimedOut(_))), "{err:#}");
        assert!(started.elapsed() < Duration::from_secs(5), "took {:?}", started.elapsed());
        let pid = std::fs::read_to_string(&pidfile).expect("pid written");
        assert!(!alive(pid.trim()), "the process must be gone, not left running");
        let _ = std::fs::remove_file(pidfile);
    }

    #[test]
    fn output_over_the_cap_kills_the_command() {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg("yes | head -c 50000000");
        let started = Instant::now();
        let err = run_bounded(&mut cmd, Duration::from_secs(20), Limit::Stop(1000)).expect_err("too large");
        let stopped = GitError::OutputTooLarge { limit: 1000, printed: None };
        assert_eq!(err.downcast_ref::<GitError>(), Some(&stopped));
        assert!(started.elapsed() < Duration::from_secs(5));
        // Exactly at the cap is fine.
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg("printf 12345");
        assert_eq!(
            run_bounded(&mut cmd, Duration::from_secs(20), Limit::Stop(5)).expect("fits").stdout,
            b"12345"
        );
    }

    #[test]
    fn output_over_the_cap_can_be_measured_up_to_a_bound() {
        let run = |script: &str, keep, stop| {
            let mut cmd = Command::new("sh");
            cmd.arg("-c").arg(script);
            run_bounded(&mut cmd, Duration::from_secs(20), Limit::Count { keep, stop })
        };
        // Over the cap: read to the end and counted, none of it kept.
        let err = run("head -c 300000 /dev/zero", 1000, 1 << 20).expect_err("too large");
        assert_eq!(GitError::of(&err), GitError::OutputTooLarge { limit: 1000, printed: Some(300_000) });
        // Past the bound it is stopped like any output over its cap, and not measured.
        let started = Instant::now();
        let err = run("yes | head -c 50000000", 1000, 100_000).expect_err("too large");
        assert_eq!(GitError::of(&err), GitError::OutputTooLarge { limit: 1000, printed: None });
        assert!(started.elapsed() < Duration::from_secs(5));
        // Within the cap it all comes back.
        assert_eq!(run("printf 12345", 5, 100).expect("fits").stdout, b"12345");
    }

    #[test]
    fn output_and_exit_status_come_back_whole() {
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg("printf out; printf err >&2; exit 3");
        let output = run_bounded(&mut cmd, Duration::from_secs(20), Limit::None).expect("runs");
        assert_eq!((output.stdout.as_slice(), output.stderr.as_slice()), (&b"out"[..], &b"err"[..]));
        assert_eq!(output.status.code(), Some(3));
        // Plenty of output on both pipes at once does not stall it.
        let mut cmd = Command::new("sh");
        cmd.arg("-c").arg("head -c 3000000 /dev/zero; head -c 3000000 /dev/zero >&2");
        let output = run_bounded(&mut cmd, Duration::from_secs(20), Limit::None).expect("runs");
        assert_eq!((output.stdout.len(), output.stderr.len()), (3_000_000, 1 << 20));
    }

    /// What git (or the macOS stub in its place) prints, and what that is.
    const SAID: [(&str, Option<GitError>); 9] = [
        // The Command Line Tools stub, after a macOS upgrade removed the tools...
        (
            "xcrun: error: invalid active developer path (/Library/Developer/CommandLineTools), \
             missing xcrun at: /Library/Developer/CommandLineTools/usr/bin/xcrun",
            Some(GitError::GitMissing),
        ),
        // ...and on a Mac that never had them (this one also opens the install dialog).
        (
            "xcode-select: note: No developer tools were found, requesting install.\n\
             If developer tools are located at a non-default location on disk, use `sudo \
             xcode-select --switch path/to/Xcode.app` to specify the Xcode that you wish to use \
             for command line developer tools, and cancel the installation dialog.\n\
             See `man xcode-select' for more details.",
            Some(GitError::GitMissing),
        ),
        (
            "xcrun: error: unable to find utility \"git\", not a developer tool or in PATH",
            Some(GitError::GitMissing),
        ),
        ("fatal: not a git repository (or any of the parent directories): .git", Some(GitError::NotARepo)),
        (
            "fatal: not a git repository (or any parent up to mount point /Volumes)\n\
             Stopping at filesystem boundary (GIT_DISCOVERY_ACROSS_FILESYSTEM not set).",
            Some(GitError::NotARepo),
        ),
        // A worktree whose git directory is gone is a broken repository, not none: git's words.
        ("fatal: not a git repository: /work/app/.git/worktrees/gone", None),
        (crate::git::testing::DUBIOUS, None),
        ("error: bad signature 0x00000000\nfatal: index file corrupt", None),
        ("  fatal: bad revision 'gone'\n", None),
    ];

    #[test]
    fn classify_tells_a_missing_git_and_a_missing_repository_from_a_failure() {
        let status = crate::platform::process::exit_status_from_raw(128 << 8);
        for (stderr, kind) in SAID {
            // Anything else is a failure in git's own words (trimmed), all of them: the view shows
            // the first line and copies the rest.
            let expected = kind.unwrap_or_else(|| GitError::Failed(stderr.trim().to_string()));
            assert_eq!(classify(status, stderr), expected, "{stderr}");
        }
        assert_eq!(classify(status, " \n"), GitError::Failed("git exited with exit status: 128".into()));
    }

    #[test]
    fn a_stand_in_git_is_told_apart_by_what_it_says() {
        use crate::git::testing::fake_git;
        let dir = tempfile::tempdir().expect("tempdir");
        for (n, (stderr, kind)) in SAID.into_iter().enumerate() {
            let program = fake_git(dir.path(), &format!("git{n}"), stderr, 128);
            let err = GitCmd::with_program(&program, dir.path()).arg("status").run().expect_err("fails");
            let expected = kind.unwrap_or_else(|| GitError::Failed(stderr.trim().to_string()));
            assert_eq!(GitError::of(&err), expected, "{stderr}");
        }
        // The stub exits 1, but what it says decides, not how it exits.
        let stub = fake_git(dir.path(), "stub", SAID[0].0, 1);
        let err = GitCmd::with_program(&stub, dir.path()).arg("status").run().expect_err("fails");
        assert_eq!(GitError::of(&err), GitError::GitMissing);
    }

    #[test]
    fn a_git_that_is_not_there_is_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let absent = dir.path().join("no-git-here");
        let err = GitCmd::with_program(&absent, dir.path()).arg("status").run().expect_err("fails");
        assert_eq!(GitError::of(&err), GitError::GitMissing);
        // Also for the callers that look at the exit status themselves.
        let err = GitCmd::with_program(&absent, dir.path()).arg("status").output().expect_err("fails");
        assert_eq!(GitError::of(&err), GitError::GitMissing);
    }

    #[test]
    fn a_git_that_cannot_be_started_is_a_failure_not_missing() {
        // A folder where the program should be: there is something, it just cannot run.
        let dir = tempfile::tempdir().expect("tempdir");
        let err = GitCmd::with_program(dir.path(), dir.path()).arg("status").run().expect_err("fails");
        match GitError::of(&err) {
            GitError::Failed(message) => assert!(message.starts_with("running git: "), "{message}"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_failure_carries_gits_message() {
        let dir = crate::git::testing::init();
        let err =
            GitCmd::new(dir.path()).args(["rev-parse", "--verify", "no-such-rev"]).run().expect_err("fails");
        match err.downcast_ref::<GitError>() {
            Some(GitError::Failed(message)) => assert!(!message.is_empty()),
            other => panic!("{other:?}"),
        }
        // Outside a repository, git says so, and that is told apart from a failure.
        let plain = tempfile::tempdir().expect("tempdir");
        let err = GitCmd::new(plain.path()).args(["rev-parse", "--show-toplevel"]).run().expect_err("fails");
        assert_eq!(GitError::of(&err), GitError::NotARepo);
    }

    #[test]
    fn a_stuck_git_status_is_killed_and_reported() {
        use crate::git::testing::{git, repo_with, write};
        // A clean filter that does not return stands in for git stuck on a dead network volume.
        let dir = repo_with(&[("a.txt", "hello one\n")]);
        let root = dir.path();
        git(root, &["config", "filter.slow.clean", "sleep 8; cat"]);
        write(root, ".gitattributes", "a.txt filter=slow\n");
        std::thread::sleep(Duration::from_millis(30));
        write(root, "a.txt", "hello two\n"); // same size, new mtime

        let started = Instant::now();
        let err = GitCmd::new(root)
            .timeout(Duration::from_millis(500))
            .args(["status", "--porcelain=v2"])
            .run()
            .expect_err("times out");
        assert!(matches!(err.downcast_ref::<GitError>(), Some(GitError::TimedOut(_))), "{err:#}");
        assert!(started.elapsed() < Duration::from_secs(4), "took {:?}", started.elapsed());
    }
}
