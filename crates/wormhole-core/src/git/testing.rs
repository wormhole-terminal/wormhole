//! Throwaway repositories for the unit tests of this module (the end-to-end ones live in
//! `tests/git_repo.rs`, which can only see the public API).

use std::path::{Path, PathBuf};
use std::process::Command;

/// Run git in `dir` with a fixed identity and no user or system configuration.
pub(crate) fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args(["-c", "user.name=Test", "-c", "user.email=test@example.com", "-c", "init.defaultBranch=main"])
        .args(args)
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .status()
        .expect("git runs");
    assert!(status.success(), "git {args:?} failed");
}

pub(crate) fn write(dir: &Path, path: &str, text: &str) {
    let path = dir.join(path);
    std::fs::create_dir_all(path.parent().expect("has parent")).expect("mkdir");
    std::fs::write(path, text).expect("write");
}

/// An empty repository (no commits). The code under test reads the user's global config, so
/// the repository's own config switches off a global excludes file that could hide test files.
pub(crate) fn init() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    git(dir.path(), &["init", "-q"]);
    git(dir.path(), &["config", "core.excludesFile", "/dev/null"]);
    dir
}

/// A repository with one commit holding `files`.
pub(crate) fn repo_with(files: &[(&str, &str)]) -> tempfile::TempDir {
    let dir = init();
    for (path, text) in files {
        write(dir.path(), path, text);
    }
    git(dir.path(), &["add", "-A"]);
    git(dir.path(), &["commit", "-q", "-m", "initial"]);
    dir
}

/// How many git processes the closure started on this thread.
pub(crate) fn spawns(f: impl FnOnce()) -> usize {
    let before = super::cmd::SPAWNS.with(std::cell::Cell::get);
    f();
    super::cmd::SPAWNS.with(std::cell::Cell::get) - before
}

/// Run `f` with `program` in place of git for every command it starts on this thread.
pub(crate) fn with_git<T>(program: impl AsRef<Path>, f: impl FnOnce() -> T) -> T {
    super::cmd::PROGRAM.set(Some(program.as_ref().into()));
    let out = f();
    super::cmd::PROGRAM.set(None);
    out
}

/// A stand-in for git, `dir/<name>`, that prints `stderr` and exits with `code` whatever it is
/// asked: git as it fails on someone else's machine.
pub(crate) fn fake_git(dir: &Path, name: &str, stderr: &str, code: i32) -> PathBuf {
    let said = dir.join(format!("{name}.stderr"));
    std::fs::write(&said, stderr).expect("write stderr");
    let path = dir.join(name);
    std::fs::write(&path, format!("#!/bin/sh\ncat '{}' >&2\nexit {code}\n", said.display())).expect("script");
    std::fs::set_permissions(&path, crate::platform::fs::permissions(0o755)).expect("chmod");
    path
}

/// What git says in a folder that is on a drive owned by someone else.
pub(crate) const DUBIOUS: &str = "fatal: detected dubious ownership in repository at '/Volumes/work/app'
To add an exception for this directory, call:

\tgit config --global --add safe.directory /Volumes/work/app";
