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


class Move:
    """One file or folder: `src` and `dst` are relative to the app crate's src/, as paths."""

    def __init__(self, src: str, dst: str, is_dir: bool) -> None:
        self.src, self.dst, self.is_dir = src, dst, is_dir

    @staticmethod
    def module_of(path: str) -> str:
        """`workspace/app_menu.rs` -> `workspace::app_menu`, `a/b/mod.rs` -> `a::b`."""
        if path.endswith("/mod.rs"):
            path = path[:-len("/mod.rs")]
        elif path.endswith(".rs"):
            path = path[:-3]
        return path.replace("/", "::")

    @property
    def old(self) -> str:
        return self.module_of(self.src)

    @property
    def new(self) -> str:
        return self.module_of(self.dst)

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
        self.layers: dict = spec.get("layers", {})
        self.fixups: List[dict] = spec.get("fixups", [])
        self.moves: List[Move] = []
        for m in spec["moves"]:
            is_dir = not m["from"].endswith(".rs")
            self.moves.append(Move(m["from"], m["to"], is_dir))

    def root_file(self, parent: str) -> str:
        """The file that declares the children of a module (relative to src): the crate root, or its mod.rs."""
        if not parent:
            return "main.rs"
        return parent.replace("::", "/") + "/mod.rs"

    def old_parent_file(self, m: Move) -> str:
        if m.old_parent == "workspace":
            return "workspace/mod.rs"
        return self.root_file(m.old_parent)

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

    # The declarations first (while the old files are where they were): out of the old parents.
    shims: List[str] = []
    removed: Dict[str, List[Tuple[Move, Optional[Tuple[str, str]]]]] = {}
    for m in plan.moves:
        parent_file = plan.old_parent_file(m)
        path = os.path.join(plan.src, parent_file)
        text = read(path)
        text, decl = take_decl(text, m.old_leaf)
        if decl is None:
            raise SystemExit("app-moves: %s doesn't declare `mod %s;`" % (parent_file, m.old_leaf))
        write(path, text)
        removed.setdefault(parent_file, []).append((m, decl))

    # The files.
    for m in plan.moves:
        dst = os.path.join(plan.src, m.dst)
        os.makedirs(os.path.dirname(dst), exist_ok=True)
        git(plan.root, "mv", os.path.join(plan.src, m.src), dst)

    # New parents: the ones the mapping makes up (a mod.rs of their own) and the declarations in all of them.
    new_decls: Dict[str, List[str]] = {}
    for m in plan.moves:
        attrs, vis = next(d for mm, d in removed[plan.old_parent_file(m)] if mm is m)
        is_test = "test" in attrs
        keep_vis = "pub(crate) " if (not is_test and m.new_parent) else vis
        if not m.new_parent and not is_test:
            keep_vis = ""
        new_decls.setdefault(plan.root_file(m.new_parent), []).append("%smod %s;\n" % (attrs + keep_vis, m.new_leaf))
        if m.old_parent == "workspace" and not is_test:
            shims.append("%spub(crate) use crate::%s%s;\n" % (
                attrs, m.new, "" if m.new_leaf == m.old_leaf else " as %s" % m.old_leaf))
        elif m.old_parent == "" and not is_test:
            shims.append("%suse crate::%s%s;\n" % (
                attrs, m.new, "" if m.new_leaf == m.old_leaf else " as %s" % m.old_leaf))
    for name, parent in plan.parents.items():
        path = os.path.join(plan.src, name, "mod.rs")
        doc = ["//! " + line if line else "//!" for line in parent["doc"]]
        decls = sorted(new_decls.pop(plan.root_file(name), []))
        write(path, "\n".join(doc + [""]) + "\n" + "".join(decls))
        git(plan.root, "add", path)
        new_decls.setdefault("main.rs", []).append("mod %s;\n" % name)
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
    by_file: Dict[str, List[str]] = {}
    for m in plan.moves:
        file = plan.old_parent_file(m)
        for s in shims:
            if re.search(r"use crate::%s(?: as \w+)?;" % re.escape(m.new), s):
                by_file.setdefault(file, []).append(s)
    for file, lines in sorted(by_file.items()):
        path = os.path.join(plan.src, file)
        text = read(path)
        block = SHIM_MARK + ": the next commit names every module by its new path and removes them.\n" \
            + "".join(sorted(lines))
        decls = list(_DECL.finditer(text))
        at = decls[-1].end() if decls else 0
        text = text[:at] + "\n" + block + text[at:]
        write(path, text)
    print("app-moves: moved %d entries (%d files); shims in %s" % (
        len(plan.moves), len(fmap), ", ".join(sorted(by_file))))
    restructure(plan, fmap)
    return 0


