#!/usr/bin/env python3
"""Move the app's modules out of `workspace/` (phase 4 of the app-structure plan), in two scripted steps.

  scripts/app-moves.py --series 1 --move      commit A: `git mv` every entry of the series (scripts/app-moves.json),
                                              the new parents' mod.rs files, the declarations moved with the files
                                              (the crate root's new top-level modules, `app`'s children), shims in
                                              workspace/mod.rs (and main.rs) that keep every old path working for
                                              one commit, and scripts/structure.json's layers and file keys
  scripts/app-moves.py --series 1 --rewrite   commit B: every path names the module where it is now (code, comments
                                              and doc links of the app crate: `crate::workspace::x` becomes
                                              `crate::new::x`, a `use crate::workspace::{...}` group gives the moved
                                              names their own lines, a renamed module is called by its new name
                                              where a file imports it), the shims and the re-exports of moved things
                                              removed, and the file paths in the other tracked text files

A series may move files and folders anywhere (nested parents too, `workspace::welcome` and the like) and may
rename a top-level folder with no per-file moves (`{"from": "workspace", "to": "window"}`: the shim is
`use crate::window as workspace;`, every `crate::workspace` path, `pub(in crate::workspace)` included, and the
structure checker's layer, `track_dirs`, `homes` and module-pair keys follow it). A moved file's `super::`
paths (at any depth, but not the ones inside the file's own inline `mod`s) and `pub(super)` keep the meaning
they had where the file was: a `super::` whose target is no longer the file's `super` becomes
`crate::<old target>::`, and a `pub(super)` becomes `pub(in crate::<old parent>)` (`pub(crate)` when the file
moved out of the old parent, which can no longer be named: the step lists those); the rewrite step then maps
both onto the new places. In a file that did not move, a `super::` that names something that moved is made
absolute and rewritten too.

Both are deterministic, and idempotent: a step that has run already finds nothing to do. Neither runs
cargo: after each step run `cargo fmt --all` (it sorts the module declarations and imports the step adds),
then the gate. What the script cannot know is which private items the moved files now reach across the
new boundary: those are widened by hand, in a commit before `--move` (a whole signature at a time, so no
`private_interfaces`), and the step names what it could not place.

To redo a series after a rebase: drop the two commits, rebase, run `--move` (+ fmt), commit, run `--rewrite`
(+ fmt), commit. Only paths are rewritten: the action namespace `workspace` (`actions!(workspace, ...)`,
`namespace = workspace`), `key_context("Workspace")` and devctl's `action workspace::...` docs stay.
Python 3.9, standard library only.
"""
from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
from typing import Dict, List, Optional, Tuple

REPO = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
MAPPING = os.path.join("scripts", "app-moves.json")
SHIM_MARK = "// Old paths, for one commit only"


def read(path: str) -> str:
    with open(path, encoding="utf-8") as f:
        return f.read()


def write(path: str, text: str) -> None:
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)


def git(root: str, *args: str) -> None:
    subprocess.run(["git", "-C", root] + list(args), check=True)


def tracked(root: str, *patterns: str) -> List[str]:
    out = subprocess.run(["git", "-C", root, "ls-files", "--"] + list(patterns), check=True,
                         capture_output=True, text=True).stdout.split("\n")
    return [p for p in out if p]


def module_of(path: str) -> str:
    """`workspace/app_menu.rs` -> `workspace::app_menu`, `a/b/mod.rs` -> `a::b`, `main.rs` -> ``."""
    if path == "main.rs":
        return ""
    if path.endswith("/mod.rs"):
        path = path[:-len("/mod.rs")]
    elif path.endswith(".rs"):
        path = path[:-3]
    return path.replace("/", "::")


def ancestor(module: str, up: int) -> Optional[str]:
    """The module `up` levels above `module` (`super::` is 1), or None when that is above the crate root."""
    parts = module.split("::") if module else []
    if up > len(parts):
        return None
    return "::".join(parts[:len(parts) - up])


def crate_path(module: str) -> str:
    return "crate::" + module if module else "crate"


class Move:
    """One file or folder: `src` and `dst` are relative to the app crate's src/, as paths."""

    def __init__(self, src: str, dst: str, is_dir: bool) -> None:
        self.src, self.dst, self.is_dir = src, dst, is_dir

    @property
    def old(self) -> str:
        return module_of(self.src)

    @property
    def new(self) -> str:
        return module_of(self.dst)

    @property
    def old_parent(self) -> str:
        return self.old.rpartition("::")[0]

    @property
    def new_parent(self) -> str:
        return self.new.rpartition("::")[0]

    @property
    def new_leaf(self) -> str:
        return self.new.rpartition("::")[2]

    @property
    def old_leaf(self) -> str:
        return self.old.rpartition("::")[2]

    @property
    def is_rename(self) -> bool:
        """A top-level folder or file that only changes its name (`workspace` -> `window`)."""
        return not self.old_parent and not self.new_parent


