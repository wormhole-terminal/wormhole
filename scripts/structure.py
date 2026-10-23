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
                    the crate's OS-edge paths, except what a file is sanctioned for (`os_edge_sanctioned`:
                    `allow(unsafe_code)` that isn't OS code, an exact count and a reason each; a higher
                    count fails, a lower one or a gone file is stale)
  os-edge-neutral   OS code in the neutral layer of an edge: the files of an `os_edge_neutral` folder that are not
                    under its `unix/`, `macos/` or `linux/` folders may select an OS (`cfg(`, `cfg_attr(`,
                    `cfg!(`) and nothing else: no `libc::`, `std::os::*`, `objc2`, `allow(unsafe_code)`
  bucket-names      modules named text, utils, util, misc, helpers, helper, common (also under `tests/`)
  orphans           .rs files no `mod` declaration reaches from lib.rs / main.rs / src/bin
  glob-imports      `use super::*;` outside test code
  cycles            strongly connected groups of modules (an edge: a file of A names `crate::B`, or B through
                    `super::`/`self::`); a module is
                    a top-level one, or for a crate in `module_depth` (`{"wormhole-core": 2}`) the module
                    that many levels down (`view::sidebar`), and a module's own parent or children don't
                    count as an edge (a domain and the modules in it are one tree)
  layers            per crate in `layers`, its top-level modules (whatever `module_depth` says) as tiers, lowest first (modules in one tier
                    may use each other): an edge from a lower tier up to a higher one fails (`crate: a -> b`),
                    and so does a top-level module the list leaves out (`crate: unplaced x`) or names but the
                    crate doesn't have (`crate: no module x`). Edges as the cycles rule reads them
  cross-reexports   `pub use crate::B::...` in a file of another module A (modules as the cycles rule has them)
  impl-outside      `impl Workspace` / `impl Session` blocks outside their home folder (app crate); the
                    `pending` folders inside a home are not exempt: their blocks are counted in the allowlist
  feature-api       a file of the features folder naming a module of the window other than the ones its
                    contract lists (`feature_rules.api`; `Workspace` only in the file names of
                    `feature_rules.workspace_files`): the features reach the window through the contract, the
                    facts and the repository read, not its insides. A bare `use crate::window;` and a
                    `super::super::...` path that climbs out of the features folder count too
  features-siblings a file of one feature naming another (`crate::features::<b>`, also as `super::super::<b>` or
                    a bare `use crate::features;`), except the `bridges` the config lists with a reason (a bridge
                    that no longer names its sibling is stale)
  frame-names-no-feature
                    a file of the frame (window, sidebar, onboarding) containing a word that names a feature:
                    the flags and helpers a feature owns, a panel tab by its variant (`feature_rules.frame_bans`)
  state-fields      the number of fields of the big state structs (`feature_rules.state_types`: type -> file), as a ceiling:
                    the allowlist holds today's count and only drops

Why a script that reads text: it runs in a second without building anything (tests only run on CI
here), so check.sh can call it first and every phase of the restructuring has a number that goes to
zero. What a text reader cannot see, said plainly:
  - macros: a `mod` or `crate::x` path produced by macro_rules is invisible (none today);
  - `use super::*` hides the edges behind it from the cycle rule, which is why that rule exists;
  - cfg: every `#[cfg(...)] mod x;` counts as declared, whatever the target; a cfg condition is read
    as words, not evaluated;
  - paths: `crate::B`, `super::B` (any number of `super`s) and `self::B` are edges, also in groups
    (`crate::{B::x, C}`, `super::B::{x, y}`); `super` and `self` start from the file's module (a/b.rs
    and a/b/mod.rs are both `a::b`, an inline `mod x { }` adds `x`). Not seen: a path that starts with a
    child module's bare name (`use sidebar::X;` in view/mod.rs, which only reaches its own tree) and
    `use` of a module through an alias. A module the crate root re-exports (`pub use view::sidebar;`, as
    a move leaves for one commit) is followed to where it is, and so is an item a module above
    `module_depth` re-exports from one of its own modules (`pub use font_slots::X;` in config/mod.rs makes
    `crate::config::X` an edge to `config::font_slots`); a re-export from elsewhere, a glob (`pub use x::*`)
    or an alias (read under its original name) is not followed, and the cross-reexports rule reads
    `pub use crate::...` only.

