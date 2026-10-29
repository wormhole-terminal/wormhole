#!/usr/bin/env python3
"""Move the app's source checks out of `crates/wormhole/tests/` into a crate of their own, `wormhole-rules`.

  scripts/rules-crate.py                  in this repository: `git mv` the four test files and `support/`,
                                          create the crate (Cargo.toml, a lib.rs saying what it is), point
                                          `support::src()` at the app's sources, and rewrite every path and
                                          command that names the old place in the tracked text files
  scripts/rules-crate.py --docs <repo>    only the rewrite, in another checkout (the private notes next to
                                          this repository: CLAUDE.md, repo-docs/)

Why: Cargo builds a package's binaries whenever it has integration tests, so the app's `tests/` folder made
every `cargo test` of the app compile all of it a second time, as the plain binary none of these tests runs:
they read files as text and run bash. Moved, the app compiles once (its unit tests), and a change to the
wording checks (`design_rules`) builds nothing but std.

Deterministic and idempotent: a second run finds nothing to do. It runs no cargo; after it, `cargo fmt --all`
and the gate. Python 3.9, standard library only.
"""
from __future__ import annotations

import argparse
import re
import subprocess
import sys
from pathlib import Path

OLD = "crates/wormhole/tests"
NEW = "crates/wormhole-rules/tests"
TESTS = ["background_rules", "design_rules", "isolated_instance", "text_rules"]

CARGO_TOML = """\
[package]
name = "wormhole-rules"
description = "Checks over the repository's sources and scripts, read as text: the app's design, words and window rules, and the test instance launcher. Nothing here links the app."
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true
publish.workspace = true

[lib]
doctest = false

# None on purpose: the checks read files, so a change to them compiles std and nothing else.
[dependencies]

[lints]
workspace = true
"""

LIB_RS = """\
//! Checks over the repository's sources and scripts, read as text. The tests are in `tests/`; this file
//! only gives Cargo a target to hang them on.
//!
//! They live outside the app crate because Cargo builds a package's binaries for its integration tests:
//! in `crates/wormhole/tests/` they made every test run compile the whole app a second time, as a binary
//! none of them runs.
"""

# Old spelling → new, in every tracked text file. Order matters: the full paths first, so the bare
# `tests/x.rs` rule never sees one it already rewrote.
REWRITES: list[tuple[re.Pattern[str], str]] = [
    (re.compile(re.escape(OLD + "/")), NEW + "/"),
    (re.compile(r"(?<![\w/-])tests/(" + "|".join(TESTS) + r")\.rs"), NEW + r"/\1.rs"),
    (re.compile(r"-p wormhole --features devctl --test (" + "|".join(TESTS) + r")\b"), r"-p wormhole-rules --test \1"),
]

# History, not instructions: what an old changelog entry or an as-built note says stays as it was written.
KEEP = {"CHANGELOG.md", ".git-blame-ignore-revs", "scripts/rules-crate.py", "crates/wormhole-rules/src/lib.rs"}
KEEP_PREFIXES = ("repo-docs/docs/landing/", "repo-docs/docs/app-structure-phase")


def git(root: Path, *args: str) -> str:
    return subprocess.run(["git", "-C", str(root), *args], check=True, capture_output=True, text=True).stdout


def move(root: Path) -> list[str]:
    done = []
    for name in TESTS:
        if (root / OLD / f"{name}.rs").exists():
            (root / NEW).mkdir(parents=True, exist_ok=True)
            git(root, "mv", f"{OLD}/{name}.rs", f"{NEW}/{name}.rs")
            done.append(f"moved {name}.rs")
    if (root / OLD / "support").exists():
        git(root, "mv", f"{OLD}/support", f"{NEW}/support")
        done.append("moved support/")
    crate = root / "crates/wormhole-rules"
    for path, text in [(crate / "Cargo.toml", CARGO_TOML), (crate / "src/lib.rs", LIB_RS)]:
        if not path.exists():
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_text(text)
            git(root, "add", str(path.relative_to(root)))
            done.append(f"created {path.relative_to(root)}")
    support = root / NEW / "support/mod.rs"
    old_src = 'Path::new(env!("CARGO_MANIFEST_DIR")).join("src")'
    new_src = 'Path::new(env!("CARGO_MANIFEST_DIR")).join("../wormhole/src")'
    if support.exists() and old_src in support.read_text():
        support.write_text(support.read_text().replace(old_src, new_src))
        done.append("support::src() reads the app's sources")
    # The folder picker's tests record two real folders of the app crate; `tests/` was one of them and is
    # gone (git leaves the empty folder on disk, so only a fresh checkout shows it). `assets/` is tracked
    # and, typed, matches none of the test tree's folders, as `tests` didn't.
    picker = root / "crates/wormhole/src/folder_picker/tests.rs"
    if picker.exists():
        text = picker.read_text()
        new = (
            text.replace('manifest.join("tests")', 'manifest.join("assets")')
            .replace("let (src, tests) = real_folders();", "let (src, assets) = real_folders();")
            .replace("recent_folders::record(cx, &profile, &tests);", "recent_folders::record(cx, &profile, &assets);")
            .replace("go_to(cx, &window, &tests);", "go_to(cx, &window, &assets);")
            .replace('recent.contains(&"tests")', 'recent.contains(&"assets")')
            .replace('typed(cx, &window, "tests");', 'typed(cx, &window, "assets");')
            .replace('row.label == "tests"', 'row.label == "assets"')
        )
        if new != text:
            picker.write_text(new)
            done.append("the folder picker's tests use assets/ as their second real folder")
    return done


def rewrite(root: Path) -> list[str]:
    done = []
    for rel in git(root, "ls-files", "-z").split("\0"):
        if not rel or rel in KEEP or rel.startswith(KEEP_PREFIXES):
            continue
        path = root / rel
        if path.is_symlink() or not path.is_file():
            continue
        try:
            text = path.read_text()
        except (UnicodeDecodeError, OSError):
            continue
        new = text
        for pattern, replacement in REWRITES:
            new = pattern.sub(replacement, new)
        if new != text:
            path.write_text(new)
            done.append(f"rewrote {rel}")
    return done


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.split("\n")[0])
    parser.add_argument("--docs", metavar="REPO", help="only rewrite the paths, in this other checkout")
    args = parser.parse_args()
    if args.docs:
        done = rewrite(Path(args.docs).resolve())
    else:
        root = Path(__file__).resolve().parent.parent
        done = move(root) + rewrite(root)
    print("\n".join(done) if done else "nothing to do")
    return 0


if __name__ == "__main__":
    sys.exit(main())
