//! `wait` and `run` from the command line: what they ask (the specs), the plans the app checks them
//! into (patterns compiled within limits, timeouts within caps), and what they answer.
//!
//! Both are answered on a stream thread of their own, once, when the condition is met or the time is
//! up (`ipc::wait_job`): the socket thread that serves the hooks never waits for either. A request is
//! hostile input like any other: a pattern is capped in size and compiled by an engine without
//! backtracking within a memory limit, a timeout has a ceiling, and a command is one short line of
//! printable characters.
//!
//! `wait` takes several conditions and ends at the first that holds (`met` says which), so a caller can
//! wait for "the prompt came back, or it needs me". `run` types a command at a shell's prompt, which
//! `control_write` allows, and waits for its block to finish.

use std::time::Duration;

use regex::{Regex, RegexBuilder};
use serde::{Deserialize, Serialize};

use super::event::{Topic, Topics};
use super::keys::text_problem;
use super::structure::COMMAND_MAX_BYTES;
use super::{Error, ErrorCode, Target, TerminalState};

/// The longest pattern `--text` and `--gone` take.
pub const PATTERN_MAX_BYTES: usize = 1024;
/// The memory a compiled pattern may take, and its lazy automaton while it matches.
const REGEX_BYTES: usize = 1 << 20;

/// How long `wait` waits without `--timeout`, and the longest it may be told to.
pub const WAIT_TIMEOUT_DEFAULT: Duration = Duration::from_secs(30);
pub const WAIT_TIMEOUT_MAX: Duration = Duration::from_secs(600);
/// The longest quiet `--stable` may ask for.
pub const STABLE_MAX: Duration = Duration::from_secs(60);
/// How long `run` waits without `--timeout`, and the longest it may be told to.
pub const RUN_TIMEOUT_DEFAULT: Duration = Duration::from_secs(600);
pub const RUN_TIMEOUT_MAX: Duration = Duration::from_secs(3600);
/// The most of a command's output `run` returns (its newest bytes).
pub const RUN_OUTPUT_BYTES: usize = 256 * 1024;
/// The most the reply's `matched` repeats of what a pattern found.
pub const MATCHED_MAX_BYTES: usize = 1024;

/// `wait`: block until a condition holds. The first of the conditions given that does ends it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WaitSpec {
    pub target: Target,
    /// The terminal's state is one of these.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub status: Vec<TerminalState>,
    /// The regular expression matches the live screen (rows joined by newlines; soft-wrapped rows are one
    /// line). Needs the right to read the terminal.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// The regular expression does not match the screen (any more). Needs the right to read it too.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gone: Option<String>,
    /// Nothing was printed for this long: for a full-screen program that has no prompt to wait for.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub stable_ms: Option<u64>,
    /// A command finishes: the running one, or else the next to run.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub block_done: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

/// `run`: type a command at the shell's prompt and wait for it to finish.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunSpec {
    pub target: Target,
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub timeout_ms: Option<u64>,
}

/// A [`WaitSpec`] that passed: what to wait for, ready to evaluate.
#[derive(Debug, Clone)]
pub struct WaitPlan {
    pub statuses: Vec<TerminalState>,
    pub text: Option<Regex>,
    pub gone: Option<Regex>,
    pub stable: Option<Duration>,
    pub block_done: bool,
    pub timeout: Duration,
}

/// A [`RunSpec`] that passed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunPlan {
    pub command: String,
    pub timeout: Duration,
}

impl WaitSpec {
    /// The request as a plan, or what is wrong with it: no condition, a pattern too long or too big, a
    /// timeout past its ceiling.
    pub fn plan(&self) -> Result<WaitPlan, Error> {
        let bad = |message: String| Error::new(ErrorCode::BadRequest, message);
        if self.status.is_empty()
            && self.text.is_none()
            && self.gone.is_none()
            && self.stable_ms.is_none()
            && !self.block_done
        {
            return Err(bad(
                "wait needs a condition: --status, --text, --gone, --stable or --block-done".into()
            ));
        }
        let mut statuses = Vec::new();
        for status in &self.status {
            if *status == TerminalState::Other {
                return Err(
                    bad("--status takes idle, working, needs_you, finished, failed or exited".into()),
                );
            }
            if !statuses.contains(status) {
                statuses.push(*status);
            }
        }
        let stable = match self.stable_ms {
            None => None,
            Some(ms) if (1..=STABLE_MAX.as_millis() as u64).contains(&ms) => Some(Duration::from_millis(ms)),
            Some(_) => {
                return Err(bad(format!("--stable takes 1 to {} milliseconds", STABLE_MAX.as_millis())));
            }
        };
        Ok(WaitPlan {
            statuses,
            text: self.text.as_deref().map(|pattern| compile("--text", pattern)).transpose()?,
            gone: self.gone.as_deref().map(|pattern| compile("--gone", pattern)).transpose()?,
            stable,
            block_done: self.block_done,
            timeout: timeout_of(self.timeout_ms, WAIT_TIMEOUT_DEFAULT, WAIT_TIMEOUT_MAX)?,
        })
    }

