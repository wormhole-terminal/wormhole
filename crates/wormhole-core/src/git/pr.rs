//! The pull request of the branch a terminal is on (the title row's PR link and the mark on its
//! folder's band), asked of the GitHub CLI: `gh pr view --json
//! number,url,state,isDraft,reviewDecision,statusCheckRollup` in the repository. Besides the
//! number and whether it is open it says how its checks stand and what its reviewers decided.
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
use std::time::{Duration, Instant};

use super::cmd::{GitError, Limit, run_bounded};
use crate::platform::process;

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

/// What `gh pr view` is asked for.
const FIELDS: &str = "number,url,state,isDraft,reviewDecision,statusCheckRollup";

/// A pull request's JSON is a few hundred bytes, plus a few hundred for each of its checks: room
/// for a repository with hundreds of them.
const MAX_OUTPUT: u64 = 512 * 1024;

/// What a lookup comes back to, after this long, whether it found a pull request or not.
pub const REFRESH_AFTER: Duration = Duration::from_secs(5 * 60);

/// While its checks run, a pull request is asked about again this soon, so the mark turns green
/// or red about when the checks finish.
pub const PENDING_REFRESH: Duration = Duration::from_secs(60);

/// How long to wait after the `n`th answer in a row that said the checks still run: a minute,
/// then twice as long each time up to [`REFRESH_AFTER`], so a check that never reports (a required
/// status nothing sends) doesn't have `gh` asked every minute for ever.
fn pending_wait(n: u32) -> Duration {
    let doubled = PENDING_REFRESH.saturating_mul(1 << n.saturating_sub(1).min(16));
    doubled.min(REFRESH_AFTER)
}

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

/// How a pull request's checks (CI runs and commit statuses together) stand.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Checks {
    /// Some are still queued or running, and none has failed.
    Pending,
    /// At least one failed, timed out or was cancelled.
    Failing,
    /// Every one finished without failing.
    Passing,
}

impl Checks {
    pub fn words(self) -> &'static str {
        match self {
            Self::Pending => "checks running",
            Self::Failing => "checks failing",
            Self::Passing => "checks passing",
        }
    }

    /// The checks of `gh`'s `statusCheckRollup`: GitHub Actions runs (`status` and `conclusion`)
    /// and commit statuses (`state`) alike. `None` without any check it can read.
    fn of(rollup: &serde_json::Value) -> Option<Self> {
        let mut seen = None;
        for check in rollup.as_array()? {
            let field = |name: &str| check.get(name).and_then(|v| v.as_str()).unwrap_or_default();
            let this = match (field("status"), field("conclusion"), field("state")) {
                (_, _, "PENDING" | "EXPECTED") => Self::Pending,
                (_, _, "FAILURE" | "ERROR") => Self::Failing,
                (_, _, "SUCCESS") => Self::Passing,
                (_, "FAILURE" | "TIMED_OUT" | "CANCELLED" | "ACTION_REQUIRED" | "STARTUP_FAILURE", _) => {
                    Self::Failing
                }
                ("COMPLETED", _, _) => Self::Passing,
                ("QUEUED" | "IN_PROGRESS" | "WAITING" | "PENDING" | "REQUESTED", _, _) => Self::Pending,
                _ => continue,
            };
            // One failure fails them all; one still running keeps the rest from passing.
            seen = Some(match (seen, this) {
                (Some(Self::Failing), _) | (_, Self::Failing) => Self::Failing,
                (Some(Self::Pending), _) | (_, Self::Pending) => Self::Pending,
                _ => Self::Passing,
            });
        }
        seen
    }
}

/// What the reviewers decided, where the repository asks for reviews.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Review {
    Approved,
    ChangesRequested,
    /// Waiting for a review it needs.
    Required,
}

