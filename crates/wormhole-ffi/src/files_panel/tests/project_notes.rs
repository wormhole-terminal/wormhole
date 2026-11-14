//! notes.md sync: only in a trusted project, only where the person wants it, never through a link.

use std::fs;

use super::{Owner, Repo, link, service};
use crate::files_panel::{ApproveOutcome, NotesDocRecord, OwnerAnswer, TaskDocRecord};

fn doc(tasks: &[(&str, bool)], notes: &[&str]) -> NotesDocRecord {
    NotesDocRecord {
        tasks: tasks
            .iter()
            .map(|(text, done)| TaskDocRecord { text: text.to_string(), done: *done })
            .collect(),
        notes: notes.iter().map(|n| n.to_string()).collect(),
    }
}

fn notes_md(repo: &Repo) -> String {
    fs::read_to_string(repo.path().join(".wormhole/notes.md")).unwrap_or_default()
}

#[test]
fn an_untrusted_project_neither_reads_nor_writes_its_notes_and_the_tab_says_why() {
    let repo = Repo::new(&[("notes.md", "# Notes\n\n## Next steps\n\n- [ ] from the repo\n")]);
    let (_data, service) = service(&Owner::answering([]));
    let before = notes_md(&repo);
    let out = service.sync_notes(1, repo.root(), doc(&[("mine", false)], &["my note"]), false);
    assert_eq!(out, Default::default(), "nothing is applied");
    assert_eq!(notes_md(&repo), before, "and nothing is written");
    let state = service.project(repo.root()).expect("project");
    assert!(
        state.errors.iter().any(|e| e.starts_with("Notes sync: waits for the project to be trusted")),
        "{:?}",
        state.errors
    );
    assert!(state.applies.contains("notes.md sync") && state.asks_for_trust);
}

#[test]
fn a_trusted_project_merges_both_ways_and_keeps_the_editor_of_a_note_that_stays() {
    let repo = Repo::new(&[("notes.md", "# Notes\n\n## Next steps\n\n- [ ] from the repo\n")]);
    let owner = Owner::answering([OwnerAnswer::Approved]);
    let (_data, service) = service(&owner);
    service.project(repo.root());
    assert_eq!(service.approve(repo.root()), ApproveOutcome::Approved);

    // First look: both sides are kept, the file gets mine and I get the file's.
    let out = service.sync_notes(1, repo.root(), doc(&[("mine", false)], &["my note"]), false);
    let applied = out.apply.expect("the file's step comes in");
    assert_eq!(applied.tasks.iter().map(|t| t.text.as_str()).collect::<Vec<_>>(), ["from the repo", "mine"]);
    assert_eq!(out.note_slots, [Some(0)], "my note is the same note: it keeps its editor");
    assert!(notes_md(&repo).contains("- [ ] mine") && notes_md(&repo).contains("my note"));

    // Settled: nothing more to do.
    assert_eq!(service.sync_notes(1, repo.root(), applied.clone(), false), Default::default());

    // Someone edits the file (a checkout, an editor): it comes in.
    repo.put("notes.md", &notes_md(&repo).replace("- [ ] mine", "- [x] mine"));
    let out = service.sync_notes(1, repo.root(), applied.clone(), false);
    let ticked = out.apply.expect("the tick comes in");
    assert!(ticked.tasks.iter().any(|t| t.text == "mine" && t.done));

    // While a note is being written the merge waits.
    repo.put("notes.md", &notes_md(&repo).replace("from the repo", "from the repo, edited"));
    assert_eq!(service.sync_notes(1, repo.root(), applied, true), Default::default());
}

#[test]
fn the_switch_decides_and_a_closed_terminal_forgets_what_it_agreed() {
    let repo = Repo::new(&[("notes.md", "# Notes\n")]);
    let (_data, service) = service(&Owner::answering([OwnerAnswer::Approved]));
    service.project(repo.root());
    service.approve(repo.root());
    service.set_feature(repo.root(), "notes".into(), false).expect("off");
    let before = notes_md(&repo);
    assert_eq!(service.sync_notes(1, repo.root(), doc(&[("mine", false)], &[]), false), Default::default());
    assert_eq!(notes_md(&repo), before, "off: not written");
    service.set_feature(repo.root(), "notes".into(), true).expect("on");
    service.sync_notes(1, repo.root(), doc(&[("mine", false)], &[]), false);
    assert!(notes_md(&repo).contains("mine"));
    service.notes_session_closed(1);
    service.notes_ids_start_at(1);
}

#[test]
fn a_notes_file_that_links_out_of_the_project_is_never_read() {
    let outside = tempfile::tempdir().expect("temp dir");
    fs::write(outside.path().join("secret.md"), "# Notes\n\n## Next steps\n\n- [ ] stolen\n").expect("write");
    let repo = Repo::new(&[("config.json", "{}")]);
    link(&outside.path().join("secret.md"), &repo.path().join(".wormhole/notes.md"));
    let (_data, service) = service(&Owner::answering([OwnerAnswer::Approved]));
    service.project(repo.root());
    service.approve(repo.root());
    let out = service.sync_notes(1, repo.root(), doc(&[], &[]), false);
    assert_eq!(out, Default::default(), "the secret does not become a note");
    let state = service.project(repo.root()).expect("project");
    assert!(
        state.errors.iter().any(|e| e.starts_with("Notes sync:") && e.contains("outside")),
        "{:?}",
        state.errors
    );
    assert!(
        fs::read_to_string(outside.path().join("secret.md")).expect("read").contains("stolen"),
        "and is not overwritten"
    );
}

#[test]
fn the_windows_of_a_profile_merge_a_terminal_against_the_same_base() {
    let repo = Repo::new(&[("notes.md", "# Notes\n\n## Next steps\n\n- [ ] from the repo\n")]);
    let owner = Owner::answering([OwnerAnswer::Approved]);
    let (data, first) = service(&owner);
    first.project(repo.root());
    assert_eq!(first.approve(repo.root()), ApproveOutcome::Approved);
    let second = crate::files_panel::ProjectService::in_dir(data.path(), owner.clone());
    let agreed = first
        .sync_notes(1, repo.root(), doc(&[("mine", false)], &[]), false)
        .apply
        .expect("the file's step comes in");
    // The file loses a step while the terminal moves to the other window, whose merge must know what was agreed: without
    // the base the step reads as added by the person and is written back.
    let before = notes_md(&repo);
    let without: Vec<&str> =
        before.lines().filter(|line| *line != "- [ ] mine").map(str::trim_end).collect();
    repo.put("notes.md", &format!("{}\n", without.join("\n")));
    let out = second.sync_notes(1, repo.root(), agreed, false);
    let steps = out.apply.map(|doc| doc.tasks.into_iter().map(|t| t.text).collect::<Vec<_>>());
    assert_eq!(steps, Some(vec!["from the repo".to_string()]), "the file's deletion comes in");
    assert!(!notes_md(&repo).contains("mine"), "and is not undone");
}
