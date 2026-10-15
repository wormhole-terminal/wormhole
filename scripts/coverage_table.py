#!/usr/bin/env python3
"""Per-module line and branch coverage of wormhole-vt from cargo-llvm-cov's JSON export.

    coverage_table.py [--files N] tests.json [tests-and-suites.json]

With two files the second column group is what running the outside suites headless adds.
"""
import json
import os
import sys


def module_of(path):
    """parser, unicode, types, grid, grid/page, grid/images, term, term/graphics, term/charset."""
    parts = path.split("/crates/wormhole-vt/src/", 1)[-1].split("/")
    if len(parts) == 1:
        return "(lib)"
    if len(parts) >= 3 and parts[0] in ("term", "grid"):
        return f"{parts[0]}/{parts[1]}"
    return parts[0]


def load(path):
    files = {}
    for f in json.load(open(path))["data"][0]["files"]:
        s = f["summary"]
        files[f["filename"]] = {k: (s[k]["covered"], s[k]["count"]) for k in ("lines", "branches")}
    return files


def by_module(files):
    mods = {}
    for name, s in files.items():
        m = mods.setdefault(module_of(name), {"lines": [0, 0], "branches": [0, 0]})
        for k in m:
            m[k][0] += s[k][0]
            m[k][1] += s[k][1]
    return mods


def pct(pair):
    covered, count = pair
    return f"{100.0 * covered / count:5.1f}%" if count else "    - "


def main():
    args = sys.argv[1:]
    top = 12
    if "--files" in args:
        i = args.index("--files")
        top = int(args[i + 1])
        del args[i:i + 2]
    runs = [load(p) for p in args]
    mods = [by_module(r) for r in runs]
    names = sorted(mods[0])
    head = f"{'module':<16} {'lines':>7} {'(n)':>7} {'branches':>9} {'(n)':>6}"
    if len(runs) > 1:
        head += f" | {'+suites lines':>13} {'branches':>9}"
    print(head)
    total = [{"lines": [0, 0], "branches": [0, 0]} for _ in runs]
    for name in names:
        row = f"{name:<16} {pct(mods[0][name]['lines']):>7} {mods[0][name]['lines'][1]:>7} {pct(mods[0][name]['branches']):>9} {mods[0][name]['branches'][1]:>6}"
        if len(runs) > 1:
            row += f" | {pct(mods[1][name]['lines']):>13} {pct(mods[1][name]['branches']):>9}"
        print(row)
        for i, m in enumerate(mods):
            for k in ("lines", "branches"):
                total[i][k][0] += m[name][k][0]
                total[i][k][1] += m[name][k][1]
    row = f"{'all':<16} {pct(total[0]['lines']):>7} {total[0]['lines'][1]:>7} {pct(total[0]['branches']):>9} {total[0]['branches'][1]:>6}"
    if len(runs) > 1:
        row += f" | {pct(total[1]['lines']):>13} {pct(total[1]['branches']):>9}"
    print(row)
    print(f"\nthe {top} files with the most lines not run (tests only):")
    rows = sorted(runs[0].items(), key=lambda kv: kv[1]["lines"][0] - kv[1]["lines"][1])[:top]
    for name, s in rows:
        short = name.split("/crates/wormhole-vt/src/", 1)[-1]
        print(f"  {short:<34} {s['lines'][1] - s['lines'][0]:>4} of {s['lines'][1]:>4} lines not run, branches {pct(s['branches'])}")


main()
