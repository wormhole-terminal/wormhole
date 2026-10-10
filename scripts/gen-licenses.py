#!/usr/bin/env python3
"""Regenerate crates/wormhole-core/assets/licenses.json, the third-party license
list that Wormhole's Licenses window shows.

  scripts/gen-licenses.py           write the file
  scripts/gen-licenses.py --check   exit 1 if the file is out of date (writes nothing)

Run it after `cargo update` or after adding, removing or upgrading a dependency.
It lists the crates the app is built from on macOS (normal dependencies of the
workspace crates, for both architectures we ship; not dev- or build-only ones),
with each crate's declared license and the text of the license files in its
package. Crates whose package ships no license file get an empty text list, and the
window says so. Identical texts are stored once.

Needs only Python 3 and cargo; dependencies must be downloaded (`cargo fetch`).
"""

import json
import os
import subprocess
import sys

ROOT = os.path.normpath(os.path.join(os.path.dirname(os.path.abspath(__file__)), ".."))
OUT = os.path.join(ROOT, "crates", "wormhole-core", "assets", "licenses.json")
TARGETS = ["aarch64-apple-darwin", "x86_64-apple-darwin"]
LICENSE_PREFIXES = ("LICENSE", "LICENCE", "COPYING", "UNLICENSE", "NOTICE")


def metadata(target):
    result = subprocess.run(
        ["cargo", "metadata", "--format-version", "1", "--locked", "--filter-platform", target],
        cwd=ROOT,
        capture_output=True,
        text=True,
    )
    if result.returncode != 0:
        sys.exit(f"cargo metadata failed for {target}:\n{result.stderr}")
    return json.loads(result.stdout)


def shipped(meta):
    """Ids of the packages reachable from the workspace crates through normal dependencies."""
    nodes = {node["id"]: node for node in meta["resolve"]["nodes"]}
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
    return seen - set(meta["workspace_members"])


def read_text(path):
    with open(path, "rb") as handle:
        text = handle.read().decode("utf-8", errors="replace")
    lines = [line.rstrip() for line in text.replace("\r\n", "\n").replace("\r", "\n").split("\n")]
    return "\n".join(lines).strip("\n")


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


def build():
    packages = {}
    ids = set()
    for target in TARGETS:
        meta = metadata(target)
        for package in meta["packages"]:
            packages[package["id"]] = package
        ids |= shipped(meta)

    entries = sorted(
        (packages[i] for i in ids), key=lambda p: (p["name"].lower(), p["version"], p["id"])
    )
    texts = []
    index = {}
    crates = []
    for package in entries:
        used = []
        for path in license_files(package):
            text = read_text(path)
            if not text:
                continue
            if text not in index:
                index[text] = len(texts)
                texts.append(text)
            if index[text] not in used:
                used.append(index[text])
        crates.append(
            {
                "name": package["name"],
                "version": package["version"],
                "license": package.get("license"),
                "repository": package.get("repository") or package.get("homepage"),
                "texts": used,
            }
        )

    # One line per text and per crate, so a regeneration diffs readably.
    lines = ['{"texts": [']
    lines.append(",\n".join(json.dumps(text, ensure_ascii=False) for text in texts))
    lines.append('], "crates": [')
    lines.append(",\n".join(json.dumps(crate, ensure_ascii=False) for crate in crates))
    lines.append("]}")
    return "\n".join(lines) + "\n", len(crates), len(texts)


def main():
    if len(sys.argv) > 2 or (len(sys.argv) == 2 and sys.argv[1] != "--check"):
        sys.exit(__doc__)
    content, crates, texts = build()
    if len(sys.argv) == 2:
        current = open(OUT, encoding="utf-8").read() if os.path.exists(OUT) else ""
        if current != content:
            sys.exit("licenses.json is out of date: run scripts/gen-licenses.py")
        print(f"licenses.json is up to date ({crates} crates)")
        return
    with open(OUT, "w", encoding="utf-8") as handle:
        handle.write(content)
    print(f"wrote {os.path.relpath(OUT, ROOT)}: {crates} crates, {texts} distinct texts, {len(content) // 1024} KiB")


if __name__ == "__main__":
    main()
