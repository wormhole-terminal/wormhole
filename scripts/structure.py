#!/usr/bin/env python3
"""The structure checker: counts what the app-structure plan brings to zero and keeps it from growing back.

  scripts/structure.py check [--quiet]       fail on a violation the allowlist doesn't cover, and on an
                                             allowlist entry that is stale (the violation is gone or smaller)
  scripts/structure.py counts [--json]       the baseline table: per rule, per crate, totals, the files over
                                             600 and 1,000 lines, the cycles
  scripts/structure.py update [--allow-growth]
                                             rewrite the allowlist from the tree; without --allow-growth it
                                             refuses to add an entry or raise a count (the allowlist only shrinks)
  scripts/structure.py selftest              run the built-in tests (fixture trees in a temporary directory)

--root picks another repository (default: this one), --config another config file (default:
scripts/structure.json, which holds the settings and the allowlist). Rules, each per crate under
crates/<crate>/src, tests included:

  file-size         files over 1,000 lines fail; files over 600 are only counted
  os-edge           OS conditions and OS crates (cfg(target_os, objc2, libc::, allow(unsafe_code), ...) outside
                    the crate's OS-edge paths
  bucket-names      modules named text, utils, util, misc, helpers, helper, common
  orphans           .rs files no `mod` declaration reaches from lib.rs / main.rs / src/bin
  glob-imports      `use super::*;` outside test code
  cycles            strongly connected groups of top-level modules (an edge: a file of A names `crate::B`)
  cross-reexports   `pub use crate::B::...` in a file of another top-level module A
  impl-outside      `impl Workspace` / `impl Session` blocks outside their home folder (app crate)

Why a script that reads text: it runs in a second without building anything (tests only run on CI
here), so check.sh can call it first and every phase of the restructuring has a number that goes to
zero. What a text reader cannot see, said plainly:
  - macros: a `mod` or `crate::x` path produced by macro_rules is invisible (none today);
  - `use super::*` hides the edges behind it from the cycle rule, which is why that rule exists;
  - cfg: every `#[cfg(...)] mod x;` counts as declared, whatever the target; a cfg condition is read
    as words, not evaluated;
  - paths: only `crate::B` (also in `crate::{B::x, C}` groups) is an edge, not `super::super::B`, not a
    name re-exported at the crate root, not `use` of a module through an alias.

The allowlist keys are stable text: `path`, `path :: token`, `path :: crate::B`, `crate: a, b, c` (a
cycle), each with a count. Python 3.9, standard library only.
"""
from __future__ import annotations

import argparse
import bisect
import contextlib
import io
import json
import os
import re
import sys
import tempfile
import time
from typing import Dict, Iterable, List, Optional, Set, Tuple

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CONFIG_NAME = os.path.join("scripts", "structure.json")

RULES = [
    "file-size", "os-edge", "bucket-names", "orphans", "glob-imports", "cycles", "cross-reexports", "impl-outside",
]
RULE_HELP = {
    "file-size": "files over {fail} lines",
    "os-edge": "OS code outside the OS edge",
    "bucket-names": "bucket module names",
    "orphans": "files no mod declaration reaches",
    "glob-imports": "`use super::*` outside tests",
    "cycles": "cycles between top-level modules",
    "cross-reexports": "re-exports across top-level modules",
    "impl-outside": "impl Workspace/Session outside the home",
}

DEFAULT_CONFIG = {
    # Paths are relative to a crate's folder; a folder ends with a slash, anything else is one file.
    "os_edge": {
        "wormhole": ["src/native/"],
        "wormhole-core": [],
        "wormhole-host": ["src/sys.rs", "src/peer.rs"],
        "wormhole-term": ["src/pty/"],
        "wormhole-wire": [],
    },
    # The inherent impls of these types belong in the home (a folder or file, relative to the crate).
    "impl_outside": {"crate": "wormhole", "types": ["Workspace", "Session"], "homes": ["src/workspace/"]},
    "bucket_names": ["text", "utils", "util", "misc", "helpers", "helper", "common"],
    "size": {"warn": 600, "fail": 1000},
    # Folders whose size the plan tracks (files and lines, listed by `counts`).
    "track_dirs": ["crates/wormhole/src/workspace"],
}

# ---------------------------------------------------------------------------------------------------
# Reading Rust as text: comments and the inside of strings and chars become spaces (same length, same
# line breaks), so offsets and line numbers of the clean text are the file's.

_LEX = re.compile(
    r"""//[^\n]*                                   # line comment (also /// and //!)
      | /\*                                        # block comment, nested: scanned by hand
      | \b(?:br|cr|r)(\#*)"                        # raw string r#"..."#
      | "                                          # string
      | '(?:\\(?:x[0-9A-Fa-f]{2}|u\{[0-9A-Fa-f_]+\}|[^\n])|[^\\'\n])'   # char literal; a lifetime has no closing quote
    """,
    re.X,
)
_STR_END = re.compile(r'[^"\\]*(?:\\.[^"\\]*)*"', re.S)
_NOT_NL = re.compile(r"[^\n]")


def _blank(s: str) -> str:
    return " " * len(s) if "\n" not in s else _NOT_NL.sub(" ", s)


def strip_source(text: str) -> Tuple[str, Dict[int, str]]:
    """The text without comments and string contents, and the strings by the offset of their opening quote."""
    out: List[str] = []
    strings: Dict[int, str] = {}
    pos, n = 0, len(text)
    search = _LEX.search
    while True:
        m = search(text, pos)
        if m is None:
            out.append(text[pos:])
            break
        out.append(text[pos:m.start()])
        tok = m.group(0)
        end = m.end()
        if tok.startswith("//"):
            out.append(_blank(tok))
        elif tok == "/*":
            depth, i = 1, end
            while depth and i < n:
                a, b = text.find("/*", i), text.find("*/", i)
                if b < 0:
                    i = n
                    break
                if 0 <= a < b:
                    depth, i = depth + 1, a + 2
                else:
                    depth, i = depth - 1, b + 2
            out.append(_blank(text[m.start():i]))
            end = i
        elif tok[0] == "'":
            out.append("'" + " " * (len(tok) - 2) + "'")
        elif tok == '"' or m.group(1) is not None:
            if m.group(1) is None:
                e = _STR_END.match(text, end)
                close_at = e.end() if e else n  # just past the closing quote
                body_end = close_at - 1 if e else n
                closer = '"' if e else ""
            else:
                closer = '"' + m.group(1)
                f = text.find(closer, end)
                body_end = n if f < 0 else f
                close_at = n if f < 0 else f + len(closer)
                closer = closer if f >= 0 else ""
            body = text[end:body_end]
            strings[end - 1] = body
            out.append(text[m.start():end] + _blank(body) + closer)
            end = close_at
        pos = end
    return "".join(out), strings


_BRACE = re.compile(r"[{}]")
_PAREN = re.compile(r"[()]")


