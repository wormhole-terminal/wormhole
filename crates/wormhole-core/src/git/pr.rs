//! The pull request of the branch a terminal is on (the title row's PR link),
//! asked of the GitHub CLI: `gh pr view --json number,url,state` in the repository.
//!
//! Wormhole has no network code of its own, and git here is local and read-only; `gh` is the one
//! way this talks to a host, and only where the person has it installed and signed in. A repository
//! is someone else's input, and `gh` reads its git configuration and remotes, so the caller asks
//! only for a project the person trusts. Everything else is as strict as [`GitCmd`](super::GitCmd)
//! is:
//!
//! - **No shell.** The program is run by its path, with a fixed argument list; nothing from the
//!   repository, the branch included, is on the command line.
//! - **A scrubbed environment.** Everything is cleared and only what `gh` needs is passed on: the
//!   home folder (its config and token), the tokens and proxy settings the person's own
//!   environment has, certificates. The variables that redirect it to another repository
//!   (`GH_REPO`, `GIT_DIR`, …) and a `PATH` with a relative entry (`./bin/git` in the repository
//!   would be found first) never get through; `PATH` is rebuilt from absolute folders.
//! - **No prompts, no colour, no update notices.**
//! - **Bounded.** A deadline and a cap on the output, from the same runner as git's.
//!
//! Failures are quiet by design: `gh` missing, not signed in, a remote that isn't GitHub, no
//! network. The caller shows nothing and asks again much later ([`PrCache`]).

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use super::cmd::{GitError, Limit, run_bounded};

/// What `gh` is given to reach a host and nothing more. Lower-case proxy variables are the ones
/// curl-like programs read first.
const PASSED_ENV: [&str; 19] = [
    "HOME",
    "USER",
    "LOGNAME",
    "TMPDIR",
    "XDG_CONFIG_HOME",
    "GH_TOKEN",
    "GITHUB_TOKEN",
    "GH_ENTERPRISE_TOKEN",
    "GITHUB_ENTERPRISE_TOKEN",
    "GH_HOST",
    "GH_CONFIG_DIR",
    "HTTPS_PROXY",
    "HTTP_PROXY",
    "ALL_PROXY",
    "NO_PROXY",
    "https_proxy",
    "http_proxy",
    "no_proxy",
    "SSL_CERT_FILE",
];

/// Set on every run: no questions, no colour, no news about updates, and git's own prompts off.
const FIXED_ENV: [(&str, &str); 9] = [
    ("GH_PROMPT_DISABLED", "1"),
    ("GH_NO_UPDATE_NOTIFIER", "1"),
    ("GH_NO_EXTENSION_UPDATE_NOTIFIER", "1"),
    ("GH_SPINNER_DISABLED", "1"),
    ("NO_COLOR", "1"),
    ("GIT_TERMINAL_PROMPT", "0"),
    ("GIT_OPTIONAL_LOCKS", "0"),
    ("GH_PAGER", "cat"),
    ("LC_ALL", "C"),
];

/// The usual homes of `git` and `gh` after the folder `gh` itself is in, all absolute.
const PATH_TAIL: [&str; 6] =
    ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin", "/bin", "/usr/sbin", "/sbin"];

/// Long enough for a slow connection, short enough not to hold a worker for a stuck one.
const TIMEOUT: Duration = Duration::from_secs(20);

/// `gh pr view --json number,url,state` prints a few dozen bytes.
const MAX_OUTPUT: u64 = 64 * 1024;

/// What a lookup comes back to, after this long, whether it found a pull request or not.
pub const REFRESH_AFTER: Duration = Duration::from_secs(5 * 60);

/// After a failure (no `gh`, signed out, offline) nothing asks for this long.
pub const RETRY_AFTER: Duration = Duration::from_secs(30 * 60);

/// How many branches' answers are kept.
const KEPT: usize = 64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrState {
    Open,
    Merged,
    Closed,
}

