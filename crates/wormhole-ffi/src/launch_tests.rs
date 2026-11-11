//! What a terminal runs and how its engine starts (`launch`), over a scratch data folder: the shell the GPUI app's
//! window would pick, a trusted project's environment and shell and only a trusted one's, the profile's environment,
//! Settings' engine options, and a real shell started with all of it.

use std::path::Path;
use std::time::{Duration, Instant};

use wormhole_core::config::profile::ProfileList;
use wormhole_core::project::{ProjectFolder, TrustStore};
use wormhole_core::storage::paths::DataRoot;
use wormhole_core::terminal::shells;
use wormhole_term::GridSize;

use crate::hosting::HostedStart;
use crate::launch::{options, profile_in};

fn scratch(name: &str) -> tempfile::TempDir {
    tempfile::Builder::new()
        .prefix(&format!("Wormhole-swift-launch-{name}-"))
        .tempdir_in("/tmp")
        .expect("scratch")
}

fn request(cwd: Option<&Path>) -> HostedStart {
    HostedStart {
        tab: 7,
        shell: None,
        program: None,
        args: Vec::new(),
        cwd: cwd.map(|dir| dir.display().to_string()),
        cols: 80,
        rows: 24,
        cell_width: 8.0,
        cell_height: 16.0,
        old_host: None,
        saved_text: None,
        profile: None,
    }
}

fn config(root: &DataRoot, json: &str) {
    std::fs::write(root.base().join("config.json"), json).expect("config");
}

#[test]
fn the_person_s_login_shell_starts_as_a_login_shell_and_settings_reach_the_engine() {
    let dir = scratch("login");
    let root = DataRoot::new(dir.path());
    config(
        &root,
        r#"{"scrollback_lines": 1234, "image_memory_mib": 64, "allow_window_ops": true, "allow_title_report": true,
            "allow_screen_checksum": true}"#,
    );
    let options = options(&root, &profile_in(&root, None), &request(Some(dir.path())));
    let shell = options.shell.expect("a shell");
    assert_eq!((shell.program, shell.args), (shells::login_shell(), vec!["-l".to_string()]));
    assert_eq!(options.tab_id, "7");
    assert_eq!(options.scrollback_lines, 1234);
    assert_eq!(options.image_budget, Some(64 * 1024 * 1024));
    assert!(options.allow_window_ops && options.allow_title_report && options.allow_screen_checksum);
    assert_eq!(options.cwd.as_deref(), Some(dir.path()));
    // A folder that is not there starts where the process is.
    let gone = options_of(&root, Some(&dir.path().join("gone")));
    assert_eq!(gone.cwd, None);
}

fn options_of(root: &DataRoot, cwd: Option<&Path>) -> wormhole_core::terminal::SpawnOptions {
    options(root, &profile_in(root, None), &request(cwd))
}

#[test]
fn the_terminal_s_own_shell_wins_then_default_shell_and_a_named_program_runs_as_it_is() {
    let dir = scratch("shells");
    let root = DataRoot::new(dir.path());
    config(
        &root,
        r#"{"default_shell": "/bin/sh", "shells": [{"name": "fish-ish", "command": "/bin/bash", "args": ["--noprofile"]}]}"#,
    );
    let profile = profile_in(&root, None);
    let default = options(&root, &profile, &request(None)).shell.expect("a shell");
    assert_eq!(
        (default.program.as_str(), default.args.as_slice()),
        ("/bin/sh", ["-l".to_string()].as_slice())
    );
    let chosen = options(&root, &profile, &HostedStart { shell: Some("fish-ish".into()), ..request(None) });
    let chosen = chosen.shell.expect("a shell");
    assert_eq!((chosen.program.as_str(), chosen.name.as_str()), ("/bin/bash", "bash"));
    assert_eq!(chosen.args, ["--noprofile"]);
    // A profile that is gone falls back to the default, as core says.
    let gone = options(&root, &profile, &HostedStart { shell: Some("nothing".into()), ..request(None) });
    assert_eq!(gone.shell.map(|s| s.program), Some("/bin/sh".into()));
    let named = options(&root, &profile, &HostedStart { program: Some("/bin/cat".into()), ..request(None) });
    let named = named.shell.expect("a shell");
    assert_eq!((named.program.as_str(), named.args.len(), named.name.as_str()), ("/bin/cat", 0, "cat"));
    // The terminal's own shell still wins over it (New Terminal with ▸ in a `--shell` run).
    let both =
        HostedStart { program: Some("/bin/cat".into()), shell: Some("fish-ish".into()), ..request(None) };
    assert_eq!(options(&root, &profile, &both).shell.map(|s| s.program), Some("/bin/bash".into()));
}

