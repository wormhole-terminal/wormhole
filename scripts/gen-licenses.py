#!/usr/bin/env python3
"""Regenerate the third-party license list that Wormhole's Licenses window shows:
crates/wormhole-core/assets/licenses.json for the 1.x app, or with --ffi
crates/wormhole-ffi/assets/licenses.json for the Swift app (its Rust library is a
workspace of its own, with its own Cargo.lock).

  scripts/gen-licenses.py                 write the 1.x app's file
  scripts/gen-licenses.py --check         exit 1 if it is out of date (writes nothing)
  scripts/gen-licenses.py --ffi           write the Swift app's file
  scripts/gen-licenses.py --ffi --check   the same check for that one

Run it after `cargo update` or after adding, removing or upgrading a dependency.
It lists the crates the app is built from on macOS (normal dependencies of the
workspace crates, for both architectures we ship; not dev- or build-only ones, and not
this repository's own crates), with each crate's declared license and the text of the
license files in its package. Crates whose package ships no license file get an empty
text list, and the window says so; with --ffi, one that declares exactly MPL-2.0 or exactly
Apache-2.0 (licenses with a fixed text and nothing to fill in) gets that standard text from
`STANDARD_TEXTS` instead, and its entry says `"standard_text": true`. MIT, BSD and ISC need the
copyright holder's own line, so those stay empty. Identical texts are stored once. What is compiled in
without being a crate (the bundled font; for the Swift app also its icons and logos) is
listed from `ASSETS` and `FFI_ASSETS` below, marked with its `kind`.

Needs only Python 3 and cargo; dependencies must be downloaded (`cargo fetch`).
"""

import json
import os
import subprocess
import sys

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
OUT = os.path.join(ROOT, "crates", "wormhole-core", "assets", "licenses.json")
# The Swift app's Rust library: a workspace of its own, of which only `wormhole-ffi` ends up in the app
# (the other member is the bindings generator, a build tool).
FFI_DIR = os.path.join(ROOT, "crates", "wormhole-ffi")
FFI_OUT = os.path.join(FFI_DIR, "assets", "licenses.json")
FFI_ROOT_PACKAGE = "wormhole-ffi"
TARGETS = ["aarch64-apple-darwin", "x86_64-apple-darwin"]
LICENSE_PREFIXES = ("LICENSE", "LICENCE", "COPYING", "UNLICENSE", "NOTICE")
# The licenses whose text is the same for everyone (no year, no holder to fill in), by SPDX identifier, for a
# crate that declares exactly that one and ships no license file. Copied unmodified from crates' own license
# files (`assets/license-texts/README.md` says which, and how they were compared).
STANDARD_TEXTS = {
    "Apache-2.0": os.path.join(FFI_DIR, "assets", "license-texts", "Apache-2.0.txt"),
    "MPL-2.0": os.path.join(FFI_DIR, "assets", "license-texts", "MPL-2.0.txt"),
}

# Compiled into the app without being a crate: each with its license file, relative to ROOT.
# Update the version with the files (the folder's README says which release they are).
ASSETS = [
    {
        "name": "JetBrains Mono",
        "version": "2.304",
        "license": "OFL-1.1",
        "repository": "https://github.com/JetBrains/JetBrainsMono",
        "kind": "font",
        "files": ["crates/wormhole-core/assets/fonts/JetBrainsMono/OFL.txt"],
    },
]

