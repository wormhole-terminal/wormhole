use std::fs;
use std::os::unix::fs::PermissionsExt as _;

use tempfile::TempDir;

use super::plan::{monogram_of, name_of};
use super::*;
use crate::probe::fixture;
use crate::probe::read::ProbeFolder;
use crate::probe::rules::CompiledRules;
use crate::probe::validate::validate;

#[test]
fn every_kind_validates_with_nothing_to_say_and_passes_its_own_test() {
    for kind in Kind::ALL {
        let tmp = TempDir::new().expect("tempdir");
        let made = create(tmp.path(), "my-probe", kind).unwrap_or_else(|err| panic!("{kind:?}: {err}"));
        assert_eq!(made.root, tmp.path().join("my-probe"));

        let folder = ProbeFolder::read(&made.root).unwrap_or_else(|err| panic!("{kind:?}: {err}"));
        let report = validate(&folder);
        assert_eq!(report.findings, [], "{kind:?}");

        let rules = CompiledRules::compile(&folder.manifest).expect("compiles");
        let tests = fixture::run(&folder, &rules);
        assert!(tests.is_ok(), "{kind:?}: {tests:#?}");
        let has_rules = !folder.manifest.contributes.rules.is_empty();
        assert_eq!(has_rules, matches!(kind, Kind::Rules | Kind::Command), "{kind:?}");
        assert_eq!(tests.cases.is_empty(), !has_rules, "{kind:?}: a case for every Probe that has rules");
    }
}

#[test]
fn the_preview_is_what_is_written() {
    for kind in Kind::ALL {
        let tmp = TempDir::new().expect("tempdir");
        let preview = preview("my-probe", kind).expect("preview");
        let made = create(tmp.path(), "my-probe", kind).expect("creates");
        let read = ProbeFolder::read(&made.root).expect("reads");
        assert_eq!(read.files, preview.files, "{kind:?}");
        assert_eq!(read.manifest, preview.manifest, "{kind:?}");
    }
}

#[test]
fn each_kind_brings_its_own_files() {
    let files = |kind| {
        let tmp = TempDir::new().expect("tempdir");
        create(tmp.path(), "my-probe", kind).expect("creates").files
    };
    let common = ["PROBE.md", "probe.toml", "tests/example.toml"];
    assert_eq!(files(Kind::Rules), common);
    assert_eq!(files(Kind::Status), common);
    assert_eq!(files(Kind::Command), ["PROBE.md", "probe.toml", "scripts/hello.sh", "tests/example.toml"]);
    assert_eq!(files(Kind::Agent), ["PROBE.md", "assets/icon.svg", "probe.toml", "tests/example.toml"]);
}

#[test]
fn the_kinds_contribute_what_they_are_named_for() {
    let contributes = |kind| preview("my-probe", kind).expect("preview").manifest.contributes;
    assert_eq!(contributes(Kind::Rules).rules.len(), 3);
    assert!(contributes(Kind::Rules).commands.is_empty());
    let command = contributes(Kind::Command);
    assert_eq!((command.rules.len(), command.commands.len()), (1, 2));
    assert_eq!(contributes(Kind::Status).status.len(), 1);
    let agent = contributes(Kind::Agent);
    assert_eq!(agent.agents.len(), 1);
    assert_eq!(agent.agents[0].command, "my-probe");
    assert_eq!(agent.agents[0].monogram, "MY");
}

#[test]
fn the_script_can_be_run_and_the_rest_cannot() {
    let tmp = TempDir::new().expect("tempdir");
    let made = create(tmp.path(), "my-probe", Kind::Command).expect("creates");
    let mode = |path: &str| fs::metadata(made.root.join(path)).expect("exists").permissions().mode();
    assert_ne!(mode("scripts/hello.sh") & 0o100, 0, "owner can execute");
    assert_eq!(mode("probe.toml") & 0o111, 0);
    let script = fs::read_to_string(made.root.join("scripts/hello.sh")).expect("reads");
    assert!(script.starts_with("#!/bin/sh\n") && script.contains("Hello from my-probe"), "{script}");
}

#[test]
fn probe_md_points_at_the_guide_and_lists_the_traps() {
    let tmp = TempDir::new().expect("tempdir");
    let made = create(tmp.path(), "my-probe", Kind::Rules).expect("creates");
    let text = fs::read_to_string(made.root.join("PROBE.md")).expect("reads");
    assert!(text.starts_with("# My probe"), "{}", &text[..40]);
    for needle in [
        "docs/probes.md",
        "wh probe validate",
        "wh probe test",
        "hostile input",
        "literal string",
        "(?-u:\\b)",
        "look-around",
        "Unknown keys are errors",
        "leftmost-longest",
        "[[case.expect]]",
        "percent-encoded",
        "Do not enable a Probe for them",
    ] {
        assert!(text.contains(needle), "PROBE.md lacks {needle:?}");
    }
    assert!(!text.contains("{{"), "every placeholder is filled");
    assert!(text.contains("Text rules."), "the notes of the kind are in");
}

