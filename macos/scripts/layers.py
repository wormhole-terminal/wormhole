#!/usr/bin/env python3
"""Layer checker for the Swift package: a module may only use modules of a lower layer it is allowed to.

    python3 -I macos/scripts/layers.py            # check Package.swift's dependencies and every `import`
    python3 -I macos/scripts/layers.py --selftest

The table below is the rule (swift/STANDARDS.md, "Layers"). Each target has a kind and a rank:

  found   WorkspaceModel, WordsModel, AppLogging, TerminalCore, DesignSystem: use no module of the package;
  model   the `...Model` modules: plain values and pure logic; use `found`;
  ffi     the generated bindings and UniFFISource: the Rust seam; use `found`;
  ui      feature UI and TerminalMetal: use `found`, `model` and a `ui` of a lower rank;
  bridge  the `...Bridge` modules, built only with the FFI linked: use `found`, `model`, `ffi` and a `bridge` of
          a lower rank, never a `ui`;
  app     the executable: uses everything.

An edge is allowed when its target's kind is in the user's `USES` set and, within one kind, its rank is lower
(an edge to the same rank is a sideways edge and fails). A target missing from the table fails, so a new
module is placed on purpose. Edges come from Package.swift's `dependencies` and from the `import` lines of the
target's sources (test targets are not checked: they may reach across layers). Exit status 1 on a violation.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

SWIFT = Path(__file__).resolve().parent.parent / "swift"

# name: (kind, rank)
TABLE: dict[str, tuple[str, int]] = {
    "WorkspaceModel": ("found", 0), "WordsModel": ("found", 0), "TerminalCore": ("found", 0),
    "DesignSystem": ("found", 0), "AppLogging": ("found", 0),
    "WindowsModel": ("model", 1), "PanelModel": ("model", 1), "PaletteModel": ("model", 1),
    "PanesModel": ("model", 1), "SettingsModel": ("model", 1), "PlatformModel": ("model", 1),
    "AuxModel": ("model", 1), "StudioModel": ("model", 1), "FontPickerModel": ("model", 0),
    "WormholeFFI": ("ffi", 0), "WormholeBindings": ("ffi", 1), "UniFFISource": ("ffi", 2),
    # Leaves other UI modules draw: the group colours (SidebarUI) and the font picker (FontPickerUI) use no other UI module.
    "SidebarUI": ("ui", 0), "FontPickerUI": ("ui", 0), "PanelUI": ("ui", 1), "PaletteUI": ("ui", 1), "PanesUI": ("ui", 1),
    "SettingsUI": ("ui", 1), "PlatformUI": ("ui", 1), "AuxWindows": ("ui", 1), "TerminalUI": ("ui", 1),
    "TerminalMetal": ("ui", 1), "FileIconTheme": ("ui", 1),
    "StudioUI": ("ui", 2), "WindowUI": ("ui", 3),
    "WorkspaceBridge": ("bridge", 1), "FilesBridge": ("bridge", 1), "GitBridge": ("bridge", 1),
    "SettingsBridge": ("bridge", 1), "StudioBridge": ("bridge", 1), "AuxBridge": ("bridge", 1), "HookBridge": ("bridge", 1),
    "PlatformBridge": ("bridge", 1), "WordsBridge": ("bridge", 1), "WindowsBridge": ("bridge", 2),
    "Wormhole": ("app", 9),
}
USES = {
    "found": set(),
    "model": {"found", "model"},
    "ffi": {"found", "ffi"},
    "ui": {"found", "model", "ui"},
    "bridge": {"found", "model", "ffi", "bridge"},
    "app": {"found", "model", "ffi", "ui", "bridge"},
}
IMPORT = re.compile(r"^\s*(?:@\w+(?:\([^)]*\))?\s+)*(?:public\s+|internal\s+|package\s+)?import\s+(\w+)", re.M)


def violation(src: str, dst: str) -> str | None:
    """Why `src` may not use `dst`, or None."""
    if src not in TABLE or dst not in TABLE:
        return None  # an unplaced module is reported once, by `unplaced`
    (sk, sr), (dk, dr) = TABLE[src], TABLE[dst]
    if dk not in USES[sk]:
        return f"{sk} {src} must not use a {dk} ({dst})"
    if sk == dk and sk != "app" and dr >= sr:
        return f"{src} (rank {sr}) uses {dst} (rank {dr}) of the same kind: sideways or upward"
    return None


def balanced(text: str, start: int) -> str:
    """The text inside the parenthesis that opens at `start`."""
    depth = 0
    for i in range(start, len(text)):
        depth += {"(": 1, ")": -1}.get(text[i], 0)
        if depth == 0:
            return text[start + 1 : i]
    return text[start + 1 :]


def parse_package(text: str) -> dict[str, list[str]]:
    """Non-test targets and the names in their `dependencies:`, read as text (Package.swift is code)."""
    uses: dict[str, list[str]] = {}
    app_deps = [n for body in re.findall(r"appDependencies[^=\n]*[+]?=\s*\[(.*?)\]", text, re.S)
                for n in re.findall(r'"(\w+)"', body)]
    for match in re.finditer(r"\.(target|executableTarget|binaryTarget)\(", text):
        body = balanced(text, match.end() - 1)
        name = re.search(r'name:\s*"(\w+)"', body)
        if not name:
            continue
        deps = re.search(r"dependencies:\s*\[(.*?)\]", body, re.S)
        names = re.findall(r'"(\w+)"', deps.group(1)) if deps else []
        if "dependencies: appDependencies" in body:
            names = app_deps
        uses[name.group(1)] = names
    return uses


def imports(target: str) -> dict[str, str]:
    """module -> first `file:line` that imports it, over the target's sources."""
    found: dict[str, str] = {}
    for path in sorted((SWIFT / "Sources" / target).rglob("*.swift")):
        source = path.read_text(encoding="utf-8")
        for m in IMPORT.finditer(source):
            line = source.count("\n", 0, m.start(1)) + 1
            found.setdefault(m.group(1), f"{path.relative_to(SWIFT)}:{line}")
    return found