# What the Swift app compiles in or bundles besides crates: the font above, the file icon theme (its
# SVGs and `icons/LICENSES.md` are the app's resources) and the agents' logos (their paths are
# `AgentLogoData.swift`, generated from `agent-icons`), and Lucide's icons in `LucideData.swift`.
# A license is a `(file, heading)` pair: the fenced block under that `## heading` of a markdown file,
# for licenses that live in a notice file beside the assets rather than in a file of their own.
# Versions are the ones the notice files name; where they name none the entry has none.
ICON_NOTICE = "macos/swift/Sources/FileIconTheme/Resources/icons/LICENSES.md"
LOGO_NOTICE = "crates/wormhole-core/assets/agent-icons/LICENSES.md"
FFI_ASSETS = [
    ASSETS[0],
    {
        "name": "Catppuccin VSCode Icons",
        "version": "",
        "license": "MIT",
        "repository": "https://github.com/catppuccin/vscode-icons",
        "kind": "icons",
        "sections": [(ICON_NOTICE, "## Catppuccin VSCode Icons — MIT")],
    },
    {
        "name": "Lucide",
        "version": "",
        "license": "ISC",
        "repository": "https://github.com/lucide-icons/lucide",
        "kind": "icons",
        "sections": [(ICON_NOTICE, "## Lucide — ISC")],
    },
    {
        "name": "Feather",
        "version": "",
        "license": "MIT",
        "repository": "https://github.com/feathericons/feather",
        "kind": "icons",
        "sections": [(ICON_NOTICE, "## Feather (chevron-right, chevron-down) — MIT")],
    },
    {
        # CC0 asks for no notice to travel, and the notice file carries none.
        "name": "Simple Icons",
        "version": "16.33.0, 16.36.0",
        "license": "CC0-1.0",
        "repository": "https://github.com/simple-icons/simple-icons",
        "kind": "icons",
    },
    {
        "name": "Lobe Icons",
        "version": "1.101.0",
        "license": "MIT",
        "repository": "https://github.com/lobehub/lobe-icons",
        "kind": "icons",
        "sections": [(LOGO_NOTICE, "## Lobe Icons files — MIT License")],
    },
    {
        "name": "Primer Octicons",
        "version": "",
        "license": "MIT",
        "repository": "https://github.com/primer/octicons",
        "kind": "icons",
        "sections": [(LOGO_NOTICE, "## `copilot.svg` — MIT License (Primer Octicons)")],
    },
]


def metadata(target, cwd):
    result = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked", "--filter-platform", target],
        cwd=cwd,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        sys.exit(f"cargo metadata failed for {target}:\n{result.stderr}")
    return json.loads(result.stdout)


def shipped(meta):
    """Ids of the packages reachable from the workspace crates through normal dependencies, without
    this repository's own crates: the ones with no registry or git source."""
    nodes = {node["id"]: node for node in meta["resolve"]["nodes"]}
    packages = {package["id"]: package for package in meta["packages"]}
    seen = set()
    stack = list(meta["workspace_members"])
    while stack:
        pkg = stack.pop()
        if pkg in seen:
            continue
        seen.add(pkg)
        for dep in nodes[pkg]["deps"]:
            if any(kind["kind"] is None for kind in dep["dep_kinds"]):
                stack.append(dep["pkg"])
    return {pkg for pkg in seen if packages[pkg]["source"] is not None}


def tidy(text):
    lines = [line.rstrip() for line in text.replace("\r\n", "\n").replace("\r", "\n").split("\n")]
    return "\n".join(lines).strip("\n")