def restructure(plan: Plan, fmap: Dict[str, str]) -> None:
    """scripts/structure.json: the layers, and the allowlist keys that name a moved file."""
    path = os.path.join(plan.root, "scripts", "structure.json")
    data = json.loads(read(path))
    crate = os.path.basename(plan.crate_dir)
    layers = data["config"]["layers"][crate]
    spec = plan.layers

    def names(tier) -> List[str]:
        return tier if isinstance(tier, list) else [tier]

    drop = set(spec.get("remove", []))
    out = []
    for tier in layers:
        keep = [n for n in names(tier) if n not in drop]
        if keep:
            out.append(keep if isinstance(tier, list) and len(keep) > 1 else keep[0])
    present = {n for tier in out for n in names(tier)}
    at = next(i for i, tier in enumerate(out) if spec["after"] in names(tier)) + 1
    for tier in spec.get("insert", []):
        if not all(n in present for n in names(tier)):
            out.insert(at, tier)
            at += 1
    data["config"]["layers"][crate] = out
    prefix = plan.crate_dir + "/src/"
    renamed = 0
    for rule, entries in data["allowlist"].items():
        moved = {}
        for key, n in entries.items():
            head, sep, rest = key.partition(" :: ")
            if head.startswith(prefix) and head[len(prefix):] in fmap:
                head = prefix + fmap[head[len(prefix):]]
                renamed += 1
            moved[head + sep + rest] = n
        data["allowlist"][rule] = dict(sorted(moved.items()))
    write(path, json.dumps(data, indent=2, ensure_ascii=False) + "\n")
    print("app-moves: structure.json: layers, %d allowlist keys moved with their files" % renamed)


# ---------------------------------------------------------------------------------------------------
# --rewrite

def rewrite(plan: Plan) -> int:
    ws_mod = read(os.path.join(plan.src, "workspace", "mod.rs"))
    if SHIM_MARK not in ws_mod and not any(os.path.exists(os.path.join(plan.src, m.dst)) for m in plan.moves):
        raise SystemExit("run --move first")
    moved = {m.old: m for m in plan.moves if m.old_parent == "workspace"}
    top = {m.old: m for m in plan.moves if m.old_parent == ""}
    names = sorted(moved, key=len, reverse=True)
    alt = "|".join(re.escape(n.partition("::")[2]) for n in names)
    leaf_renames = {m.old_leaf: m.new_leaf for m in list(moved.values()) + list(top.values())
                    if m.old_leaf != m.new_leaf}
    by_leaf = {m.old_leaf: m for m in moved.values()}
    # Names that only the moved modules were called by, so a backticked one in a comment is the module.
    prose = {leaf: m.new for leaf, m in by_leaf.items() if leaf.startswith("app_")}

    full = re.compile(r"(?<![\w$])crate::workspace::(%s)\b" % alt)
    reexp = re.compile(r"(?<![\w$])crate::workspace::(%s)\b" % "|".join(plan.reexports))
    group_start = re.compile(r"(?m)^((?:[ \t]*#\[[^\n]*\]\n)*)([ \t]*)((?:pub(?:\([^)]*\))? )?)use crate::workspace::\{")
    changed = 0
    app_src = os.path.join(plan.root, plan.crate_dir) + os.sep
    for rel in tracked(plan.root, plan.crate_dir + "/*.rs"):
        path = os.path.join(plan.root, rel)
        text = read(path)
        out = text
        rebound: set = set()
        out = split_groups(out, group_start, by_leaf, plan, rebound)
        # `use crate::workspace::x;` a file imports itself: it calls it by the new name.
        for m in re.finditer(r"(?m)^[ \t]*(?:pub(?:\([^)]*\))? )?use crate::workspace::(%s);$" % alt, out):
            mod = by_leaf[m.group(1)]
            if mod.old_leaf != mod.new_leaf:
                rebound.add(mod.old_leaf)
        out = reexp.sub(lambda m: "crate::" + plan.reexports[m.group(1)], out)
        out = full.sub(lambda m: "crate::" + by_leaf[m.group(1)].new, out)
        rel_src = os.path.relpath(path, os.path.join(plan.root, plan.crate_dir, "src")).replace(os.sep, "/")
        if rel_src == "workspace/mod.rs":
            out = drop_shims(out, plan)
            out = re.sub(r"\bself::(%s)\b" % alt, lambda m: "crate::" + by_leaf[m.group(1)].new, out)
            out = re.sub(r"\bself::(%s)\b" % "|".join(plan.reexports), lambda m: "crate::" + plan.reexports[m.group(1)], out)
        elif os.path.dirname(rel_src) == "workspace":
            out = re.sub(r"(?<![\w:])super::(%s)\b" % alt, lambda m: "crate::" + by_leaf[m.group(1)].new, out)
        if rel_src == "main.rs":
            out = drop_root_shims(out, plan)
            out = re.sub(r"(?<![\w:])workspace::(%s)\b" % "|".join(plan.reexports),
                         lambda m: plan.reexports[m.group(1)], out)
            out = re.sub(r"(?<![\w:])workspace::(%s)\b" % alt, lambda m: by_leaf[m.group(1)].new, out)
            out = re.sub(r"(?<![\w:])(%s)::" % "|".join(top_leaves(top)), lambda m: top_new(top, m.group(1)) + "::", out)
        out = re.sub(r"(?<![\w$])crate::(%s)::" % "|".join(top_leaves(top)),
                     lambda m: "crate::" + top_new(top, m.group(1)) + "::", out) if top else out
        # Prose: `workspace::default_terminal` and the renamed modules in backticks (`app_tick::x`).
        out = re.sub(r"(?<![\w:])workspace::(%s)\b" % alt, lambda m: by_leaf[m.group(1)].new, out)
        out = re.sub(r"`(%s)(?=::|`)" % "|".join(sorted(prose, key=len, reverse=True)),
                     lambda m: "`" + prose[m.group(1)], out)
        for old_leaf in sorted(rebound):
            out = re.sub(r"(?<![\w:])%s(\s*::)" % re.escape(old_leaf), leaf_renames[old_leaf] + r"\1", out)
        out = file_paths(out, plan)
        if out != text:
            write(path, out)
            changed += 1
    # The shims go (mod.rs above), and the other tracked text files learn the files' new places.
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
    restructure_keys(plan)
    print("app-moves: rewrote paths in %d files; shims and the moved things' re-exports removed. "
          "Now: cargo fmt --all" % changed)
    return 0


