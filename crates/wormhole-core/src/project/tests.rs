//! Tests of the folder loaders and their safety rules, and helpers the other project tests share.
use super::*;
use crate::folder_color::FolderColor;
use serde_json::json;
use std::cell::Cell;
use std::fs;
use tempfile::TempDir;

thread_local! {
    /// Files and directories read from project folders on this thread.
    static READS: Cell<usize> = const { Cell::new(0) };
}

pub(crate) fn count_read() {
    READS.with(|reads| reads.set(reads.get() + 1));
}

/// Reads by `f` (tests run each on their own thread).
pub(crate) fn reads_during(f: impl FnOnce()) -> usize {
    let before = READS.with(Cell::get);
    f();
    READS.with(Cell::get) - before
}

/// Trust lines as the dialog's plain text writes them.
pub(crate) fn strings(lines: &[TrustLine]) -> Vec<String> {
    lines.iter().map(ToString::to_string).collect()
}

pub(crate) fn project(files: &[(&str, &str)]) -> (TempDir, ProjectFolder) {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path().canonicalize().expect("canonical");
    fs::create_dir_all(root.join(DIR_NAME)).expect("mkdir");
    for (name, text) in files {
        let path = root.join(DIR_NAME).join(name);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(path, text).expect("write");
    }
    let folder = ProjectFolder::open(&root).expect("folder");
    (tmp, folder)
}

#[test]
fn a_project_folder_from_before_the_rename_is_still_read() {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path().canonicalize().expect("canonical");
    let old = root.join(LEGACY_DIR_NAME);
    fs::create_dir_all(&old).expect("mkdir");
    fs::write(old.join(CONFIG_FILE), r#"{ "font_size": 20 }"#).expect("write");
    let found = find_project(&root.join(".")).expect("found");
    assert_eq!((found.dir.clone(), found.dir_name()), (old, ".andromeda".to_string()));
    let config = found.config().expect("reads").expect("a config");
    assert_eq!(config.settings.get("font_size"), Some(&json!(20)));

    // Once there is a `.wormhole/`, it is the project folder.
    fs::create_dir_all(root.join(DIR_NAME)).expect("mkdir");
    let found = find_project(&root).expect("found");
    assert_eq!((found.dir_name(), found.dir), (".wormhole".to_string(), root.join(DIR_NAME)));
}

#[test]
fn discovery_walks_up_to_nearest() {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path().canonicalize().expect("canonical");
    let deep = root.join("a/b/c");
    fs::create_dir_all(&deep).expect("mkdir");
    fs::create_dir_all(root.join(DIR_NAME)).expect("mkdir");
    assert_eq!(find_project(&deep).map(|p| p.root), Some(root.clone()));
    // Nearest wins over outer.
    fs::create_dir_all(root.join("a").join(DIR_NAME)).expect("mkdir");
    assert_eq!(find_project(&deep).map(|p| p.root), Some(root.join("a")));
    // A file as start uses its parent.
    fs::write(deep.join("f.txt"), "").expect("write");
    assert_eq!(find_project(&deep.join("f.txt")).map(|p| p.root), Some(root.join("a")));
    // Non-canonical input yields a canonical root.
    assert_eq!(find_project(&deep.join("..").join("c")).map(|p| p.root), Some(root.join("a")));
}

#[test]
fn discovery_stops_at_git_root() {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path().canonicalize().expect("canonical");
    fs::create_dir_all(root.join(DIR_NAME)).expect("mkdir");
    let repo = root.join("repo");
    fs::create_dir_all(repo.join("src")).expect("mkdir");
    fs::create_dir_all(repo.join(".git")).expect("mkdir");
    assert_eq!(find_project(&repo.join("src")), None, "folder above the repo must not leak in");
    // `.wormhole` in the git root itself is found.
    fs::create_dir_all(repo.join(DIR_NAME)).expect("mkdir");
    assert_eq!(find_project(&repo.join("src")).map(|p| p.root), Some(repo.clone()));
    // `.git` as a file (worktree/submodule) also bounds the walk.
    let wt = root.join("wt");
    fs::create_dir_all(&wt).expect("mkdir");
    fs::write(wt.join(".git"), "gitdir: x").expect("write");
    assert_eq!(find_project(&wt), None);
}

#[test]
fn discovery_ignores_wormhole_file_and_missing_dirs() {
    let tmp = TempDir::new().expect("tempdir");
    let root = tmp.path().canonicalize().expect("canonical");
    fs::create_dir_all(root.join(".git")).expect("mkdir");
    fs::write(root.join(DIR_NAME), "not a dir").expect("write");
    assert_eq!(find_project(&root), None);
    assert_eq!(find_project(&root.join("does-not-exist")), None);
}

#[test]
fn absent_files_are_none() {
    let (_tmp, p) = project(&[]);
    assert!(p.config().expect("ok").is_none());
    assert!(p.theme_json().expect("ok").is_none());
    assert!(p.theme_path().is_none());
    assert!(p.actions().expect("ok").is_none());
    assert!(p.prompts().expect("ok").is_none());
    assert!(p.layout().expect("ok").is_none());
    assert!(p.notes().expect("ok").is_none());
    assert!(Feature::ALL.iter().all(|f| !p.has(*f)));
}

#[test]
fn config_splits_env_and_shell() {
    let text = r#"{
        // project overrides
        "$schema": "x",
        "font_size": 13,
        "env": { "RUST_LOG": "debug", },
        "shell": "/bin/zsh",
    }"#;
    let (_tmp, p) = project(&[(CONFIG_FILE, text)]);
    let config = p.config().expect("ok").expect("present");
    assert_eq!(serde_json::Value::Object(config.settings), json!({"font_size": 13}));
    assert_eq!(config.env.get("RUST_LOG").map(String::as_str), Some("debug"));
    assert_eq!(config.shell.as_deref(), Some("/bin/zsh"));
    assert!(p.has(Feature::Settings));
}

