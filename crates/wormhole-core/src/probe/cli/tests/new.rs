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
fn a_probes_folder_that_is_not_there_yet_is_not_made_into_a_probe() {
    let tmp = TempDir::new().expect("tempdir");
    let data = tmp.path().join("data");
    fs::create_dir(&data).expect("mkdir");
    let probes = data.join("probes");
    let env = Env { cwd: tmp.path(), data_probes: Some(&probes), command: "wh" };
    let words = |args: &[&str]| args.iter().map(ToString::to_string).collect::<Vec<_>>();
    // The id is the folder's name: `probes` made in the data folder is where installed Probes go.
    let out = run(&words(&["new", "probes", "--dir", "data"]), &env);
    assert_eq!(out.code, 2, "{out:?}");
    assert!(out.stderr.contains("installed Probes live"), "{}", out.stderr);
    assert!(!probes.exists());
    // Through a link to the data folder, and through `..`, it is the same place.
    crate::platform::fs::symlink(&data, tmp.path().join("shortcut")).expect("link");
    for dir in ["shortcut", "data/../data"] {
        assert_eq!(run(&words(&["new", "probes", "--dir", dir]), &env).code, 2, "{dir}");
    }
    assert_eq!(fs::read_dir(&data).expect("lists").count(), 0);
    // Another name in the data folder is no concern of the guard.
    assert_eq!(run(&words(&["new", "other", "--dir", "data"]), &env).code, 0);
}

#[test]
fn the_next_step_names_the_folder_as_it_is_reached_from_here() {
    let tmp = TempDir::new().expect("tempdir");
    fs::create_dir(tmp.path().join("work")).expect("mkdir");
    let out = go(tmp.path(), &["new", "xx", "--dir", "work"]);
    assert!(out.stdout.contains("next: cd work/xx && wh probe validate"), "{}", out.stdout);
    let out = go(tmp.path(), &["new", "yy"]);
    assert!(out.stdout.contains("next: cd yy && wh probe validate"), "{}", out.stdout);
    // Outside the working folder it is the whole path.
    let elsewhere = TempDir::new().expect("tempdir");
    let abs = elsewhere.path().to_str().expect("utf-8");
    let out = go(tmp.path(), &["new", "zz", "--dir", abs]);
    assert!(out.stdout.contains(&format!("next: cd {abs}/zz && wh probe validate")), "{}", out.stdout);
}

#[test]
fn a_folder_with_a_space_or_a_hostile_name_is_one_word_and_written_out() {
    let tmp = TempDir::new().expect("tempdir");
    fs::create_dir(tmp.path().join("my work")).expect("mkdir");
    fs::create_dir(tmp.path().join("bad\u{1b}[2J")).expect("mkdir");
    let out = go(tmp.path(), &["new", "xx", "--dir", "my work"]);
    assert!(out.stdout.contains("next: cd 'my work/xx' && "), "{}", out.stdout);
    let out = go(tmp.path(), &["new", "xx", "--dir", "bad\u{1b}[2J"]);
    assert!(out.stdout.contains("next: cd 'bad\\u{1b}[2J/xx' && "), "{}", out.stdout);
    assert!(!out.stdout.contains('\u{1b}'));
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
