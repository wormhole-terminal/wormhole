#!/usr/bin/env python3
"""Group core's modules by domain (phase 3b of the app-structure plan), in two scripted steps.

  scripts/core-domains.py --move      commit A: `git mv` every module to its domain (scripts/core-domains.json),
                                      the new domains' mod.rs files, lib.rs as one `pub mod` per domain plus
                                      shims that keep every old path working for one commit, child modules
                                      declared in their new parents, relative include_str!/include_bytes!
                                      paths and `pub(in crate::old)` recomputed, and scripts/structure.json's
                                      layers, module depth and allowlist keys moved with the files
  scripts/core-domains.py --rewrite   commit B: every path names the module by its domain (`crate::old` in
                                      core, `wormhole_core::old` everywhere else, in code, comments and doc
                                      links), renamed modules rebound where a file imports them, the shims
                                      removed, and the paths in every other tracked text file (docs,
                                      .claude/, scripts) but the changelog

Both are deterministic, and idempotent: a step that has run already finds nothing to do. Neither runs
cargo: after each step run `cargo fmt --all` (it sorts the module declarations the move adds), after
`--move` also `python3 scripts/structure.py update` (entries the move made stale), then the gate. The mapping (`scripts/core-domains.json`)
has one entry per old top-level module: `old` (its name), `new` (its path under its domain) and `path`
(its root file afterwards, which the script checks it computes the same), with `vis` when the module
isn't `pub`. Files are only ever moved with `git mv`; nothing is deleted.

To redo it after a rebase: drop the two commits, rebase, run `--move` (+ fmt, update), commit, run
`--rewrite` (+ fmt), redo the hand fixes B's message lists, commit.
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
MAPPING = os.path.join("scripts", "core-domains.json")
SHIM_MARK = "// Old paths, for one commit only"
DOMAINS_MARK = "//! Its modules are grouped by domain"


class Plan:
    def __init__(self, root: str) -> None:
        self.root = root
        with open(os.path.join(root, MAPPING), encoding="utf-8") as f:
            data = json.load(f)
        self.crate_dir = data["crate"]
        self.src = os.path.join(root, self.crate_dir, "src")
        self.domains: List[dict] = data["domains"]
        self.modules: List[dict] = data["modules"]
        self.data: List[dict] = data["data"]
        self.by_old = {m["old"]: m for m in self.modules}
        names = [d["name"] for d in self.domains]
        for m in self.modules:
            if m["new"].split("::")[0] not in names:
                raise SystemExit("%s: %s is in no domain" % (MAPPING, m["new"]))
        if len(self.by_old) != len(self.modules):
            raise SystemExit("%s: a module is listed twice" % MAPPING)

    # -- where things are ------------------------------------------------------------------------

    def is_parent(self, new: str) -> bool:
        return any(d["name"] == new for d in self.domains) or any(
            m["new"].startswith(new + "::") for m in self.modules)

    def new_root_file(self, m: dict) -> str:
        """The module's root file afterwards, relative to src."""
        parts = m["new"].split("::")
        # A folder module, a file with a folder of children, or a module that gets children: mod.rs.
        if os.path.isdir(os.path.join(self.src, m["old"])) or self.is_parent(m["new"]):
            return "/".join(parts) + "/mod.rs"
        return "/".join(parts[:-1] + [parts[-1] + ".rs"])

    def old_layout(self, m: dict) -> Tuple[Optional[str], Optional[str]]:
        """(the module's .rs file, its folder), relative to src, as they are before the move."""
        f = m["old"] + ".rs"
        d = m["old"]
        return (f if os.path.isfile(os.path.join(self.src, f)) else None,
                d if os.path.isdir(os.path.join(self.src, d)) else None)

    def vis(self, m: dict, during_move: bool) -> str:
        v = m.get("vis_during_move", m.get("vis", "pub")) if during_move else m.get("vis", "pub")
        return (v + " ") if v else ""

    def file_map(self) -> Dict[str, str]:
        """Every file of a moved module or data folder: old path -> new path, relative to src."""
        out: Dict[str, str] = {}
        for m in self.modules:
            f, d = self.old_layout(m)
            new_root = self.new_root_file(m)
            new_dir = os.path.dirname(new_root) if new_root.endswith("/mod.rs") else None
            if f:
                out[f] = new_root
            if d:
                for dirpath, _, files in os.walk(os.path.join(self.src, d)):
                    for fn in files:
                        rel = os.path.relpath(os.path.join(dirpath, fn), self.src).replace(os.sep, "/")
                        if rel == d + "/mod.rs":
                            out[rel] = new_root
                        else:
                            if new_dir is None:
                                raise SystemExit("%s has a folder but moves to a single file" % m["old"])
                            out[rel] = new_dir + rel[len(d):]
        for entry in self.data:
            old, new = entry["old"].rstrip("/"), entry["new"].rstrip("/")
            for dirpath, _, files in os.walk(os.path.join(self.src, old)):
                for fn in files:
                    rel = os.path.relpath(os.path.join(dirpath, fn), self.src).replace(os.sep, "/")
                    out[rel] = new + rel[len(old):]
        return {k: v for k, v in sorted(out.items())}


