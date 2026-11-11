//! `probe validate` and `probe test` on a Probe with hooks.

use tempfile::TempDir;

use super::{fixture_dir, go, json, probe_in};

const HOOK: &str = r#"
[permissions]
notify = true
[[contributes.hooks]]
id = "slow"
on = "command.finished"
match = { exit = "!=0", took_ms = ">=60000" }
do = [{ type = "notify", title = "Failed", body = "{took_s} s in {folder_name}" }]
"#;

const CASES: &str = r#"
[[hook_case]]
name = "slow"
hook = "slow"
event = { exit = 1, took_ms = 61000, folder_name = "acme" }
[[hook_case.expect]]
type = "notify"
title = "Failed"
body = "61 s in acme"

[[hook_case]]
name = "quick"
hook = "slow"
event = { exit = 1, took_ms = 100 }
"#;

#[test]
fn validate_lists_each_hook_with_when_it_fires_and_what_it_does() {
    let dir = fixture_dir("ok", "hooks");
    let out = go(std::path::Path::new("/"), &["validate", dir.to_str().expect("utf-8")]);
    assert_eq!(out.code, 0, "{out:?}");
    let text = &out.stdout;
    assert!(text.contains("6 hooks"), "{text}");
    assert!(text.contains("Hooks (6)\n  slow-build-failed: on command.finished when exit != 0, took_ms >= 60000\n      notify \"Build failed\"\n"), "{text}");
    assert!(text.contains("  quiet-green-builds: on command.finished when exit = 0, output_lines >= 40\n      fold block-output \"build output\", keeping 1 and 3 rows (hides output on screen; copy still gives it)\n"), "{text}");
    assert!(text.contains("Touch ID  fold"), "fold is a Touch ID permission: {text}");
}

#[test]
fn validate_json_has_the_hooks() {
    let dir = fixture_dir("ok", "hooks");
    let out = go(std::path::Path::new("/"), &["validate", "--json", dir.to_str().expect("utf-8")]);
    let value = json(&out);
    let hooks = value["summary"]["hooks"].as_array().expect("hooks");
    assert_eq!(hooks.len(), 6);
    assert_eq!(hooks[0]["id"], "slow-build-failed");
    assert_eq!(hooks[0]["on"], "command.finished");
    assert_eq!(hooks[0]["when"], "exit != 0, took_ms >= 60000");
    assert_eq!(hooks[0]["actions"][0], "notify \"Build failed\"");
    assert!(
        value["summary"]["permissions"]
            .as_array()
            .expect("array")
            .iter()
            .any(|p| p["key"] == "fold" && p["confirmation"] == "touch_id")
    );
}

#[test]
fn test_runs_the_hook_cases_beside_the_rule_cases() {
    let tmp = TempDir::new().expect("tempdir");
    let dir = probe_in(tmp.path(), "p", HOOK, &[("tests/hooks.toml", CASES)]);
    let out = go(tmp.path(), &["test", dir.to_str().expect("utf-8")]);
    assert_eq!(out.code, 0, "{out:?}");
    assert_eq!(
        out.stdout,
        "tests/hooks.toml\n  ok    slow\n  ok    quick\n\n2 cases: 2 passed, 0 failed (1 file)\n"
    );
}

#[test]
fn a_hook_case_that_differs_says_what_was_planned() {
    let tmp = TempDir::new().expect("tempdir");
    let wrong = CASES.replace("61 s in acme", "60 s in acme");
    let dir = probe_in(tmp.path(), "p", HOOK, &[("tests/hooks.toml", &wrong)]);
    let out = go(tmp.path(), &["test", dir.to_str().expect("utf-8")]);
    assert_eq!(out.code, 1, "{out:?}");
    assert!(
        out.stdout.contains(
            "  FAIL  slow\n        action 1: expected body \"60 s in acme\", found \"61 s in acme\"\n"
        ),
        "{}",
        out.stdout
    );
    assert!(
        out.stdout.contains("        planned 1: notify title \"Failed\" body \"61 s in acme\"\n"),
        "{}",
        out.stdout
    );
}

#[test]
fn a_hook_nobody_tests_is_said() {
    let tmp = TempDir::new().expect("tempdir");
    let dir = probe_in(tmp.path(), "p", HOOK, &[]);
    let out = go(tmp.path(), &["test", dir.to_str().expect("utf-8")]);
    assert_eq!(out.code, 1, "{out:?}");
    assert!(out.stdout.contains("tiny has 1 hook but no test cases"), "{}", out.stdout);
}

#[test]
fn a_hook_case_with_a_field_the_event_lacks_or_a_bad_type_fails() {
    let tmp = TempDir::new().expect("tempdir");
    let bad = "[[hook_case]]\nhook = \"slow\"\nevent = { bell = 1 }\n[[hook_case]]\nhook = \"nope\"\nevent = {}\n\
               [[hook_case]]\nhook = \"slow\"\nevent = { exit = \"x\" }\n";
    let dir = probe_in(tmp.path(), "p", HOOK, &[("tests/hooks.toml", bad)]);
    let out = go(tmp.path(), &["test", dir.to_str().expect("utf-8")]);
    assert_eq!(out.code, 1, "{out:?}");
    assert!(out.stdout.contains("event: `bell` is not a field of command.finished"), "{}", out.stdout);
    assert!(out.stdout.contains("no hook `nope` (the Probe has: slow)"), "{}", out.stdout);
    assert!(out.stdout.contains("event: `exit` is a number"), "{}", out.stdout);
}

#[test]
fn a_typo_in_a_hook_case_is_an_error_and_not_a_pass() {
    let tmp = TempDir::new().expect("tempdir");
    let typo = "[[hook_case]]\nhook = \"slow\"\nevent = {}\nexpects = []\n";
    let dir = probe_in(tmp.path(), "p", HOOK, &[("tests/hooks.toml", typo)]);
    let out = go(tmp.path(), &["test", dir.to_str().expect("utf-8")]);
    assert_eq!(out.code, 1, "{out:?}");
    assert!(out.stdout.contains("the file is not a fixture"), "{}", out.stdout);
}