class Plan:
    def __init__(self, root: str, series: str) -> None:
        self.root = root
        with open(os.path.join(root, MAPPING), encoding="utf-8") as f:
            data = json.load(f)
        self.crate_dir = data["crate"]
        self.src = os.path.join(root, self.crate_dir, "src")
        spec = data["series"][series]
        self.parents: Dict[str, dict] = spec.get("parents", {})
        self.reexports: Dict[str, str] = spec.get("reexports", {})
        # A re-export is named by its path below this module (`workspace::init_actions`).
        self.reexport_base: str = spec.get("reexport_base", "workspace")
        self.layers = spec.get("layers", {})  # a spec, or a list of them
        self.fixups: List[dict] = spec.get("fixups", [])
        # Leaves of moved modules that comments name in backticks (`app_tick::x`): by default the renamed ones.
        self.prose_leaves: Optional[List[str]] = spec.get("prose")
        self.moves: List[Move] = []
        for m in spec["moves"]:
            is_dir = not m["from"].endswith(".rs")
            self.moves.append(Move(m["from"], m["to"], is_dir))
        self._by_len = sorted(self.moves, key=lambda m: -len(m.old))

    def root_file(self, parent: str) -> str:
        """The file that declares the children of a module (relative to src): the crate root, or its mod.rs."""
        if not parent:
            return "main.rs"
        return parent.replace("::", "/") + "/mod.rs"

    def old_parent_file(self, m: Move) -> str:
        return self.root_file(m.old_parent)

    def translate(self, module: str) -> str:
        """Where the module an old path names is now (a folder's children go with it)."""
        for m in self._by_len:
            if module == m.old or module.startswith(m.old + "::"):
                return m.new + module[len(m.old):]
        return module

    def reexport_path(self, key: str) -> str:
        return (self.reexport_base + "::" + key) if self.reexport_base else key

    def file_map(self) -> Dict[str, str]:
        """Every file that moves: old path -> new path, relative to src."""
        out: Dict[str, str] = {}
        for m in self.moves:
            if not m.is_dir:
                out[m.src] = m.dst
                continue
            base = os.path.join(self.src, m.src)
            if not os.path.isdir(base):
                # Already moved: read the new place.
                base = os.path.join(self.src, m.dst)
                for dirpath, _, files in os.walk(base):
                    for fn in files:
                        rel = os.path.relpath(os.path.join(dirpath, fn), self.src).replace(os.sep, "/")
                        out[m.src + rel[len(m.dst):]] = rel
                continue
            for dirpath, _, files in os.walk(base):
                for fn in files:
                    rel = os.path.relpath(os.path.join(dirpath, fn), self.src).replace(os.sep, "/")
                    out[rel] = m.dst + rel[len(m.src):]
        return dict(sorted(out.items()))

    def module_pairs(self) -> List[Tuple[str, str]]:
        """Old module path -> new, longest old first: every move, and the children of a renamed folder."""
        pairs = {m.old: m.new for m in self.moves}
        for m in self.moves:
            if m.is_dir and m.is_rename:
                for old_file, new_file in self.file_map().items():
                    if old_file.startswith(m.src + "/"):
                        old = module_of(old_file)
                        pairs.setdefault(old, self.translate(old))
        return sorted(pairs.items(), key=lambda p: -len(p[0]))

    def renames(self) -> Dict[str, str]:
        return {m.old_leaf: m.new_leaf for m in self.moves if m.is_rename and m.old_leaf != m.new_leaf}


# ---------------------------------------------------------------------------------------------------
# Reading Rust text: the inline `mod x { … }` a position is inside of (`super::` inside one names the file)

_TOKEN = re.compile(
    r"""//[^\n]*|/\*|(?<![\w])r(?P<hash>\#*)"|b?"|'(?:\\.[^']*|[^\\'])'|'\w+|\bmod\s+\w+\s*\{|[{}]""")


