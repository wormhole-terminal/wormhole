//! Running the conformance cases on every engine and putting the results side by side.
//!
//! A case ends up "pass", "fail" or "n/a" on each engine ([`Outcome`]); [`Matrix`] holds one row
//! per case with an outcome per engine, and answers the questions the report asks: passes per
//! area, who fails what, where the engines disagree, and which of our `convention` cases the
//! majority of engines fail (a convention is a choice, so those are the ones to look at again).

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::sync::{Mutex, PoisonError};

use wormhole_vt_bench::Engine;
use wormhole_vt_bench::engines::EngineVisitor;
use wormhole_vt_bench::vtt::{self, Basis, Case, CaseResult};

use crate::caps::{caps_of, not_applicable};

/// How one case went on one engine.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Pass,
    Fail(Failure),
    /// The adapter cannot answer a question the case asks; the reason is short.
    Na(&'static str),
}

/// Why a case failed: enough to find the step, not the whole diff (`vtbench`'s conformance
/// test prints that).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    /// The kind of step that differed: `screen`, `cursor`, `reply`, `cell`, ... `panic` when the
    /// engine crashed.
    pub step: String,
    /// The first lines of what was expected and what the engine has.
    pub detail: String,
    /// It passes with the input handed over whole and fails one byte at a time.
    pub split_only: bool,
    /// The case carries a `known-fail:` line for this engine.
    pub known: bool,
}

impl Outcome {
    /// The case passed.
    pub fn is_pass(&self) -> bool {
        matches!(self, Outcome::Pass)
    }

    /// The case failed (a crash included).
    pub fn is_fail(&self) -> bool {
        matches!(self, Outcome::Fail(_))
    }

    /// The adapter cannot ask the case.
    pub fn is_na(&self) -> bool {
        matches!(self, Outcome::Na(_))
    }
}

/// One case and how every engine did on it (in the order of [`Matrix::engines`]).
#[derive(Debug, Clone)]
pub struct Row {
    /// The file the case is in, without `.vtt`: the area it tests.
    pub area: String,
    /// The case's name, unique across the files.
    pub name: String,
    /// The line of its `test` line in the file.
    pub line: usize,
    /// `convention` (no published text says it) rather than `spec`.
    pub convention: bool,
    /// The `spec` or `convention` line.
    pub basis: String,
    /// One per engine, in the order of [`Matrix::engines`].
    pub outcomes: Vec<Outcome>,
}

impl Row {
    /// Engines that ran the case: `(passed, failed)`.
    pub fn tally(&self) -> (usize, usize) {
        let passed = self.outcomes.iter().filter(|o| o.is_pass()).count();
        let failed = self.outcomes.iter().filter(|o| o.is_fail()).count();
        (passed, failed)
    }

    /// Not every engine that ran it agrees.
    pub fn disputed(&self) -> bool {
        let (passed, failed) = self.tally();
        passed > 0 && failed > 0
    }

    /// More engines fail than pass it (and at least two ran it).
    pub fn failed_by_majority(&self) -> bool {
        let (passed, failed) = self.tally();
        passed + failed >= 2 && failed > passed
    }
}

/// What one engine did over a set of cases.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Totals {
    /// Cases passed.
    pub pass: usize,
    /// Cases failed.
    pub fail: usize,
    /// Cases the adapter cannot ask.
    pub na: usize,
}

impl Totals {
    /// Count one more outcome.
    pub fn add(&mut self, outcome: &Outcome) {
        match outcome {
            Outcome::Pass => self.pass += 1,
            Outcome::Fail(_) => self.fail += 1,
            Outcome::Na(_) => self.na += 1,
        }
    }

    /// Cases the engine could be asked.
    pub fn applicable(&self) -> usize {
        self.pass + self.fail
    }
}

#[derive(Debug, Clone, Default)]
/// Every case on every engine.
pub struct Matrix {
    /// The engines, in the order they were run.
    pub engines: Vec<String>,
    /// One row per case, in file order.
    pub rows: Vec<Row>,
}

impl Matrix {
    /// Areas in the order the files sort, each with its cases' totals per engine.
    pub fn areas(&self) -> Vec<(String, usize, Vec<Totals>)> {
        let mut areas: Vec<(String, usize, Vec<Totals>)> = Vec::new();
        for row in &self.rows {
            let index = match areas.iter().position(|(a, ..)| *a == row.area) {
                Some(i) => i,
                None => {
                    areas.push((row.area.clone(), 0, vec![Totals::default(); self.engines.len()]));
                    areas.len() - 1
                }
            };
            areas[index].1 += 1;
            for (totals, outcome) in areas[index].2.iter_mut().zip(&row.outcomes) {
                totals.add(outcome);
            }
        }
        areas
    }