def check(uses: dict[str, list[str]], imported: dict[str, dict[str, str]]) -> list[str]:
    problems = [f"unplaced: target {t} is not in layers.py's TABLE" for t in uses if t not in TABLE]
    for src in sorted(uses):
        for dst in sorted(set(uses[src])):
            if (why := violation(src, dst)):
                problems.append(f"{why}  [Package.swift]")
        for dst, where in sorted(imported.get(src, {}).items()):
            if (why := violation(src, dst)):
                problems.append(f"{why}  [import at {where}]")
    return problems


def selftest() -> int:
    cases = [
        ("WorkspaceModel", "DesignSystem", True), ("PanelModel", "WorkspaceModel", False),
        ("PanelModel", "PaletteModel", True), ("SidebarUI", "PanelModel", False),
        ("SidebarUI", "PanelUI", True), ("WindowUI", "StudioUI", False), ("StudioUI", "WindowUI", True),
        ("PanelUI", "UniFFISource", True), ("FilesBridge", "PanelUI", True),
        ("WindowsBridge", "WorkspaceBridge", False), ("WorkspaceBridge", "WindowsBridge", True),
        ("Wormhole", "WindowUI", False), ("WindowUI", "Wormhole", True),
    ]
    bad = [c for c in cases if (violation(c[0], c[1]) is not None) != c[2]]
    package = 'let a = 1\n.target(name: "PanelModel", dependencies: ["PaletteModel"]),\n.target(name: "Extra")'
    assert parse_package(package) == {"PanelModel": ["PaletteModel"], "Extra": []}
    assert check(parse_package(package), {}) == [
        "unplaced: target Extra is not in layers.py's TABLE",
        "model PanelModel must not use a model (PaletteModel)  [Package.swift]",
    ]
    for c in bad:
        print(f"selftest: {c[0]} -> {c[1]} expected {'violation' if c[2] else 'allowed'}")
    print("selftest: ok" if not bad else f"selftest: {len(bad)} failed")
    return 1 if bad else 0


def main() -> int:
    if "--selftest" in sys.argv:
        return selftest()
    uses = parse_package((SWIFT / "Package.swift").read_text(encoding="utf-8"))
    imported = {t: imports(t) for t in uses}
    problems = check(uses, imported)
    edges = sum(len(set(v) | set(imported[k])) for k, v in uses.items())
    for p in problems:
        print(p)
    print(f"layers: {len(uses)} targets, {edges} edges, {len(problems)} violation(s)")
    return 1 if problems else 0


if __name__ == "__main__":
    sys.exit(main())
