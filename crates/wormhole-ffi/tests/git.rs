//! The git objects against real repositories in a temp dir: status with renames, binaries, untracked and
//! LFS-free counts; diff text, large and binary rules; compare with branches, tags and commits; log,
//! details and stashes; the watch; and the worktree round trip. Real git, real core, no mocks. The worktree
//! list is kept in the data folder `crates/wormhole-ffi/.cargo/config.toml` gives the tests (`Wormhole-swift-cargo-test`).

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use wormhole_ffi::{
    Git, GitBackendError, GitChangeKind, GitDiff, GitDiffFile, GitLineKind, GitListener, GitRev, GitRevKind,
    GitWatch, GitWorktreePlan, Terminal, TerminalListener, WorktreeService,
};

struct Quiet;

impl TerminalListener for Quiet {
    fn wake(&self) {}
}

/// What a shell started in `folder` sees of the worktree ports: `PORT=<WORMHOLE_PORT> PORTS=<WORMHOLE_PORTS>`.
fn ports_seen_in(folder: &str) -> String {
    let script = "printf 'PORT=%s PORTS=%s\\n' \"$WORMHOLE_PORT\" \"$WORMHOLE_PORTS\"; exec cat".to_string();
    let terminal = Terminal::start_in(
        "/bin/sh".into(),
        vec!["-c".into(), script],
        Some(folder.to_owned()),
        60,
        5,
        8.0,
        16.0,
        Arc::new(Quiet),
    )
    .expect("starts");
    let until = Instant::now() + Duration::from_secs(5);
    let text = loop {
        terminal.frame();
        let text = terminal.screen_text();
        if text.contains("PORTS=") || Instant::now() > until {
            break text;
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    terminal.close();
    text.lines().find(|line| line.starts_with("PORT=")).unwrap_or_default().to_owned()
}

fn git(dir: &Path, args: &[&str]) -> String {
    let output = wormhole_term::sys::process::command("git")
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.com", "-c", "init.defaultBranch=main"])
        .args(["-c", "protocol.file.allow=always"])
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .output()
        .expect("git runs");
    assert!(output.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn write(dir: &Path, path: &str, bytes: impl AsRef<[u8]>) {
    let path = dir.join(path);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(path, bytes).expect("write");
}

/// A repository with one commit holding `files`, in a real (symlink-free) temp path.
fn repo_with(files: &[(&str, &str)]) -> (tempfile::TempDir, String) {
    let dir = tempfile::tempdir().expect("tempdir");
    let root = dir.path().canonicalize().expect("canonical");
    git(&root, &["init", "-q"]);
    git(&root, &["config", "core.excludesFile", "/dev/null"]);
    for (path, text) in files {
        write(&root, path, text);
    }
    git(&root, &["add", "-A"]);
    git(&root, &["commit", "-q", "-m", "initial"]);
    let folder = root.to_string_lossy().into_owned();
    (dir, folder)
}

/// A parsed diff as the lines a person reads: what happened to the file, then each hunk with its marks.
fn render(file: &GitDiffFile) -> String {
    let mut out = match (&file.old_path, &file.new_path) {
        (None, _) => "new file\n".to_string(),
        (_, None) => "deleted file\n".to_string(),
        (Some(old), Some(new)) if old != new => format!("rename from {old}\nrename to {new}\n"),
        _ => String::new(),
    };
    for hunk in &file.hunks {
        out += &format!("{}\n", hunk.header);
        for line in &hunk.lines {
            let mark = match line.kind {
                GitLineKind::Context => ' ',
                GitLineKind::Added => '+',
                GitLineKind::Removed => '-',
            };
            out += &format!("{mark}{}\n", line.text);
            if line.no_newline {
                out += "\\ No newline at end of file\n";
            }
        }
    }
    out
}

fn text(diff: GitDiff) -> String {
    match diff {
        GitDiff::Text { file } => render(&file),
        other => panic!("expected text, got {other:?}"),
    }
}

#[test]
fn status_lists_each_kind_with_counts_and_binaries_without() {
    let (_dir, folder) =
        repo_with(&[("a.txt", "1\n2\n3\n"), ("old.txt", "keep\nme\nhere\nplease\n"), ("gone.txt", "x\n")]);
    let root = Path::new(&folder);
    write(root, "a.txt", "1\n2 changed\n3\n4\n");
    git(root, &["mv", "old.txt", "new.txt"]);
    std::fs::remove_file(root.join("gone.txt")).expect("remove");
    write(root, "fresh.txt", "a\nb\n");
    write(root, "logo.bin", [0u8, 1, 2, 0, 255]);
    let git_obj = Git::new().expect("isolated");
    assert!(git_obj.is_repository(folder.clone()));
    assert!(
        !git_obj
            .is_repository(std::env::temp_dir().join("no-such-repo-folder").to_string_lossy().into_owned())
    );

    let status = git_obj.status(folder, None).expect("status");
    assert_eq!(status.branch.as_deref(), Some("main"));
    assert!(status.dirty);
    let by = |path: &str| status.files.iter().find(|f| f.path == path).unwrap_or_else(|| panic!("{path}"));
    assert_eq!(
        (by("a.txt").kind, by("a.txt").added, by("a.txt").removed),
        (GitChangeKind::Modified, Some(2), Some(1))
    );
    assert_eq!(by("new.txt").kind, GitChangeKind::Renamed);
    assert_eq!(by("new.txt").original_path.as_deref(), Some("old.txt"));
    assert_eq!(by("gone.txt").kind, GitChangeKind::Deleted);
    assert_eq!((by("fresh.txt").kind, by("fresh.txt").added), (GitChangeKind::Untracked, Some(2)));
    assert_eq!(
        (by("logo.bin").kind, by("logo.bin").added),
        (GitChangeKind::Untracked, None),
        "binary: no counts"
    );
}

#[test]
fn a_folder_outside_any_repository_says_so() {
    let dir = tempfile::tempdir().expect("tempdir");
    let git_obj = Git::new().expect("isolated");
    let outside = dir.path().to_string_lossy().into_owned();
    assert!(matches!(git_obj.status(outside.clone(), None), Err(GitBackendError::NotARepository)));
    assert!(matches!(git_obj.log(outside, None, vec![]), Err(GitBackendError::NotARepository)));
}

#[test]
fn diffs_are_unified_text_with_renames_binaries_and_untracked_files() {
    let (_dir, folder) =
        repo_with(&[("a.txt", "1\n2\n3\n"), ("old.txt", "k\ne\ne\np\n"), ("img.bin", "\u{0}\u{1}")]);
    let root = Path::new(&folder);
    write(root, "a.txt", "1\nTWO\n3");
    git(root, &["mv", "old.txt", "new.txt"]);
    write(root, "img.bin", [0u8, 9, 9, 9, 0, 7]);
    write(root, "fresh.txt", "hello\n");
    let git_obj = Git::new().expect("isolated");
    git_obj.status(folder.clone(), None).expect("status");

    let modified = text(git_obj.diff(folder.clone(), "a.txt".into(), None, false).expect("diff"));
    assert!(modified.starts_with("@@ "), "{modified}");
    assert!(modified.contains("\n-2\n-3\n+TWO\n+3\n"), "{modified}");
    assert!(modified.ends_with("+3\n\\ No newline at end of file\n"), "{modified}");

    let renamed = text(git_obj.diff(folder.clone(), "new.txt".into(), None, false).expect("diff"));
    assert!(renamed.contains("rename from old.txt\nrename to new.txt\n"), "{renamed}");

    let untracked = text(git_obj.diff(folder.clone(), "fresh.txt".into(), None, false).expect("diff"));
    assert!(untracked.starts_with("new file\n") && untracked.contains("\n+hello\n"), "{untracked}");

    match git_obj.diff(folder, "img.bin".into(), None, false).expect("diff") {
        GitDiff::Binary { file, sizes } => {
            assert!(file.hunks.is_empty() && file.new_path.as_deref() == Some("img.bin"));
            assert_eq!(sizes.as_deref(), Some("2 bytes → 6 bytes"));
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_diff_over_the_cap_asks_first_and_loads_when_told() {
    let (_dir, folder) = repo_with(&[("a.txt", "a\n")]);
    // Just over core's 8 MiB ask-first size.
    let big = "0123456789abcdef0123456789abcdef\n".repeat(280_000);
    write(Path::new(&folder), "big.txt", &big);
    let git_obj = Git::new().expect("isolated");
    git_obj.status(folder.clone(), None).expect("status");
    match git_obj.diff(folder.clone(), "big.txt".into(), None, false).expect("diff") {
        GitDiff::TooLarge { title, detail, .. } => {
            assert!(title.starts_with("This diff is "), "{title}");
            assert!(detail.contains("Loading it can take a few seconds"), "{detail}");
        }
        other => panic!("{other:?}"),
    }
    let loaded = text(git_obj.diff(folder, "big.txt".into(), None, true).expect("loads anyway"));
    assert!(loaded.contains("+0123456789abcdef"));
}

#[test]
fn lfs_pointers_are_summarised_not_listed() {
    let (_dir, folder) = repo_with(&[(".gitattributes", "*.dat filter=lfs diff=lfs merge=lfs -text\n")]);
    let root = Path::new(&folder);
    let pointer = |oid: char, size: u32| {
        format!(
            "version https://git-lfs.github.com/spec/v1\noid sha256:{}\nsize {size}\n",
            oid.to_string().repeat(64)
        )
    };
    write(root, "model.dat", pointer('a', 1000));
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "lfs"]);
    write(root, "model.dat", pointer('b', 52_900));
    let git_obj = Git::new().expect("isolated");
    let status = git_obj.status(folder.clone(), None).expect("status");
    assert!(status.files.iter().any(|f| f.path == "model.dat" && f.lfs));
    match git_obj.diff(folder, "model.dat".into(), None, false).expect("diff") {
        GitDiff::Lfs { summary, .. } => assert_eq!(summary.as_deref(), Some("1 KB → 52.9 KB · oid bbbbbb…")),
        other => panic!("{other:?}"),
    }
}

#[test]
fn compare_with_a_branch_a_tag_or_a_commit_lists_what_changed_since() {
    let (_dir, folder) = repo_with(&[("a.txt", "a\n")]);
    let root = Path::new(&folder);
    git(root, &["tag", "v1"]);
    git(root, &["checkout", "-q", "-b", "feature"]);
    write(root, "b.txt", "b\nb\n");
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "add b"]);
    let tip = git(root, &["rev-parse", "HEAD"]).trim().to_string();
    git(root, &["checkout", "-q", "main"]);
    write(root, "c.txt", "c\n");
    git(root, &["add", "-A"]);
    git(root, &["commit", "-q", "-m", "add c on main"]);
    let git_obj = Git::new().expect("isolated");

    // Against the merge-base with the branch: main's own change since the split.
    for rev in ["feature", "v1", tip.as_str()] {
        let files = git_obj.status(folder.clone(), Some(rev.into())).expect("status").files;
        let paths: Vec<_> = files.iter().map(|f| f.path.as_str()).collect();
        assert!(paths.contains(&"c.txt"), "{rev}: {paths:?}");
    }
    // The diff of a file is against the same base.
    git_obj.status(folder.clone(), Some("feature".into())).expect("status");
    let patch =
        text(git_obj.diff(folder.clone(), "c.txt".into(), Some("feature".into()), false).expect("diff"));
    assert!(patch.contains("+c\n"), "{patch}");

    let missing = git_obj.status(folder, Some("gone".into())).expect("falls back to HEAD");
    assert_eq!(missing.compare_missing.as_deref(), Some("gone"));
}

#[test]
fn the_picker_lists_branches_tags_commits_and_stashes_for_history() {
    let (_dir, folder) = repo_with(&[("a.txt", "a\n")]);
    let root = Path::new(&folder);
    git(root, &["tag", "v1"]);
    git(root, &["branch", "side"]);
    let git_obj = Git::new().expect("isolated");
    let revs = git_obj.revs(folder.clone(), true).expect("revs");
    let kinds = |kind| revs.iter().filter(|r| r.kind == kind).map(|r| r.name.as_str()).collect::<Vec<_>>();
    assert_eq!(kinds(GitRevKind::Branch), ["main", "side"], "current first");
    assert_eq!(kinds(GitRevKind::Tag), ["v1"]);
    assert_eq!(
        revs.iter().find(|r| r.kind == GitRevKind::Commit).map(|r| r.detail.as_str()),
        Some("initial")
    );
    assert!(kinds(GitRevKind::Stashes).is_empty(), "no stash yet");

    write(root, "a.txt", "changed\n");
    git(root, &["stash", "push", "-q", "-m", "wip"]);
    let with = git_obj.revs(folder.clone(), true).expect("revs");
    assert_eq!(with.last().map(|r| r.kind), Some(GitRevKind::Stashes));
    assert!(
        git_obj.revs(folder, false).expect("revs").iter().all(|r| r.kind != GitRevKind::Stashes),
        "not in compare"
    );
}

#[test]
fn history_log_details_and_commit_diffs() {
    let (_dir, folder) = repo_with(&[("a.txt", "a\n"), ("dir/b.txt", "b\n")]);
    let root = Path::new(&folder);
    write(root, "dir/b.txt", "b2\nmore\n");
    write(root, "added.txt", "new\n");
    git(root, &["add", "-A"]);
    git(
        root,
        &[
            "commit",
            "-q",
            "-m",
            "second subject",
            "-m",
            "A body line.\n\nCo-authored-by: Ada <ada@example.com>",
        ],
    );
    git(root, &["mv", "a.txt", "renamed.txt"]);
    git(root, &["commit", "-q", "-m", "rename a"]);
    git(root, &["tag", "v2"]);
    let git_obj = Git::new().expect("isolated");

    let log = git_obj.log(folder.clone(), None, vec![]).expect("log");
    assert_eq!(log.branch.as_deref(), Some("main"));
    assert!(!log.limited);
    let subjects: Vec<_> = log.commits.iter().map(|c| c.subject.as_str()).collect();
    assert_eq!(subjects, ["rename a", "second subject", "initial"]);
    assert!(log.commits[0].refs.iter().any(|r| r.name == "v2"));

    let only_b = git_obj.log(folder.clone(), None, vec!["dir/b.txt".into()]).expect("log");
    assert_eq!(only_b.commits.len(), 2, "the commits that touched it");

    let sha = log.commits[1].sha.clone();
    let details = git_obj.commit_details(folder.clone(), sha.clone()).expect("details");
    assert_eq!(details.subject, "second subject");
    assert!(details.whole.starts_with("A body line."), "{}", details.whole);
    assert_eq!(details.trailers, "Co-authored-by: Ada <ada@example.com>");
    assert!(details.message.ends_with("Co-authored-by: Ada <ada@example.com>"), "{}", details.message);
    assert_eq!(
        details.parents.iter().map(|p| p.sha.clone()).collect::<Vec<_>>(),
        [log.commits[2].sha.clone()]
    );
    assert_eq!(details.author_line, "Test <test@example.com>");
    let labels: Vec<_> = details.people.iter().map(|fact| fact.label.as_str()).collect();
    assert_eq!(labels, ["Author", "Co-author"], "the committer is the author, so one line");
    let changes: String = details.changes.spans.iter().map(|span| span.text.as_str()).collect();
    assert_eq!(changes, "2 files +3 \u{2212}1");
    // The estimate of the compact view: a short body shows whole, a narrow line cuts what is left.
    let compact = git_obj.compact_message(folder.clone(), sha.clone(), 80).expect("opened");
    assert!(compact.text.starts_with("A body line.") && compact.more, "the trailers are left out");
    assert!(git_obj.compact_message(folder.clone(), "0".repeat(40), 80).is_none());
    let paths: Vec<_> = details.files.iter().map(|f| (f.path.as_str(), f.kind)).collect();
    assert_eq!(paths, [("added.txt", GitChangeKind::Added), ("dir/b.txt", GitChangeKind::Modified)]);

    let patch = text(git_obj.commit_diff(folder.clone(), sha, "dir/b.txt".into(), false).expect("diff"));
    assert!(patch.contains("-b\n+b2\n+more\n"), "{patch}");
    // A commit not opened first (the cache is empty for it) reads its files itself.
    let rename = log.commits[0].sha.clone();
    let patch = text(git_obj.commit_diff(folder.clone(), rename, "renamed.txt".into(), false).expect("diff"));
    assert_eq!(patch, "rename from a.txt\nrename to renamed.txt\n");

    let bad = git_obj.log(
        folder,
        Some(GitRev { name: "no-such".into(), kind: GitRevKind::Branch, detail: String::new() }),
        vec![],
    );
    assert!(matches!(bad, Err(GitBackendError::Failed { message }) if message.contains("no-such")));
}

#[test]
fn stashes_are_a_log_of_their_own() {
    let (_dir, folder) = repo_with(&[("a.txt", "a\n")]);
    let root = Path::new(&folder);
    write(root, "a.txt", "wip\n");
    git(root, &["stash", "push", "-q", "-m", "wip"]);
    let git_obj = Git::new().expect("isolated");
    let stash = GitRev { name: "stash".into(), kind: GitRevKind::Stashes, detail: String::new() };
    let log = git_obj.log(folder.clone(), Some(stash), vec![]).expect("stashes");
    assert_eq!(log.commits.len(), 1);
    let files = git_obj.commit_details(folder, log.commits[0].sha.clone()).expect("details").files;
    assert_eq!(files.iter().map(|f| f.path.as_str()).collect::<Vec<_>>(), ["a.txt"]);
}

#[test]
fn a_submodule_is_a_row_in_core_s_words_and_a_diff_of_its_commits() {
    let (_sub, sub_folder) = repo_with(&[("lib.txt", "1\n")]);
    let (_dir, folder) = repo_with(&[("a.txt", "a\n")]);
    let root = Path::new(&folder);
    git(root, &["submodule", "add", "-q", &sub_folder, "vendor"]);
    git(root, &["commit", "-q", "-m", "add submodule"]);
    write(Path::new(&sub_folder), "lib.txt", "2\n");
    git(Path::new(&sub_folder), &["commit", "-q", "-am", "newer"]);
    git(&root.join("vendor"), &["pull", "-q"]);
    let git_obj = Git::new().expect("isolated");
    let status = git_obj.status(folder.clone(), None).expect("status");
    let row = status.files.iter().find(|f| f.path == "vendor").expect("the submodule");
    assert_eq!(row.submodule.as_deref(), Some("1 new commit"));
    match git_obj.diff(folder, "vendor".into(), None, false).expect("diff") {
        GitDiff::Submodule { label, card } => {
            assert_eq!(label, "1 new commit");
            assert_eq!(card.ahead.iter().map(|c| c.subject.as_str()).collect::<Vec<_>>(), ["newer"]);
        }
        other => panic!("{other:?}"),
    }
}

struct Counting(Arc<AtomicUsize>);

impl GitListener for Counting {
    fn changed(&self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

fn wait_until(what: &str, until: impl Fn() -> bool) {
    let limit = Instant::now() + Duration::from_secs(10);
    while !until() {
        assert!(Instant::now() < limit, "timed out: {what}");
        std::thread::sleep(Duration::from_millis(25));
    }
}

#[test]
fn the_watch_calls_once_after_a_burst_of_edits_and_not_for_ignored_files() {
    let (_dir, folder) = repo_with(&[(".gitignore", "target/\n"), ("a.txt", "a\n")]);
    let root = Path::new(&folder);
    // The ignored folder exists before the watch starts, as core lists only the ignored paths that exist
    // (a new top-level folder is read once, in case it is an ignored one: a new `node_modules`).
    write(root, "target/first.o", "x");
    let calls = Arc::new(AtomicUsize::new(0));
    let watch = GitWatch::start(folder.clone(), Arc::new(Counting(calls.clone()))).expect("starts");
    wait_until("the watch is live", || watch.is_live());
    // The ignored list is read right after it starts; let it land before the quiet edit.
    std::thread::sleep(Duration::from_millis(600));
    write(root, "target/out.o", "x");
    std::thread::sleep(Duration::from_millis(900));
    assert_eq!(calls.load(Ordering::SeqCst), 0, "an ignored file is not a change");

    for n in 0..5 {
        write(root, "a.txt", format!("edit {n}\n"));
    }
    wait_until("the edit is reported", || calls.load(Ordering::SeqCst) >= 1);
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(calls.load(Ordering::SeqCst), 1, "a burst is one call");

    git(root, &["add", "-A"]);
    wait_until("the index change is reported", || calls.load(Ordering::SeqCst) >= 2);
    watch.stop();
}

#[test]
fn a_watch_of_no_repository_is_refused() {
    let dir = tempfile::tempdir().expect("tempdir");
    let calls = Arc::new(AtomicUsize::new(0));
    let refused = GitWatch::start(dir.path().to_string_lossy().into_owned(), Arc::new(Counting(calls)));
    assert!(matches!(refused, Err(GitBackendError::NotARepository)));
}

fn worktrees_of(service: &WorktreeService) -> Vec<String> {
    service.list().into_iter().map(|w| w.branch).collect()
}

#[test]
fn a_worktree_is_made_remembered_and_removed_but_never_forced() {
    let (dir, folder) = repo_with(&[("a.txt", "a\n")]);
    let service = WorktreeService::new(None, false).expect("isolated");
    assert_eq!(service.problem(folder.clone()), None);
    assert!(
        service.problem(std::env::temp_dir().join("nowhere-at-all").to_string_lossy().into_owned()).is_some()
    );

    // Planning writes nothing and says why a name is refused.
    let expected = dir.path().canonicalize().expect("canonical").parent().expect("parent").join(format!(
        "{}-worktrees/fix-login",
        Path::new(&folder).file_name().expect("name").to_string_lossy()
    ));
    assert_eq!(
        service.plan("fix/login".into(), folder.clone()),
        GitWorktreePlan::Ready {
            branch: "fix/login".into(),
            folder: expected.to_string_lossy().into_owned()
        }
    );
    assert!(!expected.exists());
    assert!(matches!(service.plan("has space".into(), folder.clone()), GitWorktreePlan::Refused { .. }));
    assert!(matches!(service.plan("-x".into(), folder.clone()), GitWorktreePlan::Refused { .. }));

    let made = service.create("fix/login".into(), folder.clone()).expect("created");
    assert_eq!(PathBuf::from(&made.folder), expected);
    assert!(expected.join("a.txt").exists(), "checked out");
    assert!(made.env.iter().any(|v| v.name == "WORMHOLE_PORT"), "its own ports");
    // A terminal started there has them in its shell's environment, one in the repository itself does not.
    let value = |name: &str| made.env.iter().find(|v| v.name == name).map(|v| v.value.clone()).expect(name);
    assert_eq!(
        ports_seen_in(&made.folder),
        format!("PORT={} PORTS={}", value("WORMHOLE_PORT"), value("WORMHOLE_PORTS"))
    );
    assert_eq!(ports_seen_in(&folder), "PORT= PORTS=", "the main working tree has none");
    assert!(git(Path::new(&folder), &["branch", "--list", "fix/login"]).contains("fix/login"));
    assert!(worktrees_of(&service).contains(&"fix/login".to_string()));

    // From inside it, it is known as made; a terminal there makes siblings of the same repository.
    assert_eq!(
        service.made_at(expected.join("sub").to_string_lossy().into_owned()).map(|w| w.branch),
        Some("fix/login".into())
    );
    assert_eq!(service.made_at(folder.clone()), None);
    let sibling = service.plan("other".into(), made.folder.clone());
    assert!(
        matches!(&sibling, GitWorktreePlan::Ready { folder, .. } if folder.ends_with("-worktrees/other")),
        "{sibling:?}"
    );
    assert!(service.create("fix/login".into(), folder.clone()).is_err(), "the folder is there already");

    // Changes keep it from being removed, and nothing is forced.
    write(&expected, "dirty.txt", "x");
    assert!(service.is_dirty(made.folder.clone()).expect("asked"));
    let refused = service.remove(made.folder.clone());
    assert!(
        matches!(&refused, Err(GitBackendError::Failed { message }) if message.contains("uncommitted")),
        "{refused:?}"
    );
    assert!(expected.exists());
    std::fs::remove_file(expected.join("dirty.txt")).expect("clean");
    service.remove(made.folder.clone()).expect("removed");
    assert!(!expected.exists());
    assert_eq!(service.made_at(made.folder), None);
    assert!(
        git(Path::new(&folder), &["branch", "--list", "fix/login"]).contains("fix/login"),
        "the branch stays"
    );

    // One it did not make is never removed.
    git(Path::new(&folder), &["worktree", "add", "-q", "-b", "by-hand", &format!("{folder}-by-hand")]);
    let by_hand = format!("{folder}-by-hand");
    assert!(matches!(service.remove(by_hand.clone()), Err(GitBackendError::Failed { .. })));
    assert!(Path::new(&by_hand).exists());
    git(Path::new(&folder), &["worktree", "remove", &by_hand]);
}

#[test]
fn an_existing_branch_is_checked_out_and_the_base_setting_moves_the_folder() {
    let (_dir, folder) = repo_with(&[("a.txt", "a\n")]);
    git(Path::new(&folder), &["branch", "existing"]);
    let base = tempfile::tempdir().expect("tempdir");
    let base_path = base.path().canonicalize().expect("canonical");
    let service =
        WorktreeService::new(Some(base_path.to_string_lossy().into_owned()), false).expect("isolated");
    let made = service.create("existing".into(), folder.clone()).expect("created");
    let name = Path::new(&folder).file_name().expect("name").to_owned();
    assert_eq!(PathBuf::from(&made.folder), base_path.join(name).join("existing"));
    service.remove(made.folder).expect("removed");
}

/// Two windows of one profile each hold a service: what one makes the other's next write must not lose, and the two
/// never hand out the same ports.
#[test]
fn two_services_of_one_profile_keep_each_others_worktrees_and_ports() {
    let (_dir, folder) = repo_with(&[("a.txt", "a\n")]);
    let (first, second) = (
        WorktreeService::new(None, false).expect("isolated"),
        WorktreeService::new(None, false).expect("isolated"),
    );
    // Both read the (empty) list before either writes.
    assert!(!worktrees_of(&first).contains(&"two/a".to_string()));
    assert!(!worktrees_of(&second).contains(&"two/b".to_string()));
    let a = first.create("two/a".into(), folder.clone()).expect("created");
    let b = second.create("two/b".into(), folder.clone()).expect("created");
    for service in [&first, &second] {
        let branches = worktrees_of(service);
        assert!(
            branches.contains(&"two/a".to_string()) && branches.contains(&"two/b".to_string()),
            "{branches:?}"
        );
    }
    let port = |w: &wormhole_ffi::GitWorktree| {
        w.env.iter().find(|v| v.name == "WORMHOLE_PORT").map(|v| v.value.clone())
    };
    assert!(port(&a).is_some() && port(&a) != port(&b), "one block of ports each");
    // The second forgets nothing of the first's when it removes its own.
    second.remove(b.folder).expect("removed");
    assert!(worktrees_of(&first).contains(&"two/a".to_string()));
    first.remove(a.folder).expect("removed");
}
