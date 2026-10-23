use std::path::{Path, PathBuf};
use std::process::Command;

use super::{
    COMMAND_NAME, Error, Folder, MARKER, Status, Targets, install, is_ours, keep_current_in, script_path,
    shim, status, terminal_dir_in, uninstall,
};

/// A stand-in for the app's binary that prints its arguments, one per line, in a folder whose name
/// needs quoting (a space and an apostrophe), like a person's `~/Apps/Bob's Mac/`.
fn fake_app(root: &Path) -> PathBuf {
    let dir = root.join("Bob's apps");
    std::fs::create_dir_all(&dir).expect("folder");
    let app = dir.join("Wormhole");
    std::fs::write(&app, "#!/bin/sh\nprintf '%s\\n' \"$@\"\n").expect("binary");
    std::fs::set_permissions(&app, crate::platform::fs::permissions(0o755)).expect("chmod");
    app
}

fn run(script: &Path, args: &[&str]) -> std::process::Output {
    Command::new(script).args(args).output().expect("the script runs")
}

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn two_folders(root: &Path) -> Targets {
    Targets {
        dirs: vec![
            Folder { path: root.join("system"), create: false },
            Folder { path: root.join("home/.local/bin"), create: true },
        ],
    }
}

#[test]
fn the_script_runs_the_binary_with_every_argument_as_it_was_given() {
    let root = tempfile::tempdir().expect("temp dir");
    let app = fake_app(root.path());
    let targets = Targets::only(root.path().join("bin"));
    let installed = install(&targets, &app).expect("installs");

    let out = run(&installed.path, &["notify", "--title", "two words", "it's", "$HOME", "*"]);
    assert_eq!(stdout(&out), "notify\n--title\ntwo words\nit's\n$HOME\n*\n");
    assert!(out.status.success());
}

#[test]
fn without_arguments_it_asks_for_the_usage_instead_of_starting_the_app() {
    let root = tempfile::tempdir().expect("temp dir");
    let app = fake_app(root.path());
    let installed = install(&Targets::only(root.path().join("bin")), &app).expect("installs");
    assert_eq!(stdout(&run(&installed.path, &[])), "help\n");
}

#[test]
fn a_binary_that_has_moved_is_said_so_with_the_exit_code_of_a_missing_command() {
    let root = tempfile::tempdir().expect("temp dir");
    let app = fake_app(root.path());
    let installed = install(&Targets::only(root.path().join("bin")), &app).expect("installs");
    std::fs::remove_file(&app).expect("the app moves away");

    let out = run(&installed.path, &["list"]);
    assert_eq!(out.status.code(), Some(127));
    let said = String::from_utf8_lossy(&out.stderr);
    assert!(said.starts_with(&format!("{COMMAND_NAME}: Wormhole is no longer at ")), "{said}");
    assert!(said.contains("Bob's apps/Wormhole"), "{said}");
}

#[test]
fn a_path_a_script_cannot_name_gets_no_script() {
    assert!(shim(Path::new("/Applications/Wormhole.app/Contents/MacOS/Wormhole")).is_some());
    assert!(shim(Path::new("/odd\nname/Wormhole")).is_none());
    assert!(
        shim(Path::new("Wormhole")).is_none(),
        "the script runs from anywhere: a relative path is no use"
    );
    assert!(shim(Path::new("")).is_none());
    assert!(shim(crate::platform::fs::path_from_bytes(b"/not/utf8/\xff")).is_none());
    let root = tempfile::tempdir().expect("temp dir");
    let refused = install(&Targets::only(root.path().to_path_buf()), Path::new("/odd\nname/Wormhole"));
    assert!(matches!(refused, Err(Error::Unusable)));
    assert!(!script_path(root.path()).exists(), "nothing was written");
}

#[test]
fn only_a_file_with_the_marker_is_ours() {
    let script = shim(Path::new("/opt/Wormhole")).expect("script");
    assert!(script.lines().nth(1) == Some(MARKER));
    assert!(is_ours(&script));
    // An earlier build's script, which ran another binary.
    assert!(is_ours(&format!("#!/bin/sh\n{MARKER}\nexec /old/Wormhole \"$@\"\n")));
    assert!(!is_ours("#!/bin/sh\nexec magic-wormhole \"$@\"\n"));
    assert!(!is_ours(""));
    assert!(!is_ours(&format!("#!/bin/sh\n{}\n", MARKER.trim_start_matches("# ")))); // not a comment line
}