impl PrState {
    pub fn label(self) -> &'static str {
        match self {
            Self::Open => "open",
            Self::Merged => "merged",
            Self::Closed => "closed",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PullRequest {
    pub number: u64,
    /// Where it is on the host: an `https://` address, which is what a click opens.
    pub url: String,
    pub state: PrState,
}

impl PullRequest {
    /// What the chip says.
    pub fn label(&self) -> String {
        format!("#{}", self.number)
    }

    /// For the tooltip and VoiceOver: "Pull request #142, merged".
    pub fn words(&self) -> String {
        format!("Pull request #{}, {}", self.number, self.state.label())
    }
}

/// Why there is no answer. None of them is shown: the row simply has no link.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PrError {
    /// No `gh` to run.
    GhMissing,
    /// `gh` ran and failed: signed out, not a GitHub remote, offline. What it said.
    Failed(String),
    TimedOut,
}

/// `gh pr view`'s JSON, if it is a pull request this row can show: a number, a state it knows and
/// an `https://` address.
pub fn parse(json: &[u8]) -> Option<PullRequest> {
    let value: serde_json::Value = serde_json::from_slice(json).ok()?;
    let number = value.get("number")?.as_u64()?;
    let state = match value.get("state")?.as_str()? {
        "OPEN" => PrState::Open,
        "MERGED" => PrState::Merged,
        "CLOSED" => PrState::Closed,
        _ => return None,
    };
    let url = value.get("url")?.as_str()?;
    // A click opens it: a host's own https address, nothing a program could be started with.
    let usable = url.len() <= 2048
        && url.strip_prefix("https://").is_some_and(|rest| !rest.is_empty())
        && !url.chars().any(|c| c.is_control() || c.is_whitespace());
    usable.then(|| PullRequest { number, url: url.to_string(), state })
}

/// The pull request of the branch checked out in `repo`, `Ok(None)` when there is none.
pub fn pull_request(repo: &Path) -> Result<Option<PullRequest>, PrError> {
    let gh = program().ok_or(PrError::GhMissing)?;
    let mut command = Command::new(&gh);
    command
        .current_dir(repo)
        .args(["pr", "view", "--json", "number,url,state"])
        .env_clear()
        .envs(environment(std::env::vars_os(), &gh));
    let output = match run_bounded(&mut command, TIMEOUT, Limit::Stop(MAX_OUTPUT)) {
        Ok(output) => output,
        Err(err) => {
            return Err(match GitError::of(&err) {
                // The runner's wording for a program that isn't there.
                GitError::GitMissing => PrError::GhMissing,
                GitError::TimedOut(_) => PrError::TimedOut,
                GitError::OutputTooLarge { .. } => PrError::Failed("gh printed too much".into()),
                other => PrError::Failed(other.to_string()),
            });
        }
    };
    let said = String::from_utf8_lossy(&output.stderr);
    if output.status.success() {
        return parse(&output.stdout).map(Some).ok_or_else(|| PrError::Failed("unreadable answer".into()));
    }
    // `gh pr view` fails the same way for "no pull request" and for everything else; its words
    // are the only difference.
    if said.contains("no pull requests found") {
        return Ok(None);
    }
    Err(PrError::Failed(said.lines().next().unwrap_or("gh failed").trim().to_string()))
}

/// The `gh` to run: the one `WORMHOLE_GH_BIN` names in a debug build (so a test instance never
/// reaches a host), else the first on `PATH` or in the usual spots.
fn program() -> Option<PathBuf> {
    #[cfg(test)]
    if let Some(program) = PROGRAM.with_borrow(Clone::clone) {
        return Some(program);
    }
    if cfg!(debug_assertions)
        && let Some(path) = std::env::var_os("WORMHOLE_GH_BIN")
    {
        return Some(PathBuf::from(path));
    }
    crate::agents::find_on_path("gh")
}

#[cfg(test)]
thread_local! {
    /// What runs on this thread in place of `gh` (see `with_gh`).
    static PROGRAM: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

/// The environment `gh` gets: what [`PASSED_ENV`] names from `vars`, [`FIXED_ENV`], and a `PATH`
/// of absolute folders only, starting with the one `gh` is in.
fn environment(vars: impl Iterator<Item = (OsString, OsString)>, gh: &Path) -> Vec<(OsString, OsString)> {
    let mut env: Vec<(OsString, OsString)> =
        vars.filter(|(name, _)| PASSED_ENV.iter().any(|kept| OsStr::new(kept) == name.as_os_str())).collect();
    env.extend(FIXED_ENV.iter().map(|(name, value)| (OsString::from(name), OsString::from(value))));
    let own = gh.parent().filter(|dir| dir.is_absolute()).map(Path::to_path_buf);
    let path = std::env::join_paths(own.into_iter().chain(PATH_TAIL.iter().map(PathBuf::from)));
    env.extend(path.ok().map(|path| (OsString::from("PATH"), path)));
    env
}

/// Which branch of which repository an answer is about.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Branch {
    pub root: PathBuf,
    pub name: String,
}

#[derive(Debug)]
struct Entry {
    found: Option<PullRequest>,
    /// When to ask again.
    due: Instant,
    /// A lookup is out.
    asked: bool,
}

/// What `gh` said about each branch, so a terminal asks once in a long while, not at every
/// redraw: an answer holds for [`REFRESH_AFTER`], a failure for [`RETRY_AFTER`] (the last good
/// answer stays shown meanwhile). Nothing here reads a clock or starts a lookup; the caller does.
#[derive(Debug, Default)]
pub struct PrCache {
    entries: HashMap<Branch, Entry>,
}

impl PrCache {
    /// The pull request known for the branch, if any.
    pub fn get(&self, branch: &Branch) -> Option<&PullRequest> {
        self.entries.get(branch)?.found.as_ref()
    }