def match_close(clean: str, open_pos: int, rx: "re.Pattern[str]") -> int:
    """The offset of the bracket closing the one at open_pos (the end of the text when it never closes)."""
    depth = 0
    for b in rx.finditer(clean, open_pos):
        depth += 1 if b.group() in "{(" else -1
        if depth == 0:
            return b.start()
    return len(clean)


def scan(rx: "re.Pattern[str]", text: str) -> Iterable["re.Match[str]"]:
    """rx.finditer, keeping the matches that start a word. The patterns begin with a literal and carry no
    `\\b` or lookbehind: that lets the regex engine search for the literal, which is several times faster."""
    for m in rx.finditer(text):
        i = m.start()
        if i and (text[i - 1].isalnum() or text[i - 1] in "_$"):
            continue
        yield m


class Src:
    """One source file: its text, its clean text and line lookups."""

    def __init__(self, rel: str, path: str, raw: str) -> None:
        self.rel, self.path, self.raw = rel, path, raw
        self.lines = raw.count("\n") + (1 if raw and not raw.endswith("\n") else 0)
        self.clean, self.strings = strip_source(raw)
        self._starts: Optional[List[int]] = None

    def line(self, offset: int) -> int:
        if self._starts is None:
            self._starts = [0] + [m.end() for m in re.finditer(r"\n", self.clean)]
        return bisect.bisect_right(self._starts, offset)

    def at(self, offset: int) -> str:
        return "%s:%d" % (self.rel, self.line(offset))


# ---------------------------------------------------------------------------------------------------
# Module items: `mod x;` and `mod x { ... }` with their attributes.

_MOD = re.compile(r"mod\s+(\w+)\s*([;{])")
# What precedes a `mod`: attributes and the visibility, matched backwards from the keyword.
_MOD_HEAD = re.compile(r"((?:#\s*!?\s*\[[^\]]*\]\s*)*)(?:pub\s*(?:\([^)]*\)\s*)?)?$")
_INNER_CFG_TEST = re.compile(r"#\s*!\s*\[\s*cfg\s*\(\s*test\s*\)\s*\]")
_CFG_ATTR = re.compile(r"cfg\s*\(")
_PATH_ATTR = re.compile(r'#\s*\[\s*path\s*=\s*"')
_INCLUDE = re.compile(r'include\s*!\s*\(\s*"')


def cfg_test(attrs: str) -> bool:
    """Does an attribute list say `cfg(test)` (also in all(...)), and not `not(test)`?"""
    for m in scan(_CFG_ATTR, attrs):
        close = match_close(attrs, m.end() - 1, _PAREN)
        pred = attrs[m.end():close]
        if re.search(r"\btest\b", pred) and not re.search(r"\bnot\s*\(\s*test\b", pred):
            return True
    return False


class ModItem:
    def __init__(self, name: str, inline: bool, start: int, end: int, attrs: str, path: Optional[str]) -> None:
        self.name, self.inline, self.start, self.end = name, inline, start, end
        self.path = path
        self.test = cfg_test(attrs)
        self.parents: List["ModItem"] = []

    @property
    def in_test(self) -> bool:
        return self.test or any(p.test for p in self.parents)


def parse_mods(src: Src) -> List[ModItem]:
    items: List[ModItem] = []
    clean = src.clean
    for m in scan(_MOD, clean):
        lo = max(0, m.start() - 800)
        head = _MOD_HEAD.search(clean, lo, m.start())
        attrs = head.group(1) if head else ""
        path = None
        pm = _PATH_ATTR.search(attrs) if attrs else None
        if pm:
            path = src.strings.get(head.start(1) + pm.end() - 1)
        inline = m.group(2) == "{"
        end = match_close(clean, m.end() - 1, _BRACE) if inline else m.end()
        items.append(ModItem(m.group(1), inline, head.start() if head else m.start(), end, attrs, path))
    for it in items:
        it.parents = [p for p in items if p.inline and p.start < it.start < p.end]
    return items


# ---------------------------------------------------------------------------------------------------
# A crate: its files, and which of them the module tree reaches.

def is_test_name(rel_in_src: str) -> bool:
    parts = rel_in_src.split("/")
    base = parts[-1]
    return "tests" in parts[:-1] or base == "tests.rs" or base.endswith("_tests.rs")


class Crate:
    def __init__(self, name: str, root: str) -> None:
        self.name = name
        self.dir = os.path.join(root, "crates", name)
        self.src = os.path.join(self.dir, "src")
        self.prefix = "crates/%s/" % name
        self.files: Dict[str, Src] = {}  # by path relative to the crate folder: src/a/b.rs
        self.mods: Dict[str, List[ModItem]] = {}
        self.reached: Dict[str, Tuple[Optional[str], bool]] = {}  # file -> (top-level module, test code)
        self.tops: List[str] = []
        self.unresolved: List[str] = []
        self.cycles: List[List[str]] = []  # filled by the cycles rule
        self._load()
        self._walk()

    def _load(self) -> None:
        for dirpath, dirnames, filenames in os.walk(self.src):
            dirnames.sort()
            for fn in sorted(filenames):
                if not fn.endswith(".rs"):
                    continue
                path = os.path.join(dirpath, fn)
                rel = os.path.relpath(path, self.dir).replace(os.sep, "/")
                with open(path, encoding="utf-8", errors="replace") as f:
                    self.files[rel] = Src(self.prefix + rel, path, f.read())

    def _roots(self) -> List[str]:
        roots = [r for r in ("src/lib.rs", "src/main.rs") if r in self.files]
        for rel in self.files:
            parts = rel.split("/")
            if parts[:2] == ["src", "bin"] and (len(parts) == 3 or (len(parts) == 4 and parts[3] == "main.rs")):
                roots.append(rel)
        return roots

    def _walk(self) -> None:
        queue: List[Tuple[str, Optional[str], bool, str]] = []
        roots = self._roots()
        for r in roots:
            queue.append((r, None, bool(_INNER_CFG_TEST.search(self.files[r].clean)), os.path.dirname(r)))
            self.reached[r] = (None, queue[-1][2])
        while queue:
            rel, top, test, base = queue.pop()
            src = self.files[rel]
            mods = self.mods.setdefault(rel, parse_mods(src))
            is_root = top is None and rel in roots
            for it in mods:
                if it.inline:
                    if is_root and not it.parents and it.name not in self.tops:
                        self.tops.append(it.name)
                    continue
                inline_base = os.path.join(base, *[p.name for p in it.parents]) if it.parents else base
                child_top = top
                if is_root:
                    child_top = it.parents[0].name if it.parents else it.name
                    if child_top not in self.tops:
                        self.tops.append(child_top)
                child_test = test or it.in_test
                if it.path is not None:
                    start = os.path.dirname(rel) if not it.parents else inline_base
                    cands = [(os.path.normpath(os.path.join(start, it.path)).replace(os.sep, "/"), None)]
                else:
                    cands = [(os.path.join(inline_base, it.name + ".rs").replace(os.sep, "/"),
                              os.path.join(inline_base, it.name)),
                             (os.path.join(inline_base, it.name, "mod.rs").replace(os.sep, "/"),
                              os.path.join(inline_base, it.name))]
                for target, child_base in cands:
                    if target not in self.files:
                        continue
                    if target in self.reached:
                        old_top, old_test = self.reached[target]
                        self.reached[target] = (old_top, old_test and child_test)
                        break
                    if child_base is None:
                        child_base = os.path.dirname(target)
                    if target.endswith("/mod.rs"):
                        child_base = os.path.dirname(target)
                    self.reached[target] = (child_top, child_test)
                    queue.append((target, child_top, child_test, child_base))
                    break
                else:
                    self.unresolved.append("%s: mod %s" % (src.at(it.start), it.name))
            # include!("x.rs") pulls a file in without a module: it is not an orphan.
            for m in scan(_INCLUDE, src.clean):
                name = src.strings.get(m.end() - 1)
                target = os.path.normpath(os.path.join(os.path.dirname(rel), name or "")).replace(os.sep, "/")
                if name and target in self.files and target not in self.reached:
                    self.reached[target] = (top, test)
                    queue.append((target, top, test, os.path.dirname(target)))

    def top_of(self, rel: str) -> Optional[str]:
        return self.reached.get(rel, (None, False))[0]

    def is_test_file(self, rel: str) -> bool:
        return is_test_name(rel[len("src/"):]) or self.reached.get(rel, (None, False))[1]

    def test_regions(self, rel: str) -> List[Tuple[int, int]]:
        return [(it.start, it.end) for it in self.mods.get(rel, []) if it.inline and it.test]


