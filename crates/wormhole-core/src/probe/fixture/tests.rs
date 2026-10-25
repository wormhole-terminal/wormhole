use std::path::PathBuf;

use super::*;
use crate::probe::manifest::Manifest;
use crate::probe::read::{MANIFEST_FILE, ProbeFile};
use crate::probe::rules::CompiledRules;

const MANIFEST: &str = r#"[probe]
id = "tiny"
name = "Tiny"
description = "d"
author = "a"
version = "0.1.0"
wormhole = ">=1.3"
api = 1
[permissions]
open = ["acme.atlassian.net"]
[[contributes.rules]]
id = "ticket"
pattern = '(?-u:\b)(ACME-[0-9]{1,6})(?-u:\b)'
tone = "link"
action = { type = "open-url", url = "https://acme.atlassian.net/browse/{1}" }
[[contributes.rules]]
id = "key"
pattern = 'sk-[a-z0-9]{6,}'
look = "redact"
"#;

fn folder(tests: &[(&str, &str)]) -> ProbeFolder {
    let parsed = Manifest::parse(MANIFEST).expect("manifest");
    let mut files = vec![ProbeFile::new(MANIFEST_FILE, MANIFEST.as_bytes())];
    files.extend(tests.iter().map(|(path, body)| ProbeFile::new(*path, body.as_bytes())));
    ProbeFolder { root: PathBuf::new(), manifest: parsed.manifest, warnings: parsed.warnings, files }
}

fn run_text(text: &str) -> TestReport {
    let folder = folder(&[("tests/t.toml", text)]);
    let rules = CompiledRules::compile(&folder.manifest).expect("compiles");
    run(&folder, &rules)
}

const GOOD: &str = r#"
[[case]]
name = "a ticket"
input = "see ACME-123 now"
[[case.expect]]
rule = "ticket"
start = 4
end = 12
text = "ACME-123"
look = "none"
target = "https://acme.atlassian.net/browse/ACME-123"

[[case]]
input = "nothing here"
"#;

#[test]
fn a_case_that_gets_what_it_expects_passes() {
    let report = run_text(GOOD);
    assert!(report.is_ok(), "{report:#?}");
    assert_eq!(report.cases.len(), 2);
    assert_eq!(report.cases[0].name, "a ticket");
    assert_eq!(report.cases[1].name, "case 2", "unnamed cases are numbered");
    assert_eq!(report.passed(), 2);
}

#[test]
fn spans_are_in_characters_not_bytes() {
    let report = run_text(
        "[[case]]\ninput = \"\u{e9}\u{e9} ACME-1\"\n[[case.expect]]\nrule = \"ticket\"\nstart = 3\nend = 9\n",
    );
    assert!(report.is_ok(), "{report:#?}");
}

#[test]
fn a_wrong_span_target_look_or_rule_is_named() {
    let report = run_text(
        r#"
[[case]]
input = "ACME-1"
[[case.expect]]
rule = "key"
start = 1
end = 5
text = "nope"
look = "redact"
target = "https://evil.example/"
"#,
    );
    assert_eq!(report.failed(), 1);
    let problems = report.cases[0].problems.join("\n");
    for needle in [
        "expected rule `key`, found `ticket`",
        "expected the span 1..5, found 0..6",
        "expected the text \"nope\", found \"ACME-1\"",
        "expected look redact, found none",
        "expected the target \"https://evil.example/\", found \"https://acme.atlassian.net/browse/ACME-1\"",
        "found 1: rule `ticket` 0..6",
    ] {
        assert!(problems.contains(needle), "{needle}\n{problems}");
    }
}

#[test]
fn a_missing_or_extra_match_fails() {
    let missing =
        run_text("[[case]]\ninput = \"none\"\n[[case.expect]]\nrule = \"ticket\"\nstart = 0\nend = 4\n");
    assert!(missing.cases[0].problems[0].contains("expected rule `ticket` 0..4, but nothing more matched"));
    assert!(missing.cases[0].problems.contains(&"found no matches".to_string()));
    let extra = run_text(
        "[[case]]\ninput = \"ACME-1 ACME-2\"\n[[case.expect]]\nrule = \"ticket\"\nstart = 0\nend = 6\n",
    );
    assert!(
        extra.cases[0].problems[0].contains("not expected, but rule `ticket` 7..13"),
        "{:?}",
        extra.cases[0].problems
    );
}

#[test]
fn a_target_is_expected_where_the_rule_has_none() {
    let report = run_text(
        "[[case]]\ninput = \"sk-abcdef\"\n[[case.expect]]\nrule = \"key\"\nstart = 0\nend = 9\ntarget = \"x\"\n",
    );
    assert!(report.cases[0].problems[0].contains("this match has no target"), "{:?}", report.cases[0]);
}

#[test]
fn several_rules_give_their_matches_in_the_order_they_lie_in_the_line() {
    let report = run_text(
        r#"
[[case]]
input = "sk-aaaaaa ACME-1"
[[case.expect]]
rule = "key"
start = 0
end = 9
look = "redact"
[[case.expect]]
rule = "ticket"
start = 10
end = 16
look = "none"
target = "https://acme.atlassian.net/browse/ACME-1"
"#,
    );
    assert!(report.is_ok(), "{report:#?}");
}

#[test]
fn a_line_break_in_the_input_is_refused() {
    let report = run_text("[[case]]\ninput = \"a\\nb\"\n");
    assert!(report.cases[0].problems[0].contains("one logical line"));
}

#[test]
fn a_typo_in_a_fixture_is_an_error_and_not_a_pass() {
    for bad in [
        "[[case]]\ninput = \"x\"\n[[case.expect]]\nrule = \"a\"\nstart = 0\nend = 1\nlooks = \"none\"\n",
        "[[cases]]\ninput = \"x\"\n",
        "[[case]]\nimput = \"x\"\n",
        "[[case]]\ninput = \"x\"\n[[case.expect]]\nrule = \"a\"\nstart = -1\nend = 1\n",
        "[[case]\n",
    ] {
        let report = run_text(bad);
        assert_eq!(report.unreadable.len(), 1, "{bad}");
        assert!(report.cases.is_empty());
        assert!(!report.is_ok());
        assert_eq!(report.unreadable[0].file, "tests/t.toml");
    }
}

#[test]
fn only_toml_files_directly_under_tests_are_fixtures() {
    let folder = folder(&[
        ("tests/a.toml", ""),
        ("tests/b.txt", "x"),
        ("tests/nested/c.toml", ""),
        ("tests/.toml", ""),
        ("src/d.toml", ""),
        ("tests.toml", ""),
    ]);
    let found: Vec<_> = fixtures(&folder).into_iter().map(|f| f.expect("reads").file).collect();
    assert_eq!(found, ["tests/a.toml"]);
}

#[test]
fn a_fixture_that_is_not_text_is_an_error_in_its_place() {
    let mut folder = folder(&[]);
    folder.files.push(ProbeFile::new("tests/bin.toml", vec![0xff, 0xfe]));
    let all = fixtures(&folder);
    assert_eq!(all.len(), 1);
    assert!(all[0].as_ref().expect_err("not text").message.contains("UTF-8"));
}

#[test]
fn an_empty_file_has_no_cases() {
    let report = run_text("# nothing yet\n");
    assert!(report.cases.is_empty() && report.is_ok());
}
