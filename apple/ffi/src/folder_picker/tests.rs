#![allow(clippy::disallowed_methods, reason = "the tests build a throwaway tree of their own")]

use std::fs;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{Receiver, Sender, channel};
use std::time::Duration;

use super::*;

/// Hears a job's answer on a channel.
struct Hears(Mutex<Sender<Heard>>);

enum Heard {
    Answer(PickerAnswerRecord),
    Decorations(Vec<PickerDecorationRecord>),
}

impl PickerListener for Hears {
    fn answered(&self, answer: PickerAnswerRecord) {
        let _ = self.0.lock().expect("lock").send(Heard::Answer(answer));
    }

    fn decorated(&self, found: Vec<PickerDecorationRecord>) {
        let _ = self.0.lock().expect("lock").send(Heard::Decorations(found));
    }
}

fn listener() -> (Arc<dyn PickerListener>, Receiver<Heard>) {
    let (tx, rx) = channel();
    (Arc::new(Hears(Mutex::new(tx))), rx)
}

fn temp(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("Wormhole-swift-picker-{name}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).expect("temp dir");
    // macOS reports the real path of /var.
    fs::canonicalize(&dir).expect("canonical")
}

fn query(text: &str, cwd: &Path, home: &Path) -> PickerQueryRecord {
    PickerQueryRecord {
        text: text.into(),
        context: PickerContextRecord {
            cwd: cwd.display().to_string(),
            home: home.display().to_string(),
            profile: "default".into(),
        },
    }
}

fn answer(picker: &FolderPicker, index: u32, query: PickerQueryRecord) -> PickerAnswerRecord {
    let (listener, rx) = listener();
    let _job = picker.ask(index, query, listener);
    match rx.recv_timeout(Duration::from_secs(20)).expect("an answer") {
        Heard::Answer(answer) => answer,
        Heard::Decorations(_) => panic!("decorations for a query"),
    }
}

fn labels(answer: &PickerAnswerRecord) -> Vec<&str> {
    answer.rows.iter().map(|row| row.label.as_str()).collect()
}

#[test]
fn the_folders_below_the_terminals_come_with_crumbs_and_what_to_insert() {
    let _turn = gate::TURN.lock().unwrap_or_else(PoisonError::into_inner);
    let root = temp("below");
    for dir in ["alpha/beta", "Projects/terminal/wormhole", "zeta"] {
        fs::create_dir_all(root.join(dir)).expect("tree");
    }
    fs::write(root.join("a-file"), "x").expect("file");
    let picker = FolderPicker::new(None, "Recent".into());
    let all = answer(&picker, 0, query("", &root, &root));
    assert_eq!(labels(&all), ["alpha", "Projects", "zeta"], "ls order, no files");
    let alpha = &all.rows[0];
    assert_eq!(alpha.descend.as_deref(), Some("alpha/"));
    assert_eq!(alpha.insert.as_deref(), Some("alpha/"));
    assert_eq!(alpha.kind, "folder");
    assert_eq!(alpha.key, root.join("alpha").display().to_string());
    assert!(!all.crumbs.is_empty());
    // A name typed matches forgivingly across levels, with the marks in bytes.
    let fuzzy = answer(&picker, 0, query("pr/te", &root, &root));
    assert_eq!(labels(&fuzzy), ["Projects/terminal"]);
    assert_eq!(fuzzy.rows[0].insert.as_deref(), Some("Projects/terminal/"));
    assert!(!fuzzy.rows[0].marks.is_empty(), "where the typed text matched");
    let walked = answer(&picker, 0, query("alpha/", &root, &root));
    assert_eq!(labels(&walked), ["beta"]);
    assert!(walked.up_at_end, "the text ends where a level does");
    assert_eq!(walked.up.as_deref(), Some(""), "one level up from alpha/ is where it started");
}

#[test]
fn a_symlink_loop_and_a_very_deep_folder_do_not_hang_or_run_away() {
    let _turn = gate::TURN.lock().unwrap_or_else(PoisonError::into_inner);
    let root = temp("loop");
    fs::create_dir_all(root.join("real/inner")).expect("tree");
    symlink(&root, root.join("real/loop")).expect("a link back to the root");
    symlink(root.join("real"), root.join("alias")).expect("a link");
    let mut deep = root.join("deep");
    for n in 0..120 {
        deep = deep.join(format!("d{n}"));
    }
    fs::create_dir_all(&deep).expect("a very deep folder");
    let picker = FolderPicker::new(None, "Recent".into());
    // Typed text that goes round the loop again and again: bounded by the walk's levels, and the answer comes.
    let looped = answer(&picker, 0, query("real/loop/real/loop/real/loop/", &root, &root));
    assert!(looped.rows.len() < 100, "bounded: {}", looped.rows.len());
    let around = answer(&picker, 0, query("re/lo/re/lo", &root, &root));
    assert!(around.rows.len() < 100, "{}", around.rows.len());
    // The picker is opened in the deepest folder: its crumbs name every level, and it answers.
    let there = answer(&picker, 0, query("", &deep, &root));
    assert!(there.rows.is_empty() && there.notes.contains(&PickerNoteRecord::Empty) || there.rows.is_empty());
    assert!(there.crumbs.len() > 100, "{}", there.crumbs.len());
    // From the top, a deep name is found through at most the walk's four levels, not all 120.
    let found = answer(&picker, 0, query("d5", &root.join("deep"), &root));
    assert!(found.rows.len() < 100);
}

#[test]
fn recent_folders_rank_by_visits_and_a_forgotten_one_goes() {
    let _turn = gate::TURN.lock().unwrap_or_else(PoisonError::into_inner);
    let root = temp("recent");
    let (a, b) = (root.join("a"), root.join("b"));
    fs::create_dir_all(&a).expect("a");
    fs::create_dir_all(&b).expect("b");
    // Under the temp folder, which is never recorded: nothing is kept.
    let picker = FolderPicker::new(None, "Recent".into());
    assert!(!picker.record_visit(a.display().to_string(), 1_700_000_000));
    // A saved list reads back, and the recent provider (the second) lists it under the title given.
    let saved = format!(
        r#"{{"entries":[{{"path":"{}","score":1.0,"last":{}}},{{"path":"{}","score":3.0,"last":{}}}]}}"#,
        a.display(),
        now(),
        b.display(),
        now()
    );
    let picker = FolderPicker::new(Some(saved), "Recent".into());
    let listed = answer(&picker, 1, query("", &root, &root));
    assert_eq!(listed.sections.last().map(String::as_str), Some("Recent"));
    let recent: Vec<&str> = listed.rows.iter().map(|r| r.label.as_str()).collect();
    assert_eq!(recent, ["b", "a"], "the more visited first");
    assert!(listed.rows.iter().all(|r| r.forgettable));
    assert!(picker.forget(b.display().to_string()));
    assert_eq!(labels(&answer(&picker, 1, query("", &root, &root))), ["a"]);
    assert!(picker.recents_json().contains(&a.display().to_string()));
    assert!(!picker.recents_json().contains(&b.display().to_string()));
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

#[test]
fn decorators_add_chevrons_and_name_their_kinds() {
    let _turn = gate::TURN.lock().unwrap_or_else(PoisonError::into_inner);
    let root = temp("decorate");
    fs::create_dir_all(root.join("with-children/child")).expect("tree");
    fs::create_dir_all(root.join("leaf")).expect("leaf");
    let picker = FolderPicker::new(None, "Recent".into());
    assert_eq!(picker.provider_count(), 2);
    assert!(picker.decorator_count() >= 1);
    assert_eq!(picker.decorator_kinds(0), ["folder"]);
    let keys =
        vec![root.join("with-children").display().to_string(), root.join("leaf").display().to_string()];
    let (listener, rx) = listener();
    let _job = picker.decorate(0, keys.clone(), listener);
    let Heard::Decorations(found) = rx.recv_timeout(Duration::from_secs(20)).expect("decorations") else {
        panic!("an answer for a decoration")
    };
    let chevron = |key: &str| found.iter().find(|d| d.key == key).and_then(|d| d.chevron);
    assert_eq!(chevron(&keys[0]), Some(true));
    assert_ne!(chevron(&keys[1]), Some(true));
}

#[test]
fn a_git_folder_gets_its_branch_as_a_badge() {
    let _turn = gate::TURN.lock().unwrap_or_else(PoisonError::into_inner);
    let root = temp("branch");
    let repo = root.join("repo");
    fs::create_dir_all(repo.join(".git")).expect("repo");
    fs::write(repo.join(".git/HEAD"), "ref: refs/heads/feature/x\n").expect("HEAD");
    let picker = FolderPicker::new(None, "Recent".into());
    let (listener, rx) = listener();
    let _job = picker.decorate(1, vec![repo.display().to_string()], listener);
    let Heard::Decorations(found) = rx.recv_timeout(Duration::from_secs(20)).expect("decorations") else {
        panic!("decorations")
    };
    assert_eq!(found.first().and_then(|d| d.badge.as_deref()), Some("feature/x"), "{found:?}");
}

#[test]
fn a_job_cancelled_while_it_waits_for_a_place_answers_once_with_nothing() {
    let _turn = gate::TURN.lock().unwrap_or_else(PoisonError::into_inner);
    let root = temp("cancel");
    fs::create_dir_all(root.join("x")).expect("x");
    let picker = FolderPicker::new(None, "Recent".into());
    // Every place taken, so the job waits.
    let held: Vec<_> = (0..gate::READERS).filter_map(|_| gate::enter(|| true)).collect();
    assert_eq!(held.len(), gate::READERS);
    let (listener, rx) = listener();
    let job = picker.ask(0, query("", &root, &root), listener);
    assert!(rx.recv_timeout(Duration::from_millis(150)).is_err(), "it waits");
    job.cancel();
    let Heard::Answer(answer) = rx.recv_timeout(Duration::from_secs(5)).expect("one answer") else {
        panic!("an answer for a query")
    };
    assert!(answer.rows.is_empty(), "it never read");
    assert!(rx.recv_timeout(Duration::from_millis(200)).is_err(), "only once");
    drop(held);
}

#[test]
fn an_unknown_provider_answers_with_nothing_instead_of_hanging() {
    let _turn = gate::TURN.lock().unwrap_or_else(PoisonError::into_inner);
    let root = temp("unknown");
    let picker = FolderPicker::new(None, "Recent".into());
    let nothing = answer(&picker, 9, query("", &root, &root));
    assert!(nothing.rows.is_empty());
}