#[test]
fn only_a_trusted_project_sets_the_environment_and_the_shell() {
    let dir = scratch("project");
    let root = DataRoot::new(dir.path());
    let project = dir.path().join("app");
    std::fs::create_dir_all(project.join(".wormhole")).expect("project folder");
    std::fs::write(
        project.join(".wormhole/config.json"),
        r#"{"env": {"PROJECT_FLAG": "on", "WORMHOLE_TAB_ID": "spoofed"}, "shell": "/bin/sh"}"#,
    )
    .expect("project config");
    let untrusted = options_of(&root, Some(&project));
    assert!(untrusted.env.get("PROJECT_FLAG").is_none(), "an untrusted project sets nothing");
    assert_eq!(untrusted.shell.map(|s| s.program), Some(shells::login_shell()));

    let profile = profile_in(&root, None);
    let folder = ProjectFolder::open(&project.canonicalize().expect("canonical")).expect("a project");
    let mut trust = TrustStore::load(profile.dir()).expect("trust store");
    trust.trust(&folder);
    trust.save().expect("saves");
    let trusted = options_of(&root, Some(&project));
    assert_eq!(trusted.env.get("PROJECT_FLAG").map(String::as_str), Some("on"));
    assert_eq!(trusted.shell.map(|s| (s.program, s.args)), Some(("/bin/sh".into(), vec!["-l".into()])));

    // Project folders off: nothing, trusted or not.
    config(&root, r#"{"feature_flags": {"project_folders": false}}"#);
    assert!(options_of(&root, Some(&project)).env.get("PROJECT_FLAG").is_none());
}

/// A real shell, started as the window starts it: the shell `default_shell` names as a login shell, the profile's
/// environment and git identity, the tab's own variables a project cannot redirect, and the saved text above it.
#[test]
fn a_real_shell_starts_with_the_profile_s_environment_above_its_saved_text() {
    let dir = scratch("real");
    let root = DataRoot::new(dir.path());
    let script = dir.path().join("myshell");
    std::fs::write(
        &script,
        "#!/bin/sh\nprintf '[%s] [%s] [%s] [%s] [%s]\\n' \"$1\" \"$AWS_PROFILE\" \"$GIT_AUTHOR_EMAIL\" \"$WORMHOLE_PROFILE\" \"$WORMHOLE_TAB_ID\"\nsleep 5\n",
    )
    .expect("script");
    std::fs::set_permissions(&script, std::os::unix::fs::PermissionsExt::from_mode(0o755))
        .expect("executable");
    config(&root, &format!(r#"{{"default_shell": "{}"}}"#, script.display()));
    let (mut list, _) = ProfileList::load_or_recover(&root);
    let id = list.active_id().clone();
    list.edit(&id, |profile| {
        profile.env.insert("AWS_PROFILE".into(), "acme".into());
        profile.git.email = "ada@example.com".into();
    })
    .expect("edits");
    list.save(&root).expect("saves");

    let what = HostedStart { saved_text: Some("before-the-restart".into()), ..request(Some(dir.path())) };
    let options = options(&root, &profile_in(&root, None), &what);
    let size = GridSize { cols: 100, rows: 24, cell_width: 8.0, cell_height: 16.0 };
    let (pty, _events) = wormhole_core::terminal::spawn(size, &options).expect("spawns");
    let deadline = Instant::now() + Duration::from_secs(10);
    while !pty.screen_text().contains("[7]") {
        assert!(Instant::now() < deadline, "timed out:\n{}", pty.screen_text());
        std::thread::sleep(Duration::from_millis(10));
    }
    let screen = pty.screen_text();
    assert!(screen.contains("[-l] [acme] [ada@example.com] [default] [7]"), "{screen}");
    assert!(screen.contains("before-the-restart"), "the saved text is drawn above the shell:\n{screen}");
}

/// A program asks the window to resize itself (XTWINOPS 8) and to move (XTWINOPS 3): with `allow_window_ops` off (the
/// default) nothing reaches the window and the engine keeps its size; switched on in Settings, both reach Swift.
#[test]
fn a_program_acts_on_the_window_only_when_settings_allow_it() {
    use std::sync::Arc;

    use crate::events::{TermEvent, TerminalListener};
    use crate::terminal::Terminal;
    use crate::window_ops::{ExtentRecord, WindowOpRecord};

    struct Quiet;
    impl TerminalListener for Quiet {
        fn wake(&self) {}
    }

    let ops = |allowed: bool| -> (Vec<WindowOpRecord>, String) {
        let dir = scratch("ops");
        let root = DataRoot::new(dir.path());
        config(&root, &format!(r#"{{"allow_window_ops": {allowed}}}"#));
        let script = r"printf '\033[8;30;100t\033[3;10;20t'; sleep 1; stty size; sleep 5";
        let what = HostedStart {
            program: Some("/bin/sh".into()),
            args: vec!["-c".into(), script.into()],
            ..request(None)
        };
        let options = options(&root, &profile_in(&root, None), &what);
        assert_eq!(options.allow_window_ops, allowed);
        let size = GridSize { cols: 80, rows: 24, cell_width: 8.0, cell_height: 16.0 };
        let (pty, events) = wormhole_core::terminal::spawn(size, &options).expect("spawns");
        let terminal =
            Terminal::from_parts(pty, events, &options, Arc::new(Quiet), None).expect("a terminal");
        let (mut seen, deadline) = (Vec::new(), Instant::now() + Duration::from_secs(10));
        while !terminal.screen_text().split_whitespace().any(|word| word == "80" || word == "100") {
            assert!(Instant::now() < deadline, "timed out:\n{}", terminal.screen_text());
            std::thread::sleep(Duration::from_millis(20));
        }
        std::thread::sleep(Duration::from_millis(100));
        for event in terminal.drain_events() {
            if let TermEvent::WindowOp { op } = event {
                seen.push(op);
            }
        }
        (seen, terminal.screen_text())
    };

    let (seen, screen) = ops(false);
    assert!(seen.is_empty(), "nothing reaches the window: {seen:?}");
    assert!(screen.contains("24 80"), "the engine kept its size:\n{screen}");
    let (seen, _) = ops(true);
    assert_eq!(
        seen,
        [
            WindowOpRecord::ResizeCells {
                cols: ExtentRecord::Size { cells: 100 },
                rows: ExtentRecord::Size { cells: 30 }
            },
            WindowOpRecord::Move { x: 10, y: 20 },
        ]
    );
}