def top_leaves(top: Dict[str, Move]) -> List[str]:
    return sorted(top, key=len, reverse=True) or ["\0"]


def top_new(top: Dict[str, Move], leaf: str) -> str:
    return top[leaf].new


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
        # `workspace/x.rs` is clear by itself; a top-level file is only named with its whole path.
        prefix = r"(crates/wormhole/src/)?" if old.startswith("workspace/") else r"(crates/wormhole/src/)"
        text = re.sub(r"(?<![\w/])%s%s(?=[/\s`'\")\],:.;]|$)" % (prefix, re.escape(old)),
                      lambda m: (m.group(1) or "") + new, text)
    return text


def drop_shims(text: str, plan: Plan) -> str:
    """The shim block, and the `pub(crate) use self::<moved>::...;` lines that re-export a moved thing."""
    text = re.sub(r"\n%s[^\n]*\n(?:(?:[ \t]*#\[[^\n]*\]\n)*pub\(crate\) use crate::[^\n]*;\n)*" % re.escape(SHIM_MARK), "\n", text)
    moved = "|".join(re.escape(m.old_leaf) for m in plan.moves if m.old_parent == "workspace")
    return re.sub(r"(?m)^(?:[ \t]*#\[[^\n]*\]\n)*pub\(crate\) use self::(?:%s)::[^;]*;\n" % moved, "", text)


def drop_root_shims(text: str, plan: Plan) -> str:
    return re.sub(r"\n%s[^\n]*\n(?:(?:[ \t]*#\[[^\n]*\]\n)*use crate::[^\n]*;\n)*" % re.escape(SHIM_MARK), "\n", text)


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


def split_groups(text: str, group_start: "re.Pattern[str]", by_leaf: Dict[str, Move], plan: Plan,
                 rebound: set) -> str:
    """`use crate::workspace::{a, moved, b::x};` keeps a and b::x; the moved name gets a line of its own."""
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
        stay, lifted = [], []
        for p in pieces:
            head = re.match(r"\s*(\w+)", p)
            leaf = head.group(1) if head else ""
            if leaf in by_leaf:
                mod = by_leaf[leaf]
                rest = p.strip()[len(leaf):]
                lifted.append((mod, rest))
            else:
                stay.append(p.strip())
        if not lifted:
            continue
        attrs, indent, vis = m.group(1), m.group(2), m.group(3)
        lines = []
        if stay:
            lines.append("%s%s%suse crate::workspace::{%s};" % (attrs, indent, vis, ", ".join(stay)))
        # Lifted names with nothing after them share a line by new parent.
        plain: Dict[str, List[str]] = {}
        for mod, rest in lifted:
            if mod.old_leaf != mod.new_leaf and not rest.strip():
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


def restructure_keys(plan: Plan) -> None:
    pass


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