def git(root: str, *args: str) -> None:
    subprocess.run(["git", "-C", root] + list(args), check=True)


def read(path: str) -> str:
    with open(path, encoding="utf-8") as f:
        return f.read()


def write(path: str, text: str) -> None:
    with open(path, "w", encoding="utf-8") as f:
        f.write(text)


# ---------------------------------------------------------------------------------------------------
# --move

def lib_text(plan: Plan, old_lib: str, shims: bool) -> str:
    """lib.rs: its own doc, the domains in order, one `pub mod` each, and (for the move's commit) the shims."""
    doc = []
    for line in old_lib.splitlines():
        if not line.startswith("//!") or line.startswith(DOMAINS_MARK):
            break
        doc.append(line)
    while doc and doc[-1].strip() == "//!":
        doc.pop()
    tiers: Dict[int, List[str]] = {}
    for d in plan.domains:
        tiers.setdefault(d["tier"], []).append(d["name"])
    order = "; ".join(", ".join("`%s`" % n for n in tiers[t]) for t in sorted(tiers))
    doc += [
        "//!",
        DOMAINS_MARK + ", lowest first; a domain uses only the ones",
        "//! before it (`scripts/structure.json`'s `layers` checks it):",
    ]
    words, line = order.split(" "), "//!"
    for w in words:
        if len(line) + 1 + len(w) > 110:
            doc.append(line)
            line = "//!"
        line += " " + w
    doc.append(line + ".")
    out = doc + [""] + ["pub mod %s;" % n for n in sorted(d["name"] for d in plan.domains)]
    if shims:
        out += ["", SHIM_MARK + ": the next commit names every module by its domain and removes them."]
        for m in sorted(plan.modules, key=lambda m: m["new"]):
            if m["new"] == m["old"]:
                continue
            leaf = m["new"].split("::")[-1]
            alias = "" if leaf == m["old"] else " as %s" % m["old"]
            if m.get("vis", "pub") == "pub":
                out += ["#[doc(hidden)]", "pub use %s%s;" % (m["new"], alias)]
            else:
                out += ["use %s%s;" % (m["new"], alias)]
    return "\n".join(out) + "\n"


_MOD_DECL = re.compile(r"^(?:pub(?:\([^)]*\))? )?mod (\w+);$", re.M)


def declare_children(path: str, children: List[Tuple[str, str]]) -> bool:
    """Add `<vis>mod <name>;` for each child a parent file doesn't declare yet, after its last module
    declaration (or after its leading doc when it has none). True when the file changed."""
    text = read(path)
    have = {m.group(1) for m in _MOD_DECL.finditer(text)}
    lines = ["%smod %s;" % (vis, name) for vis, name in sorted(children, key=lambda c: c[1]) if name not in have]
    if not lines:
        return False
    decls = list(_MOD_DECL.finditer(text))
    if decls:
        at = decls[-1].end() + 1
        text = text[:at] + "\n".join(lines) + "\n" + text[at:]
    else:
        body = text.split("\n")
        i = 0
        while i < len(body) and body[i].startswith("//!"):
            i += 1
        while i < len(body) and not body[i].strip():
            i += 1
        text = "\n".join(body[:i] + lines + [""] + body[i:])
    write(path, text)
    return True


_INCLUDE = re.compile(r'(include_(?:str|bytes)!\(\s*")([^"]+)(")')
_PUB_IN = re.compile(r"(pub\s*\(\s*in\s+crate::)(\w+)")