    /// How long the caller should be ready to wait for the answer: the timeout, or the default when the
    /// request is not one that passes (it is answered at once then).
    pub fn wait_for(&self) -> Duration {
        timeout_of(self.timeout_ms, WAIT_TIMEOUT_DEFAULT, WAIT_TIMEOUT_MAX).unwrap_or(WAIT_TIMEOUT_DEFAULT)
    }
}

impl RunSpec {
    /// The request as a plan, or what is wrong with it: a command that is empty, over one line, over the
    /// cap or not printable, a timeout past its ceiling.
    pub fn plan(&self) -> Result<RunPlan, Error> {
        let command = self.command.trim();
        if command.is_empty() {
            return Err(Error::new(ErrorCode::BadRequest, "run needs a command to run"));
        }
        if command.len() > COMMAND_MAX_BYTES {
            return Err(Error::new(
                ErrorCode::TooLarge,
                format!("run takes a command of at most {COMMAND_MAX_BYTES} bytes, not {}", command.len()),
            ));
        }
        if command.contains(['\n', '\r']) {
            return Err(Error::new(ErrorCode::BadRequest, "run takes one line"));
        }
        if let Some(problem) = text_problem(command, false) {
            return Err(Error::new(ErrorCode::BadRequest, problem));
        }
        Ok(RunPlan {
            command: command.to_string(),
            timeout: timeout_of(self.timeout_ms, RUN_TIMEOUT_DEFAULT, RUN_TIMEOUT_MAX)?,
        })
    }

    /// How long the caller should be ready to wait for the answer.
    pub fn wait_for(&self) -> Duration {
        timeout_of(self.timeout_ms, RUN_TIMEOUT_DEFAULT, RUN_TIMEOUT_MAX).unwrap_or(RUN_TIMEOUT_DEFAULT)
    }
}

impl WaitPlan {
    /// Whether it looks at the screen's text, which only a caller who may read the terminal gets to.
    pub fn needs_read(&self) -> bool {
        self.text.is_some() || self.gone.is_some()
    }

    /// Whether anything in it depends on what the terminal prints.
    pub fn watches_screen(&self) -> bool {
        self.needs_read() || self.stable.is_some()
    }

    /// The topics of the events it needs: the terminal ending always (closed, or its shell exited), and
    /// the commands' when it waits for one.
    pub fn topics(&self) -> Topics {
        let mut topics = Topics::default().with(Topic::Terminals).with(Topic::Status);
        if self.block_done {
            topics = topics.with(Topic::Blocks);
        }
        topics
    }
}

/// The topics a `run` needs: its block starting and finishing, the shell ending, the terminal closing.
pub fn run_topics() -> Topics {
    Topics::default().with(Topic::Blocks).with(Topic::Status).with(Topic::Terminals)
}

/// `pattern` compiled for matching terminal text: lines are matched one at a time (`^` and `$` are a
/// line's ends), within [`PATTERN_MAX_BYTES`] and a memory limit. The engine does not backtrack, so a
/// match takes time in proportion to the text, whatever the pattern.
pub fn compile(flag: &str, pattern: &str) -> Result<Regex, Error> {
    if pattern.is_empty() {
        return Err(Error::new(ErrorCode::BadRequest, format!("{flag} takes a pattern, not an empty one")));
    }
    if pattern.len() > PATTERN_MAX_BYTES {
        return Err(Error::new(
            ErrorCode::TooLarge,
            format!("{flag} takes a pattern of at most {PATTERN_MAX_BYTES} bytes, not {}", pattern.len()),
        ));
    }
    RegexBuilder::new(pattern)
        .multi_line(true)
        .size_limit(REGEX_BYTES)
        .dfa_size_limit(REGEX_BYTES)
        .build()
        .map_err(|problem| {
            // The library's message is about the pattern, which is the caller's own text: it is cut to
            // its first line (the rest is a copy of the pattern with a marker) and not repeated whole.
            let first = problem.to_string();
            let first = first.lines().last().unwrap_or("it can't be compiled").trim().to_string();
            Error::new(ErrorCode::BadRequest, format!("{flag}: {first}"))
        })
}

fn timeout_of(asked_ms: Option<u64>, default: Duration, max: Duration) -> Result<Duration, Error> {
    let Some(ms) = asked_ms else { return Ok(default) };
    let timeout = Duration::from_millis(ms);
    if ms == 0 || timeout > max {
        return Err(Error::new(
            ErrorCode::BadRequest,
            format!("--timeout takes 1 to {} seconds", max.as_secs()),
        ));
    }
    Ok(timeout)
}