def shipped_ffi(meta, target):
    """Ids of the packages `wormhole-ffi` is built from, as `cargo tree -p` resolves them. Not `shipped`'s
    walk of `cargo metadata`: metadata resolves features for the whole workspace, where the bindings
    generator turns on uniffi's tooling (askama, clap, goblin, ...), which the library is not built with."""
    result = subprocess.run(
        ["cargo", "tree", "--locked", "-p", FFI_ROOT_PACKAGE, "-e", "normal", "--target", target]
        + ["--prefix", "none", "--format", "{p}"],
        cwd=FFI_DIR,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        sys.exit(f"cargo tree failed for {target}:\n{result.stderr}")
    # Lines read `name vX.Y.Z (source)` with a `(*)` mark on repeats.
    built = {tuple(line.split()[:2]) for line in result.stdout.splitlines() if line.strip()}
    return {
        package["id"]
        for package in meta["packages"]
        if package["source"] is not None and (package["name"], "v" + package["version"]) in built
    }


def read_text(path):
    with open(path, "rb") as handle:
        return tidy(handle.read().decode("utf-8", errors="replace"))


def fenced_block(path, heading):
    """The text of the first fenced block under `heading` (a `## ...` line) of a markdown file."""
    lines = read_text(path).split("\n")
    if heading not in lines:
        sys.exit(f"{path} has no section {heading!r}")
    block = []
    fenced = False
    for line in lines[lines.index(heading) + 1 :]:
        if line.startswith("```"):
            if fenced:
                return tidy("\n".join(block))
            fenced = True
        elif fenced:
            block.append(line)
        elif line.startswith("## "):
            break
    sys.exit(f"{path}: no fenced block under {heading!r}")


def license_files(package):
    """Paths of the license files in a package's folder (and in a LICENSES folder)."""
    root = os.path.dirname(package["manifest_path"])
    found = []
    for name in sorted(os.listdir(root)):
        path = os.path.join(root, name)
        if not name.upper().startswith(LICENSE_PREFIXES):
            continue
        if os.path.isfile(path):
            found.append(path)
        elif os.path.isdir(path):
            found.extend(
                os.path.join(path, inner)
                for inner in sorted(os.listdir(path))
                if os.path.isfile(os.path.join(path, inner))
            )
    declared = package.get("license_file")
    if declared:
        path = os.path.normpath(os.path.join(root, declared))
        if os.path.isfile(path) and path not in found:
            found.append(path)
    return found


def build(ffi):
    packages = {}
    ids = set()
    for target in TARGETS:
        meta = metadata(target, FFI_DIR if ffi else ROOT)
        for package in meta["packages"]:
            packages[package["id"]] = package
        ids |= shipped_ffi(meta, target) if ffi else shipped(meta)

    entries = []
    for package in (packages[i] for i in ids):
        found = [read_text(path) for path in license_files(package)]
        standard = ffi and not found and package.get("license") in STANDARD_TEXTS
        if standard:
            found = [read_text(STANDARD_TEXTS[package["license"]])]
        entries.append(({**package, "standard_text": True} if standard else package, found))
    for asset in FFI_ASSETS if ffi else ASSETS:
        found = [read_text(os.path.join(ROOT, f)) for f in asset.get("files", [])]
        found += [fenced_block(os.path.join(ROOT, f), heading) for f, heading in asset.get("sections", [])]
        entries.append(({**asset, "id": asset["name"]}, found))
    entries.sort(key=lambda e: (e[0]["name"].lower(), e[0]["version"], e[0]["id"]))
    texts = []
    index = {}
    crates = []
    for package, found in entries:
        used = []
        for text in found:
            if not text:
                continue
            if text not in index:
                index[text] = len(texts)
                texts.append(text)
            if index[text] not in used:
                used.append(index[text])
        entry = {
            "name": package["name"],
            "version": package["version"],
            "license": package.get("license"),
            "repository": package.get("repository") or package.get("homepage"),
            "texts": used,
        }
        if "kind" in package:
            entry["kind"] = package["kind"]
        if package.get("standard_text"):
            entry["standard_text"] = True
        crates.append(entry)

    # One line per text and per crate, so a regeneration diffs readably.
    lines = ['{"texts": [']
    lines.append(",\n".join(json.dumps(text, ensure_ascii=False) for text in texts))
    lines.append('], "crates": [')
    lines.append(",\n".join(json.dumps(crate, ensure_ascii=False) for crate in crates))
    lines.append("]}")
    return "\n".join(lines) + "\n", len(crates), len(texts)


def main():
    args = sys.argv[1:]
    ffi = "--ffi" in args
    check = "--check" in args
    if len(args) != ffi + check:
        sys.exit(__doc__)
    out = FFI_OUT if ffi else OUT
    content, crates, texts = build(ffi)
    if check:
        current = open(out, encoding="utf-8").read() if os.path.exists(out) else ""
        if current != content:
            sys.exit(f"{os.path.relpath(out, ROOT)} is out of date: run scripts/gen-licenses.py{' --ffi' if ffi else ''}")
        print(f"{os.path.relpath(out, ROOT)} is up to date ({crates} crates)")
        return
    os.makedirs(os.path.dirname(out), exist_ok=True)
    with open(out, "w", encoding="utf-8") as handle:
        handle.write(content)
    print(f"wrote {os.path.relpath(out, ROOT)}: {crates} crates, {texts} distinct texts, {len(content) // 1024} KiB")


if __name__ == "__main__":
    main()