    /// Whether a lookup for the branch should start now; if so it is marked as out, and
    /// [`finish`](Self::finish) ends it. At most one is out per branch.
    pub fn begin(&mut self, branch: &Branch, now: Instant) -> bool {
        match self.entries.get_mut(branch) {
            Some(entry) if entry.asked || now < entry.due => false,
            Some(entry) => {
                entry.asked = true;
                true
            }
            None => {
                self.entries.insert(branch.clone(), Entry { found: None, due: now, asked: true });
                true
            }
        }
    }

    /// A lookup's answer.
    pub fn finish(&mut self, branch: &Branch, result: Result<Option<PullRequest>, PrError>, now: Instant) {
        let entry =
            self.entries.entry(branch.clone()).or_insert(Entry { found: None, due: now, asked: false });
        entry.asked = false;
        match result {
            Ok(found) => (entry.found, entry.due) = (found, now + REFRESH_AFTER),
            // The last good answer stays; only the next try is put off.
            Err(_) => entry.due = now + RETRY_AFTER,
        }
        // Branches come and go (a feature branch a day): keep the latest few.
        while self.entries.len() > KEPT {
            let oldest = self
                .entries
                .iter()
                .filter(|(_, e)| !e.asked)
                .min_by_key(|(_, e)| e.due)
                .map(|(k, _)| k.clone());
            match oldest {
                Some(key) => self.entries.remove(&key),
                None => break,
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::Path;
    use std::time::{Duration, Instant};

    use super::{
        Branch, FIXED_ENV, PASSED_ENV, PrCache, PrError, PrState, PullRequest, REFRESH_AFTER, RETRY_AFTER,
        environment, parse, pull_request,
    };

    const OPEN: &[u8] = br#"{"number":142,"state":"OPEN","url":"https://github.com/acme/app/pull/142"}"#;

    fn pr(number: u64) -> PullRequest {
        PullRequest {
            number,
            url: format!("https://github.com/acme/app/pull/{number}"),
            state: PrState::Open,
        }
    }

    fn branch(name: &str) -> Branch {
        Branch { root: "/repo".into(), name: name.into() }
    }

    #[test]
    fn gh_pr_view_json_is_a_pull_request() {
        assert_eq!(parse(OPEN), Some(pr(142)));
        let merged =
            br#"{"url":"https://github.com/acme/app/pull/7","number":7,"state":"MERGED","title":"x"}"#;
        assert_eq!(parse(merged).map(|p| (p.number, p.state)), Some((7, PrState::Merged)));
        let closed = br#"{"number":8,"state":"CLOSED","url":"https://git.corp.example/a/b/pull/8"}"#;
        assert_eq!(parse(closed).map(|p| p.state), Some(PrState::Closed));
        assert_eq!(pr(142).label(), "#142");
        assert_eq!(pr(142).words(), "Pull request #142, open");
    }

    /// A click opens the address: only a host's own https page, and only a state it knows.
    #[test]
    fn what_cannot_be_opened_or_read_is_no_pull_request() {
        for bad in [
            &br#"{"number":1,"state":"OPEN","url":"http://github.com/a/b/pull/1"}"#[..],
            br#"{"number":1,"state":"OPEN","url":"file:///etc/passwd"}"#,
            br#"{"number":1,"state":"OPEN","url":"javascript:alert(1)"}"#,
            br#"{"number":1,"state":"OPEN","url":"https://"}"#,
            br#"{"number":1,"state":"OPEN","url":"https://github.com/a b"}"#,
            br#"{"number":1,"state":"OPEN","url":"https://github.com/a\nb"}"#,
            br#"{"number":1,"state":"DRAFT","url":"https://github.com/a/b/pull/1"}"#,
            br#"{"number":-1,"state":"OPEN","url":"https://github.com/a/b/pull/1"}"#,
            br#"{"state":"OPEN","url":"https://github.com/a/b/pull/1"}"#,
            br#"[]"#,
            b"not json",
            b"",
        ] {
            assert_eq!(parse(bad), None, "{}", String::from_utf8_lossy(bad));
        }
        let long = format!(r#"{{"number":1,"state":"OPEN","url":"https://h/{}"}}"#, "a".repeat(3000));
        assert_eq!(parse(long.as_bytes()), None);
    }

    fn vars(list: &[(&str, &str)]) -> Vec<(OsString, OsString)> {
        list.iter().map(|(k, v)| (OsString::from(k), OsString::from(v))).collect()
    }

    #[test]
    fn the_environment_is_the_persons_credentials_and_nothing_that_redirects_gh() {
        let env = environment(
            vars(&[
                ("HOME", "/Users/ada"),
                ("GH_TOKEN", "t"),
                ("HTTPS_PROXY", "http://proxy:3128"),
                ("PATH", "./bin:/usr/bin"),
                // What would send it to another repository, give it another program, or leak.
                ("GH_REPO", "evil/repo"),
                ("GIT_DIR", "/elsewhere/.git"),
                ("GIT_SSH_COMMAND", "evil"),
                ("GH_BROWSER", "evil"),
                ("AWS_SECRET_ACCESS_KEY", "s"),
                ("GH_PROMPT_DISABLED", "0"),
            ])
            .into_iter(),
            Path::new("/opt/homebrew/bin/gh"),
        );
        let get = |name: &str| -> Vec<&OsString> {
            env.iter().filter(|(k, _)| k == name).map(|(_, v)| v).collect()
        };
        assert_eq!(get("HOME"), [&OsString::from("/Users/ada")]);
        assert_eq!(get("GH_TOKEN"), [&OsString::from("t")]);
        assert_eq!(get("HTTPS_PROXY"), [&OsString::from("http://proxy:3128")]);
        for gone in ["GH_REPO", "GIT_DIR", "GIT_SSH_COMMAND", "GH_BROWSER", "AWS_SECRET_ACCESS_KEY"] {
            assert!(get(gone).is_empty(), "{gone}");
        }
        // Ours wins over the person's, and appears once.
        assert_eq!(get("GH_PROMPT_DISABLED"), [&OsString::from("1")]);
        for (name, value) in FIXED_ENV {
            assert_eq!(get(name), [&OsString::from(value)], "{name}");
        }
        // `./bin` is gone, gh's own folder leads, every entry is absolute.
        let path = get("PATH");
        assert_eq!(path.len(), 1);
        let dirs: Vec<_> = std::env::split_paths(path[0]).collect();
        assert_eq!(dirs[0], Path::new("/opt/homebrew/bin"));
        assert!(dirs.iter().all(|dir| dir.is_absolute()) && dirs.contains(&"/usr/bin".into()), "{dirs:?}");
        assert!(PASSED_ENV.iter().all(|name| !name.starts_with("GIT_")), "no git variables pass");
    }

    #[test]
    fn a_gh_given_by_a_relative_path_adds_no_relative_folder() {
        let env = environment(std::iter::empty(), Path::new("gh"));
        let path = env.iter().find(|(k, _)| k == "PATH").map(|(_, v)| v.clone()).expect("PATH");
        assert!(std::env::split_paths(&path).all(|dir| dir.is_absolute()));
    }

    // -- cache

    #[test]
    fn a_branch_is_asked_once_and_again_after_a_long_while() {
        let (mut cache, main, t0) = (PrCache::default(), branch("main"), Instant::now());
        assert!(cache.begin(&main, t0));
        assert!(!cache.begin(&main, t0), "one lookup out at a time");
        assert_eq!(cache.get(&main), None, "nothing known while it runs");
        cache.finish(&main, Ok(Some(pr(142))), t0);
        assert_eq!(cache.get(&main), Some(&pr(142)));
        assert!(!cache.begin(&main, t0 + REFRESH_AFTER - Duration::from_secs(1)));
        assert!(cache.begin(&main, t0 + REFRESH_AFTER));
        // The old answer shows while the new one is out, and a closed branch's PR goes when it is gone.
        assert_eq!(cache.get(&main), Some(&pr(142)));
        cache.finish(&main, Ok(None), t0 + REFRESH_AFTER);
        assert_eq!(cache.get(&main), None);
    }

    #[test]
    fn a_failure_puts_the_next_try_off_and_keeps_the_last_answer() {
        let (mut cache, feature, t0) = (PrCache::default(), branch("feature"), Instant::now());
        assert!(cache.begin(&feature, t0));
        cache.finish(&feature, Err(PrError::GhMissing), t0);
        assert!(
            !cache.begin(&feature, t0 + RETRY_AFTER - Duration::from_secs(1)),
            "gh missing: not asked again soon"
        );
        assert!(cache.begin(&feature, t0 + RETRY_AFTER));
        cache.finish(&feature, Ok(Some(pr(9))), t0 + RETRY_AFTER);
        let later = t0 + RETRY_AFTER + REFRESH_AFTER;
        assert!(cache.begin(&feature, later));
        cache.finish(&feature, Err(PrError::TimedOut), later);
        assert_eq!(cache.get(&feature), Some(&pr(9)), "offline for a moment: the link stays");
    }

    #[test]
    fn branches_and_repositories_are_answered_separately_and_not_kept_forever() {
        let (mut cache, t0) = (PrCache::default(), Instant::now());
        let other_repo = Branch { root: "/other".into(), name: "main".into() };
        for key in [branch("main"), other_repo.clone()] {
            assert!(cache.begin(&key, t0));
        }
        cache.finish(&branch("main"), Ok(Some(pr(1))), t0);
        assert_eq!(cache.get(&other_repo), None);
        for n in 0..200 {
            let key = branch(&format!("b{n}"));
            cache.begin(&key, t0 + Duration::from_secs(n));
            cache.finish(&key, Ok(None), t0 + Duration::from_secs(n));
        }
        assert!(cache.entries.len() <= super::KEPT);
        assert!(cache.begin(&branch("b0"), t0 + Duration::from_secs(999)), "the oldest went first");
    }

    // -- the real command, with a stand-in for gh

    #[cfg(unix)]
    mod run {
        use std::os::unix::fs::PermissionsExt;
        use std::path::{Path, PathBuf};

        use super::{PrError, PrState, pr, pull_request};

        /// A `gh` that prints `stdout` and `stderr` and exits with `code`, run on this thread
        /// instead of the real one.
        fn with_gh<T>(dir: &Path, script: &str, f: impl FnOnce() -> T) -> T {
            let path: PathBuf = dir.join("gh");
            std::fs::write(&path, format!("#!/bin/sh\n{script}\n")).expect("script");
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("chmod");
            super::super::PROGRAM.set(Some(path));
            let out = f();
            super::super::PROGRAM.set(None);
            out
        }

        #[test]
        fn a_pull_request_comes_back_and_gh_is_run_with_a_fixed_argument_list_in_the_repo() {
            let tools = tempfile::tempdir().expect("tempdir");
            let repo = tempfile::tempdir().expect("tempdir");
            let script = format!(
                "[ \"$*\" = 'pr view --json number,url,state' ] || exit 9\n\
                 [ \"$PWD\" = \"$(cd '{}' && pwd -P)\" ] || exit 8\n\
                 [ \"$GH_PROMPT_DISABLED\" = 1 ] || exit 7\n\
                 echo '{{\"number\":142,\"state\":\"OPEN\",\"url\":\"https://github.com/acme/app/pull/142\"}}'",
                repo.path().display()
            );
            let got = with_gh(tools.path(), &script, || pull_request(repo.path()));
            assert_eq!(got, Ok(Some(pr(142))));
            assert_eq!(PrState::Open.label(), "open");
        }

        #[test]
        fn no_pull_request_for_the_branch_is_none_and_every_other_failure_is_quiet() {
            let tools = tempfile::tempdir().expect("tempdir");
            let repo = tempfile::tempdir().expect("tempdir");
            let none = "echo 'no pull requests found for branch \"feature\"' >&2; exit 1";
            assert_eq!(with_gh(tools.path(), none, || pull_request(repo.path())), Ok(None));
            let signed_out = "echo 'To get started with GitHub CLI, please run:  gh auth login' >&2; exit 4";
            let got = with_gh(tools.path(), signed_out, || pull_request(repo.path()));
            assert_eq!(
                got,
                Err(PrError::Failed("To get started with GitHub CLI, please run:  gh auth login".into()))
            );
            let nonsense = "echo 'hello'";
            assert!(matches!(
                with_gh(tools.path(), nonsense, || pull_request(repo.path())),
                Err(PrError::Failed(_))
            ));
        }

        #[test]
        fn a_gh_that_is_not_there_is_missing() {
            let repo = tempfile::tempdir().expect("tempdir");
            super::super::PROGRAM.set(Some("/nonexistent/gh".into()));
            let got = pull_request(repo.path());
            super::super::PROGRAM.set(None);
            assert_eq!(got, Err(PrError::GhMissing));
        }
    }
}