def fix_includes(plan: Plan, fmap: Dict[str, str]) -> List[str]:
    """Relative include_str!/include_bytes! paths in moved files, pointed at the same file again."""
    changed = []
    for old, new in fmap.items():
        if not new.endswith(".rs"):
            continue
        path = os.path.join(plan.src, new)
        text = read(path)

        def repl(m: "re.Match[str]") -> str:
            rel = m.group(2)
            target = os.path.normpath(os.path.join(os.path.dirname(old), rel)).replace(os.sep, "/")
            target = fmap.get(target, target)
            return m.group(1) + os.path.relpath(target, os.path.dirname(new)).replace(os.sep, "/") + m.group(3)

        out = _INCLUDE.sub(repl, text)
        out = _PUB_IN.sub(lambda m: m.group(1) + plan.by_old[m.group(2)]["new"]
                          if m.group(2) in plan.by_old else m.group(0), out)
        if out != text:
            write(path, out)
            changed.append(new)
    return changed


def move(plan: Plan) -> int:
    lib_path = os.path.join(plan.src, "lib.rs")
    old_lib = read(lib_path)
    if DOMAINS_MARK in old_lib:
        print("core-domains: already moved, nothing to do")
        return 0
    for line in old_lib.splitlines():
        if line.strip() and not line.startswith("//!") and not _MOD_DECL.match(line):
            raise SystemExit("lib.rs has more than its doc and module declarations: %r" % line)
    declared = {m.group(1) for m in _MOD_DECL.finditer(old_lib)}
    if declared != set(plan.by_old):
        raise SystemExit("lib.rs declares %s, the mapping lists %s" % (
            sorted(declared - set(plan.by_old)), sorted(set(plan.by_old) - declared)))
    for m in plan.modules:
        if plan.new_root_file(m) != m["path"]:
            raise SystemExit("%s: the mapping says %s, the layout gives %s" % (m["old"], m["path"], plan.new_root_file(m)))
    fmap = plan.file_map()

    # The moves, shallowest first, so a domain's folder exists before its modules go into it.
    for m in sorted(plan.modules, key=lambda m: (m["new"].count("::"), m["new"])):
        f, d = plan.old_layout(m)
        new_root = m["path"]
        new_dir = new_root[:-len("/mod.rs")] if new_root.endswith("/mod.rs") else None
        if d and new_dir and d != new_dir:
            os.makedirs(os.path.dirname(os.path.join(plan.src, new_dir)) or plan.src, exist_ok=True)
            git(plan.root, "mv", os.path.join(plan.src, d), os.path.join(plan.src, new_dir))
        if f:
            os.makedirs(os.path.dirname(os.path.join(plan.src, new_root)), exist_ok=True)
            git(plan.root, "mv", os.path.join(plan.src, f), os.path.join(plan.src, new_root))
    for entry in plan.data:
        old, new = entry["old"].rstrip("/"), entry["new"].rstrip("/")
        if os.path.isdir(os.path.join(plan.src, old)):
            os.makedirs(os.path.dirname(os.path.join(plan.src, new)), exist_ok=True)
            git(plan.root, "mv", os.path.join(plan.src, old), os.path.join(plan.src, new))

    # The new domains' mod.rs, and every parent's new children.
    children: Dict[str, List[Tuple[str, str]]] = {}
    for m in plan.modules:
        parts = m["new"].split("::")
        if len(parts) > 1:
            children.setdefault("::".join(parts[:-1]), []).append((plan.vis(m, True), parts[-1]))
    for d in plan.domains:
        if not d["new"]:
            continue
        path = os.path.join(plan.src, d["name"], "mod.rs")
        doc = ["//! " + line if line else "//!" for line in d["doc"]]
        decls = ["%smod %s;" % (vis, name) for vis, name in sorted(children.pop(d["name"]), key=lambda c: c[1])]
        write(path, "\n".join(doc + [""] + decls) + "\n")
        git(plan.root, "add", path)
    for parent, kids in sorted(children.items()):
        root_file = next((m["path"] for m in plan.modules if m["new"] == parent), None)
        if root_file is None:
            raise SystemExit("no module %s to declare %s in" % (parent, kids))
        declare_children(os.path.join(plan.src, root_file), kids)
    write(lib_path, lib_text(plan, old_lib, shims=True))
    changed = fix_includes(plan, fmap)
    print("core-domains: moved %d modules (%d files) into %d domains; paths recomputed in %s" % (
        len(plan.modules), len(fmap), len(plan.domains), ", ".join(changed) or "no file"))
    restructure(plan, fmap)
    return 0