# ---------------------------------------------------------------------------------------------------
# The rules. Each yields findings: key -> Item (count, where). Keys are what the allowlist stores.

class Item:
    __slots__ = ("crate", "count", "locs")

    def __init__(self, crate: str) -> None:
        self.crate, self.count, self.locs = crate, 0, []

    def add(self, loc: str, n: int = 1) -> None:
        self.count += n
        self.locs.append(loc)


Findings = Dict[str, Item]


def _add(found: Findings, key: str, crate: str, loc: str, n: int = 1) -> None:
    item = found.get(key)
    if item is None:
        item = found[key] = Item(crate)
    item.add(loc, n)


def under(rel_in_crate: str, paths: Iterable[str]) -> bool:
    return any(rel_in_crate == p or (p.endswith("/") and rel_in_crate.startswith(p)) for p in paths)


# One small regex per token: each starts with a literal, which the regex engine searches for quickly (a
# single alternation over all of them was five times slower on the whole workspace).
_PLAIN = [
    ("objc2", re.compile(r"objc2")),
    ("block2", re.compile(r"block2\b")),
    ("raw_window_handle", re.compile(r"raw_window_handle")),
    ("libc::", re.compile(r"libc\s*::")),
    ("std::os::unix", re.compile(r"std\s*::\s*os\s*::\s*unix\b")),
    ("std::os::fd", re.compile(r"std\s*::\s*os\s*::\s*fd\b")),
    ("allow(unsafe_code)", re.compile(r"allow\s*\(\s*unsafe_code\b")),
    ("target_vendor", re.compile(r"target_vendor\b")),
    ("target_family", re.compile(r"target_family\b")),
]
_CFG_CALL = re.compile(r"cfg(_attr)?\s*(!)?\s*\(")
_OS_WORD = re.compile(r"\b(target_os|unix|windows)\b")
_NEG = re.compile(r"\bnot\s*\(\s*$")


def os_edge_hits(src: Src) -> Dict[str, List[int]]:
    """OS tokens in a file: key -> offsets. A cfg predicate counts each key once, so
    `cfg(any(target_os = "a", target_os = "b"))` is one `cfg(target_os` site."""
    clean = src.clean
    hits: Dict[str, List[int]] = {}
    for key, rx in _PLAIN:
        found = [m.start() for m in scan(rx, clean)]
        if found:
            hits[key] = found
    for m in scan(_CFG_CALL, clean):
        name, bang = ("cfg_attr" if m.group(1) else "cfg"), m.group(2)
        close = match_close(clean, m.end() - 1, _PAREN)
        pred = clean[m.end():close]
        if name == "cfg_attr":
            depth = 0
            for i, ch in enumerate(pred):
                depth += (ch in "([{") - (ch in ")]}")
                if ch == "," and depth == 0:
                    pred = pred[:i]
                    break
            if _OS_WORD.search(pred):
                hits.setdefault("cfg_attr(", []).append(m.start())
            continue
        prefix = "cfg!(" if bang else "cfg("
        seen: Set[str] = set()
        for w in _OS_WORD.finditer(pred):
            word = w.group(1)
            if word == "target_os":
                key = prefix + "target_os"
            else:
                key = prefix + ("not(" if _NEG.search(pred[:w.start()]) else "") + word
            if key not in seen:
                seen.add(key)
                hits.setdefault(key, []).append(m.start())
    return hits


def rule_file_size(crate: Crate, cfg: dict, found: Findings) -> None:
    for rel, src in crate.files.items():
        if src.lines > cfg["size"]["fail"]:
            _add(found, src.rel, crate.name, "%s:1" % src.rel, src.lines)


def rule_os_edge(crate: Crate, cfg: dict, found: Findings) -> None:
    edge = cfg["os_edge"].get(crate.name, [])
    for rel, src in crate.files.items():
        if under(rel, edge):
            continue
        for key, offsets in os_edge_hits(src).items():
            for off in offsets:
                _add(found, "%s :: %s" % (src.rel, key), crate.name, src.at(off))


def rule_bucket_names(crate: Crate, cfg: dict, found: Findings) -> None:
    buckets = set(cfg["bucket_names"])
    for rel, src in crate.files.items():
        parts = rel.split("/")
        stem = parts[-1][:-3]
        name = parts[-2] if stem == "mod" and len(parts) > 2 else stem
        if name in buckets and rel not in ("src/lib.rs", "src/main.rs"):
            _add(found, src.rel, crate.name, "%s:1" % src.rel)
        for it in crate.mods.get(rel, []):
            if it.inline and it.name in buckets:
                _add(found, "%s :: mod %s" % (src.rel, it.name), crate.name, src.at(it.start))


def rule_orphans(crate: Crate, cfg: dict, found: Findings) -> None:
    for rel, src in crate.files.items():
        if rel not in crate.reached:
            _add(found, src.rel, crate.name, "%s:1" % src.rel)


_GLOB = re.compile(r"use\s+(?:super\s*::\s*)+\*\s*;")


def rule_glob_imports(crate: Crate, cfg: dict, found: Findings) -> None:
    for rel, src in crate.files.items():
        if crate.is_test_file(rel):
            continue
        regions = crate.test_regions(rel)
        for m in scan(_GLOB, src.clean):
            if any(a <= m.start() < b for a, b in regions):
                continue
            _add(found, src.rel, crate.name, src.at(m.start()))