def inline_mod_depths(text: str) -> List[Tuple[int, int]]:
    """(position, depth) events: how many inline `mod x { … }` blocks enclose each position."""
    events: List[Tuple[int, int]] = []
    stack: List[bool] = []
    depth = 0
    pos = 0
    while True:
        m = _TOKEN.search(text, pos)
        if not m:
            break
        tok = m.group(0)
        pos = m.end()
        if tok.startswith("//"):
            continue
        if tok == "/*":
            level, i = 1, pos
            while i < len(text) and level:
                if text.startswith("/*", i):
                    level += 1
                    i += 2
                elif text.startswith("*/", i):
                    level -= 1
                    i += 2
                else:
                    i += 1
            pos = i
        elif tok.endswith('"') and (tok[0] in "rb\""):
            if tok[0] == "r":
                end = text.find('"' + m.group("hash"), pos)
                pos = len(text) if end < 0 else end + 1 + len(m.group("hash"))
            else:
                i = pos
                while i < len(text) and text[i] != '"':
                    i += 2 if text[i] == "\\" else 1
                pos = i + 1
        elif tok[0] == "'":
            continue
        elif tok == "}":
            if stack:
                if stack.pop():
                    depth -= 1
                    events.append((m.start(), depth))
        elif tok == "{":
            stack.append(False)
        else:  # `mod name {`
            stack.append(True)
            depth += 1
            events.append((m.end(), depth))
    return events


def depth_at(events: List[Tuple[int, int]], pos: int) -> int:
    depth = 0
    for at, d in events:
        if at > pos:
            break
        depth = d
    return depth


_SUPERS = re.compile(r"(?<![\w:$])((?:super::)+)")
_PUB_SUPER = re.compile(r"\bpub\(super\)")


def keep_meaning(text: str, old_file: str, new_file: str, plan: Plan, notes: Optional[List[str]] = None) -> str:
    """A file that moved: `super::` and `pub(super)` still mean what they did where the file was.

    `super::` is the file's parent module, so it stays unless the parent is not where the old one went; then
    it is the old parent's absolute path (the rewrite step names it by its new place). Inside the file's own
    inline `mod x { … }` the first `super::` is the file itself and is left alone. A `pub(super)` can only be
    pinned to an ancestor of the file's new module: when the old parent is not one (a folder that moved to
    the top, its mod.rs: `pub(super)` was `workspace`) it becomes `pub(crate)`, and `notes` says where.
    """
    old_mod, new_mod = module_of(old_file), module_of(new_file)
    events = inline_mod_depths(text)

    def supers(m: "re.Match[str]") -> str:
        count = len(m.group(1)) // len("super::")
        up = count - depth_at(events, m.start())
        if up <= 0:
            return m.group(0)
        old_anc, new_anc = ancestor(old_mod, up), ancestor(new_mod, up)
        if old_anc is None or (new_anc is not None and plan.translate(old_anc) == new_anc):
            return m.group(0)
        return crate_path(old_anc) + "::"

    out = _SUPERS.sub(supers, text)
    old_parent, new_parent = ancestor(old_mod, 1), ancestor(new_mod, 1)
    if old_parent is not None and (new_parent is None or plan.translate(old_parent) != new_parent):
        target = plan.translate(old_parent)
        pinned = not target or new_mod.startswith(target + "::")
        pin = "pub(in %s)" % crate_path(old_parent) if old_parent and pinned else "pub(crate)"

        def pub_super(m: "re.Match[str]") -> str:
            if depth_at(events, m.start()):
                return m.group(0)
            if notes is not None and old_parent and not pinned:
                notes.append("%s:%d: pub(super) is pub(crate) now (%s is no ancestor)" % (
                    new_file, text.count("\n", 0, m.start()) + 1, crate_path(old_parent)))
            return pin

        out = _PUB_SUPER.sub(pub_super, out)
    return out


def absolute_supers(text: str, file_mod: str, plan: Plan) -> str:
    """A file that did not move: a `super::` naming something that moved becomes `crate::<its parent>::`."""
    events = inline_mod_depths(text)
    moved = {old for old, _ in plan.module_pairs()}

    def involves(parent: str, name: str) -> bool:
        full = (parent + "::" + name) if parent else name
        return any(full == old or old.startswith(full + "::") or full.startswith(old + "::") for old in moved)

    def supers(m: "re.Match[str]") -> str:
        count = len(m.group(1)) // len("super::")
        up = count - depth_at(events, m.start())
        anc = ancestor(file_mod, up) if up > 0 else None
        if anc is None:
            return m.group(0)
        rest = text[m.end():]
        head = re.match(r"(\w+)", rest)
        if head:
            names = [head.group(1)]
        elif rest.startswith("{"):
            close = matching_brace(rest)
            names = [re.match(r"\s*(\w+)", p).group(1) for p in parse_pieces(rest[1:close])
                     if re.match(r"\s*(\w+)", p)]
        else:
            return m.group(0)
        return crate_path(anc) + "::" if any(involves(anc, n) for n in names) else m.group(0)

    return _SUPERS.sub(supers, text)


