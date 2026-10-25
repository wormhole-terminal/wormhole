use tempfile::TempDir;

use super::{go, json, probe_in};

const RULE: &str =
    "[[contributes.rules]]\nid = \"n\"\npattern = '(?-u:\\b)([0-9]{3})(?-u:\\b)'\ntone = \"link\"\n";

fn case(input: &str, expect: &str) -> String {
    format!("[[case]]\nname = \"c\"\ninput = \"{input}\"\n{expect}")
}

const SPAN: &str = "[[case.expect]]\nrule = \"n\"\nstart = 4\nend = 7\n";

#[test]
fn a_passing_probe_prints_ok_for_each_case_and_exits_zero() {
    let tmp = TempDir::new().expect("tempdir");
    let dir = probe_in(
        tmp.path(),
        "p",
        RULE,
        &[("tests/a.toml", &case("see 123", SPAN)), ("tests/b.toml", &case("none", ""))],
    );
    let out = go(tmp.path(), &["test", dir.to_str().expect("utf-8")]);
    assert_eq!(out.code, 0, "{out:?}");
    assert_eq!(
        out.stdout,
        "tests/a.toml\n  ok    c\ntests/b.toml\n  ok    c\n\n2 cases: 2 passed, 0 failed (2 files)\n"
    );
}

#[test]
fn a_failing_case_names_what_differs_and_exits_one() {
    let tmp = TempDir::new().expect("tempdir");
    let wrong = case("see 123", "[[case.expect]]\nrule = \"n\"\nstart = 5\nend = 7\n");
    let dir =
        probe_in(tmp.path(), "p", RULE, &[("tests/a.toml", &wrong), ("tests/b.toml", &case("none", ""))]);
    let out = go(tmp.path(), &["test", dir.to_str().expect("utf-8")]);
    assert_eq!(out.code, 1, "{out:?}");
    let text = out.stdout;
    assert!(text.contains("  FAIL  c\n        match 1: expected the span 5..7, found 4..7\n"), "{text}");
    assert!(text.contains("        found 1: rule `n` 4..7 \"123\" look none\n"), "{text}");
    assert!(text.contains("  ok    c\n"), "the other case still ran: {text}");
    assert!(text.trim_end().ends_with("2 cases: 1 passed, 1 failed (2 files)"), "{text}");
}

#[test]
fn the_default_folder_is_the_working_folder() {
    let tmp = TempDir::new().expect("tempdir");
    let dir = probe_in(tmp.path(), "p", RULE, &[("tests/a.toml", &case("see 123", SPAN))]);
    assert_eq!(go(&dir, &["test"]).code, 0);
}

#[test]
fn json_lists_every_case() {
    let tmp = TempDir::new().expect("tempdir");
    let wrong = case("see 123", "[[case.expect]]\nrule = \"n\"\nstart = 5\nend = 7\n");
    let dir = probe_in(
        tmp.path(),
        "p",
        RULE,
        &[("tests/a.toml", &wrong), ("tests/b.toml", &case("see 123", SPAN))],
    );
    let out = go(tmp.path(), &["test", "--json", dir.to_str().expect("utf-8")]);
    assert_eq!(out.code, 1);
    let value = json(&out);
    assert_eq!((value["version"].as_u64(), value["ok"].as_bool()), (Some(1), Some(false)));
    assert_eq!((value["passed"].as_u64(), value["failed"].as_u64()), (Some(1), Some(1)));
    assert_eq!(value["untested"], false);
    assert_eq!(value["unreadable"], serde_json::json!([]));
    let cases = value["cases"].as_array().expect("array");
    assert_eq!(cases[0]["file"], "tests/a.toml");
    assert_eq!(cases[0]["ok"], false);
    assert!(cases[0]["problems"][0].as_str().expect("problem").contains("expected the span 5..7"));
    assert_eq!(
        (cases[1]["name"].as_str(), cases[1]["ok"].as_bool(), cases[1]["problems"].as_array().map(Vec::len)),
        (Some("c"), Some(true), Some(0))
    );
}

#[test]
fn a_fixture_that_is_not_one_fails_the_test() {
    let tmp = TempDir::new().expect("tempdir");
    let dir = probe_in(
        tmp.path(),
        "p",
        RULE,
        &[("tests/a.toml", &case("see 123", SPAN)), ("tests/typo.toml", "[[case]]\nimput = \"x\"\n")],
    );
    let out = go(tmp.path(), &["test", dir.to_str().expect("utf-8")]);
    assert_eq!(out.code, 1, "{out:?}");
    assert!(out.stdout.contains("tests/typo.toml\n  FAIL  the file is not a fixture: "), "{}", out.stdout);
    assert!(
        out.stdout.contains("1 case: 1 passed, 0 failed, 1 file could not be read (2 files)"),
        "{}",
        out.stdout
    );
    let value = json(&go(tmp.path(), &["test", "--json", dir.to_str().expect("utf-8")]));
    assert_eq!(value["unreadable"][0]["file"], "tests/typo.toml");
}