The allowlist keys are stable text: `path`, `path :: token`, `path :: crate::B`, `crate: a, b, c` (a
cycle), each with a count. Python 3.9, standard library only.
"""
from __future__ import annotations

import argparse
import bisect
import contextlib
import io
import itertools
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
    "file-size", "os-edge", "os-edge-neutral", "bucket-names", "orphans", "glob-imports", "cycles", "layers",
    "cross-reexports", "impl-outside", "feature-api", "features-siblings", "frame-names-no-feature",
    "state-fields",
]
RULE_HELP = {
    "file-size": "files over {fail} lines",
    "os-edge": "OS code outside the OS edge",
    "os-edge-neutral": "OS code in the neutral files of an OS edge",
    "bucket-names": "bucket module names",
    "orphans": "files no mod declaration reaches",
    "glob-imports": "`use super::*` outside tests",
    "cycles": "cycles between modules",
    "layers": "top-level modules using a higher layer",
    "cross-reexports": "re-exports across modules",
    "impl-outside": "impl Workspace/Session outside the home",
    "feature-api": "features naming the window's insides",
    "features-siblings": "features naming each other",
    "frame-names-no-feature": "the frame naming a feature",
    "state-fields": "fields of the big state structs",
}

DEFAULT_CONFIG = {
    # Paths are relative to a crate's folder; a folder ends with a slash, anything else is one file.
    "os_edge": {
        "wormhole": ["src/platform/"],
        "wormhole-core": [],
        "wormhole-host": ["src/sys.rs", "src/peer.rs"],
        "wormhole-term": ["src/pty/"],
        "wormhole-wire": [],
    },
    # Per crate, files that use an OS-edge token for something that isn't OS code: the tokens they may
    # use, and why. A reason is required.
    "os_edge_sanctioned": {},
    # Per crate, the edge folders whose top-level files are the neutral layer (the OS-specific code is
    # in their `unix/`, `macos/` and `linux/` folders): those files may only select an OS with `cfg`.
    "os_edge_neutral": {},
    # The inherent impls of these types belong in the home (a folder or file, relative to the crate).
    "impl_outside": {"crate": "wormhole", "types": ["Workspace", "Session"], "homes": ["src/window/"]},
    # Per crate, its top-level modules in tiers, lowest first; a tier is a module or a list of them.
    "layers": {},
    # Per crate, how many levels of modules the cycles and cross-reexports rules tell apart (default 1,
    # the top level): 2 for a crate whose top level is domains of modules.
    "module_depth": {},
    "bucket_names": ["text", "utils", "util", "misc", "helpers", "helper", "common"],
    "size": {"warn": 600, "fail": 1000},
    # Folders whose size the plan tracks (files and lines, listed by `counts`).
    "track_dirs": ["crates/wormhole/src/window"],
    # The rules about the window and its features (see the docs above); empty: not checked.
    "feature_rules": {},
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
        self.modpath: Dict[str, Tuple[str, ...]] = {}  # file -> its module's path below the crate root
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
            self.modpath[r] = ()
        while queue:
            rel, top, test, base = queue.pop()
            here = self.modpath.get(rel, ())
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
                    self.modpath[target] = here + tuple(p.name for p in it.parents) + (it.name,)
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
                    self.modpath[target] = here
                    queue.append((target, top, test, os.path.dirname(target)))

    def top_of(self, rel: str) -> Optional[str]:
        return self.reached.get(rel, (None, False))[0]

    def root_aliases(self) -> Dict[str, List[str]]:
        """Modules the crate root re-exports under a top-level name: `pub use view::sidebar;` makes
        `crate::sidebar` mean `crate::view::sidebar`."""
        out: Dict[str, List[str]] = {}
        for r in self._roots():
            for m in re.finditer(r"pub\s+use\s+(?:crate\s*::\s*)?(\w+(?:\s*::\s*\w+)+)\s*(?:as\s+(\w+)\s*)?;",
                                 self.files[r].clean):
                path = [p.strip() for p in m.group(1).split("::")]
                if path[0] in self.tops:
                    out[m.group(2) or path[-1]] = path
        return out

    def node_of(self, rel: str, depth: int) -> Optional[str]:
        """The module a file counts for at `depth`: its module path's first `depth` names (`view::sidebar`
        for view/sidebar/wording.rs at 2; `config` for config/mod.rs, which has only one)."""
        if self.top_of(rel) is None:
            return None
        path = self.modpath.get(rel, ())
        return "::".join(path[:depth]) if depth > 1 and path else self.top_of(rel)

    def nodes(self, depth: int) -> List[str]:
        """Every module at `depth` or above it, the top-level ones first."""
        out = list(self.tops)
        if depth > 1:
            seen = set(out)
            for rel in sorted(self.files):
                path = self.modpath.get(rel, ()) if self.top_of(rel) is not None else ()
                for k in range(2, min(depth, len(path)) + 1):
                    name = "::".join(path[:k])
                    if name not in seen:
                        seen.add(name)
                        out.append(name)
        return out

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


# The only token a file may be sanctioned for: `unsafe` has uses that aren't the OS's (a global
# allocator); every other token (cfg(target_os, objc2, libc::, ...) is OS code by definition.
SANCTIONABLE = {"allow(unsafe_code)"}


def sanctioned_tokens(cfg: dict, crate: str) -> Dict[str, Dict[str, int]]:
    """The `os_edge_sanctioned` entries of `crate`: file -> token -> exact count, after checking each
    has a reason, names only a sanctionable token and gives a positive count."""
    out: Dict[str, Dict[str, int]] = {}
    for rel, entry in cfg.get("os_edge_sanctioned", {}).get(crate, {}).items():
        where = "os_edge_sanctioned: %s/%s" % (crate, rel)
        if not str(entry.get("why", "")).strip():
            raise SystemExit("%s needs a `why`" % where)
        tokens = entry.get("tokens", {})
        if not isinstance(tokens, dict) or not tokens:
            raise SystemExit("%s: `tokens` maps each token to its exact count" % where)
        for token, n in tokens.items():
            if token not in SANCTIONABLE:
                raise SystemExit("%s: `%s` is OS code and can't be sanctioned (only %s)"
                                 % (where, token, ", ".join(sorted(SANCTIONABLE))))
            if not isinstance(n, int) or isinstance(n, bool) or n < 1:
                raise SystemExit("%s: the count of `%s` must be a positive number" % (where, token))
        out[rel] = dict(tokens)
    return out


def stale_sanctions(crate: Crate, cfg: dict) -> List[str]:
    """Sanctions that allow more than the file has (or name a file that is gone): they would hide
    the next one silently, so they must shrink with the code."""
    out = []
    for rel, tokens in sorted(sanctioned_tokens(cfg, crate.name).items()):
        src = crate.files.get(rel)
        hits = os_edge_hits(src) if src else {}
        for token, n in sorted(tokens.items()):
            now = len(hits.get(token, []))
            if now < n:
                out.append("%s/%s :: %s: %s" % (crate.name, rel, token, "gone" if now == 0 else "%d -> %d" % (n, now)))
    return out


def rule_os_edge(crate: Crate, cfg: dict, found: Findings) -> None:
    edge = cfg["os_edge"].get(crate.name, [])
    sanctioned = sanctioned_tokens(cfg, crate.name)
    for rel, src in crate.files.items():
        if under(rel, edge):
            continue
        for key, offsets in os_edge_hits(src).items():
            # Exactly the sanctioned count is left out; one more and every site is reported.
            if len(offsets) == sanctioned.get(rel, {}).get(key):
                continue
            for off in offsets:
                _add(found, "%s :: %s" % (src.rel, key), crate.name, src.at(off))


# The folders under a neutral edge that hold the OS-specific code.
OS_FOLDERS = ("unix", "macos", "linux")


def rule_os_edge_neutral(crate: Crate, cfg: dict, found: Findings) -> None:
    for folder in cfg.get("os_edge_neutral", {}).get(crate.name, []):
        for rel, src in crate.files.items():
            if not under(rel, [folder]) or rel[len(folder):].split("/")[0] in OS_FOLDERS:
                continue
            for key, offsets in os_edge_hits(src).items():
                if key.startswith("cfg"):
                    continue
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
    # The integration tests' folder is not in `crate.files`, but a `tests/common/` is as vague as a `common/`.
    tests = os.path.join(crate.dir, "tests")
    for folder, _, names in sorted(os.walk(tests)):
        for file in sorted(names):
            if not file.endswith(".rs"):
                continue
            rel = os.path.relpath(os.path.join(folder, file), crate.dir).replace(os.sep, "/")
            parts = rel.split("/")
            stem = parts[-1][:-3]
            name = parts[-2] if stem == "mod" and len(parts) > 2 else stem
            if name in buckets:
                _add(found, crate.prefix + rel, crate.name, "%s%s:1" % (crate.prefix, rel))


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


def _split_top(body: str) -> List[str]:
    """`a::x, {b, c}, d` -> its pieces at the top level."""
    pieces, cur, depth = [], [], 0
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
    return pieces


_SEGMENT = re.compile(r"\s*(?:pub\s+)?(\w+)")
_PATH_SEP = re.compile(r"\s*::\s*")


def use_paths(tree: str, limit: int) -> List[List[str]]:
    """The paths a use tree names, each cut to `limit` names: `view::{sidebar::X, layout}` ->
    [view, sidebar], [view, layout] at 2. A group's `self`, a glob or an alias ends a path."""
    tree = tree.strip()
    if tree.startswith("{"):
        close = match_close(tree, 0, _BRACE)
        return [p for piece in _split_top(tree[1:close]) for p in use_paths(piece, limit)]
    m = _SEGMENT.match(tree)
    if not m:
        return []
    segs, pos = [m.group(1)], m.end()
    while len(segs) < limit:
        sep = _PATH_SEP.match(tree, pos)
        if not sep:
            break
        pos = sep.end()
        if tree.startswith("{", pos):
            close = match_close(tree, pos, _BRACE)
            subs = [p for piece in _split_top(tree[pos + 1:close]) for p in use_paths(piece, limit - len(segs))]
            return [segs + sub for sub in subs] or [segs]
        n = re.match(r"\w+", tree[pos:])
        if not n:
            break
        segs.append(n.group(0))
        pos += n.end()
    return [segs]