_CRATE_REF = re.compile(r"crate\s*::\s*(\{|\w+)")


def crate_refs(clean: str, m: "re.Match[str]") -> List[str]:
    """The first segments after `crate::` (several for a `crate::{a::x, b}` group)."""
    if m.group(1) != "{":
        return [m.group(1)]
    close = match_close(clean, m.end() - 1, _BRACE)
    names, depth, start = [], 0, m.end()
    body = clean[m.end():close]
    pieces, cur = [], []
    for ch in body:
        if ch in "{(":
            depth += 1
        elif ch in "})":
            depth -= 1
        if ch == "," and depth == 0:
            pieces.append("".join(cur))
            cur = []
        else:
            cur.append(ch)
    pieces.append("".join(cur))
    for p in pieces:
        w = re.match(r"\s*(?:pub\s+)?(\w+)", p)
        if w:
            names.append(w.group(1))
    return names


def module_edges(crate: Crate, with_tests: bool = False) -> Dict[Tuple[str, str], str]:
    """(A, B) -> where a file of top-level module A first names crate::B. Test code is left out unless
    asked for: a use in a test can't make the build go in circles, and it moves with the test."""
    edges: Dict[Tuple[str, str], str] = {}
    tops = set(crate.tops)
    for rel, src in crate.files.items():
        a = crate.top_of(rel)
        if a is None or (not with_tests and crate.is_test_file(rel)):
            continue
        regions = [] if with_tests else crate.test_regions(rel)
        for m in scan(_CRATE_REF, src.clean):
            if regions and any(x <= m.start() < y for x, y in regions):
                continue
            for b in crate_refs(src.clean, m):
                if b in tops and b != a and (a, b) not in edges:
                    edges[(a, b)] = src.at(m.start())
    return edges


def sccs(nodes: List[str], edges: Iterable[Tuple[str, str]]) -> List[List[str]]:
    """Tarjan: the strongly connected components with more than one node, each sorted."""
    adj: Dict[str, List[str]] = {n: [] for n in nodes}
    for a, b in edges:
        adj.setdefault(a, []).append(b)
    index: Dict[str, int] = {}
    low: Dict[str, int] = {}
    stack: List[str] = []
    on: Set[str] = set()
    out: List[List[str]] = []
    counter = [0]
    sys.setrecursionlimit(max(1000, len(nodes) * 4))

    def visit(v: str) -> None:
        index[v] = low[v] = counter[0]
        counter[0] += 1
        stack.append(v)
        on.add(v)
        for w in sorted(adj.get(v, [])):
            if w not in index:
                visit(w)
                low[v] = min(low[v], low[w])
            elif w in on:
                low[v] = min(low[v], index[w])
        if low[v] == index[v]:
            comp = []
            while True:
                w = stack.pop()
                on.discard(w)
                comp.append(w)
                if w == v:
                    break
            if len(comp) > 1:
                out.append(sorted(comp))

    for n in sorted(adj):
        if n not in index:
            visit(n)
    return sorted(out)


def rule_cycles(crate: Crate, cfg: dict, found: Findings) -> None:
    """The violations are the edges inside a cycle (`crate: a -> b`), so a new dependency between modules
    that are already tangled is caught too; the groups themselves are kept for the report."""
    edges = module_edges(crate, cfg.get("cycles_with_tests", False))
    crate.cycles = sccs(crate.tops, edges)
    for comp in crate.cycles:
        for (a, b), where in sorted(edges.items()):
            if a in comp and b in comp:
                _add(found, "%s: %s -> %s" % (crate.name, a, b), crate.name, where)


_REEXPORT = re.compile(r"pub\s*(?:\([^)]*\)\s*)?use\s+crate\s*::\s*(\{|\w+)")


def rule_cross_reexports(crate: Crate, cfg: dict, found: Findings) -> None:
    tops = set(crate.tops)
    for rel, src in crate.files.items():
        a = crate.top_of(rel)
        if a is None:
            continue
        for m in scan(_REEXPORT, src.clean):
            # crate_refs wants a match whose group(1) is the first segment, or a group's brace.
            for b in crate_refs(src.clean, m):
                if b in tops and b != a:
                    _add(found, "%s :: crate::%s" % (src.rel, b), crate.name, src.at(m.start()))


def impl_regex(types: List[str]) -> "re.Pattern[str]":
    return re.compile(
        r"impl\s*(?:<(?:->|[^<>]|<[^<>]*>)*>)?\s*(?:\w+\s*::\s*)*(" + "|".join(map(re.escape, types)) + r")\b"
        r"\s*(?:<[^{};]*?>)?\s*(?:where\b[^{;]*)?\{")


def rule_impl_outside(crate: Crate, cfg: dict, found: Findings) -> None:
    conf = cfg["impl_outside"]
    if crate.name != conf["crate"]:
        return
    rx = impl_regex(conf["types"])
    for rel, src in crate.files.items():
        if under(rel, conf["homes"]):
            continue
        for m in scan(rx, src.clean):
            _add(found, "%s :: impl %s" % (src.rel, m.group(1)), crate.name, src.at(m.start()))


RULE_FUNCS = {
    "file-size": rule_file_size, "os-edge": rule_os_edge, "bucket-names": rule_bucket_names,
    "orphans": rule_orphans, "glob-imports": rule_glob_imports, "cycles": rule_cycles,
    "cross-reexports": rule_cross_reexports, "impl-outside": rule_impl_outside,
}


class Analysis:
    def __init__(self, root: str, cfg: dict) -> None:
        self.root, self.cfg = root, cfg
        names = sorted(d for d in os.listdir(os.path.join(root, "crates"))
                       if os.path.isdir(os.path.join(root, "crates", d, "src")))
        self.crates = [Crate(n, root) for n in names]
        self.found: Dict[str, Findings] = {r: {} for r in RULES}
        for crate in self.crates:
            for rule in RULES:
                RULE_FUNCS[rule](crate, cfg, self.found[rule])
        warn, fail = cfg["size"]["warn"], cfg["size"]["fail"]
        self.over: Dict[str, Tuple[int, int]] = {}
        self.big: List[Tuple[int, str]] = []
        for crate in self.crates:
            lines = [(s.lines, s.rel) for s in crate.files.values()]
            self.over[crate.name] = (sum(1 for n, _ in lines if n > warn), sum(1 for n, _ in lines if n > fail))
            self.big += [(n, r) for n, r in lines if n > warn]
        self.big.sort(key=lambda t: (-t[0], t[1]))
        self.tracked = []
        for d in cfg.get("track_dirs", []):
            files = [s for c in self.crates for rel, s in c.files.items() if s.rel.startswith(d.rstrip("/") + "/")]
            self.tracked.append((d, len(files), sum(s.lines for s in files)))

    def entries(self) -> Dict[str, Dict[str, int]]:
        """Findings as the allowlist stores them: rule -> key -> count."""
        return {r: {k: it.count for k, it in sorted(self.found[r].items())} for r in RULES}