def matching_brace(text: str) -> int:
    """The index of the `}` closing the `{` at the start of `text`."""
    depth = 0
    for i, ch in enumerate(text):
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return i
    return len(text)


# ---------------------------------------------------------------------------------------------------
# Declarations

_DECL = re.compile(r"^((?:[ \t]*#\[[^\n]*\]\n)*)([ \t]*)((?:pub(?:\([^)]*\))? )?)mod (\w+);\n", re.M)


def take_decl(text: str, name: str) -> Tuple[str, Optional[Tuple[str, str]]]:
    """Remove `mod name;` and the attributes above it; return the text and (attributes, visibility)."""
    for m in _DECL.finditer(text):
        if m.group(4) == name:
            return text[:m.start()] + text[m.end():], (m.group(1), m.group(3))
    return text, None


def add_decls(text: str, decls: List[str]) -> str:
    """Declarations (each a full text with a newline) after the last `mod` declaration, or after the leading doc."""
    have = {m.group(4) for m in _DECL.finditer(text)}
    decls = [d for d in decls if re.search(r"mod (\w+);", d).group(1) not in have]
    if not decls:
        return text
    found = list(_DECL.finditer(text))
    if found:
        at = found[-1].end()
        return text[:at] + "".join(decls) + text[at:]
    body = text.split("\n")
    i = 0
    while i < len(body) and body[i].startswith("//!"):
        i += 1
    while i < len(body) and not body[i].strip():
        i += 1
    head = "\n".join(body[:i])
    return (head + "\n" if head else "") + "".join(decls) + ("\n" if i < len(body) else "") + "\n".join(body[i:])


# ---------------------------------------------------------------------------------------------------
# --move