# ---------------------------------------------------------------------------------------------------
# scripts/structure.json: layers per domain, module depth, allowlist keys with the files.

def restructure(plan: Plan, fmap: Dict[str, str]) -> None:
    path = os.path.join(plan.root, "scripts", "structure.json")
    data = json.loads(read(path))
    crate = os.path.basename(plan.crate_dir)
    tiers: Dict[int, List[str]] = {}
    for d in plan.domains:
        tiers.setdefault(d["tier"], []).append(d["name"])
    data["config"]["layers"][crate] = [tiers[t][0] if len(tiers[t]) == 1 else sorted(tiers[t]) for t in sorted(tiers)]
    data["config"].setdefault("module_depth", {})[crate] = 2
    prefix = plan.crate_dir + "/src/"
    renamed = 0
    for rule, entries in data["allowlist"].items():
        out = {}
        for key, n in entries.items():
            head, sep, rest = key.partition(" :: ")
            if head.startswith(prefix) and head[len(prefix):] in fmap:
                head = prefix + fmap[head[len(prefix):]]
                renamed += 1
            out[head + sep + rest] = n
        data["allowlist"][rule] = dict(sorted(out.items()))
    write(path, json.dumps(data, indent=2, ensure_ascii=False) + "\n")

    # A re-export's key names the module it reaches into, now one level deeper (`crate::ipc::control`):
    # the same file with the same count under its new key is the same entry.
    sys.path.insert(0, os.path.join(plan.root, "scripts"))
    import structure  # noqa: E402
    cfg, allow = structure.load_config(path)
    an = structure.Analysis(plan.root, cfg)
    for rule in ("cross-reexports",):
        found = {k: it.count for k, it in an.found[rule].items()}
        for key in sorted(set(found) - set(allow[rule])):
            file = key.split(" :: ")[0]
            gone = [k for k in allow[rule] if k.split(" :: ")[0] == file and k not in found and allow[rule][k] == found[key]]
            if len(gone) == 1:
                print("core-domains: %s: %s is now %s" % (rule, gone[0], key))
                allow[rule][key] = allow[rule].pop(gone[0])
    data["allowlist"] = {r: dict(sorted(allow[r].items())) for r in structure.RULES}
    write(path, json.dumps(data, indent=2, ensure_ascii=False) + "\n")
    print("core-domains: structure.json: layers per domain, module depth 2, %d allowlist keys moved with their files"
          % renamed)


# ---------------------------------------------------------------------------------------------------
# --rewrite

def path_rewriter(plan: Plan, head: str):
    """A function rewriting `<head>::old` (and `<head>::{old::x, ...}` groups) to the new paths."""
    moved = {m["old"]: m["new"] for m in plan.modules if m["old"] != m["new"]}
    names = "|".join(sorted(moved, key=len, reverse=True))
    single = re.compile(r"(?<![\w$:])(%s\s*::\s*)(%s)\b" % (re.escape(head), names))
    group = re.compile(r"(?<![\w$:])%s\s*::\s*\{" % re.escape(head))

    def rewrite(text: str) -> str:
        text = single.sub(lambda m: m.group(1) + moved[m.group(2)], text)
        out, pos = [], 0
        for m in group.finditer(text):
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
            body = text[open_at + 1:i]
            pieces, cur, d = [], [], 0
            for ch in body:
                if ch in "{(":
                    d += 1
                elif ch in "})":
                    d -= 1
                if ch == "," and d == 0:
                    pieces.append("".join(cur))
                    cur = []
                else:
                    cur.append(ch)
            pieces.append("".join(cur))
            new_pieces = [re.sub(r"^(\s*)(%s)\b" % names, lambda p: p.group(1) + moved[p.group(2)], p) for p in pieces]
            out.append(text[pos:open_at + 1] + ",".join(new_pieces))
            pos = i
        out.append(text[pos:])
        return "".join(out)

    return rewrite


def rebind(plan: Plan, text: str) -> str:
    """A renamed module a file imports by itself is called by its new name in that file."""
    for m in plan.modules:
        old_leaf, new_leaf = m["old"], m["new"].split("::")[-1]
        if old_leaf == new_leaf:
            continue
        new = re.escape(m["new"]).replace("::", r"\s*::\s*")
        binds = re.search(r"\b%s\s*(?:;|,|\}|::\s*\{[^}]*\bself\b)" % new, text)
        if binds:
            text = re.sub(r"(?<![\w:])%s(\s*::)" % re.escape(old_leaf), new_leaf + r"\1", text)
        # Prose that names the module: `theme_registry` -> `theme::registry`.
        text = re.sub(r"`%s`" % re.escape(old_leaf), "`%s`" % m["new"], text) if old_leaf != "text" else text
    return text


