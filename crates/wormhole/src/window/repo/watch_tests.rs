//! A repository followed through its files is read when something in it changes, with no tick: through a
//! real repository in the test's scratch dir and the real file events of the system, end to end.
//!
//! The window never ticks here (`poll_repos` is called once, to claim the repository), so a read after
//! a write can only come from the watch.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Duration;

use gpui_kit::TestAppContext;

use super::watches::{GRACE, RepoWatches};
use crate::app_state::OpenWindow;
use crate::window::test_harness::{drive, isolate_in, read, start};

fn isolate(name: &str) -> bool {
    isolate_in(module_path!(), name)
}

fn git(dir: &Path, args: &[&str]) {
    let output = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.com", "-c", "commit.gpgsign=false"])
        .args(args)
        .output()
        .expect("runs git");
    assert!(output.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&output.stderr));
}

/// A repository on `main` in the scratch dir with `a.txt` committed and `target/` ignored.
fn repo(name: &str) -> PathBuf {
    let root = std::env::current_dir().expect("the scratch dir").join(name);
    std::fs::create_dir_all(&root).expect("mkdir");
    let root = root.canonicalize().expect("canonical");
    git(&root, &["init", "-q", "-b", "main"]);
    std::fs::write(root.join("a.txt"), "a").expect("write");
    std::fs::write(root.join(".gitignore"), "target/\n").expect("write");
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "first"]);
    root
}

/// The paths git lists as changed for the terminal in front.
fn changed(cx: &mut TestAppContext, window: &OpenWindow) -> Vec<String> {
    read(cx, window, |ws| {
        ws.sessions[0]
            .git
            .status
            .as_ref()
            .map(|status| status.files.iter().map(|file| file.path.clone()).collect())
            .unwrap_or_default()
    })
}

/// Let real time pass in small steps, with the executor's clock moved along, until `done`.
fn wait_for(cx: &mut TestAppContext, what: &str, mut done: impl FnMut(&mut TestAppContext) -> bool) {
    for _ in 0..200 {
        if done(cx) {
            return;
        }
        std::thread::sleep(Duration::from_millis(25));
        cx.executor().advance_clock(Duration::from_millis(50));
        cx.run_until_parked();
    }
    panic!("never saw: {what}");
}

#[gpui_kit::test]
fn a_write_in_a_followed_repository_is_read_without_a_tick(cx: &mut TestAppContext) {
    if isolate("a_write_in_a_followed_repository_is_read_without_a_tick") {
        return;
    }
    let root = repo("followed");
    let window = start(cx, 1);
    drive(cx, &window, |ws, _, cx| {
        ws.active_terminal().update(cx, |terminal, cx| {
            terminal.cwd = Some(root.clone());
            cx.notify();
        });
        ws.sync_summaries(cx);
        ws.refresh_git(0, cx);
    });
    // The tick's part, once: git has named the repository, so it is claimed and followed.
    drive(cx, &window, |ws, _, cx| ws.poll_repos(true, true, cx));
    assert!(cx.update(|cx| RepoWatches::live(&root, cx)), "the repository is followed");
    assert!(changed(cx, &window).is_empty());

    // Output in an ignored folder changes nothing, and a new file does.
    std::fs::create_dir_all(root.join("target")).expect("mkdir");
    std::fs::write(root.join("target/out.o"), "o").expect("write");
    std::fs::write(root.join("new.txt"), "n").expect("write");
    wait_for(cx, "new.txt among the changes", |cx| changed(cx, &window) == ["new.txt"]);

    // A commit moves the index and a ref: the file stops being a change.
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "second"]);
    wait_for(cx, "the commit emptied the changes", |cx| changed(cx, &window).is_empty());

    // Nobody in the repository any more: the watch is let go.
    drive(cx, &window, |ws, _, cx| {
        ws.active_terminal().update(cx, |terminal, cx| {
            terminal.cwd = Some(PathBuf::from("/"));
            cx.notify();
        });
        ws.sync_summaries(cx);
        ws.sessions[0].git = Default::default();
        ws.poll_repos(true, true, cx);
    });
    assert!(cx.update(|cx| RepoWatches::live(&root, cx)), "kept for a few seconds, in case it comes back");
    std::thread::sleep(GRACE + Duration::from_millis(100));
    drive(cx, &window, |ws, _, cx| ws.poll_repos(true, true, cx));
    assert!(!cx.update(|cx| RepoWatches::live(&root, cx)), "dropped with the last terminal in it");
}