# ---------------------------------------------------------------------------------------------------
# The allowlist: compare, check, update.

def load_config(path: str) -> Tuple[dict, Dict[str, Dict[str, int]]]:
    cfg = json.loads(json.dumps(DEFAULT_CONFIG))
    allow: Dict[str, Dict[str, int]] = {r: {} for r in RULES}
    if os.path.exists(path):
        with open(path, encoding="utf-8") as f:
            data = json.load(f)
        for k, v in data.get("config", {}).items():
            cfg[k] = v
        for r, entries in data.get("allowlist", {}).items():
            if r not in allow:
                raise SystemExit("%s: the allowlist names an unknown rule `%s`" % (path, r))
            allow[r] = dict(entries)
    return cfg, allow


def save_config(path: str, cfg: dict, allow: Dict[str, Dict[str, int]]) -> None:
    data = {"config": cfg, "allowlist": {r: dict(sorted(allow[r].items())) for r in RULES}}
    tmp = path + ".tmp"
    with open(tmp, "w", encoding="utf-8") as f:
        json.dump(data, f, indent=2, ensure_ascii=False)
        f.write("\n")
    os.replace(tmp, path)


class Verdict:
    def __init__(self) -> None:
        self.problems: List[str] = []  # lines for new or grown violations
        self.stale: List[Tuple[str, str, str]] = []  # (rule, key, what to do)

    @property
    def ok(self) -> bool:
        return not self.problems and not self.stale


def describe(an: Analysis, rule: str, key: str, item: Item, have: int) -> str:
    allowed = "not in the allowlist" if not have else "the allowlist allows %d" % have
    if rule == "file-size":
        return "%d lines, over %d (%s)" % (item.count, an.cfg["size"]["fail"], allowed)
    if rule == "cycles":
        return "%s depends on %s inside a cycle (%s)" % (tuple(key.split(": ", 1)[1].split(" -> ")) + (allowed,))
    if rule == "orphans":
        return "no `mod` declaration reaches this file"
    if rule == "bucket-names":
        return "bucket name (%s)" % allowed
    what = "`use super::*`" if rule == "glob-imports" else key.split(" :: ", 1)[1]
    lines = ", ".join(l.rsplit(":", 1)[1] for l in item.locs[:6]) + (", ..." if len(item.locs) > 6 else "")
    return "%s x%d (lines %s), %s" % (what, item.count, lines, allowed)


def compare(an: Analysis, allow: Dict[str, Dict[str, int]]) -> Verdict:
    v = Verdict()
    for rule in RULES:
        found, allowed = an.found[rule], allow.get(rule, {})
        for key in sorted(found):
            item = found[key]
            have = allowed.get(key, 0)
            if item.count > have:
                v.problems.append("%s: %s: %s" % (item.locs[0], rule, describe(an, rule, key, item, have)))
        for key in sorted(allowed):
            now = found[key].count if key in found else 0
            if now < allowed[key]:
                v.stale.append((rule, key, "gone" if now == 0 else "%d -> %d" % (allowed[key], now)))
    return v


def cmd_check(args: argparse.Namespace) -> int:
    started = time.time()
    cfg, allow = load_config(args.config)
    an = Analysis(args.root, cfg)
    v = compare(an, allow)
    for line in v.problems:
        print(line)
    if v.stale:
        print("stale allowlist entries in %s (the allowlist only shrinks; run `python3 scripts/structure.py update`):"
              % os.path.relpath(args.config, args.root))
        for rule, key, what in v.stale:
            print("  %s: \"%s\": %s" % (rule, key, what))
    if v.ok:
        if not args.quiet:
            print("structure: ok (%d allowlisted entries across %d rules, %.1f s)" % (
                sum(len(a) for a in allow.values()), len([r for r in RULES if allow[r]]), time.time() - started))
        return 0
    print("structure: FAILED, %d new or grown violation(s), %d stale allowlist entr%s" % (
        len(v.problems), len(v.stale), "y" if len(v.stale) == 1 else "ies"))
    return 1


# ---------------------------------------------------------------------------------------------------
# counts

def per_crate(an: Analysis, rule: str) -> Dict[str, Tuple[int, int]]:
    """crate -> (violations, files)."""
    out = {c.name: (0, 0) for c in an.crates}
    files: Dict[str, Set[str]] = {c.name: set() for c in an.crates}
    for key, item in an.found[rule].items():
        n, _ = out[item.crate]
        out[item.crate] = (n + (1 if rule in ("orphans", "bucket-names") else item.count), 0)
        files[item.crate].add(key.split(" :: ")[0] if rule != "cycles" else key)
    return {c: (out[c][0], len(files[c])) for c in out}


def mutual_pairs(cycle_items: Findings, crate: str) -> List[Tuple[str, str]]:
    edges = {tuple(k.split(": ", 1)[1].split(" -> ")) for k, it in cycle_items.items() if it.crate == crate}
    return sorted((a, b) for (a, b) in edges if a < b and (b, a) in edges)


def build_counts(an: Analysis) -> dict:
    crates = [c.name for c in an.crates]
    rows = []

    def row(label: str, values: Dict[str, int]) -> None:
        rows.append({"label": label, "crates": {c: values.get(c, 0) for c in crates},
                     "total": sum(values.get(c, 0) for c in crates)})

    warn, fail = an.cfg["size"]["warn"], an.cfg["size"]["fail"]
    row("files over %d lines" % warn, {c: an.over[c][0] for c in crates})
    row("files over %d lines" % fail, {c: an.over[c][1] for c in crates})
    for rule in RULES:
        pc = per_crate(an, rule)
        if rule == "file-size":
            continue
        if rule == "cycles":
            row("cycles: groups of modules", {c.name: len(c.cycles) for c in an.crates})
            row("cycles: modules in a group", {c.name: sum(len(g) for g in c.cycles) for c in an.crates})
            row("cycles: edges inside a group", {c: pc[c][0] for c in crates})
            row("cycles: back-and-forth pairs", {c.name: len(mutual_pairs(an.found["cycles"], c.name)) for c in an.crates})
            continue
        unit = {"os-edge": "sites", "glob-imports": "uses", "cross-reexports": "re-exports", "impl-outside": "impl blocks"}.get(rule)
        if unit:
            row("%s: %s" % (rule, unit), {c: pc[c][0] for c in crates})
            row("%s: files" % rule, {c: pc[c][1] for c in crates})
        else:
            row(rule, {c: pc[c][0] for c in crates})
    tokens: Dict[str, Dict[str, int]] = {}
    for key, item in an.found["os-edge"].items():
        tok = key.split(" :: ", 1)[1]
        tokens.setdefault(tok, {})
        tokens[tok][item.crate] = tokens[tok].get(item.crate, 0) + item.count
    cycles = []
    for c in an.crates:
        for comp in c.cycles:
            edges = sorted(k.split(": ", 1)[1] + "  " + it.locs[0] for k, it in an.found["cycles"].items()
                           if it.crate == c.name and all(x in comp for x in k.split(": ", 1)[1].split(" -> ")))
            cycles.append({"crate": c.name, "modules": comp, "edges": edges})
    return {
        "crates": crates,
        "rows": rows,
        "os_edge_tokens": {t: {c: tokens[t].get(c, 0) for c in crates} for t in sorted(tokens)},
        "cycles": cycles,
        "big_files": [{"file": f, "lines": n} for n, f in an.big],
        "tracked_dirs": [{"dir": d, "files": f, "lines": n} for d, f, n in an.tracked],
        "unresolved_mods": [u for c in an.crates for u in c.unresolved],
    }