/// Which condition ended a `wait`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Met {
    Status,
    Text,
    Gone,
    Stable,
    BlockDone,
    /// A condition a newer Wormhole has.
    #[serde(other)]
    Other,
}

impl Met {
    pub fn word(self) -> &'static str {
        match self {
            Met::Status => "status",
            Met::Text => "text",
            Met::Gone => "gone",
            Met::Stable => "stable",
            Met::BlockDone => "block done",
            Met::Other => "a condition",
        }
    }
}

/// `wait`'s result.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Waited {
    pub terminal: i64,
    pub met: Met,
    /// How long it waited.
    pub after_ms: u64,
    /// The state, for `status`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub status: Option<TerminalState>,
    /// What the pattern found, for `text`: terminal text, cut to [`MATCHED_MAX_BYTES`].
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched: Option<String>,
    /// The command that finished, for `block_done`, and how it ended.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub block: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// The terminal's output revision when it was met (compare two to tell whether it printed between).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub revision: Option<u64>,
}

/// `run`'s result. The command's own exit code is here; the reply is `ok` whatever it was, so a
/// refusal and a failing test never look alike (`--exit-code` makes the command line exit with it).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Ran {
    pub terminal: i64,
    pub block: i64,
    /// The shell's exit code for it; none when the shell gave none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
    /// How long it ran, as the shell's marks time it.
    pub duration_ms: u64,
    /// What it printed, its newest [`RUN_OUTPUT_BYTES`]; absent when the caller may not read the
    /// terminal (`output_hidden` says why).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output: Option<String>,
    /// Only the newest of the output is here.
    #[serde(default)]
    pub truncated: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub output_hidden: Option<String>,
}

/// What `output_hidden` says when `control_read` is off.
pub const OUTPUT_HIDDEN_OFF: &str = "control_read is off";
/// What it says when the output has left the history.
pub const OUTPUT_HIDDEN_GONE: &str = "the output has left the history";