impl Review {
    pub fn words(self) -> &'static str {
        match self {
            Self::Approved => "approved",
            Self::ChangesRequested => "changes requested",
            Self::Required => "review required",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PullRequest {
    pub number: u64,
    /// Where it is on the host: an `https://` address, which is what a click opens.
    pub url: String,
    pub state: PrState,
    /// Opened as a draft, not ready for review yet.
    pub draft: bool,
    pub checks: Option<Checks>,
    pub review: Option<Review>,
}

impl PullRequest {
    /// What the chip says.
    pub fn label(&self) -> String {
        format!("#{}", self.number)
    }

    /// For the tooltip and VoiceOver: "Pull request #142, open, checks failing, changes
    /// requested". Checks and reviews are only said of an open one.
    pub fn words(&self) -> String {
        let mut words = format!("Pull request #{}, {}", self.number, self.state.label());
        if self.state == PrState::Open {
            let parts = [
                self.draft.then_some("draft"),
                self.checks.map(Checks::words),
                self.review.map(Review::words),
            ];
            for part in parts.into_iter().flatten() {
                words.push_str(", ");
                words.push_str(part);
            }
        }
        words
    }

    /// The checks worth a mark: an open pull request's.
    pub fn open_checks(&self) -> Option<Checks> {
        self.checks.filter(|_| self.state == PrState::Open)
    }
}

/// A pull request's checks finished: they were running at the last look and now passed or failed.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Settled {
    pub pull_request: PullRequest,
    pub checks: Checks,
}

impl Settled {
    /// The notification's title and body: "Checks failed on #142", "feature/auth".
    pub fn notification(&self, branch: &str) -> (String, String) {
        let how = if self.checks == Checks::Failing { "failed" } else { "passed" };
        (format!("Checks {how} on {}", self.pull_request.label()), branch.to_string())
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
    let review = match value.get("reviewDecision").and_then(|v| v.as_str()) {
        Some("APPROVED") => Some(Review::Approved),
        Some("CHANGES_REQUESTED") => Some(Review::ChangesRequested),
        Some("REVIEW_REQUIRED") => Some(Review::Required),
        _ => None,
    };
    usable.then(|| PullRequest {
        number,
        url: url.to_string(),
        state,
        draft: value.get("isDraft").and_then(|v| v.as_bool()).unwrap_or(false),
        checks: value.get("statusCheckRollup").and_then(Checks::of),
        review,
    })
}

/// The pull request of the branch checked out in `repo`, `Ok(None)` when there is none.
pub fn pull_request(repo: &Path) -> Result<Option<PullRequest>, PrError> {
    let gh = program().ok_or(PrError::GhMissing)?;
    let mut command = process::command(&gh);
    command
        .current_dir(repo)
        .args(["pr", "view", "--json", FIELDS])
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
    /// How many answers in a row said its checks were still running.
    pending: u32,
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
                self.entries.insert(branch.clone(), Entry { found: None, due: now, asked: true, pending: 0 });
                true
            }
        }
    }

    /// A lookup's answer, and whether it says the pull request's checks have just finished.
    pub fn finish(
        &mut self,
        branch: &Branch,
        result: Result<Option<PullRequest>, PrError>,
        now: Instant,
    ) -> Option<Settled> {
        let entry = self.entries.entry(branch.clone()).or_insert(Entry {
            found: None,
            due: now,
            asked: false,
            pending: 0,
        });
        entry.asked = false;
        let mut settled = None;
        match result {
            Ok(found) => {
                // Only the same pull request's checks finishing is news; another one on the
                // branch is not.
                let was_running = |pr: &PullRequest| {
                    entry.found.as_ref().is_some_and(|old| {
                        old.number == pr.number && old.open_checks() == Some(Checks::Pending)
                    })
                };
                settled = found.as_ref().filter(|pr| was_running(pr)).and_then(|pr| match pr.open_checks() {
                    Some(checks @ (Checks::Passing | Checks::Failing)) => {
                        Some(Settled { pull_request: pr.clone(), checks })
                    }
                    _ => None,
                });
                let running = found.as_ref().is_some_and(|pr| pr.open_checks() == Some(Checks::Pending));
                entry.pending = if running { entry.pending.saturating_add(1) } else { 0 };
                entry.due = now + if running { pending_wait(entry.pending) } else { REFRESH_AFTER };
                entry.found = found;
            }
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
        settled
    }
}

#[cfg(test)]
mod tests {
    use std::ffi::OsString;
    use std::path::Path;
    use std::time::{Duration, Instant};