    /// Totals per engine over rows that pass `keep`.
    pub fn totals(&self, keep: impl Fn(&Row) -> bool) -> Vec<Totals> {
        let mut totals = vec![Totals::default(); self.engines.len()];
        for row in self.rows.iter().filter(|r| keep(r)) {
            for (total, outcome) in totals.iter_mut().zip(&row.outcomes) {
                total.add(outcome);
            }
        }
        totals
    }

    /// The rows an engine fails.
    pub fn failures_of(&self, engine: usize) -> impl Iterator<Item = (&Row, &Failure)> {
        self.rows.iter().filter_map(move |row| match row.outcomes.get(engine) {
            Some(Outcome::Fail(failure)) => Some((row, failure)),
            _ => None,
        })
    }
}

/// Collects the outcomes of every case on each engine it visits; [`Runner::finish`] makes the
/// matrix. Pass it to `engines::for_each_engine` (or `visit_engine` for some).
pub struct Runner<'a> {
    cases: &'a [Case],
    engines: Vec<String>,
    outcomes: Vec<Vec<Outcome>>,
}

impl<'a> Runner<'a> {
    /// A runner for `cases`.
    pub fn new(cases: &'a [Case]) -> Self {
        Runner { cases, engines: Vec::new(), outcomes: Vec::new() }
    }

    /// The matrix of everything run so far.
    pub fn finish(self) -> Matrix {
        assemble(self.cases, self.engines, self.outcomes)
    }
}

impl EngineVisitor for Runner<'_> {
    fn visit<E: Engine>(&mut self) {
        self.engines.push(E::NAME.to_string());
        self.outcomes.push(self.cases.iter().map(outcome_of::<E>).collect());
    }
}

/// What the last panic said and where, kept by the hook [`quiet_panics`] installs.
static LAST_PANIC: Mutex<String> = Mutex::new(String::new());

/// Keep a crashing engine from printing its panic: a crash is a failed case, and the message goes
/// into its report (`Failure::detail`).
pub fn quiet_panics() {
    std::panic::set_hook(Box::new(|info| {
        let place = info.location().map(|l| format!(" at {}:{}", l.file(), l.line())).unwrap_or_default();
        let message = info.payload_as_str().unwrap_or("the engine panicked");
        *LAST_PANIC.lock().unwrap_or_else(PoisonError::into_inner) = format!("{message}{place}");
    }));
}

/// How `case` goes on engine `E`: n/a when the adapter cannot answer it, a crash counted as a failure.
pub(crate) fn outcome_of<E: Engine>(case: &Case) -> Outcome {
    if let Some(why) = not_applicable(case, &caps_of(E::NAME)) {
        return Outcome::Na(why);
    }
    match catch_unwind(AssertUnwindSafe(|| vtt::run_case::<E>(case))) {
        Ok(CaseResult::Passed | CaseResult::UnexpectedPass { .. }) => Outcome::Pass,
        Ok(CaseResult::KnownFail { reason }) => {
            Outcome::Fail(Failure { step: "known".into(), detail: reason, split_only: false, known: true })
        }
        Ok(CaseResult::Failed(message)) => Outcome::Fail(parse_failure(&message)),
        Err(_) => {
            let detail = LAST_PANIC.lock().unwrap_or_else(PoisonError::into_inner).clone();
            Outcome::Fail(Failure { step: "panic".into(), detail, split_only: false, known: false })
        }
    }
}

/// Pick the step and the first lines of the diff out of the runner's failure text:
///
/// ```text
/// FAIL file.vtt:12 name [input fed one byte at a time]
///   spec: ...
///   input: "..."
///   line 14 (screen):
///     screen: expected (-) vs actual (+)
/// ```
fn parse_failure(message: &str) -> Failure {
    let split_only = message.lines().next().is_some_and(|l| l.contains("[input fed one byte at a time]"));
    let mut step = String::from("?");
    let mut detail = Vec::new();
    let mut in_detail = false;
    for line in message.lines() {
        if in_detail {
            detail.push(line.trim_start().to_string());
        } else if let Some(rest) = line.strip_prefix("  line ")
            && let Some(open) = rest.find('(')
            && let Some(close) = rest[open..].find(')')
        {
            step = rest[open + 1..open + close].to_string();
            in_detail = true;
        }
    }
    // Skip the heading line of a diff and cut a long one short.
    detail.retain(|l| !l.is_empty());
    let detail = detail.into_iter().take(6).collect::<Vec<_>>().join("\n");
    Failure { step, detail, split_only, known: false }
}

