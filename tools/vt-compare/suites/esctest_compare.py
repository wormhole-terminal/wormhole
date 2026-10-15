#!/usr/bin/env python3
"""Compare two esctest_run.py result files test by test (default: the headless run against the
in-app run of the newest results folder), or summarise one.

usage: esctest_compare.py headless.jsonl [other.jsonl] [--fails]
"""
import collections
import glob
import json
import sys


def load(path):
    tests = {}
    for line in open(path, encoding="utf-8"):
        rec = json.loads(line)
        if rec["type"] == "test":
            tests[rec["name"]] = rec
    return tests


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if not args:
        raise SystemExit(__doc__)
    mine = load(args[0])
    if len(args) > 1:
        other_path = args[1]
    else:
        found = sorted(glob.glob("tools/vt-compare/results/*/suites/esctest/wormhole.jsonl"))
        other_path = found[-1] if found else None
    counts = collections.Counter(t["status"] for t in mine.values())
    print(f"{args[0]}: {dict(counts)} of {len(mine)}")
    if "--fails" in sys.argv:
        for name, t in sorted(mine.items()):
            if t["status"] != "pass":
                print(f"  {t['status']:5} {name}: {(t['reason'] or '')[:140]}")
    if other_path:
        other = load(other_path)
        print(f"{other_path}: {dict(collections.Counter(t['status'] for t in other.values()))}")
        better = worse = 0
        for name in sorted(set(mine) | set(other)):
            a, b = mine.get(name), other.get(name)
            if a is None and len(mine) < len(other):
                continue  # a subset run
            sa, sb = a and a["status"], b and b["status"]
            if sa == sb:
                continue
            if sa == "pass":
                better += 1
            else:
                worse += 1
                print(f"  differs: {name}: headless {sa} ({a and (a['reason'] or '')[:80]}), other {sb}")
        print(f"  against the other file: {better} better, {worse} different and not passing")


main()