#[test]
fn install_writes_an_executable_script_and_the_state_follows() {
    let root = tempfile::tempdir().expect("temp dir");
    let (old, new) = (Path::new("/opt/Old/Wormhole"), Path::new("/opt/New/Wormhole"));
    let targets = Targets::only(root.path().join("bin"));
    assert_eq!(status(&targets, old), Status::NotInstalled);

    let first = install(&targets, old).expect("installs");
    assert_eq!(first, super::Installed { path: root.path().join("bin").join(COMMAND_NAME), changed: true });
    let mode = crate::platform::fs::mode(&std::fs::metadata(&first.path).expect("written"));
    assert_eq!(mode & 0o100, 0o100, "executable");
    assert_eq!(status(&targets, old), Status::Installed { path: first.path.clone(), current: true });

    // Again: nothing to do. After a move of the app: the same file, updated.
    assert!(!install(&targets, old).expect("again").changed);
    assert_eq!(status(&targets, new), Status::Installed { path: first.path.clone(), current: false });
    assert!(install(&targets, new).expect("updates").changed);
    assert_eq!(status(&targets, new), Status::Installed { path: first.path.clone(), current: true });
    assert!(std::fs::read_to_string(&first.path).expect("read").contains("'/opt/New/Wormhole'"));
}

#[test]
fn another_wh_is_never_replaced_or_removed() {
    let root = tempfile::tempdir().expect("temp dir");
    let targets = Targets::only(root.path().join("bin"));
    let theirs = script_path(&root.path().join("bin"));
    std::fs::create_dir_all(theirs.parent().expect("folder")).expect("folder");
    std::fs::write(&theirs, "#!/bin/sh\necho theirs\n").expect("their script");

    let app = Path::new("/opt/Wormhole");
    assert_eq!(status(&targets, app), Status::Foreign { path: theirs.clone() });
    match install(&targets, app) {
        Err(Error::Foreign(path)) => assert_eq!(path, theirs),
        other => panic!("{other:?}"),
    }
    assert_eq!(uninstall(&targets).expect("nothing to remove"), Vec::<PathBuf>::new());
    assert_eq!(std::fs::read_to_string(&theirs).expect("read"), "#!/bin/sh\necho theirs\n");
}

#[test]
fn what_cannot_be_read_as_a_script_counts_as_another_wh() {
    let root = tempfile::tempdir().expect("temp dir");
    let targets = Targets::only(root.path().to_path_buf());
    // A folder, then a link to nowhere, then a program (not text) where the script goes.
    let spot = script_path(root.path());
    std::fs::create_dir(&spot).expect("a folder named wh");
    assert!(matches!(status(&targets, Path::new("/opt/W")), Status::Foreign { .. }));
    std::fs::remove_dir(&spot).expect("remove");
    crate::platform::fs::symlink(root.path().join("nowhere"), &spot).expect("dangling link");
    assert!(matches!(install(&targets, Path::new("/opt/W")), Err(Error::Foreign(_))));
    std::fs::remove_file(&spot).expect("remove");
    std::fs::write(&spot, [0xff, 0xfe, 0x00, 0x01]).expect("a binary");
    assert!(matches!(status(&targets, Path::new("/opt/W")), Status::Foreign { .. }));
}

#[test]
fn uninstall_removes_our_script_and_only_that() {
    let root = tempfile::tempdir().expect("temp dir");
    let targets = Targets::only(root.path().join("bin"));
    let app = Path::new("/opt/Wormhole");
    let installed = install(&targets, app).expect("installs");
    let neighbour = root.path().join("bin").join("other-tool");
    std::fs::write(&neighbour, "keep").expect("neighbour");

    assert_eq!(uninstall(&targets).expect("removes"), vec![installed.path.clone()]);
    assert!(!installed.path.exists() && neighbour.exists());
    assert_eq!(status(&targets, app), Status::NotInstalled);
    assert!(uninstall(&targets).expect("again").is_empty(), "nothing left is no error");
}

#[test]
fn a_folder_that_does_not_exist_is_skipped_unless_install_may_make_it() {
    let root = tempfile::tempdir().expect("temp dir");
    let targets = two_folders(root.path());
    let installed = install(&targets, Path::new("/opt/Wormhole")).expect("installs");
    assert_eq!(installed.path, root.path().join("home/.local/bin").join(COMMAND_NAME), "made it");

    // Once the system folder exists and takes it, that is where it goes.
    let other = tempfile::tempdir().expect("temp dir");
    std::fs::create_dir(other.path().join("system")).expect("system folder");
    let installed = install(&two_folders(other.path()), Path::new("/opt/Wormhole")).expect("installs");
    assert_eq!(installed.path, other.path().join("system").join(COMMAND_NAME));
    assert!(!other.path().join("home").exists(), "the second folder was not needed");
}

