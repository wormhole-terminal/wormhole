//! What the diff view does through the window, pinned from outside: a change opens its diff, the arrows
//! step through the files the list shows, an open diff follows the working tree (and says when its
//! file is no longer changed), and review comments are kept for the terminal and pasted with the
//! review. The helpers at the top are the only place a test names where the state is kept.

use gpui_kit::TestAppContext;
use wormhole_core::config::TabId;
use wormhole_core::git::review::LineSide;

use super::super::testing::{change_of, git, go, read_repo, repo, write};
use super::render::LineAt;
use crate::app_state::OpenWindow;
use crate::window::view_tests::{drive, isolate_in, read, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

// ---------------------------------------------------------------- where the state is kept

/// What the diff on view is, if one is.
#[derive(Debug, PartialEq)]
struct Shown {
    path: String,
    loading: bool,
    counts: (usize, usize),
    gone: bool,
    error: bool,
    position: Option<usize>,
}

fn shown(cx: &mut TestAppContext, window: &OpenWindow) -> Option<Shown> {
    read(cx, window, |ws| {
        let view = ws.diff_view.as_ref()?;
        Some(Shown {
            path: view.file().path.clone(),
            loading: view.loading,
            counts: view.counts,
            gone: view.gone,
            error: view.error.is_some(),
            position: view.siblings.iter().position(|f| f.path == view.file().path),
        })
    })
}

fn open(cx: &mut TestAppContext, window: &OpenWindow, path: &str) {
    drive(cx, window, |ws, _, cx| {
        let file = change_of(ws, path);
        ws.open_change(file, cx);
    });
    cx.run_until_parked();
}

/// A repository with `a.txt` and `b.txt` changed (a line added to each), and the window on it.
fn changed(cx: &mut TestAppContext, window: &OpenWindow, name: &str) -> std::path::PathBuf {
    let root = repo(name, &[("a.txt", "one\ntwo\n"), ("b.txt", "x\n")]);
    write(&root, &[("a.txt", "one\ntwo\nthree\n"), ("b.txt", "x\ny\n")]);
    drive(cx, window, |ws, _, cx| go(ws, &root, cx));
    read_repo(cx, window);
    root
}

// ---------------------------------------------------------------- opening and stepping

#[gpui_kit::test]
fn a_change_opens_its_diff_and_the_arrows_step_through_the_listed_files(cx: &mut TestAppContext) {
    if isolate("a_change_opens_its_diff_and_the_arrows_step_through_the_listed_files") {
        return;
    }
    let window = start(cx, 1);
    changed(cx, &window, "stepping");
    assert_eq!(shown(cx, &window), None);
    open(cx, &window, "a.txt");
    assert_eq!(
        shown(cx, &window),
        Some(Shown {
            path: "a.txt".into(),
            loading: false,
            counts: (1, 0),
            gone: false,
            error: false,
            position: Some(0)
        })
    );
    drive(cx, &window, |ws, _, cx| ws.step_diff(1, cx));
    cx.run_until_parked();
    assert_eq!(shown(cx, &window).map(|s| (s.path, s.counts)), Some(("b.txt".into(), (1, 0))));
    drive(cx, &window, |ws, _, cx| ws.step_diff(1, cx));
    cx.run_until_parked();
    assert_eq!(shown(cx, &window).map(|s| s.path), Some("a.txt".into()), "past the last: the first");
    drive(cx, &window, |ws, _, cx| ws.step_diff(-1, cx));
    cx.run_until_parked();
    assert_eq!(shown(cx, &window).map(|s| s.path), Some("b.txt".into()), "before the first: the last");

    drive(cx, &window, |ws, window, cx| ws.close_diff(window, cx));
    assert_eq!(shown(cx, &window), None);
}

#[gpui_kit::test]
fn the_diff_covers_the_terminal_until_it_is_closed(cx: &mut TestAppContext) {
    if isolate("the_diff_covers_the_terminal_until_it_is_closed") {
        return;
    }
    let window = start(cx, 1);
    changed(cx, &window, "covers");
    open(cx, &window, "a.txt");
    crate::window::view_tests::draw(cx, &window);
    assert!(read(cx, &window, |ws| ws.diff_view.is_some()));
    // The Tabs area is not one to move to while a diff covers the terminal (the diff has the keys).
    drive(cx, &window, |ws, window, cx| ws.close_diff(window, cx));
    crate::window::view_tests::draw(cx, &window);
    assert!(read(cx, &window, |ws| ws.diff_view.is_none()));
}

// ---------------------------------------------------------------- following the working tree

#[gpui_kit::test]
fn an_open_diff_follows_the_file_and_says_when_it_is_no_longer_changed(cx: &mut TestAppContext) {
    if isolate("an_open_diff_follows_the_file_and_says_when_it_is_no_longer_changed") {
        return;
    }
    let window = start(cx, 1);
    let root = changed(cx, &window, "follows");
    open(cx, &window, "a.txt");
    assert_eq!(shown(cx, &window).map(|s| s.counts), Some((1, 0)));

    // The file is edited (an agent at work): the next look at the repository reads the diff again.
    write(&root, &[("a.txt", "one\ntwo\nthree\nfour\n")]);
    read_repo(cx, &window);
    cx.run_until_parked();
    assert_eq!(shown(cx, &window).map(|s| (s.counts, s.gone)), Some(((2, 0), false)));

    // Put back as committed: the diff stays as it was read and says so.
    write(&root, &[("a.txt", "one\ntwo\n")]);
    read_repo(cx, &window);
    cx.run_until_parked();
    assert_eq!(shown(cx, &window).map(|s| (s.counts, s.gone)), Some(((2, 0), true)));

    // The siblings follow the list: b.txt only now.
    assert_eq!(read(cx, &window, |ws| ws.diff_view.as_ref().map(|v| v.siblings.len())), Some(1));
}

#[gpui_kit::test]
fn a_diff_is_closed_when_another_terminal_is_selected(cx: &mut TestAppContext) {
    if isolate("a_diff_is_closed_when_another_terminal_is_selected") {
        return;
    }
    let window = start(cx, 2);
    changed(cx, &window, "belongs");
    open(cx, &window, "a.txt");
    drive(cx, &window, |ws, window, cx| ws.activate(1, window, cx));
    assert_eq!(shown(cx, &window), None, "the diff belongs to the terminal it was opened from");
}

// ---------------------------------------------------------------- review comments

fn comment(
    cx: &mut TestAppContext,
    window: &OpenWindow,
    path: &str,
    line: u32,
    text: &str,
) -> Result<(), &'static str> {
    drive(cx, window, |ws, window, cx| {
        let at = LineAt { side: LineSide::New, line, code: "three".into() };
        ws.comment_on_line(path.to_string(), at, window, cx);
        let input = ws.commenting.as_ref().ok_or("no dialog")?.input.clone();
        input.update(cx, |input, cx| input.set_value(text.to_string(), window, cx));
        ws.save_comment(cx);
        ws.commenting = None;
        Ok(())
    })
}