/// `matched`, cut to its cap at a character boundary.
pub fn matched_text(found: &str) -> String {
    if found.len() <= MATCHED_MAX_BYTES {
        return found.to_string();
    }
    let mut end = MATCHED_MAX_BYTES;
    while !found.is_char_boundary(end) {
        end -= 1;
    }
    found[..end].to_string()
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::{
        PATTERN_MAX_BYTES, RUN_TIMEOUT_DEFAULT, RunSpec, STABLE_MAX, WAIT_TIMEOUT_DEFAULT, WAIT_TIMEOUT_MAX,
        WaitSpec, compile, matched_text,
    };
    use crate::ipc::control::event::Topic;
    use crate::ipc::control::{ErrorCode, Target, TerminalState};

    fn wait() -> WaitSpec {
        WaitSpec {
            target: Target::Terminal(3),
            status: Vec::new(),
            text: None,
            gone: None,
            stable_ms: None,
            block_done: false,
            timeout_ms: None,
        }
    }

    fn code<T>(result: Result<T, crate::ipc::control::Error>) -> Option<ErrorCode> {
        result.err().map(|error| error.code)
    }

    #[test]
    fn a_wait_needs_a_condition_and_defaults_its_timeout() {
        assert_eq!(code(wait().plan()), Some(ErrorCode::BadRequest));
        let plan = WaitSpec { block_done: true, ..wait() }.plan().expect("a plan");
        assert_eq!(plan.timeout, WAIT_TIMEOUT_DEFAULT);
        assert!(plan.block_done && !plan.watches_screen() && !plan.needs_read());
    }

    #[test]
    fn what_a_plan_looks_at_decides_the_topics_and_the_right_to_read() {
        let status = WaitSpec { status: vec![TerminalState::Idle, TerminalState::Idle], ..wait() }
            .plan()
            .expect("a plan");
        assert_eq!(status.statuses, [TerminalState::Idle], "said twice, kept once");
        assert!(status.topics().contains(Topic::Status) && status.topics().contains(Topic::Terminals));
        assert!(!status.topics().contains(Topic::Blocks) && !status.watches_screen());
        let done = WaitSpec { block_done: true, ..wait() }.plan().expect("a plan");
        assert!(done.topics().contains(Topic::Blocks), "a command finishing is a block event");
        let text = WaitSpec { text: Some("ok$".into()), ..wait() }.plan().expect("a plan");
        assert!(text.needs_read() && text.watches_screen());
        let stable = WaitSpec { stable_ms: Some(500), ..wait() }.plan().expect("a plan");
        assert!(stable.watches_screen() && !stable.needs_read(), "quiet needs no reading");
        assert_eq!(stable.stable, Some(Duration::from_millis(500)));
    }

    #[test]
    fn the_limits_hold() {
        let ms = |s: u64| Some(s * 1000);
        assert!(WaitSpec { block_done: true, timeout_ms: ms(600), ..wait() }.plan().is_ok());
        assert_eq!(
            code(WaitSpec { block_done: true, timeout_ms: ms(601), ..wait() }.plan()),
            Some(ErrorCode::BadRequest)
        );
        assert_eq!(
            code(WaitSpec { block_done: true, timeout_ms: Some(0), ..wait() }.plan()),
            Some(ErrorCode::BadRequest)
        );
        assert!(WaitSpec { stable_ms: Some(STABLE_MAX.as_millis() as u64), ..wait() }.plan().is_ok());
        for stable in [0, STABLE_MAX.as_millis() as u64 + 1] {
            assert_eq!(
                code(WaitSpec { stable_ms: Some(stable), ..wait() }.plan()),
                Some(ErrorCode::BadRequest)
            );
        }
        assert_eq!(
            code(WaitSpec { status: vec![TerminalState::Other], ..wait() }.plan()),
            Some(ErrorCode::BadRequest)
        );
        assert_eq!(
            WaitSpec { timeout_ms: ms(9999), ..wait() }.wait_for(),
            WAIT_TIMEOUT_DEFAULT,
            "refused anyway"
        );
        assert_eq!(WaitSpec { timeout_ms: ms(90), ..wait() }.wait_for(), Duration::from_secs(90));
        assert_eq!(WAIT_TIMEOUT_MAX, Duration::from_secs(600));
    }

    #[test]
    fn a_pattern_is_capped_and_compiled_within_limits() {
        assert_eq!(code(compile("--text", "")), Some(ErrorCode::BadRequest));
        let at_cap = "a".repeat(PATTERN_MAX_BYTES);
        assert!(compile("--text", &at_cap).is_ok());
        assert_eq!(code(compile("--text", &format!("{at_cap}a"))), Some(ErrorCode::TooLarge));
        // Not a pattern at all: refused with words, without repeating the pattern whole.
        let broken = compile("--text", "foo(").expect_err("refused");
        assert_eq!(broken.code, ErrorCode::BadRequest);
        assert!(
            broken.message.starts_with("--text: ") && !broken.message.contains('\n'),
            "{}",
            broken.message
        );
        // Small as text and enormous as an automaton: the memory limit refuses it.
        assert_eq!(code(compile("--gone", "(((a{50}){50}){50}){50}")), Some(ErrorCode::BadRequest));
        assert_eq!(code(compile("--gone", r"\p{L}{1000}\p{L}{1000}")), Some(ErrorCode::BadRequest));
    }

    #[test]
    fn lines_are_matched_one_at_a_time_and_a_hostile_pattern_cannot_take_long() {
        let found = compile("--text", r"^\$ $").expect("compiles");
        assert!(found.is_match("build ok\n$ "), "the prompt's line, among others");
        assert!(!found.is_match("echo $ x"));
        // Nested repetition would take exponential time with backtracking; this engine takes linear.
        let hostile = compile("--text", "(a+)+$").expect("compiles");
        let text = format!("{}!", "a".repeat(200_000));
        let start = std::time::Instant::now();
        assert!(!hostile.is_match(&text));
        assert!(start.elapsed() < Duration::from_secs(5), "{:?}", start.elapsed());
    }

    #[test]
    fn a_command_is_one_short_printable_line() {
        let run = |command: &str, timeout_ms| RunSpec {
            target: Target::Terminal(3),
            command: command.into(),
            timeout_ms,
        };
        assert_eq!(run("  cargo test  ", None).plan().expect("a plan").command, "cargo test");
        assert_eq!(run("cargo test", None).plan().expect("a plan").timeout, RUN_TIMEOUT_DEFAULT);
        for bad in ["", "   ", "a\nb", "a\rb", "a\u{1b}[31mb", "a\u{202e}b", "a\tb"] {
            assert_eq!(code(run(bad, None).plan()), Some(ErrorCode::BadRequest), "{bad:?}");
        }
        let cap = super::COMMAND_MAX_BYTES;
        assert!(run(&"x".repeat(cap), None).plan().is_ok());
        assert_eq!(code(run(&"x".repeat(cap + 1), None).plan()), Some(ErrorCode::TooLarge));
        assert!(run("x", Some(3_600_000)).plan().is_ok());
        assert_eq!(code(run("x", Some(3_600_001)).plan()), Some(ErrorCode::BadRequest));
    }

    #[test]
    fn what_a_pattern_found_is_cut_at_a_character() {
        assert_eq!(matched_text("short"), "short");
        let long = "é".repeat(2000);
        let cut = matched_text(&long);
        assert!(cut.len() <= super::MATCHED_MAX_BYTES && cut.chars().all(|c| c == 'é'));
    }
}