def move(plan: Plan) -> int:
    pending = [m for m in plan.moves if os.path.exists(os.path.join(plan.src, m.src))]
    if not pending:
        print("app-moves: already moved, nothing to do")
        return 0
    if len(pending) != len(plan.moves):
        missing = [m.src for m in plan.moves if m not in pending]
        raise SystemExit("app-moves: half moved: %s are already in place" % ", ".join(missing))
    fmap = plan.file_map()
    # `git mv a b` into a folder that exists puts `a` inside it (b/a): refuse rather than nest.
    for m in plan.moves:
        if m.is_dir and os.path.isdir(os.path.join(plan.src, m.dst)):
            raise SystemExit("app-moves: %s already exists: `git mv` would nest %s in it" % (m.dst, m.src))

    # The declarations first (while the old files are where they were): out of the old parents.
    shims: Dict[str, List[str]] = {}
    removed: Dict[int, Tuple[str, str]] = {}
    for m in plan.moves:
        parent_file = plan.old_parent_file(m)
        path = os.path.join(plan.src, parent_file)
        text = read(path)
        text, decl = take_decl(text, m.old_leaf)
        if decl is None:
            raise SystemExit("app-moves: %s doesn't declare `mod %s;`" % (parent_file, m.old_leaf))
        write(path, text)
        removed[id(m)] = decl

    # The files.
    for m in plan.moves:
        dst = os.path.join(plan.src, m.dst)
        os.makedirs(os.path.dirname(dst), exist_ok=True)
        git(plan.root, "mv", os.path.join(plan.src, m.src), dst)
    # A visibility path cannot go through a shim (`pub(in crate::workspace::x)` must name a real ancestor),
    # so the moved files' are rewritten here, not in the next step.
    vis = re.compile(r"\bpub\(in crate::([\w:]+)\)")
    notes: List[str] = []
    for old_file, new_file in fmap.items():
        if not new_file.endswith(".rs"):
            continue
        path = os.path.join(plan.src, new_file)
        text = read(path)
        out = keep_meaning(text, old_file, new_file, plan, notes)
        out = vis.sub(lambda m: "pub(in crate::%s)" % plan.translate(m.group(1)), out)
        if out != text:
            write(path, out)

    # New parents: the ones the mapping makes up (a mod.rs of their own) and the declarations in all of them.
    new_decls: Dict[str, List[str]] = {}
    for m in plan.moves:
        attrs, vis = removed[id(m)]
        is_test = "test" in attrs
        # A module that goes deeper into the folder it was in keeps who could name it: a private one is
        # `pub(in crate::<that folder>)` where its new parent is a child of it, not `pub(crate)`.
        inside = bool(m.old_parent) and m.new.startswith(m.old_parent + "::")
        if m.is_rename:
            keep_vis = vis
        elif inside and not is_test and vis == "":
            keep_vis = "" if m.new_parent == m.old_parent else "pub(in crate::%s) " % m.old_parent
        elif m.new_parent and not is_test:
            keep_vis = "pub(crate) "
        elif not is_test:
            keep_vis = ""
        else:
            keep_vis = vis
        new_decls.setdefault(plan.root_file(m.new_parent), []).append("%smod %s;\n" % (attrs + keep_vis, m.new_leaf))
        if not is_test:
            alias = "" if m.new_leaf == m.old_leaf else " as %s" % m.old_leaf
            if inside and vis == "":
                keyword = keep_vis + "use"
            else:
                keyword = "pub(crate) use" if m.old_parent else "use"
            shims.setdefault(plan.old_parent_file(m), []).append(
                "%s%s crate::%s%s;\n" % (attrs, keyword, m.new, alias))
    for name, parent in sorted(plan.parents.items(), key=lambda p: len(p[0])):
        path = os.path.join(plan.src, name.replace("::", "/"), "mod.rs")
        os.makedirs(os.path.dirname(path), exist_ok=True)
        doc = ["//! " + line if line else "//!" for line in parent["doc"]]
        decls = sorted(new_decls.pop(plan.root_file(name), []))
        write(path, "\n".join(doc + [""]) + "\n" + "".join(decls))
        git(plan.root, "add", path)
        grandparent, _, leaf = name.rpartition("::")
        keyword = "pub(crate) " if grandparent else ""
        new_decls.setdefault(plan.root_file(grandparent), []).append("%smod %s;\n" % (keyword, leaf))
    for file, decls in sorted(new_decls.items()):
        path = os.path.join(plan.src, file)
        write(path, add_decls(read(path), sorted(decls)))

    for fix in plan.fixups:
        path = os.path.join(plan.src, fix["file"])
        text = read(path)
        if fix["old"] not in text:
            raise SystemExit("app-moves: %s: the fixup's text is not there: %s" % (fix["file"], fix["old"]))
        write(path, text.replace(fix["old"], fix["new"], 1))

    # The shims: one block at the end of the old parents' module declarations.
    for file, lines in sorted(shims.items()):
        path = os.path.join(plan.src, file)
        text = read(path)
        block = SHIM_MARK + ": the next commit names every module by its new path and removes them.\n" \
            + "".join(sorted(lines))
        decls = list(_DECL.finditer(text))
        at = decls[-1].end() if decls else 0
        text = text[:at] + "\n" + block + text[at:]
        write(path, text)
    print("app-moves: moved %d entries (%d files); shims in %s" % (
        len(plan.moves), len(fmap), ", ".join(sorted(shims))))
    for note in notes:
        print("app-moves: widened: " + note)
    restructure(plan, fmap)
    return 0


def restructure(plan: Plan, fmap: Dict[str, str]) -> None:
    """scripts/structure.json: the layers, the config's paths, and the allowlist keys that name a moved file."""
    path = os.path.join(plan.root, "scripts", "structure.json")
    data = json.loads(read(path))
    crate = os.path.basename(plan.crate_dir)
    renames = plan.renames()
    # One spec, or a list of them applied in turn (a series that puts modules at two places of the order).
    specs = plan.layers if isinstance(plan.layers, list) else [plan.layers]

    def names(tier) -> List[str]:
        return tier if isinstance(tier, list) else [tier]

    out = data["config"]["layers"][crate]
    for spec in specs:
        drop = set(spec.get("remove", []))
        kept = []
        for tier in out:
            keep = [renames.get(n, n) for n in names(tier) if n not in drop]
            if keep:
                kept.append(keep if isinstance(tier, list) and len(keep) > 1 else keep[0])
        out = kept
        present = {n for tier in out for n in names(tier)}
        if "after" in spec:
            at = next(i for i, tier in enumerate(out) if spec["after"] in names(tier)) + 1
            for tier in spec.get("insert", []):
                if not all(n in present for n in names(tier)):
                    out.insert(at, tier)
                    at += 1
    data["config"]["layers"][crate] = out

    prefix = plan.crate_dir + "/src/"
    for m in plan.moves:
        if m.is_dir and m.is_rename:
            data["config"] = rename_paths(data["config"], m.src, m.dst)
    renamed = 0
    for rule, entries in data["allowlist"].items():
        moved = {}
        for key, n in entries.items():
            head, sep, rest = key.partition(" :: ")
            if head.startswith(prefix) and head[len(prefix):] in fmap:
                head = prefix + fmap[head[len(prefix):]]
                renamed += 1
            elif head.startswith(crate + ": "):
                # A module pair (`wormhole: a -> b`) names layers.
                sides = head[len(crate) + 2:].split(" -> ")
                head = crate + ": " + " -> ".join(
                    re.sub(r"^\w+", lambda w: renames.get(w.group(0), w.group(0)), side) for side in sides)
            moved[head + sep + rest] = n
        data["allowlist"][rule] = dict(sorted(moved.items()))
    write(path, json.dumps(data, indent=2, ensure_ascii=False) + "\n")
    print("app-moves: structure.json: layers, %d allowlist keys moved with their files" % renamed)


