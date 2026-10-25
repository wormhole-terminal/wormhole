//! `probe test`: the Probe's rules over its `tests/*.toml`, a pass or a fail for each case.

use std::path::Path;

use serde::Serialize;

use super::Outcome;
use super::text::{self, Out, count};
use super::validate_cmd::{JSON_VERSION, unreadable};
use crate::probe::fixture::{self, TESTS_DIR, TestReport};
use crate::probe::read::ProbeFolder;
use crate::probe::rules::CompiledRules;
use crate::probe::validate::{Finding, validate};

pub(super) fn run(dir: &Path, json: bool) -> Outcome {
    let folder = match ProbeFolder::read(dir) {
        Ok(folder) => folder,
        Err(err) => return unreadable(dir, &err.to_string(), json),
    };
    // The rules are run as `validate` accepted them: a pattern it refuses may match the empty string
    // or take longer than the limits allow, and a test of that proves nothing.
    let report = validate(&folder);
    let errors: Vec<&Finding> = report.errors().collect();
    if !errors.is_empty() {
        return refused(&folder, &errors, json);
    }
    let rules = match CompiledRules::compile(&folder.manifest) {
        Ok(rules) => rules,
        Err(err) => return Outcome::fail(1, &err.to_string()),
    };
    let tests = fixture::run(&folder, &rules);
    let files = fixture::fixtures(&folder).len();
    // A rule nobody tests is a rule that can change without anyone noticing.
    let untested = !rules.is_empty() && tests.cases.is_empty() && tests.unreadable.is_empty();
    let ok = tests.is_ok() && !untested;
    let stdout = if json {
        text::json_line(&Json::of(&tests, ok, untested))
    } else {
        render(&folder, &rules, &tests, files, untested)
    };
    Outcome::new(i32::from(!ok), stdout, String::new())
}

/// `validate` has errors, so there is nothing sound to run.
fn refused(folder: &ProbeFolder, errors: &[&Finding], json: bool) -> Outcome {
    if json {
        let reply = JsonRefused {
            version: JSON_VERSION,
            ok: false,
            error: "the Probe does not validate; fix its errors first".into(),
            findings: errors
                .iter()
                .map(|f| JsonError { code: f.code, path: &f.path, message: &f.message })
                .collect(),
        };
        return Outcome::new(1, text::json_line(&reply), String::new());
    }
    let mut out = Out::default();
    out.line(&format!(
        "{} does not validate, so its rules are not run: fix {} first",
        folder.manifest.probe.id,
        count(errors.len(), "error", "errors")
    ));
    for finding in errors {
        let place = if finding.path.is_empty() { "(the folder)" } else { finding.path.as_str() };
        out.line(&format!("  [{}] {place}", finding.code));
        out.line(&format!("      {}", finding.message));
    }
    Outcome::new(1, out.finish(), String::new())
}

fn render(
    folder: &ProbeFolder,
    rules: &CompiledRules,
    tests: &TestReport,
    files: usize,
    untested: bool,
) -> String {
    let id = &folder.manifest.probe.id;
    let mut out = Out::default();
    if rules.is_empty() && tests.cases.is_empty() && tests.unreadable.is_empty() {
        out.line(&format!("{id} has no text rules, so there is nothing to test yet"));
        return out.finish();
    }
    if untested {
        out.line(&format!(
            "{id} has {} but no test cases: add cases to {TESTS_DIR}/*.toml (PROBE.md shows the format)",
            count(rules.len(), "rule", "rules")
        ));
        return out.finish();
    }
    let mut file = "";
    for case in &tests.cases {
        if case.file != file {
            file = &case.file;
            out.line(file);
        }
        out.line(&format!("  {}  {}", if case.passed() { "ok  " } else { "FAIL" }, case.name));
        for problem in &case.problems {
            out.line(&format!("        {problem}"));
        }
    }
    for err in &tests.unreadable {
        out.line(&err.file);
        out.line(&format!("  FAIL  the file is not a fixture: {}", err.message));
    }
    out.blank();
    let mut summary = format!(
        "{}: {} passed, {} failed",
        count(tests.cases.len(), "case", "cases"),
        tests.passed(),
        tests.failed()
    );
    if !tests.unreadable.is_empty() {
        summary.push_str(&format!(", {} could not be read", count(tests.unreadable.len(), "file", "files")));
    }
    out.line(&format!("{summary} ({})", count(files, "file", "files")));
    out.finish()
}

#[derive(Serialize)]
struct JsonRefused<'a> {
    version: u32,
    ok: bool,
    error: String,
    findings: Vec<JsonError<'a>>,
}

#[derive(Serialize)]
struct JsonError<'a> {
    code: &'a str,
    path: &'a str,
    message: &'a str,
}

#[derive(Serialize)]
struct Json<'a> {
    version: u32,
    ok: bool,
    passed: usize,
    failed: usize,
    /// Rules with no case to test them.
    untested: bool,
    cases: Vec<JsonCase<'a>>,
    /// Fixture files that are not fixtures.
    unreadable: Vec<JsonUnreadable<'a>>,
}

#[derive(Serialize)]
struct JsonCase<'a> {
    file: &'a str,
    name: &'a str,
    ok: bool,
    problems: &'a [String],
}

#[derive(Serialize)]
struct JsonUnreadable<'a> {
    file: &'a str,
    message: &'a str,
}

impl<'a> Json<'a> {
    fn of(tests: &'a TestReport, ok: bool, untested: bool) -> Self {
        Self {
            version: JSON_VERSION,
            ok,
            passed: tests.passed(),
            failed: tests.failed(),
            untested,
            cases: tests
                .cases
                .iter()
                .map(|c| JsonCase { file: &c.file, name: &c.name, ok: c.passed(), problems: &c.problems })
                .collect(),
            unreadable: tests
                .unreadable
                .iter()
                .map(|e| JsonUnreadable { file: &e.file, message: &e.message })
                .collect(),
        }
    }
}