/// The cases of a conformance folder (`crates/wormhole-vt/tests/conformance`).
pub fn load_cases(dir: &Path) -> Result<Vec<Case>, String> {
    vtt::load_dir(dir).map_err(|errors| errors.join("\n"))
}

/// The area a case belongs to: its file's name without `.vtt`.
fn area_of(case: &Case) -> String {
    let file =
        Path::new(&case.file).file_name().map(|f| f.to_string_lossy().into_owned()).unwrap_or_default();
    file.strip_suffix(".vtt").unwrap_or(&file).to_string()
}

/// Assemble the matrix from per-engine outcome lists (one `Vec` per engine, one `Outcome` per case).
fn assemble(cases: &[Case], engines: Vec<String>, outcomes: Vec<Vec<Outcome>>) -> Matrix {
    let rows = cases
        .iter()
        .enumerate()
        .map(|(i, case)| {
            let (convention, basis) = match &case.basis {
                Some(Basis::Convention(text)) => (true, text.clone()),
                Some(Basis::Spec(text)) => (false, text.clone()),
                None => (false, String::new()),
            };
            Row {
                area: area_of(case),
                name: case.name.clone(),
                line: case.line,
                convention,
                basis,
                outcomes: outcomes.iter().map(|per_engine| per_engine[i].clone()).collect(),
            }
        })
        .collect();
    Matrix { engines, rows }
}

#[cfg(test)]
mod tests {
    use wormhole_vt_bench::vtt::parse_file;

    use super::{Outcome, Runner, outcome_of, parse_failure};
    use crate::engines::{AlacrittyEngine, Vt100Engine, for_each_engine};

    fn cases(text: &str) -> Vec<wormhole_vt_bench::vtt::Case> {
        parse_file("area.vtt", text).unwrap_or_else(|e| panic!("{e:?}"))
    }

    #[test]
    fn a_failure_keeps_its_step_and_the_first_lines_of_the_diff() {
        let message = "FAIL a.vtt:3 name [input fed one byte at a time]\n  spec: s\n  input: \"x\"\n  line 6 (screen):\n    screen: expected (-) vs actual (+)\n    -  1 |ax|\n    +  1 |ab|\n";
        let failure = parse_failure(message);
        assert_eq!(failure.step, "screen");
        assert!(failure.split_only && !failure.known);
        assert_eq!(failure.detail, "screen: expected (-) vs actual (+)\n-  1 |ax|\n+  1 |ab|");
    }

    #[test]
    fn a_case_is_passed_failed_or_not_applicable_per_engine() {
        let all = cases(
            "test passes\nspec s\nsize 10 2\nfeed ab\nscreen\n|ab|\n\n\
             test fails\nspec s\nsize 10 2\nfeed ab\nscreen\n|ax|\n\n\
             test asks\nspec s\nsize 10 2\nfeed \\e[5n\nreply \\e[0n\n",
        );
        let on_vt100: Vec<Outcome> = all.iter().map(outcome_of::<Vt100Engine>).collect();
        assert_eq!(on_vt100[0], Outcome::Pass);
        assert!(on_vt100[1].is_fail());
        assert_eq!(on_vt100[2], Outcome::Na("no replies"));
        assert_eq!(outcome_of::<AlacrittyEngine>(&all[2]), Outcome::Pass);
    }

    #[test]
    fn the_runner_makes_a_row_per_case_and_an_outcome_per_engine() {
        let all = cases("test one\nconvention c\nsize 10 2\nfeed a\nscreen\n|a|\n");
        let mut runner = Runner::new(&all);
        for_each_engine(&mut runner);
        let matrix = runner.finish();
        assert_eq!(matrix.rows.len(), 1);
        assert_eq!(matrix.rows[0].outcomes.len(), matrix.engines.len());
        assert!(matrix.rows[0].convention && matrix.rows[0].area == "area");
        assert!(matrix.rows[0].outcomes.iter().all(Outcome::is_pass));
    }
}
