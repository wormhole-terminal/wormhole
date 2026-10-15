#!/usr/bin/env python3
"""Compare a headless run of an outside suite against the committed expectation, test by test.

    gate.py esctest <result.jsonl> [--update]     esctest_run.py's JSON lines (esctest-host --out)
    gate.py ucs <result.yaml> [--update]          ucs-detect's --save-yaml file (ucs-headless.py --out)

The expectations are in tools/vt-compare/expected/. The rule is the gate's: every test that passes
there must still pass. Exit status 1 names the tests that regressed. A test that passed nothing
before and passes now is reported, and the expectation is left for a person to update
(`scripts/suites-gate.sh --update`, then commit the file): the gate never rewrites it by itself.
The ucs reader needs PyYAML, which only the suites' venv has (tools/vt-compare/fetch-suites.sh).
"""
import json
import os
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
EXPECTED = os.path.join(HERE, "expected")
HOW = "scripts/suites-gate.sh --update"


def read_expectation(path):
    rows = {}
    with open(path, encoding="utf-8") as f:
        for line in f:
            line = line.rstrip("\n")
            if line and not line.startswith("#"):
                key, _, value = line.partition("\t")
                rows[key] = value
    return rows


def write_expectation(path, header, rows):
    with open(path, "w", encoding="utf-8") as f:
        f.write(header)
        for key in sorted(rows):
            f.write(f"{key}\t{rows[key]}\n")


def esctest(result, update):
    got = {}
    for line in open(result, encoding="utf-8"):
        rec = json.loads(line)
        if rec["type"] == "test":
            got[rec["name"]] = rec["status"]
    path = os.path.join(EXPECTED, "esctest.tsv")
    if update:
        header = ("# esctest2 against Wormhole's engine with no window (esctest-host): test, status.\n"
                  "# Every `pass` must keep passing (scripts/suites-gate.sh); a `fail` is a known gap\n"
                  "# (the Xcms colour forms of OSC 4/5/10..19, DA's printer port).\n")
        write_expectation(path, header, {n: "pass" if s == "pass" else "fail" for n, s in got.items()})
        return report_counts(got, "written to " + path)
    want = read_expectation(path)
    regressed = sorted(n for n, s in want.items() if s == "pass" and got.get(n) != "pass")
    new_fail = sorted(n for n in got if n not in want and got[n] != "pass")
    fixed = sorted(n for n, s in want.items() if s == "fail" and got.get(n) == "pass")
    new_pass = sorted(n for n in got if n not in want and got[n] == "pass")
    passed = sum(1 for s in got.values() if s == "pass")
    print(f"esctest2: {passed}/{len(got)} pass (expected {sum(1 for s in want.values() if s == 'pass')}/{len(want)})")
    if fixed or new_pass:
        print(f"esctest2: {len(fixed)} known failure(s) pass now and {len(new_pass)} test(s) are new and pass; "
              f"update the expectation ({HOW}) and commit it:")
        for n in fixed + new_pass:
            print(f"    + {n}")
    if regressed or new_fail:
        print(f"esctest2: REGRESSED, {len(regressed)} test(s) passed before and do not now:")
        for n in regressed:
            print(f"    - {n}: {got.get(n, 'not run')}")
        for n in new_fail:
            print(f"    - {n}: new and failing (is suites.pin newer than the expectation? {HOW})")
        return 1
    return 0


def report_counts(got, tail):
    passed = sum(1 for s in got.values() if s == "pass")
    print(f"{passed}/{len(got)} pass, {tail}")
    return 0


def ucs_items(doc):
    """(category, item) -> (errors, total, failed names) for every row ucs-detect measured."""
    out = {}
    for category, items in sorted((doc.get("test_results") or {}).items()):
        for item, v in sorted((items or {}).items()):
            if isinstance(v, dict):
                failed = v.get("failed_codepoints") or v.get("failed") or []
                out[f"{category}/{item}"] = (int(v.get("n_errors", 0)), int(v.get("n_total", 0)), failed)
    return out


def ucs(result, update):
    import yaml

    with open(result, encoding="utf-8") as f:
        got = ucs_items(yaml.safe_load(f))
    path = os.path.join(EXPECTED, "ucs.tsv")
    if update:
        header = ("# ucs-detect against Wormhole's engine with no window (ucs-headless.py): category/row,\n"
                  "# errors and the number measured. A row may not get more errors (scripts/suites-gate.sh).\n")
        write_expectation(path, header, {k: f"{e}\t{t}" for k, (e, t, _) in got.items()})
        errors = sum(e for e, _, _ in got.values())
        print(f"{len(got)} rows, {errors} errors, written to {path}")
        return 0
    want = {k: tuple(int(x) for x in v.split("\t")) for k, v in read_expectation(path).items()}
    regressed, fixed = [], []
    for k, (errors, total, failed) in got.items():
        before = want.get(k, (0, 0))[0]
        if errors > before:
            regressed.append((k, before, errors, total, failed))
        elif errors < before:
            fixed.append((k, before, errors))
    missing = sorted(k for k in want if k not in got)
    errors = sum(e for e, _, _ in got.values())
    total = sum(t for _, t, _ in got.values())
    print(f"ucs-detect: {total - errors}/{total} sized as wcwidth says in {len(got)} rows "
          f"(expected {sum(t - e for e, t in want.values())}/{sum(t for _, t in want.values())} in {len(want)})")
    if fixed:
        print(f"ucs-detect: {len(fixed)} row(s) have fewer errors than expected; update the expectation ({HOW}):")
        for k, before, now in fixed:
            print(f"    + {k}: {before} -> {now} errors")
    if regressed or missing:
        print(f"ucs-detect: REGRESSED, {len(regressed)} row(s) have more errors:")
        for k, before, now, total, failed in regressed:
            shown = " ".join(str(c) for c in failed[:8])
            print(f"    - {k}: {now} of {total} wrong (was {before}){': ' + shown if shown else ''}")
        for k in missing:
            print(f"    - {k}: not measured (was in the expectation)")
        return 1
    return 0


def main():
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    if len(args) != 2 or args[0] not in ("esctest", "ucs"):
        raise SystemExit(__doc__)
    update = "--update" in sys.argv
    sys.exit((esctest if args[0] == "esctest" else ucs)(args[1], update))


main()
