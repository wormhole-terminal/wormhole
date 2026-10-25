//! The fixtures of `probe test` (E1): `tests/*.toml`, each a list of cases that say what the Probe's
//! rules must find in one line of text (`repo-docs/docs/probes.md`, section 5).
//!
//! ```toml
//! [[case]]
//! name  = "a ticket in a sentence"      # optional
//! input = "see ACME-123 now"             # one logical line
//!
//! [[case.expect]]                        # every match, in the order they lie in the line
//! rule   = "ticket"                      # the rule's local id
//! start  = 4                             # Unicode scalar values (not bytes or graphemes), 0-based
//! end    = 12                            # exclusive
//! text   = "ACME-123"                    # optional: what the span covers
//! look   = "none"                        # optional: shorten | redact | none
//! target = "https://acme.atlassian.net/browse/ACME-123"   # optional: the action's target
//! args   = ["abc1234"]                   # optional, `command` actions: the values of its `$n`
//! ```
//!
//! A case with no `expect` says nothing matches. The matches found must be the matches expected,
//! no more and no fewer, so a test also catches a rule that has started to match too much. A field
//! that is left out is not checked. Unknown keys are errors: a typo must not make a test pass.
//!
//! Must not use anything above `rules`; it reads no file of its own (the folder was read once).

mod run;

use std::fmt;

use serde::Deserialize;

use super::manifest::Look;
use super::read::ProbeFolder;

pub use run::{CaseReport, TestReport, run};

/// The folder the fixtures are in.
pub const TESTS_DIR: &str = "tests";
/// A fixture file's extension.
const EXTENSION: &str = ".toml";

/// One case: a line of text and the matches it must give.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Case {
    pub name: Option<String>,
    /// One logical line.
    pub input: String,
    #[serde(default)]
    pub expect: Vec<Expect>,
}

/// One match a case expects.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Expect {
    /// The rule's local id.
    pub rule: String,
    /// Characters from the start of the input, 0-based.
    pub start: usize,
    /// Exclusive.
    pub end: usize,
    pub text: Option<String>,
    pub look: Option<Look>,
    /// An `open-url` link, the text a `copy` puts on the clipboard, or a `command`'s id.
    pub target: Option<String>,
    /// A `command`'s arguments that come from captures.
    pub args: Option<Vec<String>>,
}

/// A `tests/*.toml` file.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(deny_unknown_fields)]
struct Raw {
    #[serde(default)]
    case: Vec<Case>,
}

/// A fixture file read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Fixture {
    /// Relative to the Probe's folder (`tests/ticket.toml`).
    pub file: String,
    pub cases: Vec<Case>,
}

/// A fixture file that is no fixture.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FixtureError {
    pub file: String,
    pub message: String,
}

impl fmt::Display for FixtureError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.file, self.message)
    }
}

impl std::error::Error for FixtureError {}

/// Read the fixture `text`, the contents of `file`.
pub fn parse(file: &str, text: &str) -> Result<Fixture, FixtureError> {
    let raw: Raw = toml::from_str(text)
        .map_err(|err| FixtureError { file: file.to_string(), message: err.message().trim().to_string() })?;
    Ok(Fixture { file: file.to_string(), cases: raw.case })
}

/// Every `tests/*.toml` of `folder`, by name; a file that does not read is an error in its place.
pub fn fixtures(folder: &ProbeFolder) -> Vec<Result<Fixture, FixtureError>> {
    folder
        .files
        .iter()
        .filter(|file| {
            file.path.strip_prefix(TESTS_DIR).and_then(|rest| rest.strip_prefix('/')).is_some_and(|name| {
                name.ends_with(EXTENSION) && name.len() > EXTENSION.len() && !name.contains('/')
            })
        })
        .map(|file| match file.text() {
            Some(text) => parse(&file.path, text),
            None => Err(FixtureError { file: file.path.clone(), message: "is not valid UTF-8".into() }),
        })
        .collect()
}

#[cfg(test)]
mod tests;