def cmd_counts(args: argparse.Namespace) -> int:
    cfg, _ = load_config(args.config)
    an = Analysis(args.root, cfg)
    data = build_counts(an)
    if args.json:
        json.dump(data, sys.stdout, indent=2, sort_keys=True)
        print()
        return 0
    crates = data["crates"]
    short = [c.replace("wormhole-", "") if c != "wormhole" else "app" for c in crates]
    width = max(len(r["label"]) for r in data["rows"]) + 2
    print("Structure counts (crates/*/src, tests included)")
    print()
    print("%-*s" % (width, "") + "".join("%8s" % s for s in short) + "%8s" % "total")
    for r in data["rows"]:
        print("%-*s" % (width, r["label"]) + "".join("%8d" % r["crates"][c] for c in crates) + "%8d" % r["total"])
    print()
    if data["tracked_dirs"]:
        for t in data["tracked_dirs"]:
            print("%s: %d files, %d lines" % (t["dir"], t["files"], t["lines"]))
        print()
    print("os-edge sites by token")
    tw = max([len(t) for t in data["os_edge_tokens"]] + [5]) + 2
    print("%-*s" % (tw, "") + "".join("%8s" % s for s in short))
    for tok, by in data["os_edge_tokens"].items():
        print("%-*s" % (tw, tok) + "".join("%8d" % by[c] for c in crates))
    print()
    print("files over %d lines" % cfg["size"]["warn"] + " (* = over %d)" % cfg["size"]["fail"])
    for b in data["big_files"]:
        print("  %5d%s %s" % (b["lines"], "*" if b["lines"] > cfg["size"]["fail"] else " ", b["file"]))
    print()
    print("cycles between top-level modules")
    if not data["cycles"]:
        print("  none")
    for c in data["cycles"]:
        print("  %s: %s" % (c["crate"], ", ".join(c["modules"])))
        for e in c["edges"]:
            print("      %s" % e)
    if data["unresolved_mods"]:
        print()
        print("mod declarations with no file found (cfg'd out, or a missing file)")
        for u in data["unresolved_mods"]:
            print("  " + u)
    return 0


# ---------------------------------------------------------------------------------------------------
# update

def diff_entries(rule: str, old: Dict[str, int], new: Dict[str, int]) -> Tuple[List[str], List[str], List[str]]:
    """(added or raised, removed, lowered), as printable lines."""
    grew, gone, shrank = [], [], []
    for k in sorted(new):
        if k not in old:
            grew.append("%s: \"%s\": %d" % (rule, k, new[k]))
        elif new[k] > old[k]:
            grew.append("%s: \"%s\": %d -> %d" % (rule, k, old[k], new[k]))
        elif new[k] < old[k]:
            shrank.append("%s: \"%s\": %d -> %d" % (rule, k, old[k], new[k]))
    for k in sorted(old):
        if k not in new:
            gone.append("%s: \"%s\": %d" % (rule, k, old[k]))
    return grew, gone, shrank


def cmd_update(args: argparse.Namespace) -> int:
    cfg, allow = load_config(args.config)
    an = Analysis(args.root, cfg)
    new = an.entries()
    grew, gone, shrank = [], [], []
    for rule in RULES:
        g, r, s = diff_entries(rule, allow[rule], new[rule])
        grew += g
        gone += r
        shrank += s
    if grew and not args.allow_growth:
        print("refusing to update: the tree has violations the allowlist doesn't cover. Fix them, or pass "
              "--allow-growth to accept them (the first run, or a deliberate exception):", file=sys.stderr)
        for line in grew[:20]:
            print("  would add " + line, file=sys.stderr)
        if len(grew) > 20:
            print("  ... and %d more (%d in all)" % (len(grew) - 20, len(grew)), file=sys.stderr)
        return 1
    for line in grew:
        print("added   " + line)
    for line in shrank:
        print("lowered " + line)
    for line in gone:
        print("removed " + line)
    existing = os.path.exists(args.config)
    if not (grew or gone or shrank) and existing:
        print("allowlist already matches the tree")
        return 0
    save_config(args.config, cfg, new)
    print("wrote %s (%d entries)" % (os.path.relpath(args.config, args.root), sum(len(v) for v in new.values())))
    return 0


# ---------------------------------------------------------------------------------------------------
# selftest: small fixture trees, checked rule by rule, so the checker is tested without cargo.

def _write_tree(root: str, files: Dict[str, str]) -> None:
    for rel, text in files.items():
        path = os.path.join(root, rel)
        os.makedirs(os.path.dirname(path), exist_ok=True)
        with open(path, "w", encoding="utf-8") as f:
            f.write(text)


def _fixture_cfg() -> dict:
    cfg = json.loads(json.dumps(DEFAULT_CONFIG))
    cfg["os_edge"] = {"app": ["src/native/", "src/edge.rs"], "lib": []}
    cfg["impl_outside"] = {"crate": "app", "types": ["Workspace", "Session"], "homes": ["src/workspace/"]}
    cfg["size"] = {"warn": 5, "fail": 10}
    cfg["track_dirs"] = []
    return cfg