def rename_paths(value, old: str, new: str):
    """`src/<old>` and `src/<old>/…` in the strings of the structure checker's config."""
    if isinstance(value, str):
        return re.sub(r"(?<![\w/])((?:crates/[\w-]+/)?src/)%s(?=/|$)" % re.escape(old),
                      lambda m: m.group(1) + new, value)
    if isinstance(value, list):
        return [rename_paths(v, old, new) for v in value]
    if isinstance(value, dict):
        return {k: rename_paths(v, old, new) for k, v in value.items()}
    return value


# ---------------------------------------------------------------------------------------------------
# --rewrite

def rewrite(plan: Plan) -> int:
    fmap = plan.file_map()
    if not any(os.path.exists(os.path.join(plan.src, m.dst)) for m in plan.moves):
        raise SystemExit("run --move first")
    pairs = plan.module_pairs()
    new_of = dict(pairs)
    old_alt = "|".join(re.escape(old) for old, _ in pairs)
    by_parent: Dict[str, Dict[str, Move]] = {}
    for m in plan.moves:
        by_parent.setdefault(m.old_parent, {})[m.old_leaf] = m
    leaf_renames = {m.old_leaf: m.new_leaf for m in plan.moves if m.old_leaf != m.new_leaf}
    # Names that only the moved modules were called by, so a backticked one in a comment is the module.
    prose_leaves = plan.prose_leaves if plan.prose_leaves is not None else \
        [leaf for leaf in leaf_renames if "_" in leaf]
    prose = {leaf: next(m.new for m in plan.moves if m.old_leaf == leaf) for leaf in prose_leaves
             if any(m.old_leaf == leaf for m in plan.moves)}
    reexports = {plan.reexport_path(k): v for k, v in plan.reexports.items()}

    full = re.compile(r"(?<![\w$])crate::(%s)\b" % old_alt)
    reexp = re.compile(r"(?<![\w$])crate::(%s)\b" % "|".join(map(re.escape, reexports))) if reexports else None
    bare = re.compile(r"(?<![\w:.\"$])(%s)(?=::)" % old_alt)
    moved_new_paths = set(fmap.values())
    changed = 0
    for rel in tracked(plan.root, plan.crate_dir + "/*.rs"):
        path = os.path.join(plan.root, rel)
        text = read(path)
        out = text
        rel_src = os.path.relpath(path, plan.src).replace(os.sep, "/") if rel.startswith(
            plan.crate_dir + "/src/") else None
        rebound: set = set()
        if rel_src is not None and rel_src not in moved_new_paths:
            out = absolute_supers(out, module_of(rel_src), plan)
        out = split_groups(out, by_parent, plan, reexports, rebound)
        # `use crate::workspace::x;` a file imports itself: it calls it by the new name.
        for parent, leaves in by_parent.items():
            lead = "crate::%s::" % parent if parent else "crate::"
            for m in re.finditer(r"(?m)^[ \t]*(?:pub(?:\([^)]*\))? )?use %s(%s);$" % (
                    re.escape(lead), "|".join(map(re.escape, leaves))), out):
                mod = leaves[m.group(1)]
                if mod.old_leaf != mod.new_leaf:
                    rebound.add(mod.old_leaf)
        # `use crate::workspace::x::{self, y};`: `x` is then called by the new name too.
        for old, mod in ((m.old, m) for m in plan.moves if m.old_leaf != m.new_leaf):
            if re.search(r"(?m)^[ \t]*(?:pub(?:\([^)]*\))? )?use crate::%s::\{[^;{}]*\bself\b(?!\s+as\b)" %
                         re.escape(old), out):
                rebound.add(mod.old_leaf)
        if reexp:
            out = reexp.sub(lambda m: "crate::" + reexports[m.group(1)], out)
        out = full.sub(lambda m: "crate::" + new_of[m.group(1)], out)
        if rel_src is not None:
            # The old parents' own files: `self::<moved>` and the re-exports of moved things.
            for parent, leaves in by_parent.items():
                if rel_src != plan.root_file(parent):
                    continue
                out = drop_shims(out, leaves)
                names = {leaf: "crate::" + m.new for leaf, m in leaves.items()}
                names.update({leaf: "crate::" + new for leaf, new in reexported_leaves(parent, reexports).items()})
                out = re.sub(r"\bself::(%s)\b" % "|".join(map(re.escape, names)),
                             lambda m: names[m.group(1)], out)
        if rel_src == "main.rs":
            out = bare.sub(lambda m: new_of[m.group(1)], out)
            if reexports:
                out = re.sub(r"(?<![\w:.\"$])(%s)\b" % "|".join(map(re.escape, reexports)),
                             lambda m: reexports[m.group(1)], out)
        else:
            out = prose_paths(out, pairs)
        if prose:
            out = re.sub(r"`(%s)(?=::|`)" % "|".join(sorted(map(re.escape, prose), key=len, reverse=True)),
                         lambda m: "`" + prose[m.group(1)], out)
        for old_leaf in sorted(rebound):
            out = re.sub(r"(?<![\w:])%s(\s*::)" % re.escape(old_leaf), leaf_renames[old_leaf] + r"\1", out)
        out = file_paths(out, plan)
        if out != text:
            write(path, out)
            changed += 1
    # The other tracked text files learn the files' new places.
    keep = {MAPPING, "scripts/app-moves.py", "scripts/structure.json", "CHANGELOG.md"}
    for rel in tracked(plan.root):
        if rel.endswith(".rs") or rel in keep or not os.path.isfile(os.path.join(plan.root, rel)):
            continue
        path = os.path.join(plan.root, rel)
        try:
            text = read(path)
        except (UnicodeDecodeError, ValueError):
            continue
        out = file_paths(text, plan)
        if out != text:
            write(path, out)
            changed += 1
    print("app-moves: rewrote paths in %d files; shims and the moved things' re-exports removed. "
          "Now: cargo fmt --all" % changed)
    return 0