#[test]
fn rules_nobody_tests_fail_the_test() {
    let tmp = TempDir::new().expect("tempdir");
    let dir = probe_in(tmp.path(), "p", RULE, &[]);
    let out = go(tmp.path(), &["test", dir.to_str().expect("utf-8")]);
    assert_eq!(out.code, 1, "{out:?}");
    assert!(out.stdout.contains("has 1 rule but no test cases"), "{}", out.stdout);
    let empty = probe_in(tmp.path(), "q", RULE, &[("tests/a.toml", "# nothing yet\n")]);
    assert_eq!(go(tmp.path(), &["test", empty.to_str().expect("utf-8")]).code, 1);
    assert_eq!(json(&go(tmp.path(), &["test", "--json", empty.to_str().expect("utf-8")]))["untested"], true);
}

#[test]
fn a_probe_without_rules_has_nothing_to_test_and_passes() {
    let tmp = TempDir::new().expect("tempdir");
    let dir = probe_in(tmp.path(), "p", "", &[]);
    let out = go(tmp.path(), &["test", dir.to_str().expect("utf-8")]);
    assert_eq!(out.code, 0, "{out:?}");
    assert!(out.stdout.contains("no text rules, so there is nothing to test yet"), "{}", out.stdout);
    assert_eq!(json(&go(tmp.path(), &["test", "--json", dir.to_str().expect("utf-8")]))["ok"], true);
    // A case with no rule to run it cannot pass.
    let stray = probe_in(tmp.path(), "q", "", &[("tests/a.toml", &case("see 123", SPAN))]);
    assert_eq!(go(tmp.path(), &["test", stray.to_str().expect("utf-8")]).code, 1);
}

#[test]
fn a_probe_that_does_not_validate_is_not_tested() {
    let tmp = TempDir::new().expect("tempdir");
    let bad = "[[contributes.rules]]\nid = \"n\"\npattern = 'a*'\ntone = \"link\"\n";
    let dir = probe_in(tmp.path(), "p", bad, &[("tests/a.toml", &case("aaa", ""))]);
    let out = go(tmp.path(), &["test", dir.to_str().expect("utf-8")]);
    assert_eq!(out.code, 1, "{out:?}");
    assert!(
        out.stdout.contains("p")
            && out.stdout.contains("does not validate, so its rules are not run: fix 1 error first"),
        "{}",
        out.stdout
    );
    assert!(out.stdout.contains("[pattern-empty-match] contributes.rules[0].pattern"), "{}", out.stdout);
    let value = json(&go(tmp.path(), &["test", "--json", dir.to_str().expect("utf-8")]));
    assert_eq!(value["ok"], false);
    assert_eq!(value["findings"][0]["code"], "pattern-empty-match");
}

#[test]
fn warnings_do_not_stop_a_test() {
    let tmp = TempDir::new().expect("tempdir");
    let rule = "[[contributes.rules]]\nid = \"n\"\npattern = '\\b[0-9]{3}\\b'\ntone = \"link\"\n";
    let dir = probe_in(tmp.path(), "p", rule, &[("tests/a.toml", &case("see 123", SPAN))]);
    assert_eq!(go(tmp.path(), &["test", dir.to_str().expect("utf-8")]).code, 0);
}

#[test]
fn a_missing_folder_is_a_problem() {
    let tmp = TempDir::new().expect("tempdir");
    let out = go(tmp.path(), &["test", "missing"]);
    assert_eq!(out.code, 1);
    assert!(out.stderr.starts_with("Wormhole: "), "{}", out.stderr);
    let json_out = go(tmp.path(), &["test", "missing", "--json"]);
    assert_eq!(json(&json_out)["ok"], false);
}

#[test]
fn a_fixture_with_hostile_text_is_written_out() {
    let tmp = TempDir::new().expect("tempdir");
    let evil = "[[case]]\nname = \"\\u001b[2Jx\\u202e\"\ninput = \"see 123\"\n[[case.expect]]\nrule = \"n\\u001b\"\nstart = 4\nend = 7\n";
    let dir = probe_in(tmp.path(), "p", RULE, &[("tests/a.toml", evil)]);
    let out = go(tmp.path(), &["test", dir.to_str().expect("utf-8")]);
    assert_eq!(out.code, 1);
    for c in out.stdout.chars() {
        assert!(!crate::probe::hidden::is_hidden(c) || c == '\n', "{:x}", u32::from(c));
    }
    assert!(out.stdout.contains("\\u{1b}[2Jx\\u{202e}"), "{}", out.stdout);
}