def tracked(root: str, *patterns: str) -> List[str]:
    out = subprocess.run(["git", "-C", root, "ls-files", "--"] + list(patterns), check=True,
                         capture_output=True, text=True).stdout.split("\n")
    return [p for p in out if p]


def rewrite(plan: Plan) -> int:
    lib_path = os.path.join(plan.src, "lib.rs")
    lib = read(lib_path)
    if DOMAINS_MARK not in lib:
        raise SystemExit("run --move first")
    in_core = path_rewriter(plan, "crate")
    elsewhere = path_rewriter(plan, "wormhole_core")
    core_prefix = plan.crate_dir + "/"
    changed = 0
    for rel in tracked(plan.root, "*.rs"):
        if rel == plan.crate_dir + "/src/lib.rs":
            continue
        path = os.path.join(plan.root, rel)
        text = read(path)
        out = elsewhere(text)
        if rel.startswith(core_prefix):
            out = in_core(out)
        out = rebind(plan, out)
        if out != text:
            write(path, out)
            changed += 1
    # The shims go, and a module private before the move is private again.
    write(lib_path, lib_text(plan, lib, shims=False))
    for m in plan.modules:
        if "vis_during_move" in m:
            parts = m["new"].split("::")
            parent = next(x["path"] for x in plan.modules if x["new"] == "::".join(parts[:-1]))
            p = os.path.join(plan.src, parent)
            text = read(p)
            text = re.sub(r"(?m)^%smod %s;$" % (re.escape(plan.vis(m, True)), parts[-1]),
                          "%smod %s;" % (plan.vis(m, False), parts[-1]), text)
            write(p, text)
    # File paths and module paths in the docs and scripts that name them.
    file_moves = file_moves_after(plan)
    # Every other tracked text file (docs, skills, scripts, shaders' comments), but the changelog, which
    # tells what was, and this step's own files.
    keep = {MAPPING, "scripts/core-domains.py", "scripts/structure.json", "CHANGELOG.md"}
    for rel in tracked(plan.root):
        if rel.endswith(".rs") or rel in keep or not os.path.isfile(os.path.join(plan.root, rel)):
            continue
        path = os.path.join(plan.root, rel)
        try:
            text = read(path)
        except (UnicodeDecodeError, ValueError):
            continue
        out = elsewhere(text)
        for old, new in file_moves:
            out = re.sub(r"%s(?=[/\s`'\")\],:.]|$)" % re.escape(old), new, out)
        if out != text:
            write(path, out)
            changed += 1
    print("core-domains: rewrote paths in %d files; shims removed. Now: cargo fmt --all" % changed)
    return 0


def file_moves_after(plan: Plan) -> List[Tuple[str, str]]:
    """`crates/wormhole-core/src/<old>` -> the new place, for a module's file or folder, longest first."""
    out = []
    for m in plan.modules:
        if m["old"] == m["new"]:
            continue
        new_root = m["path"]
        new_dir = new_root[:-len("/mod.rs")] if new_root.endswith("/mod.rs") else new_root[:-3]
        base = plan.crate_dir + "/src/"
        out.append((base + m["old"] + ".rs", base + new_root))
        out.append((base + m["old"], base + new_dir))
    for entry in plan.data:
        base = plan.crate_dir + "/src/"
        out.append((base + entry["old"].rstrip("/"), base + entry["new"].rstrip("/")))
    return sorted(out, key=lambda p: -len(p[0]))


def main(argv: Optional[List[str]] = None) -> int:
    parser = argparse.ArgumentParser(description="Group core's modules by domain",
                                     formatter_class=argparse.RawDescriptionHelpFormatter, epilog=__doc__)
    parser.add_argument("--root", default=REPO, help="the repository (default: this one)")
    step = parser.add_mutually_exclusive_group(required=True)
    step.add_argument("--move", action="store_true", help="commit A: the moves and the shims")
    step.add_argument("--rewrite", action="store_true", help="commit B: every path by its domain, no shims")
    args = parser.parse_args(argv)
    plan = Plan(os.path.abspath(args.root))
    return move(plan) if args.move else rewrite(plan)


if __name__ == "__main__":
    sys.exit(main())