#[test]
fn config_errors_carry_position() {
    let (_tmp, p) = project(&[(CONFIG_FILE, "{\n  \"a\": 1\n  \"b\": 2\n}")]);
    match p.config() {
        Err(ProjectError::Parse { path, line, column, .. }) => {
            assert!(path.ends_with("config.json"));
            assert_eq!((line, column), (3, 3));
        }
        other => panic!("expected parse error, got {other:?}"),
    }
    let (_tmp, p) = project(&[(CONFIG_FILE, r#"{"env": {"A": 1}}"#)]);
    assert!(matches!(p.config(), Err(ProjectError::Invalid { .. })));
    let (_tmp, p) = project(&[(CONFIG_FILE, r#"{"shell": 3}"#)]);
    assert!(matches!(p.config(), Err(ProjectError::Invalid { .. })));
    let (_tmp, p) = project(&[(CONFIG_FILE, "[1]")]);
    assert!(p.config().expect_err("not object").to_string().contains("config.json"));
    let (_tmp, p) = project(&[(CONFIG_FILE, r#"{"env": {"A=B": "x"}}"#)]);
    assert!(matches!(p.config(), Err(ProjectError::Invalid { .. })));
}

#[test]
fn theme_is_opaque() {
    let (_tmp, p) = project(&[(THEME_FILE, r#"{"name": "Dusk", /* c */ "colors": {},}"#)]);
    assert_eq!(p.theme_json().expect("ok"), Some(json!({"name": "Dusk", "colors": {}})));
    assert_eq!(p.theme_path(), Some(p.dir.join(THEME_FILE)));
    let (_tmp, p) = project(&[(THEME_FILE, "{")]);
    assert!(matches!(p.theme_json(), Err(ProjectError::Parse { .. })));
}

#[test]
fn layout_loads_and_validates() {
    let text = r#"{"sessions": [
        {"name": "Claude", "run": "claude", "focus": true},
        {"name": "Web", "run": "npm run dev", "cwd": "frontend"},
    ]}"#;
    let (_tmp, p) = project(&[(LAYOUT_FILE, text)]);
    let layout = p.layout().expect("ok").expect("present");
    assert_eq!(layout.sessions.len(), 2);
    assert!(layout.sessions[0].focus);
    assert_eq!(p.resolve_cwd(layout.sessions[1].cwd.as_deref()), p.root.join("frontend"));

    for bad in [
        r#"{"sessions": [{"name": "A"}, {"name": "A"}]}"#,
        r#"{"sessions": [{"name": ""}]}"#,
        r#"{"sessions": [{"name": "A", "focus": true}, {"name": "B", "focus": true}]}"#,
        r#"{"sessions": [{"name": "A", "cwd": "../elsewhere"}]}"#,
        r#"{"sessions": [{"name": "A", "cwd": "/etc"}]}"#,
    ] {
        let (_tmp, p) = project(&[(LAYOUT_FILE, bad)]);
        assert!(matches!(p.layout(), Err(ProjectError::Invalid { .. })), "{bad}");
    }
    let (_tmp, p) = project(&[(LAYOUT_FILE, r#"{"sessions": [{"name": "A", "rnu": "x"}]}"#)]);
    assert!(matches!(p.layout(), Err(ProjectError::Parse { line: 1, .. })));
}

#[test]
fn notes_read_write() {
    let (_tmp, p) = project(&[]);
    let doc = NotesDoc {
        tasks: vec![TaskItem { text: "Ship it".into(), done: false }],
        notes: vec!["Hello".into()],
    };
    assert!(p.write_notes(&doc).expect("write"));
    assert!(!p.write_notes(&doc).expect("write"), "unchanged content is not rewritten");
    assert_eq!(p.notes().expect("ok"), Some(doc));
    assert!(!p.dir.join("notes.md.tmp").exists());
}

/// A cloned repository commits `notes.md.tmp -> ~/.ssh/authorized_keys`; syncing the
/// notes (on by default, no trust needed) must not write through it.
#[cfg(unix)]
#[test]
fn a_planted_temp_symlink_does_not_redirect_the_notes_write() {
    let (_tmp, p) = project(&[(NOTES_FILE, "# Notes\n")]);
    let home = TempDir::new().expect("tempdir");
    let victim = home.path().join("authorized_keys");
    fs::write(&victim, "ssh-ed25519 AAAA owner-key\n").expect("write");
    for name in ["notes.md.tmp", "notes.md.wormhole-tmp", ".notes.md.tmp"] {
        std::os::unix::fs::symlink(&victim, p.dir.join(name)).expect("symlink");
    }
    let doc = NotesDoc { tasks: Vec::new(), notes: vec!["ssh-ed25519 AAAA attacker-key".into()] };
    assert!(p.write_notes(&doc).expect("write"));
    assert_eq!(fs::read_to_string(&victim).expect("read"), "ssh-ed25519 AAAA owner-key\n");
    let meta = fs::symlink_metadata(p.notes_path()).expect("stat");
    assert!(meta.file_type().is_file(), "notes.md is a regular file in the repo");
    assert_eq!(p.notes().expect("ok"), Some(doc));
}

/// A file outside the project with content that would load as `name`.
#[cfg(unix)]
fn link_outside(p: &ProjectFolder, name: &str, content: &str) -> (TempDir, PathBuf) {
    let outside = TempDir::new().expect("tempdir");
    let secret = outside.path().join("secret");
    fs::write(&secret, content).expect("write");
    let link = p.dir.join(name);
    fs::create_dir_all(link.parent().expect("parent")).expect("mkdir");
    std::os::unix::fs::symlink(&secret, &link).expect("symlink");
    (outside, secret)
}

fn assert_refused<T: std::fmt::Debug>(result: Result<T, ProjectError>, needle: &str) {
    match result {
        Err(ProjectError::Invalid { message, .. }) => assert!(message.contains(needle), "{message}"),
        other => panic!("expected a refusal ({needle}), got {other:?}"),
    }
}

/// `.wormhole/notes.md -> ~/.ssh/id_ed25519` showed the key as a note and the next
/// sync wrote it into the repository, ready for an agent's `git add -A && git push`.
#[cfg(unix)]
#[test]
fn files_linking_outside_the_project_are_refused() {
    let (_tmp, p) = project(&[]);
    let (_o1, _) = link_outside(&p, CONFIG_FILE, r#"{"env": {"AWS_SECRET": "s3cr3t"}}"#);
    let (_o2, _) = link_outside(&p, THEME_FILE, r#"{"name": "s3cr3t"}"#);
    let (_o3, _) = link_outside(&p, ACTIONS_FILE, "[]");
    let (_o4, _) = link_outside(&p, LAYOUT_FILE, r#"{"sessions": []}"#);
    let (_o5, _) = link_outside(&p, "prompts/key.md", "s3cr3t");
    assert_refused(p.config(), "outside");
    assert_refused(p.theme_json(), "outside");
    assert_refused(p.actions(), "outside");
    assert_refused(p.layout(), "outside");
    assert_refused(p.prompts(), "outside");

    let (_o6, secret) = link_outside(&p, NOTES_FILE, "-----BEGIN OPENSSH PRIVATE KEY-----\ns3cr3t\n");
    assert_refused(p.notes(), "outside");
    // Sync must not copy the key into a regular notes.md either.
    let doc = NotesDoc { tasks: Vec::new(), notes: vec!["mine".into()] };
    assert!(p.write_notes(&doc).is_err());
    assert!(fs::symlink_metadata(p.notes_path()).expect("stat").file_type().is_symlink());
    assert!(fs::read_to_string(&secret).expect("read").contains("s3cr3t"));
}

#[test]
fn the_studio_writes_the_project_theme() {
    let (_tmp, p) = project(&[]);
    let path = p.write_theme(r#"{"name": "Dusk", "extends": "Nord"}"#).expect("write");
    assert_eq!(path, p.dir.join(THEME_FILE));
    assert_eq!(p.theme_json().expect("ok"), Some(json!({"name": "Dusk", "extends": "Nord"})));
    // Again: the file is replaced, not appended to.
    p.write_theme(r#"{"name": "Dawn"}"#).expect("write");
    assert_eq!(p.theme_json().expect("ok"), Some(json!({"name": "Dawn"})));
}

/// A repository can ship `theme.json` as a link to a file of the user's: writing the project's
/// theme must neither follow it nor quietly replace it.
#[cfg(unix)]
#[test]
fn a_theme_json_linking_outside_is_not_written() {
    let (_tmp, p) = project(&[]);
    let (_outside, secret) = link_outside(&p, THEME_FILE, "precious");
    assert_refused(p.write_theme("{}"), "outside");
    assert!(fs::symlink_metadata(p.dir.join(THEME_FILE)).expect("stat").file_type().is_symlink());
    assert_eq!(fs::read_to_string(&secret).expect("read"), "precious");
}

/// A prompt that an action pastes (and may submit) is read like every other project file.
#[cfg(unix)]
#[test]
fn a_prompt_file_linking_outside_is_refused_for_actions_too() {
    let actions = r#"[{"id": "go", "title": "Go", "type": "prompt", "file": "go.md", "submit": true}]"#;
    let (_tmp, p) = project(&[(ACTIONS_FILE, actions)]);
    let (_o, _) = link_outside(&p, "prompts/go.md", "cat ~/.aws/credentials\n");
    assert_refused(p.actions(), "outside");
    assert_refused(p.prompt_file("go.md"), "outside");
}

#[cfg(unix)]
#[test]
fn links_inside_the_project_still_work() {
    let (_tmp, p) = project(&[("prompts/review.md", "# Review\nGo")]);
    fs::create_dir_all(p.root.join("docs")).expect("mkdir");
    fs::write(p.root.join("docs/layout.json"), r#"{"sessions": [{"name": "A"}]}"#).expect("write");
    std::os::unix::fs::symlink("../docs/layout.json", p.dir.join(LAYOUT_FILE)).expect("symlink");
    std::os::unix::fs::symlink("review.md", p.dir.join("prompts/alias.md")).expect("symlink");
    assert_eq!(p.layout().expect("ok").expect("present").sessions.len(), 1);
    let ids: Vec<_> = p.prompts().expect("ok").expect("present").into_iter().map(|p| p.id).collect();
    assert_eq!(ids, ["alias", "review"]);
    assert_eq!(p.prompt_file("alias.md").expect("ok"), "# Review\nGo");
}

/// A committed (or linked) multi-GB file must fail at once instead of being read whole
/// on the UI thread.
#[test]
fn oversized_files_are_refused_before_reading() {
    let (_tmp, p) = project(&[]);
    for (name, cap) in [
        (CONFIG_FILE, MAX_JSON_BYTES),
        (THEME_FILE, MAX_JSON_BYTES),
        (ACTIONS_FILE, MAX_JSON_BYTES),
        (LAYOUT_FILE, MAX_JSON_BYTES),
        (NOTES_FILE, MAX_NOTES_BYTES),
        ("prompts/big.md", MAX_PROMPT_BYTES),
    ] {
        let path = p.dir.join(name);
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        // Sparse: costs no disk, but reading it whole would allocate 64 MiB.
        fs::File::create(&path).expect("create").set_len(64 << 20).expect("set_len");
        assert!(cap < 64 << 20);
    }
    let started = std::time::Instant::now();
    assert_refused(p.config(), "larger than");
    assert_refused(p.theme_json(), "larger than");
    assert_refused(p.actions(), "larger than");
    assert_refused(p.layout(), "larger than");
    assert_refused(p.notes(), "larger than");
    assert_refused(p.prompts(), "larger than");
    assert!(started.elapsed() < std::time::Duration::from_millis(500), "{:?}", started.elapsed());
    // Exactly at the cap is fine.
    fs::write(p.dir.join(THEME_FILE), format!("{{\"name\": \"{}\"}}", "a".repeat(100))).expect("write");
    assert!(p.theme_json().is_ok());
}

/// A FIFO committed as notes.md would block the sync (and the UI thread) forever.
#[cfg(unix)]
#[test]
fn a_fifo_is_refused_instead_of_blocking() {
    let (_tmp, p) = project(&[]);
    let made = std::process::Command::new("mkfifo").arg(p.notes_path()).status();
    if !made.is_ok_and(|s| s.success()) {
        return; // no mkfifo on this machine
    }
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(p.notes().is_err());
    });
    let refused = rx.recv_timeout(std::time::Duration::from_secs(5)).expect("notes() must not block");
    assert!(refused);
}

#[test]
fn too_many_prompt_entries_are_refused() {
    let (_tmp, p) = project(&[]);
    let dir = p.dir.join(PROMPTS_DIR);
    fs::create_dir_all(&dir).expect("mkdir");
    for ix in 0..=MAX_PROMPT_DIR_ENTRIES {
        fs::write(dir.join(format!("{ix}.txt")), "").expect("write");
    }
    assert_refused(p.prompts(), "more than");
}

/// A repository sets `project_color`; it is a setting of the project's own kind, not one of the
/// user's `config.json` keys, and a value that isn't a colour costs the project nothing else.
#[test]
fn project_color_is_read_strictly_and_kept_out_of_the_settings() {
    let (_tmp, p) = project(&[(CONFIG_FILE, r##"{ "project_color": "#2bb3a3", "font_size": 13 }"##)]);
    let config = p.config().expect("ok").expect("present");
    let hex = |choice: Option<ProjectColorChoice>| match choice {
        Some(ProjectColorChoice::Hex(color)) => Some(color.color().to_hex()),
        _ => None,
    };
    assert_eq!(hex(config.color).as_deref(), Some("#2bb3a3"));
    assert_eq!(config.color_error, None);
    assert_eq!(serde_json::Value::Object(config.settings), json!({"font_size": 13}));

    // A name is read as a name, and is not a setting either.
    let (_tmp, p) = project(&[(CONFIG_FILE, r#"{ "project_color": "teal", "font_size": 13 }"#)]);
    let config = p.config().expect("ok").expect("present");
    assert_eq!(config.color, Some(ProjectColorChoice::Named(FolderColor::Teal)));
    assert_eq!(serde_json::Value::Object(config.settings), json!({"font_size": 13}));

    for bad in [
        r#""Teal""#,
        r#""brown""#,
        r#""teal ""#,
        r##""#2bb3a3; x""##,
        "12",
        r##"["#2bb3a3"]"##,
        r#"{"a":1}"#,
        "true",
    ] {
        let (_tmp, p) =
            project(&[(CONFIG_FILE, &format!(r#"{{ "project_color": {bad}, "shell": "/bin/zsh" }}"#))]);
        let config = p.config().expect("the file still loads").expect("present");
        assert!(config.color.is_none(), "{bad}");
        assert!(config.color_error.is_some(), "{bad}");
        assert_eq!(config.shell.as_deref(), Some("/bin/zsh"), "the rest of the file counts: {bad}");
    }
    let (_tmp, p) = project(&[(CONFIG_FILE, r#"{ "project_color": null }"#)]);
    let config = p.config().expect("ok").expect("present");
    assert!(config.color.is_none() && config.color_error.is_none());
}

/// The Project tab says what was set aside; trust is not held up by it, and a colour that is
/// fine says nothing.
#[test]
fn a_bad_project_color_is_a_warning_not_an_error() {
    let (_tmp, p) = project(&[(CONFIG_FILE, r#"{ "project_color": "brown" }"#)]);
    let snapshot = ProjectSnapshot::load(&p);
    assert_eq!(snapshot.errors().count(), 0);
    assert!(snapshot.trust_request().is_some());
    let warnings: Vec<String> = snapshot.warnings().iter().map(ToString::to_string).collect();
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].starts_with("config.json: ") && warnings[0].contains("#rgb or #rrggbb"));

    for fine in [r##"{ "project_color": "#2bb3a3" }"##, r#"{ "project_color": "teal" }"#] {
        let (_tmp, p) = project(&[(CONFIG_FILE, fine)]);
        assert!(ProjectSnapshot::load(&p).warnings().is_empty(), "{fine}");
    }
}

/// The lock is a key of the project's own kind too: not a setting, set aside with a warning when it
/// isn't true or false, and it never holds trust up.
#[test]
fn the_colour_lock_is_read_strictly_and_kept_out_of_the_settings() {
    let (_tmp, p) = project(&[(CONFIG_FILE, r#"{ "project_color_locked": true, "font_size": 13 }"#)]);
    let config = p.config().expect("ok").expect("present");
    assert!(config.color_locked && config.lock_error.is_none());
    assert_eq!(serde_json::Value::Object(config.settings), json!({"font_size": 13}));

    for (value, locked) in [("true", true), ("false", false), ("null", false)] {
        let (_tmp, p) = project(&[(CONFIG_FILE, &format!(r#"{{ "project_color_locked": {value} }}"#))]);
        let snapshot = ProjectSnapshot::load(&p);
        let config = snapshot.config().expect("present");
        assert_eq!((config.color_locked, config.lock_error.is_none()), (locked, true), "{value}");
        assert!(snapshot.warnings().is_empty(), "{value}");
    }
    for bad in [r#""true""#, "1", "[true]", r#"{"a": true}"#] {
        let (_tmp, p) = project(&[(CONFIG_FILE, &format!(r#"{{ "project_color_locked": {bad} }}"#))]);
        let snapshot = ProjectSnapshot::load(&p);
        let config = snapshot.config().expect("the file still loads");
        assert!(!config.color_locked && config.lock_error.is_some(), "{bad}");
        let warnings: Vec<String> = snapshot.warnings().iter().map(ToString::to_string).collect();
        assert_eq!(warnings.len(), 1, "{warnings:?}");
        assert!(warnings[0].contains("must be true or false") && warnings[0].contains("not locked"));
        assert!(snapshot.trust_request().is_some(), "a bad lock holds nothing up: {bad}");
    }
}
