use std::fs;
use std::path::Path;

use tempfile::TempDir;

use super::{HEAD, fixture_dir, go, json, probe_in};

fn folders(group: &str) -> Vec<String> {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/probes").join(group);
    let mut names: Vec<String> = fs::read_dir(dir)
        .expect("lists")
        .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

#[test]
fn every_ok_fixture_exits_zero_and_every_error_fixture_one() {
    let here = Path::new("/");
    for name in folders("ok") {
        let dir = fixture_dir("ok", &name);
        let out = go(here, &["validate", dir.to_str().expect("utf-8")]);
        assert_eq!(out.code, 0, "ok/{name}: {out:?}");
        assert!(out.stderr.is_empty());
    }
    for name in folders("errors") {
        let dir = fixture_dir("errors", &name);
        let out = go(here, &["validate", dir.to_str().expect("utf-8")]);
        assert_eq!(out.code, 1, "errors/{name}: {out:?}");
        assert!(
            out.stdout.contains(&format!("[{name}]")) || out.stdout.contains("errors ("),
            "errors/{name}"
        );
    }
    for name in folders("warnings") {
        let dir = fixture_dir("warnings", &name);
        let out = go(here, &["validate", dir.to_str().expect("utf-8")]);
        assert_eq!(out.code, 0, "warnings are not errors: warnings/{name}: {out:?}");
        assert!(out.stdout.contains(&format!("[{name}]")), "warnings/{name}: {}", out.stdout);
    }
}

#[test]
fn the_text_of_a_clean_probe_says_what_it_asks_for() {
    let dir = fixture_dir("ok", "acme-tools");
    let out = go(Path::new("/"), &["validate", dir.to_str().expect("utf-8")]);
    let text = out.stdout;
    assert!(text.starts_with("acme-tools 0.3.0 in "), "{text}");
    assert!(text.contains("  Acme tools\n"), "{text}");
    assert!(text.contains("contributes 4 rules, 2 commands, 1 status item, 1 agent"), "{text}");
    assert!(text.contains("What it asks for\n"));
    for line in [
        "Touch ID  process  git",
        "Touch ID  process  ./scripts/deploy.sh",
        "Touch ID  env      ACME_TOKEN",
        "Click     open     acme.atlassian.net",
        "Click     notify   show notifications",
    ] {
        assert!(text.contains(&format!("  {line}\n")), "{line}\n{text}");
    }
    assert!(
        text.contains("Dangerous combinations\n  ! can hand these secrets to the programs it may run"),
        "{text}"
    );
    assert!(
        text.contains(
            "Programs it may run\n  git                  read-only git\n  ./scripts/deploy.sh  own script\n"
        ),
        "{text}"
    );
    assert!(text.trim_end().ends_with("ok: no errors and no warnings"), "{text}");
    // Nothing about the findings when there are none.
    assert!(!text.contains("errors (") && !text.contains("warnings ("));
}

#[test]
fn every_program_is_told_by_what_it_can_do() {
    let tmp = TempDir::new().expect("tempdir");
    let dir = probe_in(
        tmp.path(),
        "p",
        "[permissions]\nprocess = [\"git\", \"./run.sh\", \"sh\", \"make\", \"jq\"]\n",
        &[("run.sh", "#!/bin/sh\n")],
    );
    let out = go(tmp.path(), &["validate", dir.to_str().expect("utf-8")]);
    let text = out.stdout;
    // The names are padded to the longest, `./run.sh`.
    for (program, words) in [
        ("git", "read-only git"),
        ("./run.sh", "own script"),
        ("sh", "can run anything"),
        ("make", "runs code the folder brings"),
        ("jq", "other program"),
    ] {
        let line = format!("  {program:<8}  {words}\n");
        assert!(text.contains(&line), "{line:?}\n{text}");
    }
}

#[test]
fn errors_come_before_warnings_each_with_code_path_and_message() {
    let tmp = TempDir::new().expect("tempdir");
    let dir = probe_in(
        tmp.path(),
        "p",
        "[[contributes.rules]]\nid = \"a\"\npattern = 'a*'\ntone = \"link\"\n\
         [[contributes.rules]]\nid = \"b\"\npattern = 'b'\n",
        &[],
    );
    let out = go(tmp.path(), &["validate", dir.to_str().expect("utf-8")]);
    assert_eq!(out.code, 1);
    let text = out.stdout;
    let errors = text.find("errors (1)").expect("errors");
    let warnings = text.find("warnings (1)").expect("warnings");
    assert!(errors < warnings, "{text}");
    assert!(
        text.contains(
            "  [pattern-empty-match] contributes.rules[0].pattern\n      can match the empty string"
        ),
        "{text}"
    );
    assert!(text.contains("  [rule-does-nothing] contributes.rules[1]\n"), "{text}");
    assert!(text.trim_end().ends_with("1 error, 1 warning"), "{text}");
}

#[test]
fn the_default_folder_is_the_working_folder() {
    let tmp = TempDir::new().expect("tempdir");
    let dir = probe_in(tmp.path(), "p", "", &[]);
    assert_eq!(go(&dir, &["validate"]).code, 0);
    assert_eq!(go(tmp.path(), &["validate", "p"]).code, 0, "relative to where it runs");
    assert_eq!(go(tmp.path(), &["validate", "./p/"]).code, 0);
}

#[test]
fn a_folder_that_is_not_a_probe_is_a_problem_not_a_mistake() {
    let tmp = TempDir::new().expect("tempdir");
    for (dir, said) in [("empty", "no probe.toml"), ("missing", "No such file")] {
        if dir == "empty" {
            fs::create_dir(tmp.path().join(dir)).expect("mkdir");
        }
        let out = go(tmp.path(), &["validate", dir]);
        assert_eq!(out.code, 1, "{dir}: {out:?}");
        assert!(out.stdout.is_empty());
        assert!(out.stderr.contains(said), "{}", out.stderr);
        let out = go(tmp.path(), &["validate", dir, "--json"]);
        assert_eq!(out.code, 1);
        let value = json(&out);
        assert_eq!(value["ok"], false);
        assert!(value["error"].as_str().expect("error").contains(said));
        assert!(value.get("findings").is_none());
    }
    fs::write(tmp.path().join("broken"), "x").expect("write");
    let out = go(tmp.path(), &["validate", "broken"]);
    assert_eq!((out.code, out.stderr.contains("is not a folder")), (1, true), "{out:?}");
}

#[test]
fn a_manifest_that_does_not_parse_says_where() {
    let tmp = TempDir::new().expect("tempdir");
    let dir = tmp.path().join("p");
    fs::create_dir(&dir).expect("mkdir");
    fs::write(dir.join("probe.toml"), "[probe\n").expect("write");
    let out = go(tmp.path(), &["validate", "p"]);
    assert_eq!(out.code, 1);
    assert!(out.stderr.contains("probe.toml:"), "{}", out.stderr);
}

#[test]
fn json_has_a_stable_shape() {
    let dir = fixture_dir("ok", "acme-tools");
    let out = go(Path::new("/"), &["validate", dir.to_str().expect("utf-8"), "--json"]);
    assert_eq!(out.code, 0);
    let value = json(&out);
    assert_eq!(value["version"], 1);
    assert_eq!(value["ok"], true);
    assert_eq!(value["folder"], dir.to_str().expect("utf-8"));
    assert_eq!(
        value["probe"],
        serde_json::json!({"id": "acme-tools", "name": "Acme tools", "version": "0.3.0", "level": "files"})
    );
    assert_eq!(value["counts"], serde_json::json!({"errors": 0, "warnings": 0}));
    assert_eq!(value["findings"], serde_json::json!([]));
    let summary = &value["summary"];
    assert_eq!(summary["has_code"], false);
    assert!(summary["permissions"].as_array().expect("array").contains(&serde_json::json!(
        {"key": "open", "detail": "acme.atlassian.net", "confirmation": "click"}
    )));
    assert!(summary["permissions"].as_array().expect("array").contains(&serde_json::json!(
        {"key": "process", "detail": "git", "confirmation": "touch_id"}
    )));
    assert_eq!(summary["combinations"].as_array().expect("array").len(), 1);
    assert_eq!(
        summary["programs"],
        serde_json::json!([
            {"program": "git", "class": "read_only_git", "description": "read-only git"},
            {"program": "./scripts/deploy.sh", "class": "own_script", "description": "own script"}
        ])
    );
}

#[test]
fn json_findings_carry_severity_code_path_and_message() {
    let dir = fixture_dir("errors", "capture-unanchored");
    let out = go(Path::new("/"), &["validate", "--json", dir.to_str().expect("utf-8")]);
    assert_eq!(out.code, 1);
    let value = json(&out);
    assert_eq!(value["ok"], false);
    assert_eq!(value["counts"]["errors"], 1);
    let finding = &value["findings"][0];
    assert_eq!(finding["severity"], "error");
    assert_eq!(finding["code"], "capture-unanchored");
    assert_eq!(finding["path"], "contributes.rules[0].action.command");
    assert!(finding["message"].as_str().expect("message").contains("argument"));
    let warnings = go(
        Path::new("/"),
        &["validate", "--json", fixture_dir("warnings", "rule-does-nothing").to_str().expect("utf-8")],
    );
    assert_eq!(json(&warnings)["findings"][0]["severity"], "warning");
    assert_eq!(warnings.code, 0);
}

#[test]
fn code_is_a_level_of_its_own_in_json() {
    let tmp = TempDir::new().expect("tempdir");
    let dir = probe_in(tmp.path(), "p", "main = \"src/index.ts\"\ncode_api = 0\n", &[("src/index.ts", "")]);
    let out = go(tmp.path(), &["validate", dir.to_str().expect("utf-8"), "--json"]);
    let value = json(&out);
    assert_eq!(value["probe"]["level"], "code");
    assert_eq!(value["summary"]["has_code"], true);
    assert!(
        value["summary"]["permissions"]
            .as_array()
            .expect("array")
            .iter()
            .any(|p| p["key"] == "main" && p["confirmation"] == "touch_id")
    );
}

// ---- hostile text ----------------------------------------------------------------------------

/// A manifest whose every text is meant to drive a terminal.
fn hostile(tmp: &Path) -> std::path::PathBuf {
    let dir = tmp.join("hostile");
    fs::create_dir_all(&dir).expect("mkdir");
    let manifest = HEAD
        .replace("\"Tiny\"", "\"Evil\\u001b[2J\\u001b]0;pwned\\u0007\\u202eesrever\\u200bx\"")
        .replace("\"d\"", "\"\\u009b31m red\\u2028\\u000a\"")
        + "\\u001bflavour = \"x\"\n";
    // The unknown key itself is hostile: it is quoted in a finding and in its path.
    let manifest = manifest.replace("\\u001bflavour = \"x\"\n", "\"fl\\u001b[31mavour\\u202e\" = \"x\"\n");
    fs::write(dir.join("probe.toml"), manifest).expect("write");
    dir
}

fn assert_clean(text: &str) {
    for c in text.chars() {
        assert!(!crate::probe::hidden::is_hidden(c) || c == '\n', "{:x} in {text:?}", u32::from(c));
    }
}

#[test]
fn hostile_text_in_a_manifest_is_written_out_in_the_text_output() {
    let tmp = TempDir::new().expect("tempdir");
    let dir = hostile(tmp.path());
    let out = go(tmp.path(), &["validate", dir.to_str().expect("utf-8")]);
    assert_eq!(out.code, 1, "{}", out.stdout);
    assert_clean(&out.stdout);
    assert!(
        out.stdout.contains("Evil\\u{1b}[2J\\u{1b}]0;pwned\\u{7}\\u{202e}esrever\\u{200b}x"),
        "{}",
        out.stdout
    );
    assert!(
        out.stdout.contains("probe.fl\\u{1b}[31mavour\\u{202e}"),
        "the key of a finding's path: {}",
        out.stdout
    );
}

#[test]
fn hostile_text_in_a_manifest_is_written_out_in_the_json_and_reads_back_unchanged() {
    let tmp = TempDir::new().expect("tempdir");
    let dir = hostile(tmp.path());
    let out = go(tmp.path(), &["validate", dir.to_str().expect("utf-8"), "--json"]);
    assert_eq!(out.code, 1);
    assert_clean(&out.stdout);
    let value = json(&out);
    assert_eq!(
        value["probe"]["name"], "Evil\u{1b}[2J\u{1b}]0;pwned\u{7}\u{202e}esrever\u{200b}x",
        "the data is the data"
    );
    assert!(
        value["findings"]
            .as_array()
            .expect("array")
            .iter()
            .any(|f| f["path"].as_str().is_some_and(|p| p.contains("\u{202e}")))
    );
}

#[test]
fn a_folder_name_that_drives_the_terminal_is_written_out_too() {
    let tmp = TempDir::new().expect("tempdir");
    let dir = tmp.path().join("bad\u{1b}[2Jname\u{202e}");
    fs::create_dir(&dir).expect("mkdir");
    for args in [&["validate"][..], &["validate", "--json"], &["test"]] {
        let mut words = args.to_vec();
        words.push(dir.to_str().expect("utf-8"));
        let out = go(tmp.path(), &words);
        assert_eq!(out.code, 1, "{args:?}");
        assert_clean(&out.stdout);
        assert_clean(&out.stderr.replacen("Wormhole: ", "", 1).replace('\n', ""));
        let seen = format!("{}{}", out.stdout, out.stderr);
        assert!(
            seen.contains("bad\\u{1b}[2Jname\\u{202e}") || seen.contains("bad\\u001b[2Jname\\u202e"),
            "{seen}"
        );
    }
}

#[test]
fn a_file_name_that_drives_the_terminal_stops_the_read_and_is_named_safely() {
    let tmp = TempDir::new().expect("tempdir");
    let dir = probe_in(tmp.path(), "p", "", &[("a\u{1b}[2Jb\u{202e}.txt", "x")]);
    let out = go(tmp.path(), &["validate", dir.to_str().expect("utf-8")]);
    assert_eq!(out.code, 1);
    assert_clean(&out.stderr.replace('\n', ""));
    assert!(out.stderr.contains("not a usable file name"), "{}", out.stderr);
}