    use super::{
        Branch, Checks, FIXED_ENV, PASSED_ENV, PENDING_REFRESH, PrCache, PrError, PrState, PullRequest,
        REFRESH_AFTER, RETRY_AFTER, Review, Settled, environment, parse, pending_wait, pull_request,
    };

    const OPEN: &[u8] = br#"{"number":142,"state":"OPEN","url":"https://github.com/acme/app/pull/142"}"#;

    fn pr(number: u64) -> PullRequest {
        PullRequest {
            number,
            url: format!("https://github.com/acme/app/pull/{number}"),
            state: PrState::Open,
            draft: false,
            checks: None,
            review: None,
        }
    }

    fn with_checks(number: u64, checks: Checks) -> PullRequest {
        PullRequest { checks: Some(checks), ..pr(number) }
    }

    #[test]
    fn checks_and_reviews_are_read_from_the_rollup_and_the_decision() {
        let json = |rollup: &str, extra: &str| {
            format!(
                r#"{{"number":5,"state":"OPEN","url":"https://github.com/a/b/pull/5","statusCheckRollup":{rollup}{extra}}}"#
            )
        };
        let checks = |rollup: &str| parse(json(rollup, "").as_bytes()).and_then(|pr| pr.checks);
        let passed = r#"{"__typename":"CheckRun","status":"COMPLETED","conclusion":"SUCCESS"}"#;
        let skipped = r#"{"__typename":"CheckRun","status":"COMPLETED","conclusion":"SKIPPED"}"#;
        let running = r#"{"__typename":"CheckRun","status":"IN_PROGRESS","conclusion":""}"#;
        let failed = r#"{"__typename":"CheckRun","status":"COMPLETED","conclusion":"FAILURE"}"#;
        let status_ok = r#"{"__typename":"StatusContext","state":"SUCCESS"}"#;
        let status_wait = r#"{"__typename":"StatusContext","state":"PENDING"}"#;
        let status_err = r#"{"__typename":"StatusContext","state":"ERROR"}"#;
        assert_eq!(checks("[]"), None);
        assert_eq!(checks(r#"[{"name":"x"}]"#), None, "nothing it can read");
        assert_eq!(checks(&format!("[{passed},{skipped},{status_ok}]")), Some(Checks::Passing));
        assert_eq!(checks(&format!("[{passed},{running}]")), Some(Checks::Pending));
        assert_eq!(checks(&format!("[{status_wait},{passed}]")), Some(Checks::Pending));
        assert_eq!(checks(&format!("[{running},{failed},{passed}]")), Some(Checks::Failing));
        assert_eq!(checks(&format!("[{status_err}]")), Some(Checks::Failing));
        assert_eq!(parse(json("null", "").as_bytes()).map(|pr| pr.checks), Some(None));

        let full = json(&format!("[{failed}]"), r#","isDraft":true,"reviewDecision":"CHANGES_REQUESTED""#);
        let got = parse(full.as_bytes()).expect("pull request");
        assert_eq!((got.draft, got.review), (true, Some(Review::ChangesRequested)));
        assert_eq!(got.words(), "Pull request #5, open, draft, checks failing, changes requested");
        let approved = parse(json("[]", r#","reviewDecision":"APPROVED""#).as_bytes()).expect("pr");
        assert_eq!(approved.review, Some(Review::Approved));
        assert_eq!(parse(json("[]", r#","reviewDecision":"" "#).as_bytes()).expect("pr").review, None);
        // A merged one says only that, and has no checks to mark.
        let merged = PullRequest { state: PrState::Merged, ..with_checks(7, Checks::Failing) };
        assert_eq!(merged.words(), "Pull request #7, merged");
        assert_eq!(merged.open_checks(), None);
    }

    #[test]
    fn checks_that_keep_running_are_asked_about_less_and_less_often() {
        assert_eq!(pending_wait(1), PENDING_REFRESH);
        assert_eq!(pending_wait(2), PENDING_REFRESH * 2);
        assert_eq!(pending_wait(3), PENDING_REFRESH * 4);
        assert_eq!(pending_wait(30), REFRESH_AFTER, "never longer than a finished one's wait");
        let (mut cache, main, mut now) = (PrCache::default(), branch("main"), Instant::now());
        let mut waits = Vec::new();
        for _ in 0..5 {
            assert!(cache.begin(&main, now));
            cache.finish(&main, Ok(Some(with_checks(9, Checks::Pending))), now);
            let mut wait = Duration::ZERO;
            while !cache.begin(&main, now + wait) {
                wait += Duration::from_secs(30);
            }
            // `begin` marked it as out: answer as the lookup would, then go on from there.
            cache.finish(&main, Ok(Some(with_checks(9, Checks::Pending))), now + wait);
            waits.push(wait);
            now += wait + REFRESH_AFTER;
        }
        assert!(waits.windows(2).all(|w| w[0] <= w[1]), "{waits:?}");
        // Done running: back to the plain wait, and the next run starts at a minute again.
        cache.finish(&main, Ok(Some(with_checks(9, Checks::Passing))), now);
        assert!(cache.begin(&main, now + REFRESH_AFTER));
        cache.finish(&main, Ok(Some(with_checks(9, Checks::Pending))), now + REFRESH_AFTER);
        assert!(cache.begin(&main, now + REFRESH_AFTER + PENDING_REFRESH));
    }

    #[test]
    fn running_checks_are_looked_at_again_soon_and_their_end_is_told_once() {
        let (mut cache, main, t0) = (PrCache::default(), branch("main"), Instant::now());
        assert!(cache.begin(&main, t0));
        assert_eq!(cache.finish(&main, Ok(Some(with_checks(9, Checks::Pending))), t0), None);
        assert!(!cache.begin(&main, t0 + PENDING_REFRESH - Duration::from_secs(1)));
        let t1 = t0 + PENDING_REFRESH;
        assert!(cache.begin(&main, t1));
        assert_eq!(
            cache.finish(&main, Ok(Some(with_checks(9, Checks::Failing))), t1),
            Some(Settled { pull_request: with_checks(9, Checks::Failing), checks: Checks::Failing })
        );
        let told = cache.get(&main).cloned().map(|pr| Settled { pull_request: pr, checks: Checks::Failing });
        assert_eq!(
            told.map(|settled| settled.notification("main")),
            Some(("Checks failed on #9".into(), "main".into()))
        );
        let passed = Settled { pull_request: pr(9), checks: Checks::Passing };
        assert_eq!(passed.notification("main").0, "Checks passed on #9");
        // Finished: back to the long wait, and nothing more to tell.
        assert!(!cache.begin(&main, t1 + PENDING_REFRESH));
        let t2 = t1 + REFRESH_AFTER;
        assert!(cache.begin(&main, t2));
        assert_eq!(cache.finish(&main, Ok(Some(with_checks(9, Checks::Failing))), t2), None);
        // Another pull request on the branch is news of its own, not the end of the first one's
        // checks.
        let (feature, t3) = (branch("feature"), t2 + REFRESH_AFTER);
        assert!(cache.begin(&feature, t3));
        cache.finish(&feature, Ok(Some(with_checks(3, Checks::Pending))), t3);
        assert!(cache.begin(&feature, t3 + PENDING_REFRESH));
        assert_eq!(
            cache.finish(&feature, Ok(Some(with_checks(4, Checks::Passing))), t3 + PENDING_REFRESH),
            None
        );
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
    mod run {
        use std::path::{Path, PathBuf};

        use super::{PrError, PrState, pr, pull_request};

        /// A `gh` that prints `stdout` and `stderr` and exits with `code`, run on this thread
        /// instead of the real one.
        fn with_gh<T>(dir: &Path, script: &str, f: impl FnOnce() -> T) -> T {
            let path: PathBuf = dir.join("gh");
            std::fs::write(&path, format!("#!/bin/sh\n{script}\n")).expect("script");
            std::fs::set_permissions(&path, crate::platform::fs::permissions(0o755)).expect("chmod");
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
                "[ \"$*\" = 'pr view --json number,url,state,isDraft,reviewDecision,statusCheckRollup' ] || exit 9\n\
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