def crate_paths(clean: str, m: "re.Match[str]", limit: int) -> List[List[str]]:
    """The paths after `crate::` at a match whose group(1) is the first name or a group's brace, each
    cut to `limit` names (several for a `crate::{a::x, b}` group)."""
    start = m.start(1)
    if m.group(1) == "{":
        end = match_close(clean, start, _BRACE) + 1
    else:
        end = start
        tail = re.compile(r"\w+(?:\s*::\s*\w+)*(?:\s*::\s*\{)?")
        t = tail.match(clean, start)
        end = t.end() if t else start
        if clean[end - 1:end] == "{":
            end = match_close(clean, end - 1, _BRACE) + 1
    return use_paths(clean[start:end], limit)


def crate_refs(clean: str, m: "re.Match[str]") -> List[str]:
    """The first segments after `crate::` (several for a `crate::{a::x, b}` group)."""
    return [p[0] for p in crate_paths(clean, m, 1)]


def depth_of(crate: Crate, cfg: dict) -> int:
    return int(cfg.get("module_depth", {}).get(crate.name, 1))


def resolve(path: List[str], nodes: Set[str], aliases: Optional[Dict[str, List[str]]] = None,
            depth: int = 1, reexports: Optional[Dict[str, Dict[str, List[str]]]] = None) -> Optional[str]:
    """The deepest module of `nodes` a path names (`git` for `crate::git::DiffLine`), through the crate
    root's re-exported modules. With `reexports`, a name a module above `depth` re-exports from one of
    its own modules is followed there (`crate::config::X` -> `config::font_slots` when config's mod.rs says
    `pub use font_slots::X`), so a domain's re-exports don't hide which of its modules a file uses."""
    if aliases and path and path[0] not in nodes and path[0] in aliases:
        path = aliases[path[0]] + path[1:]
    for _ in range(4):  # a re-export of a re-export, not more
        for k in range(len(path), 0, -1):
            name = "::".join(path[:k])
            if name in nodes:
                break
        else:
            return None
        target = (reexports or {}).get(name, {}).get(path[k]) if k < depth and k < len(path) else None
        if target is None or len(target) <= k or target[:k] != path[:k]:
            return name
        path = target + path[k + 1:]
    return name


_PUB_USE = re.compile(r"pub\s*(?:\([^)]*\)\s*)?use\s+")


def reexports(crate: Crate) -> Dict[str, Dict[str, List[str]]]:
    """module -> {name: the full path it re-exports} from `pub use` in the files of that module itself
    (`pub use font_slots::X;`, `pub use self::font_slots::X;` or `pub use crate::config::font_slots::X;` in config/mod.rs).
    Globs and test code are left out."""
    out: Dict[str, Dict[str, List[str]]] = {}
    for rel, src in crate.files.items():
        if crate.top_of(rel) is None or crate.is_test_file(rel):
            continue
        here = list(crate.modpath.get(rel, ()))
        regions = crate.test_regions(rel) + [(it.start, it.end) for it in crate.mods.get(rel, []) if it.inline]
        for m in scan(_PUB_USE, src.clean):
            if any(x <= m.start() < y for x, y in regions):
                continue
            end = src.clean.find(";", m.end())
            for path in use_paths(src.clean[m.end():end if end >= 0 else len(src.clean)], 16):
                if path[0] == "crate":
                    full = path[1:]
                elif path[0] in ("self", "super"):
                    n = len(path) - len(list(itertools.dropwhile(lambda s: s == "super", path)))
                    rest = path[1:] if path[0] == "self" else path[n:]
                    if n > len(here):
                        continue
                    full = here[:len(here) - n] + rest
                else:
                    full = here + path
                if len(full) > len(here) and full[-1] not in ("self", "super", "crate"):
                    out.setdefault("::".join(here), {})[full[-1]] = full
    return out


def related(a: str, b: str) -> bool:
    """One module is the other, or inside it: a domain and its modules are one tree."""
    return a == b or a.startswith(b + "::") or b.startswith(a + "::")


def module_edges(crate: Crate, with_tests: bool = False, depth: int = 1) -> Dict[Tuple[str, str], str]:
    """(A, B) -> where a file of module A first names B (`crate::B`, `super::B`, `self::B`), modules at
    `depth`. Test code is left out unless asked for: a use in a test can't make the build go in
    circles, and it moves with the test."""
    edges: Dict[Tuple[str, str], str] = {}
    nodes = set(crate.nodes(depth))
    aliases = crate.root_aliases()
    reex = reexports(crate) if depth > 1 else {}
    for rel, src in crate.files.items():
        a = crate.node_of(rel, depth)
        if a is None or (not with_tests and crate.is_test_file(rel)):
            continue
        regions = [] if with_tests else crate.test_regions(rel)

        def add(path: List[str], at: int) -> None:
            b = resolve(path, nodes, aliases, depth, reex)
            if b is not None and not related(a, b) and (a, b) not in edges:
                edges[(a, b)] = src.at(at)

        for m in scan(_CRATE_REF, src.clean):
            if regions and any(x <= m.start() < y for x, y in regions):
                continue
            for path in crate_paths(src.clean, m, depth + 1):
                add(path, m.start())
        for m in scan(_RELATIVE_REF, src.clean):
            if src.clean[m.start() - 1:m.start()] == ":" or (regions and any(x <= m.start() < y for x, y in regions)):
                continue
            base = scope_at(crate, rel, m.start())
            n = len(re.findall(r"\bsuper\b", src.clean[m.start():m.start(1)]))
            if n > len(base):
                continue
            base = base[:len(base) - n]
            for path in crate_paths(src.clean, m, depth + 1):
                add(base + path, m.start())
    return edges


# `super::x`, `super::super::x` and `self::x`; one inside a longer path (`a::self`) is skipped where it's read.
_RELATIVE_REF = re.compile(r"(?:(?:super\s*::\s*)+|self\s*::\s*)(\{|\w+)")