def prose_paths(text: str, pairs: List[Tuple[str, str]]) -> str:
    """`workspace::app_menu` in a comment line: a module that moved, called by where it is now.

    Only multi-segment old paths, and only in `//` comment lines: code is rewritten by its `crate::` paths, and
    `workspace::ToggleSidebar` (an action's name) or a string that merely says `workspace` stays.
    """
    deep = [(old, new) for old, new in pairs if "::" in old]
    if not deep:
        return text
    pattern = re.compile(r"(?<![\w:.\"$])(%s)\b" % "|".join(re.escape(old) for old, _ in deep))
    new_of = dict(deep)
    out = []
    for line in text.split("\n"):
        if line.lstrip().startswith("//"):
            line = pattern.sub(lambda m: new_of[m.group(1)], line)
        out.append(line)
    return "\n".join(out)


def file_paths(text: str, plan: Plan) -> str:
    """`workspace/app_menu.rs` and `src/workspace/app_menu.rs` -> where it is now, longest first."""
    pairs: List[Tuple[str, str]] = []
    for old, new in plan.file_map().items():
        pairs.append((old, new))
    # A folder named by itself (`workspace/quick_panel/`).
    for m in plan.moves:
        if m.is_dir:
            pairs.append((m.src, m.dst))
    for old, new in sorted(pairs, key=lambda p: -len(p[0])):
        # `workspace/x.rs` is clear by itself; a top-level path is only named with `src/` before it.
        prefix = r"((?:crates/wormhole/)?src/)?" if "/" in old else r"((?:crates/wormhole/)?src/)"
        text = re.sub(r"(?<![\w/])%s%s(?=[/\s`'\")\],:.;]|$)" % (prefix, re.escape(old)),
                      lambda m: (m.group(1) or "") + new, text)
    return text


def drop_shims(text: str, leaves: Dict[str, Move]) -> str:
    """The shim block, and the `pub(crate) use <moved>::...;` lines that re-export a moved thing."""
    text = re.sub(r"\n%s[^\n]*\n(?:(?:[ \t]*#\[[^\n]*\]\n)*(?:pub(?:\([^)]*\))? )?use crate::[^\n]*;\n)*" %
                  re.escape(SHIM_MARK), "\n", text)
    moved = "|".join(re.escape(leaf) for leaf in leaves)
    return re.sub(r"(?m)^(?:[ \t]*#\[[^\n]*\]\n)*pub\(crate\) use (?:self::)?(?:%s)::[^;]*;\n" % moved, "", text)


