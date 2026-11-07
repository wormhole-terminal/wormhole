#!/usr/bin/env python3
"""Size and hygiene budgets for the Swift sources (swift/STANDARDS.md), reported as warnings.

    python3 -I apple/scripts/standards.py             # warnings; exit 0
    python3 -I apple/scripts/standards.py --strict    # exit 1 on anything above the allowlist
    python3 -I apple/scripts/standards.py --update    # the allowlist only ever shrinks (first run writes it)
    python3 -I apple/scripts/standards.py --selftest

Checks, per file under `swift/Sources` (generated `WormholeBindings` skipped):
  file-soft     over 400 lines;                       file-hard   over 600 lines;
  func-size     a function over 60 lines (counted per function);
  unchecked     `@unchecked Sendable` or `nonisolated(unsafe)` with no `why:` comment on the line or the two above;
  undocumented  `public` declarations (not `init`) with no `///` above; counted, never a warning;
  any-view      `AnyView` (it defeats SwiftUI's diffing).

`standards-allow.json` holds the per-file counts that exist today ("rule file": count), like scripts/structure.py's
allowlist: a count at or under its entry is grandfathered, a new file or a higher count is a new problem.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

APPLE = Path(__file__).resolve().parent.parent
SOURCES = APPLE / "swift" / "Sources"
ALLOW = Path(__file__).with_name("standards-allow.json")
SOFT, HARD, FUNC = 400, 600, 60

FUNC_DECL = re.compile(r"^\s*(?:@\w+(?:\([^)]*\))?\s+)*(?:(?:public|internal|private|fileprivate|package|static|"
                       r"final|override|nonisolated|mutating|class)\s+)*(?:func|init)\b")
UNCHECKED = re.compile(r"@unchecked\s+Sendable|nonisolated\(unsafe\)")
PUBLIC_DECL = re.compile(r"^\s*(?:@\w+(?:\([^)]*\))?\s+)*public\s+(?:final\s+|static\s+|class\s+|indirect\s+)*"
                         r"(?:func|var|let|struct|class|enum|protocol|actor|typealias|subscript)\b")


def strip(line: str) -> str:
    """The line without string literals and `//` comments, so braces in them are not counted."""
    return re.sub(r'"(?:\\.|[^"\\])*"', '""', line).split("//", 1)[0]


def functions(lines: list[str]) -> list[tuple[int, int]]:
    """(first line, length) of each function whose braces can be followed; protocol requirements have none."""
    found, i = [], 0
    while i < len(lines):
        if FUNC_DECL.match(lines[i]):
            depth, opened, j = 0, False, i
            while j < len(lines):
                code = strip(lines[j])
                depth += code.count("{") - code.count("}")
                opened = opened or "{" in code
                if opened and depth <= 0:
                    break
                if not opened and j > i and not code.rstrip().endswith((",", "(", "->", "&&", "||")) \
                        and "{" not in code and not code.lstrip().startswith((")", "->", "where", "async", "throws")):
                    j = i  # a declaration with no body (protocol requirement)
                    break
                j += 1
            if opened and j > i:
                found.append((i + 1, j - i + 1))
        i += 1
    return found


def check_text(text: str) -> dict[str, list[str]]:
    """rule -> one message per finding, for one file's text."""
    lines, out = text.splitlines(), {}
    def add(rule: str, msg: str): out.setdefault(rule, []).append(msg)
    if len(lines) > HARD:
        add("file-hard", f"{len(lines)} lines (hard limit {HARD})")
    elif len(lines) > SOFT:
        add("file-soft", f"{len(lines)} lines (soft limit {SOFT})")
    for start, length in functions(lines):
        if length > FUNC:
            add("func-size", f"line {start}: function of {length} lines (limit {FUNC})")
    for n, line in enumerate(lines):
        code = strip(line)
        if UNCHECKED.search(code) and not any("why:" in l for l in lines[max(0, n - 2) : n + 1]):
            add("unchecked", f"line {n + 1}: {line.strip()} needs a `// why:` comment")
        if re.search(r"\bAnyView\b", code):
            add("any-view", f"line {n + 1}: {line.strip()}")
        if PUBLIC_DECL.match(code):
            above = next((lines[k].strip() for k in range(n - 1, -1, -1) if not lines[k].strip().startswith("@")), "")
            if not (above.startswith("///") or above.endswith("*/")):
                add("undocumented", f"line {n + 1}: {line.strip()}")
    return out


def scan() -> dict[str, dict[str, list[str]]]:
    result = {}
    for path in sorted(SOURCES.rglob("*.swift")):
        rel = path.relative_to(APPLE / "swift").as_posix()
        if "/WormholeBindings/" in rel:
            continue
        if findings := check_text(path.read_text(encoding="utf-8")):
            result[rel] = findings
    return result


def counts(result) -> dict[str, int]:
    """Per rule and file, what the allowlist holds; `undocumented` is only counted, in the total."""
    return {f"{rule} {rel}": len(msgs) for rel, rules in result.items() for rule, msgs in rules.items()
            if rule != "undocumented"}


def totals(result) -> dict[str, int]:
    t: dict[str, int] = {}
    for rules in result.values():
        for rule, msgs in rules.items():
            t[rule] = t.get(rule, 0) + len(msgs)
    return t


def selftest() -> int:
    ok = check_text("public struct A {}\n/// doc\npublic func f() {}\n") == {"undocumented": ["line 1: public struct A {}"]}
    body = "func big() {\n" + "    let x = 1\n" * 61 + "}\nfunc small() { }\nprotocol P {\n    func req(a: Int)\n}\n"
    ok &= [m.split(":")[0] for m in check_text(body).get("func-size", [])] == ["line 1"]
    ok &= "unchecked" in check_text("final class A: @unchecked Sendable {}\n")
    ok &= "unchecked" not in check_text("// why: guarded by a lock\nfinal class A: @unchecked Sendable {}\n")
    ok &= "any-view" in check_text("let v: AnyView\n") and "any-view" not in check_text('let s = "AnyView"\n')
    ok &= "file-soft" in check_text("//\n" * 401) and "file-hard" in check_text("//\n" * 601)
    print("selftest: ok" if ok else "selftest: FAILED")
    return 0 if ok else 1


def main() -> int:
    if "--selftest" in sys.argv:
        return selftest()
    result = scan()
    now = counts(result)
    allow = json.loads(ALLOW.read_text()) if ALLOW.exists() else {}
    if "--update" in sys.argv:
        new = {k: min(v, allow.get(k, v)) for k, v in now.items()}  # only shrinks; a new entry is not added unless first run
        if allow:
            new = {k: v for k, v in new.items() if k in allow}
        ALLOW.write_text(json.dumps(dict(sorted(new.items())), indent=1) + "\n")
        print(f"standards: allowlist written ({len(new)} entries)")
        return 0
    over = {k: v for k, v in now.items() if v > allow.get(k, 0)}
    for key, count in sorted(over.items()):
        rule, rel = key.split(" ", 1)
        print(f"warn {rule}: {rel} ({count}, allowed {allow.get(key, 0)})")
    t = totals(result)
    print("standards: " + ", ".join(f"{r} {t.get(r, 0)}" for r in
          ("file-soft", "file-hard", "func-size", "unchecked", "undocumented", "any-view"))
          + f"; above the allowlist: {len(over)}")
    return 1 if over and "--strict" in sys.argv else 0


if __name__ == "__main__":
    sys.exit(main())