fn review(cx: &mut TestAppContext, window: &OpenWindow, session: usize) -> (String, Option<String>) {
    read(cx, window, |ws| {
        let review = &ws.sessions[session].review;
        (review.summary(), review.message())
    })
}

#[gpui_kit::test]
fn comments_are_kept_per_terminal_and_marked_on_the_diff(cx: &mut TestAppContext) {
    if isolate("comments_are_kept_per_terminal_and_marked_on_the_diff") {
        return;
    }
    let window = start(cx, 2);
    changed(cx, &window, "comments");
    open(cx, &window, "a.txt");
    comment(cx, &window, "a.txt", 3, "Why three?").expect("a comment");
    assert_eq!(review(cx, &window, 0).0, "1 comment");
    assert_eq!(review(cx, &window, 1).0, "0 comments", "the other terminal has none");
    // The diff marks the commented line; commenting on it again replaces the text; empty removes it.
    let marked = read(cx, &window, |ws| {
        let view = ws.diff_view.as_ref().expect("a diff");
        ws.commented_lines(view)
    });
    assert_eq!(marked, [(LineSide::New, 3)]);
    comment(cx, &window, "a.txt", 3, "Please explain").expect("a comment");
    let (summary, message) = review(cx, &window, 0);
    assert_eq!(summary, "1 comment");
    let message = message.expect("a message");
    assert!(message.contains("Please explain") && !message.contains("Why three?"), "{message}");
    comment(cx, &window, "a.txt", 3, "  ").expect("a comment");
    assert_eq!(review(cx, &window, 0).0, "0 comments");
}

#[gpui_kit::test]
fn a_review_is_copied_when_the_program_cant_take_a_paste_and_cleared_on_request(cx: &mut TestAppContext) {
    if isolate("a_review_is_copied_when_the_program_cant_take_a_paste_and_cleared_on_request") {
        return;
    }
    let window = start(cx, 1);
    changed(cx, &window, "pasted");
    open(cx, &window, "a.txt");
    comment(cx, &window, "a.txt", 3, "Why three?").expect("a comment");
    // `cat` has not asked for bracketed paste, so a paste would run each line: the comments go to the
    // clipboard instead, the review is kept, and the diff stays open.
    drive(cx, &window, |ws, window, cx| ws.paste_review(window, cx));
    let copied = cx.read_from_clipboard().and_then(|item| item.text()).unwrap_or_default();
    assert!(copied.starts_with("Review comments on your changes."), "{copied}");
    assert_eq!(review(cx, &window, 0).0, "1 comment");
    assert!(shown(cx, &window).is_some());
    drive(cx, &window, |ws, _, cx| ws.clear_review(cx));
    assert_eq!(review(cx, &window, 0), ("0 comments".to_string(), None));
}

#[gpui_kit::test]
fn the_files_status_is_read_again_after_a_commit_so_the_list_empties(cx: &mut TestAppContext) {
    if isolate("the_files_status_is_read_again_after_a_commit_so_the_list_empties") {
        return;
    }
    let window = start(cx, 1);
    let root = changed(cx, &window, "committed");
    open(cx, &window, "a.txt");
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "all"]);
    read_repo(cx, &window);
    cx.run_until_parked();
    assert_eq!(shown(cx, &window).map(|s| s.gone), Some(true), "no longer among the changes");
    drive(cx, &window, |ws, _, cx| ws.show_tab(TabId::Changes, cx));
}