def scope_at(crate: Crate, rel: str, offset: int) -> List[str]:
    """The module path at an offset of a file: the file's module (`a::b` for a/b.rs and a/b/mod.rs alike)
    and the inline `mod x { ... }` blocks around the offset, which `super` and `self` start from."""
    inner = [it for it in crate.mods.get(rel, []) if it.inline and it.start < offset < it.end]
    return list(crate.modpath.get(rel, ())) + [it.name for it in sorted(inner, key=lambda it: it.start)]


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
    depth = depth_of(crate, cfg)
    edges = module_edges(crate, cfg.get("cycles_with_tests", False), depth)
    crate.cycles = sccs(crate.nodes(depth), edges)
    for comp in crate.cycles:
        for (a, b), where in sorted(edges.items()):
            if a in comp and b in comp:
                _add(found, "%s: %s -> %s" % (crate.name, a, b), crate.name, where)


def layer_tiers(tiers: List) -> Dict[str, int]:
    """module -> its tier's index, from a list whose entries are a module or a list of modules."""
    return {m: i for i, tier in enumerate(tiers) for m in ([tier] if isinstance(tier, str) else tier)}


def rule_layers(crate: Crate, cfg: dict, found: Findings) -> None:
    """Lower tiers never use higher ones. Every top-level module has a tier, so a new one can't slip past
    the rule by being left out of the list."""
    tiers = cfg.get("layers", {}).get(crate.name)
    if tiers is None:
        return
    tier = layer_tiers(tiers)
    root = os.path.join("crates", crate.name, "src")
    for m in crate.tops:
        if m not in tier:
            _add(found, "%s: unplaced %s" % (crate.name, m), crate.name, root)
    for m in sorted(set(tier) - set(crate.tops)):
        _add(found, "%s: no module %s" % (crate.name, m), crate.name, root)
    listed = [m for t in tiers for m in ([t] if isinstance(t, str) else t)]
    for m in sorted({m for m in listed if listed.count(m) > 1}):
        _add(found, "%s: twice %s" % (crate.name, m), crate.name, root)
    for (a, b), where in sorted(module_edges(crate, cfg.get("cycles_with_tests", False)).items()):
        if a in tier and b in tier and tier[a] < tier[b]:
            _add(found, "%s: %s -> %s" % (crate.name, a, b), crate.name, where)


_REEXPORT = re.compile(r"pub\s*(?:\([^)]*\)\s*)?use\s+crate\s*::\s*(\{|\w+)")


def rule_cross_reexports(crate: Crate, cfg: dict, found: Findings) -> None:
    depth = depth_of(crate, cfg)
    nodes = set(crate.nodes(depth))
    aliases = crate.root_aliases()
    for rel, src in crate.files.items():
        a = crate.node_of(rel, depth)
        if a is None:
            continue
        for m in scan(_REEXPORT, src.clean):
            # crate_paths wants a match whose group(1) is the first segment, or a group's brace.
            for path in crate_paths(src.clean, m, depth + 1):
                b = resolve(path, nodes, aliases)
                if b is not None and not related(a, b):
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
        # A pending area is inside a home but still counted: its blocks are in the allowlist, which only
        # shrinks, until the area has state of its own.
        if under(rel, conf["homes"]) and not under(rel, conf.get("pending", [])):
            continue
        for m in scan(rx, src.clean):
            _add(found, "%s :: impl %s" % (src.rel, m.group(1)), crate.name, src.at(m.start()))


def _under_dirs(rel: str, dirs: Iterable[str]) -> bool:
    return any(rel.startswith(d) for d in dirs)


def _not_in(regions: List[Tuple[int, int]], offset: int) -> bool:
    return not any(x <= offset < y for x, y in regions)


def root_paths(crate: Crate, rel: str, src: "Src", limit: int) -> List[Tuple[List[str], int]]:
    """Every path a non-test part of a file names, from the crate root, each cut to `limit` names, with
    where it is: `crate::a::b` as written, and `super::super::a` or `self::a` resolved from the file's module
    (so a path that climbs out of a folder is the same as one that starts at `crate`). A bare `crate::a` is
    the one-name path `[a]`."""
    regions = crate.test_regions(rel)
    out: List[Tuple[List[str], int]] = []
    for m in scan(_CRATE_REF, src.clean):
        if _not_in(regions, m.start()):
            out.extend((path, m.start()) for path in crate_paths(src.clean, m, limit))
    for m in scan(_RELATIVE_REF, src.clean):
        before = src.clean[m.start() - 1:m.start()]
        if before == ":" or before.isalnum() or before == "_" or not _not_in(regions, m.start()):
            continue
        base = scope_at(crate, rel, m.start())
        n = len(re.findall(r"\bsuper\b", src.clean[m.start():m.start(1)]))
        if n > len(base):
            continue
        base = base[:len(base) - n]
        out.extend((base + path, m.start()) for path in crate_paths(src.clean, m, limit))
    return out


def rule_feature_api(crate: Crate, cfg: dict, found: Findings) -> None:
    conf = cfg.get("feature_rules") or {}
    if crate.name != conf.get("crate"):
        return
    allowed = set(conf["api"])
    workspace_files = set(conf.get("workspace_files", []))
    for rel, src in crate.files.items():
        if not rel.startswith(conf["features"]) or crate.is_test_file(rel):
            continue
        for path, at in root_paths(crate, rel, src, 2):
            if not path or path[0] != "window":
                continue
            # `use crate::window;` names nothing yet but lets `window::x` reach anything after it.
            name = path[1] if len(path) >= 2 else "(bare)"
            if name == "Workspace" and rel.rsplit("/", 1)[-1] in workspace_files:
                continue
            if name not in allowed:
                _add(found, "%s :: crate::window::%s" % (src.rel, name), crate.name, src.at(at))


def bridge_uses(crate: Crate, conf: dict) -> Dict[Tuple[str, str], int]:
    """(file, sibling) -> how many times the file names the sibling feature, for the bridged files only."""
    uses: Dict[Tuple[str, str], int] = {}
    for rel, src in crate.files.items():
        bridged = conf.get("bridges", {}).get(rel)
        if not bridged or crate.is_test_file(rel):
            continue
        for path, _ in root_paths(crate, rel, src, 2):
            if len(path) >= 2 and path[0] == "features" and path[1] in bridged:
                uses[(rel, path[1])] = uses.get((rel, path[1]), 0) + 1
    return uses


def rule_features_siblings(crate: Crate, cfg: dict, found: Findings) -> None:
    conf = cfg.get("feature_rules") or {}
    if crate.name != conf.get("crate"):
        return
    base = conf["features"]
    for rel, src in crate.files.items():
        if not rel.startswith(base) or crate.is_test_file(rel):
            continue
        own = rel[len(base):].split("/")[0]
        own = own[:-3] if own.endswith(".rs") else own
        if own == "mod":  # the table itself lists every feature
            continue
        bridged = conf.get("bridges", {}).get(rel, {})
        for path, at in root_paths(crate, rel, src, 2):
            if not path or path[0] != "features":
                continue
            # `use crate::features;` lets `features::b` through, unseen.
            name = path[1] if len(path) >= 2 else "(bare)"
            if name != own and name not in bridged:
                _add(found, "%s :: crate::features::%s" % (src.rel, name), crate.name, src.at(at))


