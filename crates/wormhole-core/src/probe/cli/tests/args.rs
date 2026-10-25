use tempfile::TempDir;

use super::go;

fn mistake(args: &[&str]) -> String {
    let tmp = TempDir::new().expect("tempdir");
    let out = go(tmp.path(), args);
    assert_eq!(out.code, 2, "{args:?}: {out:?}");
    assert!(out.stdout.is_empty(), "{args:?}");
    assert!(out.stderr.starts_with("Wormhole: "), "{}", out.stderr);
    assert!(out.stderr.contains("wh probe validate"), "the usage follows the mistake: {}", out.stderr);
    out.stderr
}

#[test]
fn bad_arguments_are_exit_code_two_with_the_usage() {
    assert!(mistake(&[]).contains("needs validate, new or test"));
    assert!(mistake(&["frobnicate"]).contains("unknown probe command"));
    assert!(mistake(&["validate", "a", "b"]).contains("takes one folder"));
    assert!(mistake(&["test", "a", "b"]).contains("takes one folder"));
    assert!(mistake(&["validate", "--bogus"]).contains("unknown option --bogus"));
    assert!(mistake(&["test", "--kind", "rules"]).contains("unknown option --kind"));
    assert!(mistake(&["validate", "--dir", "x"]).contains("unknown option --dir"));
    assert!(mistake(&["validate", "--json=yes"]).contains("unknown option"));
    assert!(mistake(&["new"]).contains("takes one id"));
    assert!(mistake(&["new", "a-b", "c-d"]).contains("takes one id"));
    assert!(mistake(&["new", "my-probe", "--json"]).contains("unknown option --json"));
    assert!(mistake(&["new", "my-probe", "--kind"]).contains("--kind needs a value"));
    assert!(mistake(&["new", "my-probe", "--dir"]).contains("--dir needs a value"));
    assert!(mistake(&["new", "my-probe", "--kind", "bogus"]).contains("not a kind of Probe"));
}

#[test]
fn a_probe_with_code_is_not_a_kind_yet() {
    let said = mistake(&["new", "my-probe", "--kind", "code"]);
    assert!(said.contains("not available yet"), "{said}");
    assert!(said.contains("rules, command, status, agent"), "{said}");
}

#[test]
fn an_id_that_is_not_one_is_a_mistake_in_the_arguments() {
    for id in ["A", "a", "../x", "a b", "ab-"] {
        let tmp = TempDir::new().expect("tempdir");
        let out = go(tmp.path(), &["new", id]);
        assert_eq!(out.code, 2, "{id}: {out:?}");
        assert!(out.stderr.contains("is not a Probe id"), "{}", out.stderr);
        assert_eq!(std::fs::read_dir(tmp.path()).expect("lists").count(), 0);
    }
}

#[test]
fn help_is_asked_for_in_the_usual_ways_and_names_the_command() {
    for args in [&["--help"][..], &["help"], &["-h"], &["validate", "--help"], &["new", "x", "-h"]] {
        let tmp = TempDir::new().expect("tempdir");
        let out = go(tmp.path(), args);
        assert_eq!(out.code, 0, "{args:?}");
        for line in [
            "wh probe validate [<folder>] [--json]",
            "wh probe new <id> [--kind rules|command|status|agent]",
            "wh probe test [<folder>] [--json]",
        ] {
            assert!(out.stdout.contains(line), "{args:?}: {line}");
        }
        assert!(out.stderr.is_empty());
    }
}

#[test]
fn options_come_in_any_place_and_a_double_dash_ends_them() {
    let tmp = TempDir::new().expect("tempdir");
    let out = go(tmp.path(), &["new", "--kind=status", "--dir", ".", "my-probe"]);
    assert_eq!(out.code, 0, "{out:?}");
    assert!(tmp.path().join("my-probe/probe.toml").exists());
    // A folder that starts with a dash is reachable after `--`.
    let dash = tmp.path().join("-odd");
    std::fs::create_dir(&dash).expect("mkdir");
    let out = go(tmp.path(), &["validate", "--", "-odd"]);
    assert_eq!(out.code, 1, "no probe.toml there, but it was looked for: {out:?}");
    assert!(out.stderr.contains("-odd"), "{}", out.stderr);
}