#[test]
fn no_placeholder_is_left_in_any_file() {
    for kind in Kind::ALL {
        for file in preview("my-probe", kind).expect("preview").files {
            let text = file.text().expect("text");
            assert!(!text.contains("{{"), "{kind:?} {}", file.path);
        }
    }
}

#[test]
fn an_existing_folder_or_file_is_never_touched() {
    let tmp = TempDir::new().expect("tempdir");
    fs::create_dir(tmp.path().join("taken")).expect("mkdir");
    fs::write(tmp.path().join("taken/mine.txt"), "mine").expect("write");
    let err = create(tmp.path(), "taken", Kind::Rules).expect_err("refused");
    assert!(matches!(err, ScaffoldError::Exists(_)), "{err}");
    assert_eq!(fs::read_dir(tmp.path().join("taken")).expect("lists").count(), 1);
    assert_eq!(fs::read_to_string(tmp.path().join("taken/mine.txt")).expect("reads"), "mine");

    fs::write(tmp.path().join("file"), "x").expect("write");
    assert!(matches!(create(tmp.path(), "file", Kind::Rules), Err(ScaffoldError::Exists(_))));
    assert_eq!(fs::read_to_string(tmp.path().join("file")).expect("reads"), "x");

    // A link, even to nowhere, is something at the path.
    crate::platform::fs::symlink(tmp.path().join("nowhere"), tmp.path().join("dangling")).expect("link");
    assert!(matches!(create(tmp.path(), "dangling", Kind::Rules), Err(ScaffoldError::Exists(_))));
    assert!(!tmp.path().join("nowhere").exists());
}

#[test]
fn the_parent_must_be_a_folder_that_is_there() {
    let tmp = TempDir::new().expect("tempdir");
    let missing = tmp.path().join("missing");
    assert!(matches!(create(&missing, "my-probe", Kind::Rules), Err(ScaffoldError::NoParent(_))));
    assert!(!missing.exists(), "no folder is made on the way");
    fs::write(tmp.path().join("file"), "x").expect("write");
    assert!(matches!(
        create(&tmp.path().join("file"), "my-probe", Kind::Rules),
        Err(ScaffoldError::NoParent(_))
    ));
}

#[test]
fn only_an_id_makes_a_folder() {
    let tmp = TempDir::new().expect("tempdir");
    for bad in ["", "a", "A-b", "-ab", "ab-", "a b", "a/b", "../x", ".", "..", &"a".repeat(33), "a\u{1b}b"] {
        assert!(matches!(create(tmp.path(), bad, Kind::Rules), Err(ScaffoldError::BadId(_))), "{bad:?}");
    }
    assert_eq!(fs::read_dir(tmp.path()).expect("lists").count(), 0);
    for ok in ["ab", "a1", "my-probe", &"a".repeat(32), "12"] {
        for kind in Kind::ALL {
            let sub = TempDir::new().expect("tempdir");
            create(sub.path(), ok, kind).unwrap_or_else(|err| panic!("{ok} {kind:?}: {err}"));
        }
    }
}

#[test]
fn an_id_that_would_make_the_example_invalid_is_refused_before_anything_is_written() {
    let tmp = TempDir::new().expect("tempdir");
    // `claude` is a built-in agent's name, and the agent example is run as the id.
    let err = create(tmp.path(), "claude", Kind::Agent).expect_err("refused");
    assert!(
        matches!(&err, ScaffoldError::Invalid(findings) if findings[0].code == "agent-builtin-name"),
        "{err}"
    );
    assert_eq!(fs::read_dir(tmp.path()).expect("lists").count(), 0);
    assert!(create(tmp.path(), "claude", Kind::Rules).is_ok(), "other kinds do not run the id as a program");
}

#[test]
fn words_are_made_from_the_id() {
    assert_eq!(name_of("my-probe"), "My probe");
    assert_eq!(name_of("ab"), "Ab");
    assert_eq!(name_of("9lives"), "9lives");
    assert_eq!(monogram_of("my-probe"), "MY");
    assert_eq!(monogram_of("9a-b"), "AB");
    assert_eq!(monogram_of("x9"), "XX");
    assert_eq!(monogram_of("12"), "AG");
}

#[test]
fn a_kind_is_known_by_its_word() {
    for kind in Kind::ALL {
        assert_eq!(Kind::from_word(kind.word()), Some(kind));
    }
    assert_eq!(Kind::from_word("code"), None, "code Probes are the next level");
    assert_eq!(Kind::from_word(""), None);
}