def stale_bridges(crate: Crate, cfg: dict) -> List[str]:
    conf = cfg.get("feature_rules") or {}
    if crate.name != conf.get("crate"):
        return []
    uses = bridge_uses(crate, conf)
    out = []
    for rel, siblings in sorted(conf.get("bridges", {}).items()):
        for sibling, why in sorted(siblings.items()):
            if not why:
                out.append("%s%s: the bridge to %s has no reason" % (crate.prefix, rel, sibling))
            elif (rel, sibling) not in uses:
                out.append("%s%s no longer names features::%s: take the bridge out" % (crate.prefix, rel, sibling))
    return out


def rule_frame_names_no_feature(crate: Crate, cfg: dict, found: Findings) -> None:
    conf = cfg.get("feature_rules") or {}
    if crate.name != conf.get("crate"):
        return
    bans = [(name, re.compile(rx)) for name, rx in conf["frame_bans"].items()]
    for rel, src in crate.files.items():
        if not _under_dirs(rel, conf["frame"]) or crate.is_test_file(rel):
            continue
        regions = crate.test_regions(rel)
        for name, rx in bans:
            for m in scan(rx, src.clean):
                if _not_in(regions, m.start()):
                    _add(found, "%s :: %s" % (src.rel, name), crate.name, src.at(m.start()))


def count_fields(clean: str, open_pos: int) -> int:
    """The fields of the struct whose `{` is at `open_pos`: names followed by a colon at the top level of
    its braces (attributes, comments and generics in field types don't count)."""
    close = match_close(clean, open_pos, _BRACE)
    body = clean[open_pos + 1:close]
    depth, count, i = 0, 0, 0
    field = re.compile(r"\s*(?:#\s*\[(?:[^\[\]]|\[[^\]]*\])*\]\s*)*(?:pub\s*(?:\([^)]*\)\s*)?)?(?:r#)?\w+\s*:(?!:)")
    start = 0
    for i, ch in enumerate(body):
        if ch in "{(<[":
            depth += 1
        elif ch in "})>]":
            # `->` in a fn type is not a closing angle bracket
            if ch == ">" and body[i - 1:i] == "-":
                continue
            depth -= 1
        elif ch == "," and depth == 0:
            if field.match(body, start):
                count += 1
            start = i + 1
    if body[start:].strip() and field.match(body, start):
        count += 1
    return count


def rule_state_fields(crate: Crate, cfg: dict, found: Findings) -> None:
    conf = cfg.get("feature_rules") or {}
    if crate.name != conf.get("crate"):
        return
    for name, rel in conf["state_types"].items():
        src = crate.files.get(rel)
        if src is None:
            continue
        rx = re.compile(r"\bstruct\s+%s\b\s*(?:<[^>{]*>)?\s*\{" % re.escape(name))
        for m in scan(rx, src.clean):
            _add(found, "%s :: %s" % (crate.name, name), crate.name, src.at(m.start()), count_fields(src.clean, m.end() - 1))


