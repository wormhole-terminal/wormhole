//! Running a fixture's cases: the matches found against the matches expected.

use super::{Case, Expect, Fixture, FixtureError, fixtures};
use crate::probe::manifest::Look;
use crate::probe::read::ProbeFolder;
use crate::probe::rules::{CompiledRules, Match, matches, target};

/// What happened to one case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CaseReport {
    pub file: String,
    pub name: String,
    /// What was wrong; empty when the case passed.
    pub problems: Vec<String>,
}

impl CaseReport {
    pub fn passed(&self) -> bool {
        self.problems.is_empty()
    }
}

/// What happened to a Probe's fixtures.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TestReport {
    pub cases: Vec<CaseReport>,
    /// Fixture files that could not be read.
    pub unreadable: Vec<FixtureError>,
}

impl TestReport {
    pub fn failed(&self) -> usize {
        self.cases.iter().filter(|case| !case.passed()).count()
    }

    pub fn passed(&self) -> usize {
        self.cases.len() - self.failed()
    }

    /// Whether every case passed and every file read.
    pub fn is_ok(&self) -> bool {
        self.unreadable.is_empty() && self.failed() == 0
    }
}

/// Run every case of every `tests/*.toml` of `folder` over `rules`.
pub fn run(folder: &ProbeFolder, rules: &CompiledRules) -> TestReport {
    let mut report = TestReport::default();
    for fixture in fixtures(folder) {
        match fixture {
            Ok(fixture) => report.cases.extend(run_fixture(&fixture, rules)),
            Err(err) => report.unreadable.push(err),
        }
    }
    report
}

fn run_fixture(fixture: &Fixture, rules: &CompiledRules) -> Vec<CaseReport> {
    fixture
        .cases
        .iter()
        .enumerate()
        .map(|(ix, case)| CaseReport {
            file: fixture.file.clone(),
            name: case.name.clone().unwrap_or_else(|| format!("case {}", ix + 1)),
            problems: run_case(case, rules),
        })
        .collect()
}

/// A match as the case's fixture would spell it, in characters.
struct Found {
    rule: String,
    start: usize,
    end: usize,
    text: String,
    look: Look,
    target: Option<String>,
    args: Option<Vec<String>>,
}

impl Found {
    fn of(rules: &CompiledRules, line: &str, found: &Match) -> Self {
        let chars = |bytes: usize| line[..bytes].chars().count();
        let target = target(rules, found, line);
        Self {
            rule: rules.rule(found.rule).map(|rule| rule.id().to_string()).unwrap_or_default(),
            start: chars(found.start),
            end: chars(found.end),
            text: line[found.range()].to_string(),
            look: found.look,
            args: match &target {
                Some(crate::probe::rules::Target::Command { args, .. }) => Some(args.clone()),
                _ => None,
            },
            target: target.map(|target| target.text().to_string()),
        }
    }

    fn describe(&self) -> String {
        let mut text = format!(
            "rule `{}` {}..{} {:?} look {}",
            self.rule,
            self.start,
            self.end,
            self.text,
            look(self.look)
        );
        if let Some(target) = &self.target {
            text.push_str(&format!(" target {target:?}"));
        }
        if let Some(args) = self.args.as_ref().filter(|args| !args.is_empty()) {
            text.push_str(&format!(" args {args:?}"));
        }
        text
    }
}

fn look(look: Look) -> &'static str {
    match look {
        Look::Shorten => "shorten",
        Look::Redact => "redact",
        Look::None => "none",
    }
}

fn describe_expected(expect: &Expect) -> String {
    format!("rule `{}` {}..{}", expect.rule, expect.start, expect.end)
}

/// What is wrong with `case`; nothing when it passes.
fn run_case(case: &Case, rules: &CompiledRules) -> Vec<String> {
    if case.input.contains(['\n', '\r']) {
        return vec!["`input` must be one logical line: it holds a line break".into()];
    }
    let found: Vec<Found> =
        matches(rules, &case.input).iter().map(|found| Found::of(rules, &case.input, found)).collect();
    let mut problems = Vec::new();
    for ix in 0..found.len().max(case.expect.len()) {
        match (case.expect.get(ix), found.get(ix)) {
            (Some(expect), Some(got)) => compare(expect, got, ix, &mut problems),
            (Some(expect), None) => {
                problems.push(format!(
                    "match {}: expected {}, but nothing more matched",
                    ix + 1,
                    describe_expected(expect)
                ));
            }
            (None, Some(got)) => {
                problems.push(format!("match {}: not expected, but {} matched", ix + 1, got.describe()));
            }
            (None, None) => {}
        }
    }
    if !problems.is_empty() {
        if found.is_empty() {
            problems.push("found no matches".into());
        }
        problems.extend(
            found.iter().enumerate().map(|(ix, got)| format!("found {}: {}", ix + 1, got.describe())),
        );
    }
    problems
}

fn compare(expect: &Expect, got: &Found, ix: usize, problems: &mut Vec<String>) {
    let at = format!("match {}", ix + 1);
    if expect.rule != got.rule {
        problems.push(format!("{at}: expected rule `{}`, found `{}`", expect.rule, got.rule));
    }
    if (expect.start, expect.end) != (got.start, got.end) {
        problems.push(format!(
            "{at}: expected the span {}..{}, found {}..{}",
            expect.start, expect.end, got.start, got.end
        ));
    }
    if let Some(text) = &expect.text
        && *text != got.text
    {
        problems.push(format!("{at}: expected the text {text:?}, found {:?}", got.text));
    }
    if let Some(want) = expect.look
        && want != got.look
    {
        problems.push(format!("{at}: expected look {}, found {}", look(want), look(got.look)));
    }
    if let Some(want) = &expect.target
        && got.target.as_ref() != Some(want)
    {
        match &got.target {
            Some(target) => problems.push(format!("{at}: expected the target {want:?}, found {target:?}")),
            None => {
                problems.push(format!("{at}: expected the target {want:?}, but this match has no target"))
            }
        }
    }
    if let Some(want) = &expect.args
        && got.args.as_ref() != Some(want)
    {
        problems.push(format!(
            "{at}: expected the arguments {want:?}, found {:?}",
            got.args.clone().unwrap_or_default()
        ));
    }
}
