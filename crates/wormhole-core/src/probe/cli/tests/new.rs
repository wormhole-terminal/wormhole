use std::fs;

use tempfile::TempDir;

use super::{Env, go, run};

#[test]
fn every_kind_made_by_the_command_line_validates_and_passes_its_own_test() {
    for kind in ["rules", "command", "status", "agent"] {
        let tmp = TempDir::new().expect("tempdir");
        let made = go(tmp.path(), &["new", "my-probe", "--kind", kind]);
        assert_eq!(made.code, 0, "{kind}: {made:?}");
        assert!(made.stdout.contains(&format!("({kind} Probe, ")), "{}", made.stdout);
        assert!(made.stdout.contains("  probe.toml\n"), "{}", made.stdout);
        assert!(
            made.stdout.contains("next: cd my-probe && wh probe validate && wh probe test"),
            "{}",
            made.stdout
        );

        let checked = go(tmp.path(), &["validate", "my-probe"]);
        assert_eq!(checked.code, 0, "{kind}: {checked:?}");
        assert!(checked.stdout.contains("ok: no errors and no warnings"), "{kind}: {}", checked.stdout);
        let tested = go(tmp.path(), &["test", "my-probe"]);
        assert_eq!(tested.code, 0, "{kind}: {tested:?}");
        for json in ["validate", "test"] {
            let out = go(tmp.path(), &[json, "my-probe", "--json"]);
            assert_eq!((out.code, out.stdout.contains("\"ok\":true")), (0, true), "{kind} {json}");
        }
    }
}

#[test]
fn the_default_kind_is_rules_and_the_default_parent_is_the_working_folder() {
    let tmp = TempDir::new().expect("tempdir");
    let out = go(tmp.path(), &["new", "my-probe"]);
    assert_eq!(out.code, 0, "{out:?}");
    assert!(out.stdout.contains("(rules Probe, 3 files)"), "{}", out.stdout);
    assert!(tmp.path().join("my-probe/tests/example.toml").exists());
}

#[test]
fn dir_names_the_parent_and_is_read_against_the_working_folder() {
    let tmp = TempDir::new().expect("tempdir");
    fs::create_dir(tmp.path().join("work")).expect("mkdir");
    let out = go(tmp.path(), &["new", "my-probe", "--dir", "work"]);
    assert_eq!(out.code, 0, "{out:?}");
    assert!(tmp.path().join("work/my-probe/probe.toml").exists());
    assert!(!tmp.path().join("my-probe").exists());
    let abs = tmp.path().join("work");
    assert_eq!(go(tmp.path(), &["new", "other", "--dir", abs.to_str().expect("utf-8")]).code, 0);
    assert!(abs.join("other/probe.toml").exists());
}

#[test]
fn a_folder_that_exists_is_refused_and_left_as_it_was() {
    let tmp = TempDir::new().expect("tempdir");
    assert_eq!(go(tmp.path(), &["new", "my-probe"]).code, 0);
    fs::write(tmp.path().join("my-probe/probe.toml"), "mine").expect("write");
    let again = go(tmp.path(), &["new", "my-probe", "--kind", "agent"]);
    assert_eq!(again.code, 1, "{again:?}");
    assert!(again.stderr.contains("already exists; nothing was changed"), "{}", again.stderr);
    assert_eq!(fs::read_to_string(tmp.path().join("my-probe/probe.toml")).expect("reads"), "mine");
    assert!(!tmp.path().join("my-probe/assets").exists());
}

#[test]
fn a_parent_that_is_not_there_is_a_mistake_in_the_arguments() {
    let tmp = TempDir::new().expect("tempdir");
    let out = go(tmp.path(), &["new", "my-probe", "--dir", "nowhere"]);
    assert_eq!(out.code, 2, "{out:?}");
    assert!(out.stderr.contains("is not a folder"), "{}", out.stderr);
    assert!(!tmp.path().join("nowhere").exists());
}

#[test]
fn an_id_the_example_cannot_have_is_a_problem() {
    let tmp = TempDir::new().expect("tempdir");
    let out = go(tmp.path(), &["new", "claude", "--kind", "agent"]);
    assert_eq!(out.code, 1, "{out:?}");
    assert!(out.stderr.contains("would not validate"), "{}", out.stderr);
    assert_eq!(fs::read_dir(tmp.path()).expect("lists").count(), 0);
}

#[test]
fn the_folder_of_installed_probes_is_never_scaffolded_in() {
    let tmp = TempDir::new().expect("tempdir");
    let probes = tmp.path().join("data/probes");
    fs::create_dir_all(probes.join("nested")).expect("mkdir");
    let env = Env { cwd: tmp.path(), data_probes: Some(&probes), command: "wh" };
    let words = |args: &[&str]| args.iter().map(ToString::to_string).collect::<Vec<_>>();
    for dir in ["data/probes", "data/probes/nested", "data/probes/../probes", "data/probes/nested/.."] {
        let out = run(&words(&["new", "my-probe", "--dir", dir]), &env);
        assert_eq!(out.code, 2, "{dir}: {out:?}");
        assert!(out.stderr.contains("installed Probes live"), "{}", out.stderr);
    }
    assert_eq!(fs::read_dir(&probes).expect("lists").count(), 1, "only the folder that was there");
    // A link that leads there is the same place.
    crate::platform::fs::symlink(&probes, tmp.path().join("shortcut")).expect("link");
    assert_eq!(run(&words(&["new", "my-probe", "--dir", "shortcut"]), &env).code, 2);
    // The data folder around it is not the Probes folder.
    assert_eq!(run(&words(&["new", "my-probe", "--dir", "data"]), &env).code, 0);
}

#[test]
fn the_command_is_named_as_the_caller_calls_it() {
    let tmp = TempDir::new().expect("tempdir");
    let env = Env { cwd: tmp.path(), data_probes: None, command: "Wormhole" };
    let out = run(&["new".to_string(), "my-probe".to_string()], &env);
    assert!(
        out.stdout.contains("next: cd my-probe && Wormhole probe validate && Wormhole probe test"),
        "{}",
        out.stdout
    );
}
