#!/usr/bin/env python3
"""Words check for the Swift sources, like the GPUI app's `text_rules` (crates/wormhole-rules/tests/text_rules.rs).

    python3 -I macos/scripts/words.py             # check; exit 1 on a problem
    python3 -I macos/scripts/words.py --update    # the allowlist only ever shrinks (first run writes it)
    python3 -I macos/scripts/words.py --selftest

What a person reads is a message of `crates/wormhole-text`'s catalogs, asked for through the Words seam
(`Words.text("fluent-id", fallback: "English")`, or with variables `Words.text("id", ["name": .text(x)], fallback: ...)`;
a module's `text(...)`/`tr(...)` helper that ends in it counts). Three checks, per file under `swift/Sources`:

  literal   a string literal that reads as text (two letters in a row and a space or a capital first) that is not
            the English fallback of a seam call. `// text: <why>` on its line says it is not interface text (an id,
            a token path, sample content, a shell word); `// text-file: <why>` in a file's first lines says the same of the whole file (brand names, rule
            formulas). Dev tooling (control commands, mocks, samples) is skipped.
  id        an id that is in no `locales/en/*.ftl` (the pseudo-language and a translation could not reach it).
  vars      a message with variables asked for without them, or with names other than the catalog's.

`words-allow.json` holds the per-file counts that exist today ("check file": count); a count at or under its entry is
grandfathered, a new file or a higher count is a new problem, and `--update` only lowers entries.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

APPLE = Path(__file__).resolve().parent.parent
SOURCES = APPLE / "swift" / "Sources"
LOCALES = APPLE.parent / "crates" / "wormhole-text" / "locales" / "en"
ALLOW = Path(__file__).with_name("words-allow.json")

SKIP_PATH = re.compile(r"Control|Mock|Sample|Fixture|DevCtl|Driving|FramesRun|RenderPNG|Isolation|Options|UniFFISource|WormholeBindings")
# A line that is not a place for interface text: logs, assertions, imports, ids and keys of code.
SKIP_LINE = re.compile(
    r"//\s*text:|\bprint\(|\bNSLog\(|assert|fatalError|precondition|XCTAssert|#error|\blog\w*\(|Logger|os_log|"
    r"\.debug\(|\.info\(|\.warning\(|\.error\(|^\s*import |@available|#available|#if|\.accessibilityIdentifier|"
    r"Image\(systemName|NSImage\(systemSymbolName|\.id\(|^\s*case \"|\bcase \.\w+\(\"|#Preview"
)
# `name("some-fluent-id", ` : the line holds a seam call's id and the start of its English.
SEAM_CALL = re.compile(r"""(?:\bWords\.text|\btext|\btr|\bword|\bL10n\.tr|\bpanelText)\(\s*"([a-z][a-z0-9]*(?:-[a-z0-9]+)+(?:\.[a-z0-9-]+)?)"\s*,""")
ID_ONLY = re.compile(r"""^[a-z][a-z0-9]*(?:-[a-z0-9]+)+(?:\.[a-z0-9-]+)?$""")
LITERAL = re.compile(r'"((?:[^"\\\n]|\\.)*)"')


def catalog() -> dict[str, set[str]]:
    """Every message id (and `id.attribute`) with the variables its text names."""
    ids: dict[str, set[str]] = {}
    message = target = None
    for path in sorted(LOCALES.glob("*.ftl")):
        for line in path.read_text().splitlines():
            m = re.match(r"^([a-z][a-z0-9-]*)\s*=", line)
            a = re.match(r"^\s+\.([a-z][a-z0-9-]*)\s*=", line)
            if m:
                message = target = m.group(1)
                ids[target] = set()
            elif a and message:
                target = f"{message}.{a.group(1)}"
                ids[target] = set()
            elif line.startswith("#") or not line.strip():
                continue
            if target and not line.startswith("#"):
                ids[target].update(re.findall(r"\$([a-zA-Z][a-zA-Z0-9_]*)", line))
    return ids


def reads_as_text(text: str) -> bool:
    text = re.sub(r"\\\([^)]*\)", "", text)
    text = re.sub(r"\\.", "", text)
    run = 0
    has_word = False
    for c in text:
        run = run + 1 if c.isalpha() else 0
        if run >= 2:
            has_word = True
    return has_word and (" " in text or text[:1].isupper())


def code_part(line: str) -> str:
    """The line without a trailing `//` comment (outside a literal)."""
    in_str = False
    i = 0
    while i < len(line):
        c = line[i]
        if c == "\\" and in_str:
            i += 2
            continue
        if c == '"':
            in_str = not in_str
        elif not in_str and line.startswith("//", i):
            return line[:i]
        i += 1
    return line


def call_text(text: str, start: int) -> str:
    """From the `(` at `start` to its matching `)` (strings skipped)."""
    depth = 0
    i = start
    in_str = False
    while i < len(text):
        c = text[i]
        if in_str:
            if c == "\\":
                i += 1
            elif c == '"':
                in_str = False
        elif c == '"':
            in_str = True
        elif c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
            if depth == 0:
                return text[start : i + 1]
        i += 1
    return text[start:]


def check_file(path: Path, ids: dict[str, set[str]]) -> dict[str, list[str]]:
    found: dict[str, list[str]] = {"literal": [], "id": [], "vars": []}
    text = path.read_text(errors="ignore")
    rel = path.relative_to(SOURCES)
    if "// text-file:" in "\n".join(text.splitlines()[:15]):
        return found
    for m in SEAM_CALL.finditer(text):
        ident = m.group(1)
        line = text.count("\n", 0, m.start()) + 1
        if ident not in ids:
            found["id"].append(f"{rel}:{line}: {ident}")
            continue
        call = call_text(text, text.index("(", m.start()))
        given = set(re.findall(r'"([a-zA-Z][a-zA-Z0-9_]*)"\s*:\s*\.(?:text|whole)\(', call))
        if given != ids[ident]:
            found["vars"].append(f"{rel}:{line}: {ident} takes {sorted(ids[ident])}, the call gives {sorted(given)}")
    in_multi = False
    for number, line in enumerate(text.splitlines(), 1):
        stripped = line.strip()
        if stripped.startswith("//"):
            continue
        if '"""' in line:
            if line.count('"""') == 1:
                in_multi = not in_multi
            continue
        if in_multi or SKIP_LINE.search(line) or SEAM_CALL.search(line) or "fallback:" in line:
            continue
        for lit in LITERAL.finditer(code_part(line)):
            body = lit.group(1)
            if ID_ONLY.match(body) or not reads_as_text(body):
                continue
            found["literal"].append(f"{rel}:{number}: {body[:70]}")
    return found


def scan() -> dict[str, dict[str, list[str]]]:
    ids = catalog()
    out: dict[str, dict[str, list[str]]] = {}
    for path in sorted(SOURCES.rglob("*.swift")):
        if SKIP_PATH.search(str(path)):
            continue
        found = check_file(path, ids)
        if any(found.values()):
            out[str(path.relative_to(SOURCES))] = found
    return out


def counts(scanned: dict[str, dict[str, list[str]]]) -> dict[str, int]:
    return {f"{kind} {file}": len(items) for file, found in scanned.items() for kind, items in found.items() if items}


def main(argv: list[str]) -> int:
    if "--selftest" in argv:
        return selftest()
    scanned = scan()
    now = counts(scanned)
    allow: dict[str, int] = json.loads(ALLOW.read_text()) if ALLOW.exists() else {}
    if "--update" in argv:
        new = {k: min(v, allow.get(k, v)) for k, v in now.items()} if allow else now
        ALLOW.write_text("{\n" + ",\n".join(f"  {json.dumps(k)}: {v}" for k, v in sorted(new.items())) + "\n}\n")
        print(f"words: allowlist {len(new)} entries, {sum(new.values())} grandfathered")
        return 0
    problems = []
    for key, count in sorted(now.items()):
        if count > allow.get(key, 0):
            kind, file = key.split(" ", 1)
            problems += [f"{kind}: {item}" for item in scanned[file][kind]]
    totals = {k: sum(len(f[k]) for f in scanned.values()) for k in ("literal", "id", "vars")}
    summary = f"words: {totals['literal']} literals, {totals['id']} unknown ids, {totals['vars']} variable mismatches ({sum(allow.values())} grandfathered)"
    if problems:
        print("\n".join(problems[:60]))
        print(f"{summary}; {len(problems)} new: put each word in a catalog message and ask for it through the Words seam, "
              "or say why it is not text with `// text: <why>`")
        return 1
    print(summary)
    return 0


def selftest() -> int:
    ids = {"a-b": set(), "c-d": {"name"}}
    src = SOURCES / "_selftest.swift"
    cases = [
        ('let x = Words.text("a-b", fallback: "Hello there")', 0),
        ('let x = "Hello there"', 1),
        ('let x = "Hello there" // text: sample', 0),
        ('let x = text("zz-yy", "Hi there")', 1),
        ('let x = text("c-d", "Hi \\(n)")', 1),
        ('let x = text("c-d", ["name": .text(n)], "Hi \\(n)")', 0),
        ('let id = "some-id"', 0),
    ]
    bad = 0
    for source, expect in cases:
        src.write_text(source + "\n")
        try:
            found = check_file(src, ids)
        finally:
            src.unlink()
        got = sum(len(v) for v in found.values())
        if got != expect:
            print(f"selftest: {source!r}: expected {expect}, got {got}: {found}")
            bad += 1
    print("selftest ok" if not bad else f"selftest: {bad} failed")
    return 1 if bad else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
