#!/usr/bin/env python3
"""The few structure rules that matter for the Swift UI experiment, repeated for `apple/` because
`scripts/structure.py` scans only `crates/*/src` and does not see it (plan 6.2).

    python3 -I apple/scripts/rules.py

Rules, each with the reason it exists:
  file-size      no source file over 1,000 lines (the project's limit; over 600 is only counted);
  bucket-names   no file or folder named utils, helpers, common, misc (the project bans them);
  unsafe         `unsafe` and `allow(unsafe_code)` in Rust only in the ffi crate's lib.rs (UniFFI's
                 generated scaffolding) and c_abi.rs (a hand-written seam, if the hot path ever needs it);
  direction      the app's crates never name wormhole-ffi (dependencies point one way);
  no-polling     no Timer.scheduledTimer / Timer.publish / Task.sleep in the Swift sources: an idle
                 window sleeps until an event (events.md); the measuring tool (ffi-bench) is exempt, and a call with
                 a `// one-shot: <what it waits for>` comment on its line or the two above is a deadline, not polling;
  no-process     the Swift sources never spawn a process (everything goes through the seam).

Generated and built files (dist/, target/, .build/, the links build-ffi.sh makes) are skipped.
Exit status 1 and a line per violation when a rule is broken.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

APPLE = Path(__file__).resolve().parent.parent
REPO = APPLE.parent

SKIP_DIRS = {"target", ".build", "dist", ".git"}
# What build-ffi.sh links or generates inside a package.
GENERATED = [re.compile(r"(^|/)Sources/WormholeBindings(/|$)"), re.compile(r"\.xcframework(/|$)")]
SOURCE_SUFFIXES = {".rs", ".swift", ".py", ".sh", ".metal"}
BUCKET_NAMES = {"utils", "util", "helpers", "helper", "common", "misc"}
LIMIT = 1000
UNSAFE_ALLOWED = {Path("ffi/src/lib.rs"), Path("ffi/src/c_abi.rs")}
POLLING = re.compile(r"\bTimer\.(scheduledTimer|publish)\b|\bTask\.sleep\b")
PROCESS = re.compile(r"\bProcess\(\)|\bNSTask\b|\bposix_spawn\b")
UNSAFE_RUST = re.compile(r"\bunsafe\b")


def sources():
    for path in sorted(APPLE.rglob("*")):
        rel = path.relative_to(APPLE)
        if any(part in SKIP_DIRS for part in rel.parts):
            continue
        if any(pattern.search(rel.as_posix()) for pattern in GENERATED):
            continue
        yield rel, path


def main() -> int:
    problems: list[str] = []
    for rel, path in sources():
        if rel.name.lower().split(".")[0] in BUCKET_NAMES:
            problems.append(f"bucket-names: {rel}")
        if not path.is_file() or path.suffix not in SOURCE_SUFFIXES:
            continue
        text = path.read_text(encoding="utf-8")
        lines = text.count("\n") + (0 if text.endswith("\n") or not text else 1)
        if lines > LIMIT:
            problems.append(f"file-size: {rel} has {lines} lines (limit {LIMIT})")
        if path.suffix == ".rs" and rel not in UNSAFE_ALLOWED:
            for number, line in enumerate(text.splitlines(), 1):
                if UNSAFE_RUST.search(line) and not line.lstrip().startswith("//"):
                    problems.append(f"unsafe: {rel}:{number}: {line.strip()}")
        if path.suffix == ".swift" and rel.parts[0] != "ffi-bench":
            swift_lines = text.splitlines()
            for number, line in enumerate(swift_lines, 1):
                code = line.split("//", 1)[0]
                # A one-shot deadline (a debounce, a delay) is allowed when a `// one-shot:` comment says what it waits for.
                if POLLING.search(code) and not any("one-shot:" in l for l in swift_lines[max(0, number - 3) : number]):
                    problems.append(f"no-polling: {rel}:{number}: {line.strip()}")
                if PROCESS.search(code):
                    problems.append(f"no-process: {rel}:{number}: {line.strip()}")

    for manifest in sorted((REPO / "crates").glob("*/Cargo.toml")) + [REPO / "Cargo.toml"]:
        if re.search(r"wormhole[-_]ffi", manifest.read_text(encoding="utf-8")):
            problems.append(f"direction: {manifest.relative_to(REPO)} names wormhole-ffi")
    for source in sorted((REPO / "crates").glob("*/src/**/*.rs")):
        if "wormhole_ffi" in source.read_text(encoding="utf-8"):
            problems.append(f"direction: {source.relative_to(REPO)} names wormhole_ffi")

    for problem in problems:
        print(problem)
    if problems:
        print(f"{len(problems)} problem(s)")
        return 1
    print("apple/ rules: ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
