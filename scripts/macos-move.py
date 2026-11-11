#!/usr/bin/env python3
"""Moves the Swift app out of `apple/` into its places: the UniFFI bridge to `crates/wormhole-ffi` (its bindings
generator inside it), everything else to `macos/`. `git mv` for every file, then the paths written in the
tracked text files follow. Run it from the repository's root; run again, it finds nothing left to do.

    python3 scripts/macos-move.py            # move and rewrite
    python3 scripts/macos-move.py --check    # say what a run would do, change nothing

The bridge keeps a workspace of its own (itself and `bindgen`, with its own `Cargo.lock`): the root `Cargo.toml`
lists it under `exclude`, so it is not in the app's lock file, licences or gate until that line goes.
"""
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CHECK = "--check" in sys.argv

# (from, to), in this order: a folder before what is inside it would be found twice.
MOVES = [
    ("apple/ffi", "crates/wormhole-ffi"),
    ("apple/bindgen", "crates/wormhole-ffi/bindgen"),
    ("apple/Cargo.lock", "crates/wormhole-ffi/Cargo.lock"),
    ("apple/.cargo", "crates/wormhole-ffi/.cargo"),
    ("apple/swift", "macos/swift"),
    ("apple/scripts", "macos/scripts"),
    ("apple/parity", "macos/parity"),
    ("apple/ffi-bench", "macos/ffi-bench"),
    ("apple/tools", "macos/tools"),
    ("apple/FFI-CONTRACT.md", "macos/FFI-CONTRACT.md"),
    ("apple/MEASURING.md", "macos/MEASURING.md"),
    ("apple/.gitignore", "macos/.gitignore"),
]

# The same files as the root's, which a crate under `crates/` finds by walking up.
DROPPED = ["apple/rust-toolchain.toml", "apple/rustfmt.toml", "apple/clippy.toml"]

# Paths as they are written in text, longest first. `apple/` only where nothing of a longer name runs into it
# (`developer.apple.com/`, `com.apple/` stay), and never a shell variable (`$apple/`).
REWRITES = [
    (re.compile(r"(?<![\w.$-])apple/ffi-bench"), "macos/ffi-bench"),
    (re.compile(r"(?<![\w.$-])apple/ffi(?![\w-])"), "crates/wormhole-ffi"),
    (re.compile(r"(?<![\w.$-])apple/bindgen"), "crates/wormhole-ffi/bindgen"),
    (re.compile(r"(?<![\w.$-])apple/target"), "crates/wormhole-ffi/target"),
    (re.compile(r"(?<![\w.$-])apple/\.cargo"), "crates/wormhole-ffi/.cargo"),
    (re.compile(r"(?<![\w.$-])apple/Cargo\.(toml|lock)"), r"crates/wormhole-ffi/Cargo.\1"),
    (re.compile(r"(?<![\w.$-])apple/"), "macos/"),
]


def git(*args: str) -> str:
    return subprocess.run(["git", *args], cwd=ROOT, check=True, capture_output=True, text=True).stdout


def tracked(path: str) -> bool:
    return bool(git("ls-files", "--", path).strip())


def move() -> int:
    done = 0
    for source, target in MOVES:
        if not tracked(source):
            continue
        print(f"git mv {source} {target}")
        if not CHECK:
            (ROOT / target).parent.mkdir(parents=True, exist_ok=True)
            git("mv", source, target)
        done += 1
    for path in DROPPED:
        if tracked(path):
            print(f"git rm {path}")
            if not CHECK:
                git("rm", "-q", path)
            done += 1
    return done


def workspace() -> int:
    """The bridge's own workspace table, which `apple/Cargo.toml` held, goes to the bridge's manifest."""
    old = ROOT / "apple/Cargo.toml"
    manifest = ROOT / "crates/wormhole-ffi/Cargo.toml"
    if not tracked("apple/Cargo.toml"):
        return 0
    print("apple/Cargo.toml -> the [workspace] tables of crates/wormhole-ffi/Cargo.toml")
    if CHECK:
        return 1
    tables = old.read_text()
    start = tables.index("[workspace]")
    tables = tables[start:].replace('members = ["ffi", "bindgen"]', 'members = ["bindgen"]')
    header = (
        "\n# A workspace of its own (this crate and its bindings generator), so that nothing here reaches the app's\n"
        "# `Cargo.lock`, `cargo deny`, `licenses.json` or `scripts/structure.py` yet: the root `Cargo.toml` lists this\n"
        "# folder under `exclude`. The app's crates come in by path; none of them may name anything in here.\n"
    )
    text = manifest.read_text().replace("../../crates/wormhole-", "../wormhole-")
    manifest.write_text(text.rstrip("\n") + "\n" + header + tables)
    git("rm", "-q", "apple/Cargo.toml")
    git("add", "crates/wormhole-ffi/Cargo.toml")
    return 1


def rewrite() -> int:
    changed = 0
    for name in git("ls-files", "-z").split("\0"):
        path = ROOT / name
        if not name or path.is_symlink() or not path.is_file() or name == "scripts/macos-move.py":
            continue
        if name.endswith((".lock", ".png", ".jpg", ".icns", ".ttf", ".otf", ".woff2", ".bin", ".hex", ".tar", ".gz")):
            continue
        try:
            text = path.read_text()
        except UnicodeDecodeError:
            continue
        new = text
        for pattern, to in REWRITES:
            new = pattern.sub(to, new)
        if new != text:
            changed += 1
            if CHECK:
                print(f"rewrite {name}")
            else:
                path.write_text(new)
    if changed and not CHECK:
        git("add", "-u")
    return changed


def main() -> None:
    moved, tables, rewritten = move(), workspace(), rewrite()
    if not (moved or tables or rewritten):
        print("nothing to move: apple/ is already in its places")
        return
    print(f"{'would move' if CHECK else 'moved'} {moved} paths, rewrote {rewritten} files")


if __name__ == "__main__":
    main()