#[test]
fn a_system_folder_that_cannot_be_written_hands_over_to_the_home_folder() {
    let root = tempfile::tempdir().expect("temp dir");
    let system = root.path().join("system");
    std::fs::create_dir(&system).expect("system folder");
    std::fs::set_permissions(&system, crate::platform::fs::permissions(0o555)).expect("read only");
    // Root writes anywhere, so the case can't be made.
    if std::fs::write(system.join("probe"), "").is_ok() {
        return;
    }
    let installed = install(&two_folders(root.path()), Path::new("/opt/Wormhole")).expect("installs");
    assert_eq!(installed.path, root.path().join("home/.local/bin").join(COMMAND_NAME));
}

#[test]
fn a_script_of_ours_is_updated_where_it_is_and_another_wh_anywhere_blocks_install() {
    let root = tempfile::tempdir().expect("temp dir");
    let targets = two_folders(root.path());
    let home_script = root.path().join("home/.local/bin").join(COMMAND_NAME);
    install(&targets, Path::new("/opt/Old/Wormhole")).expect("installs in the home folder");
    // The system folder appears later: the script stays where it is, and is updated there.
    std::fs::create_dir(root.path().join("system")).expect("system folder");
    let updated = install(&targets, Path::new("/opt/New/Wormhole")).expect("updates");
    assert_eq!((updated.path.as_path(), updated.changed), (home_script.as_path(), true));
    assert!(!script_path(&root.path().join("system")).exists());

    // Somebody's `wh` in the system folder would come first on a PATH: that is in the way.
    std::fs::write(script_path(&root.path().join("system")), "theirs").expect("theirs");
    assert!(matches!(install(&targets, Path::new("/opt/New/Wormhole")), Err(Error::Foreign(_))));
    assert_eq!(
        status(&targets, Path::new("/opt/New/Wormhole")),
        Status::Installed { path: home_script, current: true },
        "ours is still what the state reports"
    );
}

#[test]
fn the_override_folder_replaces_the_system_ones() {
    let home = Path::new("/Users/me");
    let system = Targets::resolve(None, Some(home));
    let folders: Vec<&Path> = system.dirs.iter().map(|f| f.path.as_path()).collect();
    assert_eq!(folders, [Path::new("/usr/local/bin"), Path::new("/Users/me/.local/bin")]);
    assert_eq!(Targets::resolve(None, None).dirs.len(), 1, "no home, no second folder");

    let over = Targets::resolve(Some("/scratch/cli".into()), Some(home));
    assert_eq!(over.dirs.len(), 1);
    assert_eq!(over.dirs[0].path, Path::new("/scratch/cli"));
    assert_eq!(Targets::resolve(Some("".into()), Some(home)).dirs.len(), 2, "empty is unset");
}

#[test]
fn the_private_copy_is_rewritten_only_when_it_changes() {
    let root = tempfile::tempdir().expect("temp dir");
    let dir = root.path().join("data/bin");
    assert!(terminal_dir_in(dir.clone()).is_none(), "no script, no folder for the PATH");

    assert!(keep_current_in(&dir, Path::new("/opt/Old/Wormhole")).expect("writes"));
    assert!(
        !keep_current_in(&dir, Path::new("/opt/Old/Wormhole")).expect("same"),
        "unchanged: not rewritten"
    );
    assert!(keep_current_in(&dir, Path::new("/opt/New/Wormhole")).expect("moved app"));
    assert!(std::fs::read_to_string(script_path(&dir)).expect("read").contains("/opt/New/Wormhole"));
    assert_eq!(terminal_dir_in(dir.clone()), Some(dir));
}

#[test]
fn the_private_copy_never_replaces_another_wh_and_a_failed_write_is_an_error_not_a_panic() {
    let root = tempfile::tempdir().expect("temp dir");
    let dir = root.path().join("bin");
    std::fs::create_dir(&dir).expect("folder");
    std::fs::write(script_path(&dir), "theirs").expect("theirs");
    assert!(matches!(keep_current_in(&dir, Path::new("/opt/Wormhole")), Err(Error::Foreign(_))));
    assert_eq!(std::fs::read_to_string(script_path(&dir)).expect("read"), "theirs");

    // A data folder whose `bin` is a file: the script can't go in, which the caller logs.
    let blocked = root.path().join("blocked");
    std::fs::write(&blocked, "a file").expect("file");
    let failed = keep_current_in(&blocked.join("bin"), Path::new("/opt/Wormhole"));
    assert!(matches!(failed, Err(Error::Io { .. })), "{failed:?}");
    assert!(terminal_dir_in(blocked.join("bin")).is_none());
}