def parse_pieces(body: str) -> List[str]:
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
    return [p for p in pieces if p.strip()]


def reexported_leaves(parent: str, reexports: Dict[str, str]) -> Dict[str, str]:
    """The re-exports `parent` has (name -> where the thing is now): `crate::workspace::init_actions` is one."""
    lead = parent + "::" if parent else ""
    return {key[len(lead):]: new for key, new in reexports.items()
            if key.startswith(lead) and "::" not in key[len(lead):]}


def split_groups(text: str, by_parent: Dict[str, Dict[str, Move]], plan: Plan, reexports: Dict[str, str],
                 rebound: set) -> str:
    """`use crate::workspace::{a, moved, b::x};` keeps a and b::x; the moved name gets a line of its own."""
    for parent, by_leaf in by_parent.items():
        lead = "crate::%s::" % parent if parent else "crate::"
        start = re.compile(r"(?m)^((?:[ \t]*#\[[^\n]*\]\n)*)([ \t]*)((?:pub(?:\([^)]*\))? )?)use %s\{" % re.escape(lead))
        text = split_group_of(text, start, lead, by_leaf, reexported_leaves(parent, reexports), rebound)
    return text


def split_group_of(text: str, group_start: "re.Pattern[str]", lead: str, by_leaf: Dict[str, Move],
                   reexported: Dict[str, str], rebound: set) -> str:
    out, pos = [], 0
    for m in group_start.finditer(text):
        if m.start() < pos:
            continue
        open_at = m.end() - 1
        depth, i = 0, open_at
        while i < len(text):
            if text[i] == "{":
                depth += 1
            elif text[i] == "}":
                depth -= 1
                if depth == 0:
                    break
            i += 1
        semi = text.index(";", i)
        pieces = parse_pieces(text[open_at + 1:i])
        stay, lifted, lines_extra = [], [], []
        attrs, indent, vis = m.group(1), m.group(2), m.group(3)
        for p in pieces:
            head = re.match(r"\s*(\w+)", p)
            leaf = head.group(1) if head else ""
            if leaf in by_leaf:
                mod = by_leaf[leaf]
                lifted.append((mod, p.strip()[len(leaf):]))
            elif leaf in reexported and not p.strip()[len(leaf):].lstrip().startswith("::"):
                lines_extra.append("%s%s%suse crate::%s%s;" % (attrs, indent, vis, reexported[leaf],
                                                               p.strip()[len(leaf):]))
            else:
                stay.append(p.strip())
        if not lifted and not lines_extra:
            continue
        lines = []
        if stay:
            lines.append("%s%s%suse %s{%s};" % (attrs, indent, vis, lead, ", ".join(stay)))
        lines.extend(lines_extra)
        # Lifted names with nothing after them share a line by new parent.
        plain: Dict[str, List[str]] = {}
        for mod, rest in lifted:
            if mod.old_leaf != mod.new_leaf and (not rest.strip() or re.search(r"\bself\b", rest)):
                rebound.add(mod.old_leaf)
            if rest.strip() == "" or rest.strip().startswith("as "):
                plain.setdefault(mod.new_parent, []).append(mod.new_leaf + rest)
            else:
                lines.append("%s%s%suse crate::%s%s;" % (attrs, indent, vis, mod.new, rest))
        for parent, leaves in plain.items():
            body = leaves[0] if len(leaves) == 1 else "{%s}" % ", ".join(sorted(leaves))
            lines.append("%s%s%suse crate::%s::%s;" % (attrs, indent, vis, parent, body) if parent
                         else "%s%s%suse crate::%s;" % (attrs, indent, vis, body))
        out.append(text[pos:m.start()] + "\n".join(lines))
        pos = semi + 1
    out.append(text[pos:])
    return "".join(out)


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="Move the app's modules out of workspace/",
                                     formatter_class=argparse.RawDescriptionHelpFormatter, epilog=__doc__)
    parser.add_argument("--root", default=REPO, help="the repository (default: this one)")
    parser.add_argument("--series", default="1", help="which series of scripts/app-moves.json (default: 1)")
    step = parser.add_mutually_exclusive_group(required=True)
    step.add_argument("--move", action="store_true", help="commit A: the moves and the shims")
    step.add_argument("--rewrite", action="store_true", help="commit B: every path by its new place, no shims")
    args = parser.parse_args(argv)
    plan = Plan(os.path.abspath(args.root), args.series)
    return move(plan) if args.move else rewrite(plan)


if __name__ == "__main__":
    sys.exit(main())