RULE_FUNCS = {
    "file-size": rule_file_size, "os-edge": rule_os_edge,
    "os-edge-neutral": rule_os_edge_neutral, "bucket-names": rule_bucket_names,
    "orphans": rule_orphans, "glob-imports": rule_glob_imports, "cycles": rule_cycles, "layers": rule_layers,
    "cross-reexports": rule_cross_reexports, "impl-outside": rule_impl_outside,
    "feature-api": rule_feature_api, "features-siblings": rule_features_siblings,
    "frame-names-no-feature": rule_frame_names_no_feature, "state-fields": rule_state_fields,
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
        self.stale_sanctions = [line for crate in self.crates for line in stale_sanctions(crate, cfg)]
        self.stale_sanctions += [line for crate in self.crates for line in stale_bridges(crate, cfg)]
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
        self.stale_sanctions: List[str] = []

    @property
    def ok(self) -> bool:
        return not self.problems and not self.stale and not self.stale_sanctions


def describe(an: Analysis, rule: str, key: str, item: Item, have: int) -> str:
    allowed = "not in the allowlist" if not have else "the allowlist allows %d" % have
    if rule == "file-size":
        return "%d lines, over %d (%s)" % (item.count, an.cfg["size"]["fail"], allowed)
    if rule == "cycles":
        return "%s depends on %s inside a cycle (%s)" % (tuple(key.split(": ", 1)[1].split(" -> ")) + (allowed,))
    if rule == "layers":
        what = key.split(": ", 1)[1]
        if what.startswith("unplaced "):
            return "top-level module %s has no tier in `layers` (%s)" % (what[9:], allowed)
        if what.startswith("no module "):
            return "`layers` names %s, which the crate doesn't have (%s)" % (what[10:], allowed)
        if what.startswith("twice "):
            return "`layers` lists %s in two tiers (%s)" % (what[6:], allowed)
        return "%s uses %s, a higher layer (%s)" % (tuple(what.split(" -> ")) + (allowed,))
    if rule == "state-fields":
        return "%s has %d fields, over the ceiling (%s)" % (key.split(" :: ", 1)[1], item.count, allowed)
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
    v.stale_sanctions = list(an.stale_sanctions)
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
    if v.stale_sanctions:
        print("stale sanctions or bridges in %s (fix them by hand):" % os.path.relpath(args.config, args.root))
        for line in v.stale_sanctions:
            print("  " + line)
    if v.ok:
        if not args.quiet:
            print("structure: ok (%d allowlisted entries across %d rules, %.1f s)" % (
                sum(len(a) for a in allow.values()), len([r for r in RULES if allow[r]]), time.time() - started))
        return 0
    print("structure: FAILED, %d new or grown violation(s), %d stale allowlist entr%s, %d stale sanction(s)" % (
        len(v.problems), len(v.stale), "y" if len(v.stale) == 1 else "ies", len(v.stale_sanctions)))
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
        files[item.crate].add(key.split(" :: ")[0] if rule not in ("cycles", "layers") else key)
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
        if rule == "layers":
            row("layers: violations", {c: pc[c][0] for c in crates})
            continue
        if rule == "state-fields":
            for key, item in sorted(an.found[rule].items()):
                row("state-fields: %s" % key.split(" :: ", 1)[1], {item.crate: item.count})
            continue
        unit = {"os-edge": "sites", "os-edge-neutral": "sites", "glob-imports": "uses",
                "cross-reexports": "re-exports", "impl-outside": "impl blocks", "feature-api": "sites",
                "features-siblings": "sites", "frame-names-no-feature": "sites"}.get(rule)
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
    print("cycles between modules")
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
    cfg["impl_outside"] = {"crate": "app", "types": ["Workspace", "Session"], "homes": ["src/window/"]}
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
        # a sanction leaves out exactly its count of `allow(unsafe_code)`, for that file only; more is
        # reported in full, fewer (or a gone file) is stale, and other tokens or no reason are refused.
        unsafe2 = "#[allow(unsafe_code)]\nfn h() {}\n#[allow(unsafe_code)]\nfn k() {}\n"
        cfg = _fixture_cfg()
        cfg["os_edge_sanctioned"] = {"app": {"src/os.rs": {"tokens": {"allow(unsafe_code)": 1}, "why": "an allocator"}}}
        an = make({"crates/app/src/main.rs": "mod os;\nmod other;\n", "crates/app/src/os.rs": os_file,
                   "crates/app/src/other.rs": "#[allow(unsafe_code)]\nfn h() {}\n"}, cfg)
        expect("os-edge sanctioned", sorted(k for k in an.found["os-edge"] if "unsafe" in k),
               ["crates/app/src/other.rs :: allow(unsafe_code)"])
        expect("os-edge sanctioned is not stale", an.stale_sanctions, [])
        an = make({"crates/app/src/main.rs": "mod os;\n", "crates/app/src/os.rs": unsafe2}, cfg)
        expect("os-edge sanctioned and one more", an.found["os-edge"]["crates/app/src/os.rs :: allow(unsafe_code)"].count, 2)
        an = make({"crates/app/src/main.rs": "mod os;\n", "crates/app/src/os.rs": "fn h() {}\n"}, cfg)
        expect("os-edge sanctioned and gone", an.stale_sanctions, ["app/src/os.rs :: allow(unsafe_code): gone"])
        an = make({"crates/app/src/main.rs": "mod other;\n", "crates/app/src/other.rs": ""}, cfg)
        expect("os-edge sanctioned file gone", an.stale_sanctions, ["app/src/os.rs :: allow(unsafe_code): gone"])
        cfg3 = _fixture_cfg()
        cfg3["os_edge_sanctioned"] = {"app": {"src/os.rs": {"tokens": {"allow(unsafe_code)": 2}, "why": "x"}}}
        an = make({"crates/app/src/main.rs": "mod os;\n", "crates/app/src/os.rs": "#[allow(unsafe_code)]\nfn h() {}\n"}, cfg3)
        expect("os-edge sanctioned shrank", an.stale_sanctions, ["app/src/os.rs :: allow(unsafe_code): 2 -> 1"])
        for bad, why in [({"objc2": 1}, "x"), ({"allow(unsafe_code)": 0}, "x"), (["allow(unsafe_code)"], "x"),
                         ({"allow(unsafe_code)": 1}, " ")]:
            cfg2 = _fixture_cfg()
            cfg2["os_edge_sanctioned"] = {"app": {"src/os.rs": {"tokens": bad, "why": why}}}
            try:
                make({"crates/app/src/main.rs": "mod os;\n", "crates/app/src/os.rs": os_file}, cfg2)
                expect("os-edge sanction refused: %r %r" % (bad, why), "accepted", "refused")
            except SystemExit:
                pass

        # os-edge-neutral: the files of a neutral edge folder outside unix/, macos/ and linux/ may only
        # select an OS with cfg; any other OS token fails there, in a subfolder too, but not in unix/.
        neutral = ('#[cfg(target_os = "macos")]\n#[path = "macos/x.rs"]\nmod os;\n#[cfg_attr(unix, inline)]\nfn a() {}\n'
                   'fn b() { if cfg!(unix) {} }\n')
        cfg = _fixture_cfg()
        cfg["os_edge_neutral"] = {"app": ["src/native/"]}
        an = make({"crates/app/src/main.rs": "mod native;\nmod other;\n",
                   "crates/app/src/native/mod.rs": neutral + "mod unix;\nmod rest;\n",
                   "crates/app/src/native/rest/mod.rs": neutral + os_file,
                   "crates/app/src/native/unix/mod.rs": os_file,
                   "crates/app/src/other.rs": os_file}, cfg)
        expect("os-edge-neutral files", sorted({k.split(" :: ")[0] for k in an.found["os-edge-neutral"]}),
               ["crates/app/src/native/rest/mod.rs"])
        expect("os-edge-neutral tokens", sorted(k.split(" :: ")[1] for k in an.found["os-edge-neutral"]),
               ["allow(unsafe_code)", "libc::", "objc2", "std::os::unix", "target_vendor"])
        expect("os-edge-neutral reports a line", an.found["os-edge-neutral"]["crates/app/src/native/rest/mod.rs :: objc2"].locs,
               ["crates/app/src/native/rest/mod.rs:20"])
        an = make({"crates/app/src/main.rs": "mod native;\n", "crates/app/src/native/mod.rs": neutral}, cfg)
        expect("os-edge-neutral clean", an.found["os-edge-neutral"], {})

        # bucket names: files, mod.rs folders, inline modules.
        an = make({"crates/app/src/main.rs": "mod text;\nmod utils;\nmod ok;\nmod inl { mod helpers {} }\n",
                   "crates/app/src/text.rs": "", "crates/app/src/utils/mod.rs": "", "crates/app/src/ok.rs": ""})
        expect("bucket-names", keys(an, "bucket-names"), [
            "crates/app/src/main.rs :: mod helpers", "crates/app/src/text.rs", "crates/app/src/utils/mod.rs"])
        an = make({"crates/app/src/main.rs": "", "crates/app/tests/common/mod.rs": "", "crates/app/tests/support/mod.rs": "",
                   "crates/app/tests/util.rs": "", "crates/app/tests/fixtures/data.json": ""})
        expect("bucket-names under tests/", keys(an, "bucket-names"),
               ["crates/app/tests/common/mod.rs", "crates/app/tests/util.rs"])

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

        # module_depth: cycles and cross-reexports between modules inside domains (`view::sidebar`), a
        # domain and its own modules never an edge, nested groups read to the depth; layers stay per domain.
        domains = {
            "crates/lib/src/lib.rs": "pub mod view;\npub mod theme;\n",
            "crates/lib/src/view/mod.rs": "pub mod sidebar;\npub mod layout;\npub use crate::view::sidebar::S;\n",
            "crates/lib/src/view/sidebar.rs": "use crate::view::layout::X;\nuse crate::view::Root;\n",
            "crates/lib/src/view/layout.rs": "use crate::view::{sidebar::Y, Root};\npub use crate::theme::tokens::Color;\n",
            "crates/lib/src/theme/mod.rs": "pub mod tokens;\npub use crate::theme::tokens::T;\n"
                                           "use crate::{view::{sidebar, layout::Z}, theme};\n",
            "crates/lib/src/theme/tokens/mod.rs": "mod color;\npub use crate::theme::tokens::color::Color;\n",
            "crates/lib/src/theme/tokens/color.rs": "fn f() { crate::theme::tokens::g(); }\n",
        }
        deep = dict(_fixture_cfg(), module_depth={"lib": 2}, layers={"lib": ["view", "theme"]})
        an = make(domains, deep)
        expect("depth: cycles inside a domain", keys(an, "cycles"), ["lib: view::layout -> view::sidebar",
                                                                    "lib: view::sidebar -> view::layout"])
        expect("depth: cross-reexports", keys(an, "cross-reexports"), ["crates/lib/src/view/layout.rs :: crate::theme::tokens"])
        expect("depth: layers per domain", keys(an, "layers"), ["lib: view -> theme"])
        expect("depth: modules", sorted(an.crates[0].nodes(2)), ["theme", "theme::tokens", "view", "view::layout", "view::sidebar"])
        an = make(domains, dict(deep, module_depth={}))
        expect("depth 1: one cycle between domains", keys(an, "cycles"), ["lib: theme -> view", "lib: view -> theme"])
        expect("depth 1: cross-reexports", keys(an, "cross-reexports"), ["crates/lib/src/view/layout.rs :: crate::theme"])
        # a module the root re-exports for a move's one commit is followed to its new place.
        shimmed = dict(domains)
        shimmed["crates/lib/src/lib.rs"] = "pub mod view;\npub mod theme;\n#[doc(hidden)]\npub use view::sidebar;\n" \
                                           "pub use theme::tokens as colours;\n"
        shimmed["crates/lib/src/theme/tokens/color.rs"] = "use crate::sidebar::Q;\nuse crate::colours::R;\n"
        an = make(shimmed, deep)
        expect("shims: edges through them", "lib: theme -> view" in an.found["layers"], False)
        edges = module_edges(an.crates[0], False, 2)
        expect("shims: followed", (("theme::tokens", "view::sidebar") in edges, ("theme::tokens", "theme::tokens") in edges),
               (True, False))
        # super:: and self:: paths resolve from the file's module (a/b.rs and a/b/mod.rs are both a::b,
        # an inline `mod x {}` adds x), and a domain's re-export of its own module's item is followed.
        rel = {
            "crates/lib/src/lib.rs": "pub mod view;\npub mod theme;\n",
            "crates/lib/src/view/mod.rs": "mod sidebar;\nmod layout;\nmod panel;\npub use self::sidebar::S;\n"
                                          "pub use layout::{L, M as N};\npub use crate::theme::T;\n",
            "crates/lib/src/view/sidebar.rs": "use super::layout::X;\n",
            "crates/lib/src/view/layout.rs": "fn f() { super::sidebar::g(); }\nmod inner { use super::super::panel::P; }\n"
                                             "#[cfg(test)]\nmod tests { use super::super::super::theme::Q; }\n",
            "crates/lib/src/view/panel/mod.rs": "use self::super::super::super::super::Nope;\nuse a::self::b;\n",
            "crates/lib/src/theme/mod.rs": "pub mod tokens;\nuse crate::view::S;\nuse super::view::L;\n",
            "crates/lib/src/theme/tokens.rs": "use super::super::view::T;\n",
        }
        an = make(rel, dict(_fixture_cfg(), module_depth={"lib": 2}))
        edges = sorted(module_edges(an.crates[0], False, 2))
        expect("relative: edges", edges, [
            ("theme", "view::layout"), ("theme", "view::sidebar"), ("theme::tokens", "view"),
            ("view", "theme"), ("view::layout", "view::panel"), ("view::layout", "view::sidebar"), ("view::sidebar", "view::layout")])
        expect("relative: a sibling cycle through super", keys(an, "cycles"), [
            "lib: view::layout -> view::sidebar", "lib: view::sidebar -> view::layout"])
        expect("relative: scope of an inline module", scope_at(an.crates[0], "src/view/layout.rs",
                                                              rel["crates/lib/src/view/layout.rs"].index("use super")),
               ["view", "layout", "inner"])
        expect("relative: re-exports", reexports(an.crates[0]).get("view"), {
            "S": ["view", "sidebar", "S"], "L": ["view", "layout", "L"], "M": ["view", "layout", "M"],
            "T": ["theme", "T"]})
        an = make(rel, _fixture_cfg())
        expect("relative: at depth 1 only paths out of a top-level module count",
               sorted(module_edges(an.crates[0], False, 1)), [("theme", "view"), ("view", "theme")])
        expect("use paths", use_paths("a::{b::{c, d}, e as f, self}, g", 3), [["a", "b", "c"], ["a", "b", "d"], ["a", "e"], ["a", "self"]])

        # layers: up a tier fails, within a tier and down are fine; every module needs a tier; test edges
        # count only with cycles_with_tests.
        layered = dict(_fixture_cfg(), layers={"app": ["low", ["mid", "side"], "top"], "lib": []})
        an = make({
            "crates/app/src/main.rs": "mod low;\nmod mid;\nmod side;\nmod top;\nmod stray;\n",
            "crates/app/src/low.rs": "use crate::top::T;\n#[cfg(test)]\nmod tests {\n    use crate::mid::M;\n}\n",
            "crates/app/src/mid.rs": "use crate::{low::L, side::S};\n",
            "crates/app/src/side.rs": "use crate::mid::M;\nuse crate::top::T;\n",
            "crates/app/src/top.rs": "use crate::{low, mid::M, stray};\n",
            "crates/app/src/stray.rs": "",
            "crates/lib/src/lib.rs": "mod a;\n", "crates/lib/src/a.rs": "",
        }, layered)
        expect("layers", keys(an, "layers"), [
            "app: low -> top", "app: side -> top", "app: unplaced stray", "lib: unplaced a"])
        an = make({"crates/app/src/main.rs": "mod low;\n", "crates/app/src/low.rs": ""},
                  dict(_fixture_cfg(), layers={"app": ["low", "gone"]}))
        expect("layers name a missing module", keys(an, "layers"), ["app: no module gone"])
        an = make({"crates/app/src/main.rs": "mod low;\nmod top;\n", "crates/app/src/low.rs": "",
                   "crates/app/src/top.rs": ""},
                  dict(_fixture_cfg(), layers={"app": ["low", ["top", "low"]]}))
        expect("layers list a module once", keys(an, "layers"), ["app: twice low"])
        an = make({"crates/app/src/main.rs": "mod a;\nmod b;\n", "crates/app/src/a.rs": "#[cfg(test)]\nmod tests {\n"
                   "    use crate::b::X;\n}\n", "crates/app/src/b.rs": ""},
                  dict(_fixture_cfg(), layers={"app": ["a", "b"]}, cycles_with_tests=True))
        expect("layers read test edges on request", keys(an, "layers"), ["app: a -> b"])
        expect("no layers, no rule", keys(make({"crates/app/src/main.rs": "mod a;\n", "crates/app/src/a.rs": ""}), "layers"), [])

        # impl-outside: inherent impls only, generics and paths included, the home exempt.
        an = make({
            "crates/app/src/main.rs": "mod window;\nmod other;\n",
            "crates/app/src/window/mod.rs": "impl Workspace {}\nimpl Session {}\n",
            "crates/app/src/other.rs": "impl Workspace {}\nimpl<'a, T: Fn(u8) -> Vec<u8>> Workspace<'a> where T: Copy {}\n"
                                       "impl Session {}\nimpl Render for Workspace {}\nimpl crate::window::Session {}\n"
                                       "// impl Workspace {}\n",
            "crates/lib/src/lib.rs": "impl Workspace {}\n"})
        expect("impl-outside", {k: it.count for k, it in an.found["impl-outside"].items()},
               {"crates/app/src/other.rs :: impl Workspace": 2, "crates/app/src/other.rs :: impl Session": 2})
        an = make({
            "crates/app/src/main.rs": "mod window;\n",
            "crates/app/src/window/mod.rs": "impl Workspace {}\n",
            "crates/app/src/window/area/mod.rs": "impl Workspace {}\nimpl Workspace {}\nimpl Session {}\n",
            "crates/app/src/window/other.rs": "impl Workspace {}\n"},
            dict(_fixture_cfg(), impl_outside=dict(_fixture_cfg()["impl_outside"], pending=["src/window/area/"])))
        expect("impl-outside pending areas stay counted inside the home",
               {k: it.count for k, it in an.found["impl-outside"].items()},
               {"crates/app/src/window/area/mod.rs :: impl Workspace": 2,
                "crates/app/src/window/area/mod.rs :: impl Session": 1})

        # feature-api, features-siblings, frame-names-no-feature, state-fields: the window and its features.
        rules_cfg = dict(_fixture_cfg(), feature_rules={
            "crate": "app", "features": "src/features/", "api": ["feature", "facts"], "workspace_files": ["def.rs"],
            "frame": ["src/window/", "src/sidebar/"],
            "frame_bans": {"notes_on": r"\bnotes_on\s*\(", "TabId variant": r"TabId\s*::\s*(?:Changes|Notes)\b"},
            "bridges": {"src/features/a/bridge.rs": {"b": "a needs b once"}},
            "state_types": {"Workspace": "src/window/mod.rs", "Big": "src/big.rs"}})
        files = {
            "crates/app/src/main.rs": "mod window;\nmod sidebar;\nmod features;\nmod big;\n",
            "crates/app/src/window/mod.rs":
                "use crate::features::a::X;\nfn f(cx: u8) { notes_on(cx); }\nlet notes_on = 1;\n"
                "// notes_on(cx)\nfn g() -> TabId { TabId::default() }\nlet t = TabId::Changes;\n"
                "pub struct Workspace {\n    /// a doc: not_a_field: x\n    pub(crate) sessions: Vec<Session>,\n"
                "    #[allow(dead_code)]\n    map: HashMap<String, (u8, u8)>,\n    hook: fn(u8) -> Option<u8>,\n"
                "    last: i64\n}\n#[cfg(test)]\nmod tests {\n    fn t() { notes_on(1); let x = TabId::Notes; }\n}\n",
            "crates/app/src/sidebar/mod.rs": "fn s() { let n = TabId::Notes; }\n",
            "crates/app/src/big.rs": "struct Big<T: Clone> {\n    a: T,\n    b: Vec<fn(u8) -> u8>,\n}\nstruct Small { a: u8 }\n",
            "crates/app/src/features/mod.rs": "mod a;\nmod b;\nuse crate::features::a::X;\nuse crate::features::b::Y;\n",
            "crates/app/src/features/a/mod.rs":
                "mod bridge;\nmod def;\nmod holes;\nuse crate::window::feature::F;\nuse crate::window::Workspace;\n"
                "use crate::window::{facts::W, sessions::S, focus::G};\nuse crate::features::a::own::Z;\n"
                "use crate::features::b::Y;\n// use crate::window::sessions::C;\n"
                "#[cfg(test)]\nmod tests {\n    use crate::window::view_tests::V;\n    use crate::features::b::T;\n}\n",
            "crates/app/src/features/a/bridge.rs": "use crate::features::b::Y;\nuse crate::features::c::W;\n",
            "crates/app/src/features/b.rs": "use crate::features::a::A;\n",
            # Workspace is the one file name's business; the bare and relative forms are caught.
            "crates/app/src/features/a/def.rs": "use crate::window::Workspace;\nuse crate::window::feature::F;\n",
            "crates/app/src/features/a/holes.rs":
                "use crate::window;\nuse crate::features;\nuse super::super::super::window::sessions::S;\n"
                "use super::super::b::Y;\nuse super::super::a::own::Z;\nuse super::sibling::Q;\n"
                "use crate::{window, features::b::T};\nfn f() { let myself::x = 1; }\n",
        }
        an = make(files, rules_cfg)
        expect("feature-api", {k: it.count for k, it in an.found["feature-api"].items()},
               {"crates/app/src/features/a/mod.rs :: crate::window::sessions": 1,
                "crates/app/src/features/a/mod.rs :: crate::window::focus": 1,
                "crates/app/src/features/a/mod.rs :: crate::window::Workspace": 1,
                "crates/app/src/features/a/holes.rs :: crate::window::(bare)": 2,
                "crates/app/src/features/a/holes.rs :: crate::window::sessions": 1})
        expect("features-siblings",
               {k: it.count for k, it in an.found["features-siblings"].items()},
               {"crates/app/src/features/a/mod.rs :: crate::features::b": 1,
                "crates/app/src/features/a/bridge.rs :: crate::features::c": 1,
                "crates/app/src/features/b.rs :: crate::features::a": 1,
                "crates/app/src/features/a/holes.rs :: crate::features::(bare)": 1,
                "crates/app/src/features/a/holes.rs :: crate::features::b": 2})
        expect("frame-names-no-feature",
               {k: it.count for k, it in an.found["frame-names-no-feature"].items()},
               {"crates/app/src/window/mod.rs :: notes_on": 1, "crates/app/src/window/mod.rs :: TabId variant": 1,
                "crates/app/src/sidebar/mod.rs :: TabId variant": 1})
        expect("state-fields", {k: it.count for k, it in an.found["state-fields"].items()},
               {"app :: Workspace": 4, "app :: Big": 2})
        expect("a bridge in use is not stale", an.stale_sanctions, [])
        files2 = dict(files, **{"crates/app/src/features/a/bridge.rs": "use crate::features::c::W;\n"})
        expect("a bridge that names nothing is stale", make(files2, rules_cfg).stale_sanctions,
               ["crates/app/src/features/a/bridge.rs no longer names features::b: take the bridge out"])
        no_reason = dict(rules_cfg, feature_rules=dict(rules_cfg["feature_rules"],
                                                        bridges={"src/features/a/bridge.rs": {"b": ""}}))
        expect("a bridge needs a reason", make(files, no_reason).stale_sanctions,
               ["crates/app/src/features/a/bridge.rs: the bridge to b has no reason"])
        expect("without feature_rules the four rules are off",
               [keys(make(files), r) for r in ("feature-api", "features-siblings", "frame-names-no-feature", "state-fields")],
               [[], [], [], []])

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