def selftest() -> int:
    failures: List[str] = []

    def expect(name: str, got, want) -> None:
        if got != want:
            failures.append("%s: got %r, wanted %r" % (name, got, want))

    # The reader: comments, strings, chars, raw strings and nested block comments never count.
    src = 'let a = "objc2 // not a comment"; // objc2\nlet b = r#"libc:: "x" "#; /* a /* nested */ objc2 */ let c = \'{\';\n' \
          "fn f<'a>(x: &'a str) -> &'a str { x } let d = '\\''; /// doc libc::\nlet e = b\"cfg(unix)\";"
    clean, strings = strip_source(src)
    expect("strip keeps length", len(clean), len(src))
    expect("strip keeps lines", clean.count("\n"), src.count("\n"))
    for word in ("objc2", "libc::", "cfg(unix)"):
        expect("strip removes %s" % word, word in clean, False)
    expect("strip keeps code and lifetimes", "fn f<'a>(x: &'a str) -> &'a str { x }" in clean, True)
    expect("strip keeps strings by position", strings.get(src.index('"')), "objc2 // not a comment")
    expect("strip char brace", "'{'" not in clean and "{" in clean, True)

    with tempfile.TemporaryDirectory() as tmp:
        def make(files: Dict[str, str], cfg: Optional[dict] = None) -> Analysis:
            root = tempfile.mkdtemp(dir=tmp)
            _write_tree(root, files)
            return Analysis(root, cfg or _fixture_cfg())

        def keys(an: Analysis, rule: str) -> List[str]:
            return sorted(an.found[rule])

        # orphans and the module tree: foo.rs + foo/, mod.rs, #[path], cfg'd and inline modules.
        an = make({
            "crates/app/src/main.rs": "mod a;\nmod b;\n#[cfg(target_os = \"macos\")]\nmod mac;\n#[path = \"other/odd.rs\"]\nmod odd;\n"
                                      "mod inl { mod deep; }\n#[cfg(test)]\nmod tests;\n// mod commented;\nconst S: &str = \"mod instring;\";\n",
            "crates/app/src/a.rs": "mod child;\n",
            "crates/app/src/a/child.rs": "mod grand;\n",
            "crates/app/src/a/child/grand.rs": "",
            "crates/app/src/b/mod.rs": "pub mod c;\n",
            "crates/app/src/b/c.rs": "",
            "crates/app/src/mac.rs": "",
            "crates/app/src/other/odd.rs": "mod sib;\n",
            "crates/app/src/other/sib.rs": "",
            "crates/app/src/inl/deep.rs": "",
            "crates/app/src/tests.rs": "",
            "crates/app/src/commented.rs": "",
            "crates/app/src/instring.rs": "",
            "crates/app/src/a/stray.rs": "",
            "crates/lib/src/lib.rs": "pub mod x;\n",
            "crates/lib/src/x.rs": "",
            "crates/lib/src/bin/tool.rs": "mod helper_for_tool;\n",
            "crates/lib/src/bin/helper_for_tool.rs": "",
            "crates/lib/src/y.rs": "",
        })
        expect("orphans", keys(an, "orphans"), [
            "crates/app/src/a/stray.rs", "crates/app/src/commented.rs", "crates/app/src/instring.rs",
            "crates/lib/src/y.rs"])

        # os-edge: tokens outside the edge, none inside it, none in comments or strings.
        os_file = ('#[cfg(target_os = "macos")]\nfn a() {}\n#[cfg(not(target_os = "macos"))]\nfn b() {}\n'
                   'fn c() { if cfg!(target_os = "linux") {} }\n#[cfg(unix)]\nfn d() {}\n#[cfg(not(windows))]\nfn e() {}\n'
                   '#[cfg_attr(target_os = "macos", expect(dead_code))]\nfn f() {}\n#[cfg(any(target_os = "a", target_os = "b"))]\n'
                   'fn g() {}\nuse objc2::rc::Retained;\nuse libc::c_int;\nuse std::os::unix::ffi::OsStrExt;\n'
                   '#[allow(unsafe_code)]\nfn h() {}\n// cfg(unix) objc2 libc::\nconst X: &str = "libc:: cfg(unix)";\n'
                   '#[cfg(all(unix, not(target_vendor = "apple")))]\nfn i() {}\n#[cfg(test)]\nmod t {}\n')
        an = make({"crates/app/src/main.rs": "mod os;\nmod native;\nmod edge;\n", "crates/app/src/os.rs": os_file,
                   "crates/app/src/native/mod.rs": os_file, "crates/app/src/edge.rs": os_file,
                   "crates/lib/src/lib.rs": "#[cfg(windows)]\nmod w;\n"})
        got = {k.split(" :: ")[1]: it.count for k, it in an.found["os-edge"].items() if k.startswith("crates/app/src/os.rs")}
        expect("os-edge tokens", got, {
            "cfg(target_os": 3, "cfg!(target_os": 1, "cfg(unix": 2, "cfg(not(windows": 1, "cfg_attr(": 1, "objc2": 1,
            "libc::": 1, "std::os::unix": 1, "allow(unsafe_code)": 1, "target_vendor": 1})
        expect("os-edge outside only", sorted({k.split(" :: ")[0] for k in an.found["os-edge"]}),
               ["crates/app/src/os.rs", "crates/lib/src/lib.rs"])
        expect("os-edge reports a line", an.found["os-edge"]["crates/app/src/os.rs :: objc2"].locs, ["crates/app/src/os.rs:14"])

        # bucket names: files, mod.rs folders, inline modules.
        an = make({"crates/app/src/main.rs": "mod text;\nmod utils;\nmod ok;\nmod inl { mod helpers {} }\n",
                   "crates/app/src/text.rs": "", "crates/app/src/utils/mod.rs": "", "crates/app/src/ok.rs": ""})
        expect("bucket-names", keys(an, "bucket-names"), [
            "crates/app/src/main.rs :: mod helpers", "crates/app/src/text.rs", "crates/app/src/utils/mod.rs"])

        # glob imports: not in tests.rs, *_tests.rs, tests/, cfg(test) modules (inline or file), nor comments.
        an = make({
            "crates/app/src/main.rs": "mod a;\nmod b;\nmod c;\nmod d;\nmod e;\n#[cfg(test)]\nmod only_tests;\n",
            "crates/app/src/a.rs": "use super::*;\nuse super::*;\n// use super::*;\n#[cfg(test)]\nmod tests {\n    use super::*;\n}\n",
            "crates/app/src/b.rs": "use super::*;\n#[cfg(test)]\nmod tests;\n",
            "crates/app/src/b/tests.rs": "use super::*;\n",
            "crates/app/src/c.rs": "use super::super::*;\n", "crates/app/src/d_tests.rs": "use super::*;\n",
            "crates/app/src/e/mod.rs": "mod tests;\npub use super::*;\n", "crates/app/src/e/tests.rs": "use super::*;\n",
            "crates/app/src/only_tests.rs": "use super::*;\n"})
        expect("glob-imports", {k: it.count for k, it in an.found["glob-imports"].items()},
               {"crates/app/src/a.rs": 2, "crates/app/src/b.rs": 1, "crates/app/src/c.rs": 1, "crates/app/src/e/mod.rs": 1})

        # cycles, with groups, comments and a glob hiding an edge; and re-exports across top-level modules.
        an = make({
            "crates/app/src/main.rs": "mod a;\nmod b;\nmod c;\nmod d;\nmod e;\nmod f;\n",
            "crates/app/src/f.rs": "pub(super) use crate::a::K;\n",
            "crates/app/src/a.rs": "use crate::b::X;\nuse crate::{c::Y, d};\n",
            "crates/app/src/b.rs": "fn f() { crate::a::g(); }\n",
            "crates/app/src/c.rs": "use crate::d::Z;\n// crate::a\n",
            "crates/app/src/d.rs": "use crate::{e::Q, c};\npub use crate::e::Q;\npub(crate) use crate::d::Self_;\n",
            "crates/app/src/e.rs": "use super::*;\n",
        })
        expect("cycles", keys(an, "cycles"), ["app: a -> b", "app: b -> a", "app: c -> d", "app: d -> c"])
        expect("cycle groups", an.crates[0].cycles if an.crates[0].name == "app" else None, [["a", "b"], ["c", "d"]])
        expect("cross-reexports", keys(an, "cross-reexports"), [
            "crates/app/src/d.rs :: crate::e", "crates/app/src/f.rs :: crate::a"])

        # impl-outside: inherent impls only, generics and paths included, the home exempt.
        an = make({
            "crates/app/src/main.rs": "mod workspace;\nmod other;\n",
            "crates/app/src/workspace/mod.rs": "impl Workspace {}\nimpl Session {}\n",
            "crates/app/src/other.rs": "impl Workspace {}\nimpl<'a, T: Fn(u8) -> Vec<u8>> Workspace<'a> where T: Copy {}\n"
                                       "impl Session {}\nimpl Render for Workspace {}\nimpl crate::workspace::Session {}\n"
                                       "// impl Workspace {}\n",
            "crates/lib/src/lib.rs": "impl Workspace {}\n"})
        expect("impl-outside", {k: it.count for k, it in an.found["impl-outside"].items()},
               {"crates/app/src/other.rs :: impl Workspace": 2, "crates/app/src/other.rs :: impl Session": 2})

        # file size: over 10 fails, over 5 is counted.
        an = make({"crates/app/src/main.rs": "mod a;\nmod b;\nmod c;\n", "crates/app/src/a.rs": "x\n" * 11,
                   "crates/app/src/b.rs": "x\n" * 10, "crates/app/src/c.rs": "x\n" * 6})
        expect("file-size", {k: it.count for k, it in an.found["file-size"].items()}, {"crates/app/src/a.rs": 11})
        expect("files over warn", an.over["app"], (3, 1))

        # the allowlist: pass, grow, stale, shrink, and update's refusal.
        root = tempfile.mkdtemp(dir=tmp)
        _write_tree(root, {"crates/app/src/main.rs": "mod a;\nmod b;\n", "crates/app/src/a.rs": "use crate::b::X;\nuse super::*;\n",
                           "crates/app/src/b.rs": "use crate::a::Y;\nuse super::*;\nuse super::*;\n"})
        conf = os.path.join(root, CONFIG_NAME)
        os.makedirs(os.path.dirname(conf))

        def run(*argv: str) -> Tuple[int, str]:
            out, err = io.StringIO(), io.StringIO()
            with contextlib.redirect_stdout(out), contextlib.redirect_stderr(err):
                code = main(["--root", root, "--config", conf] + list(argv))
            return code, out.getvalue() + err.getvalue()

        code, text = run("update")
        expect("update refuses growth", (code, os.path.exists(conf)), (1, False))
        code, text = run("update", "--allow-growth")
        expect("update first run", code, 0)
        expect("update says what it added", "added   glob-imports" in text and "added   cycles" in text, True)
        code, text = run("check")
        expect("check passes on its own allowlist", code, 0)
        _write_tree(root, {"crates/app/src/b.rs": "use crate::a::Y;\nuse super::*;\nuse super::*;\nuse super::*;\n"})
        code, text = run("check")
        expect("check fails on a grown count", code, 1)
        expect("check names file:line", "crates/app/src/b.rs:4: glob-imports:" in text or "b.rs:2: glob-imports" in text, True)
        _write_tree(root, {"crates/app/src/b.rs": "use crate::a::Y;\nuse super::*;\n"})
        code, text = run("check")
        expect("check fails on a stale entry", code, 1)
        expect("check names the stale entry", 'glob-imports: "crates/app/src/b.rs": 2 -> 1' in text, True)
        code, text = run("update")
        expect("update shrinks without --allow-growth", code, 0)
        code, text = run("check")
        expect("check passes after update", code, 0)
        _write_tree(root, {"crates/app/src/b.rs": "use super::*;\n"})  # the cycle is gone
        code, text = run("check")
        expect("a gone cycle is stale", (code, 'cycles: "app: a -> b": gone' in text), (1, True))
        run("update")
        _write_tree(root, {"crates/app/src/b.rs": "use crate::a::Y;\nuse super::*;\n"})
        code, text = run("check")
        expect("a cycle that returns fails", (code, "inside a cycle" in text), (1, True))
        code, text = run("counts")
        expect("counts runs", (code, "glob-imports: uses" in text), (0, True))
        code, text = run("counts", "--json")
        expect("counts --json", (code, json.loads(text)["crates"]), (0, ["app"]))

        # edges in test code never make a cycle, unless asked for.
        root2 = tempfile.mkdtemp(dir=tmp)
        _write_tree(root2, {"crates/app/src/main.rs": "mod a;\nmod b;\nmod c;\n",
                            "crates/app/src/a.rs": "use crate::b::X;\nuse crate::c::W;\n",
                            "crates/app/src/b.rs": "#[cfg(test)]\nmod tests {\n    use crate::a::Y;\n}\n",
                            "crates/app/src/c/mod.rs": "mod tests;\n", "crates/app/src/c/tests.rs": "use crate::a::Z;\n"})
        expect("test edges are left out", keys(Analysis(root2, _fixture_cfg()), "cycles"), [])
        with_tests = dict(_fixture_cfg(), cycles_with_tests=True)
        expect("test edges on request", keys(Analysis(root2, with_tests), "cycles"),
               ["app: a -> b", "app: a -> c", "app: b -> a", "app: c -> a"])

    if failures:
        for f in failures:
            print("FAILED: " + f, file=sys.stderr)
        return 1
    print("structure.py selftest: ok")
    return 0


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="Structure checker for the Rust workspace",
                                     formatter_class=argparse.RawDescriptionHelpFormatter, epilog=__doc__)
    parser.add_argument("--root", default=REPO, help="the repository (default: this one)")
    parser.add_argument("--config", default=None, help="config and allowlist file (default: scripts/structure.json)")
    sub = parser.add_subparsers(dest="command")
    p = sub.add_parser("check", help="fail on a violation the allowlist doesn't cover or a stale entry (the default)")
    p.add_argument("--quiet", action="store_true", help="print nothing when everything passes")
    p = sub.add_parser("counts", help="print the baseline table")
    p.add_argument("--json", action="store_true")
    p = sub.add_parser("update", help="rewrite the allowlist from the tree")
    p.add_argument("--allow-growth", action="store_true", help="accept entries that are new or larger")
    sub.add_parser("selftest", help="run the built-in tests")
    args = parser.parse_args(argv)
    args.root = os.path.abspath(args.root)
    args.config = args.config or os.path.join(args.root, CONFIG_NAME)
    command = args.command or "check"
    if command == "check" and not hasattr(args, "quiet"):
        args.quiet = False
    return {"check": cmd_check, "counts": cmd_counts, "update": cmd_update,
            "selftest": lambda a: selftest()}[command](args)


if __name__ == "__main__":
    sys.exit(main())
